use crate::persistence::{
    character::EntityId,
    models::{AbilitySets, Character, Item, SkillGroup},
};

use crate::persistence::{
    error::PersistenceError,
    json_models::{
        self, CharacterPosition, DatabaseAbilitySet, DatabaseItemProperties, GenericBody,
        HumanoidBody,
    },
};
use common::{
    character::CharacterId,
    comp::{
        inventory::{
            item::{tool::AbilityMap, Item as VelorenItem, MaterialStatManifest},
            loadout::{Loadout, LoadoutError},
            loadout_builder::LoadoutBuilder,
            slot::InvSlotId,
        },
        skillset, Body as CompBody, Waypoint, *,
    },
    resources::Time,
};
use core::{convert::TryFrom, num::NonZeroU64};
use hashbrown::HashMap;
use lazy_static::lazy_static;
use std::{collections::VecDeque, str::FromStr, sync::Arc};
use tracing::{trace, warn};

#[derive(Debug)]
pub struct ItemModelPair {
    pub comp: Arc<item::ItemId>,
    pub model: Item,
}

// Decoupled from the ECS resource because the plumbing is getting complicated;
// shouldn't matter unless someone's hot-reloading material stats on the live
// server
lazy_static! {
    pub static ref MATERIAL_STATS_MANIFEST: MaterialStatManifest =
        MaterialStatManifest::load().cloned();
    pub static ref ABILITY_MAP: AbilityMap = AbilityMap::load().cloned();
}

/// Returns a vector that contains all item rows to upsert; parent is
/// responsible for deleting items from the same owner that aren't affirmatively
/// kept by this.
///
/// NOTE: This method does not yet handle persisting nested items within
/// inventories. Although loadout items do store items inside them this does
/// not currently utilise `parent_container_id` - all loadout items have the
/// loadout pseudo-container as their parent.
pub fn convert_items_to_database_items(
    loadout_container_id: EntityId,
    inventory: &Inventory,
    inventory_container_id: EntityId,
    next_id: &mut i64,
) -> Vec<ItemModelPair> {
    let loadout = inventory
        .loadout_items_with_persistence_key()
        .map(|(slot, item)| (slot.to_string(), item, loadout_container_id));

    // Inventory slots.
    let inventory = inventory
        .slots_with_id()
        .map(|(pos, item)| {
            (
                serde_json::to_string(&pos).expect("failed to serialize InvSlotId"),
                item.as_ref(),
                inventory_container_id,
            )
        })
        .chain(inventory.overflow_items().enumerate().map(|(index, item)| {
            (
                format!("overflow_item {index}"),
                Some(item),
                inventory_container_id,
            )
        }));

    // Use Breadth-first search to recurse into containers/modular weapons to store
    // their parts
    let mut bfs_queue: VecDeque<_> = inventory.chain(loadout).collect();
    let mut upserts = Vec::new();
    let mut depth = HashMap::new();
    depth.insert(inventory_container_id, 0);
    depth.insert(loadout_container_id, 0);
    while let Some((position, item, parent_container_item_id)) = bfs_queue.pop_front() {
        // Construct new items.
        if let Some(item) = item {
            // Try using the next available id in the sequence as the default for new items.
            let new_item_id = NonZeroU64::new(u64::try_from(*next_id).expect(
                "We are willing to crash if the next entity id overflows (or is otherwise \
                 negative).",
            ))
            .expect("next_id should not be zero, either");

            // Fast (kinda) path: acquire read for the common case where an id has
            // already been assigned.
            let comp = item.get_item_id_for_database();
            let item_id = comp.load()
                // First, we filter out "impossible" entity IDs--IDs that are larger
                // than the maximum sequence value (next_id).  This is important
                // because we update the item ID atomically, *before* we know whether
                // this transaction has completed successfully, and we don't abort the
                // process on a failed transaction.  In such cases, new IDs from
                // aborted transactions will show up as having a higher value than the
                // current max sequence number.  Because the only place that modifies
                // the item_id through a shared reference is (supposed to be) this
                // function, which is part of the batch update transaction, we can
                // assume that any rollback during the update would fail to insert
                // *any* new items for the current character; this means that any items
                // inserted between the failure and now (i.e. values less than next_id)
                // would either not be items at all, or items belonging to other
                // characters, leading to an easily detectable SQLite failure that we
                // can use to atomically set the id back to None (if it was still the
                // same bad value).
                //
                // Note that this logic only requires that all the character's items be
                // updated within the same serializable transaction; the argument does
                // not depend on SQLite-specific details (like locking) or on the fact
                // that a user's transactions are always serialized on their own
                // session.  Also note that since these IDs are in-memory, we don't
                // have to worry about their values during, e.g., a process crash;
                // serializability will take care of us in those cases.  Finally, note
                // that while we have not yet implemented the "liveness" part of the
                // algorithm (resetting ids back to None if we detect errors), this is
                // not needed for soundness, and this part can be deferred until we
                // switch to an execution model where such races are actually possible
                // during normal gameplay.
                .and_then(|item_id| Some(if item_id >= new_item_id {
                    // Try to atomically exchange with our own, "correct" next id.
                    match comp.compare_exchange(Some(item_id), Some(new_item_id)) {
                        Ok(_) => {
                            let item_id = *next_id;
                            // We won the race, use next_id and increment it.
                            *next_id += 1;
                            item_id
                        },
                        Err(item_id) => {
                            // We raced with someone, and they won the race, so we know
                            // this transaction must abort unless they finish first.  So,
                            // just assume they will finish first, and use their assigned
                            // item_id.
                            EntityId::try_from(item_id?.get())
                                .expect("We always choose legal EntityIds as item ids")
                        },
                    }
                } else { EntityId::try_from(item_id.get()).expect("We always choose legal EntityIds as item ids") }))
                // Finally, we're in the case where no entity was assigned yet (either
                // ever, or due to corrections after a rollback).  This proceeds
                // identically to the "impossible ID" case.
                .unwrap_or_else(|| {
                    // Try to atomically compare with the empty id.
                    match comp.compare_exchange(None, Some(new_item_id)) {
                        Ok(_) => {
                            let item_id = *next_id;
                            *next_id += 1;
                            item_id
                        },
                        Err(item_id) => {
                            EntityId::try_from(item_id.expect("TODO: Fix handling of reset to None when we have concurrent writers.").get())
                                .expect("We always choose legal EntityIds as item ids")
                        },
                    }
                });

            depth.insert(item_id, depth[&parent_container_item_id] + 1);

            for (i, component) in item.components().iter().enumerate() {
                // recursive items' children have the same position as their parents, and since
                // they occur afterwards in the topological sort of the parent graph (which
                // should still always be a tree, even with recursive items), we
                // have enough information to put them back into their parents on load
                bfs_queue.push_back((format!("component_{}", i), Some(component), item_id));
            }

            let item_properties = json_models::item_properties_to_db_model(item);

            let upsert = ItemModelPair {
                model: Item {
                    item_definition_id: String::from(item.persistence_item_id()),
                    position,
                    parent_container_item_id,
                    item_id,
                    stack_size: if item.is_stackable() {
                        item.amount() as i32
                    } else {
                        1
                    },
                    properties: serde_json::to_string(&item_properties)
                        .expect("Failed to convert item properties to a json string."),
                },
                // Continue to remember the atomic, in case we detect an error later and want
                // to roll back to preserve liveness.
                comp,
            };
            upserts.push(upsert);
        }
    }
    upserts.sort_by_key(|pair| (depth[&pair.model.item_id], pair.model.item_id));
    trace!("upserts: {:#?}", upserts);
    upserts
}

pub fn convert_body_to_database_json(
    comp_body: &CompBody,
) -> Result<(&str, String), PersistenceError> {
    Ok(match comp_body {
        Body::Humanoid(body) => (
            "humanoid",
            serde_json::to_string(&HumanoidBody::from(body))?,
        ),
        Body::QuadrupedLow(body) => (
            "quadruped_low",
            serde_json::to_string(&GenericBody::from(body))?,
        ),
        Body::QuadrupedMedium(body) => (
            "quadruped_medium",
            serde_json::to_string(&GenericBody::from(body))?,
        ),
        Body::QuadrupedSmall(body) => (
            "quadruped_small",
            serde_json::to_string(&GenericBody::from(body))?,
        ),
        Body::BirdMedium(body) => (
            "bird_medium",
            serde_json::to_string(&GenericBody::from(body))?,
        ),
        Body::Crustacean(body) => (
            "crustacean",
            serde_json::to_string(&GenericBody::from(body))?,
        ),
        _ => {
            return Err(PersistenceError::ConversionError(format!(
                "Unsupported body type for persistence: {:?}",
                comp_body
            )));
        },
    })
}

pub fn convert_waypoint_to_database_json(
    waypoint: Option<Waypoint>,
    map_marker: Option<MapMarker>,
) -> Option<String> {
    if waypoint.is_some() || map_marker.is_some() {
        let charpos = CharacterPosition {
            waypoint: waypoint.map(|w| w.get_pos()),
            map_marker: map_marker.map(|m| m.0),
        };
        Some(
            serde_json::to_string(&charpos)
                .map_err(|err| {
                    PersistenceError::ConversionError(format!("Error encoding waypoint: {:?}", err))
                })
                .ok()?,
        )
    } else {
        None
    }
}

pub fn convert_waypoint_from_database_json(
    position: &str,
) -> Result<(Option<Waypoint>, Option<MapMarker>), PersistenceError> {
    let character_position =
        serde_json::de::from_str::<CharacterPosition>(position).map_err(|err| {
            PersistenceError::ConversionError(format!(
                "Error de-serializing waypoint: {} err: {}",
                position, err
            ))
        })?;
    Ok((
        character_position
            .waypoint
            .map(|pos| Waypoint::new(pos, Time(0.0))),
        character_position.map_marker.map(MapMarker),
    ))
}

// Used to handle cases of modular items that are composed of components.
// When called with the index of a component's parent item, it can get a mutable
// reference to that parent item so that the component can be added to the
// parent item. If the item corresponding to the index this is called on is
// itself a component, recursively goes through inventory until it grabs
// component.
fn get_mutable_item<'a, 'b, T>(
    index: usize,
    inventory_items: &'a [Item],
    item_indices: &'a HashMap<i64, usize>,
    inventory: &'b mut T,
    get_mut_item: &'a impl Fn(&'b mut T, &str) -> Option<&'b mut VelorenItem>,
) -> Result<&'a mut VelorenItem, PersistenceError>
where
    'b: 'a,
{
    // First checks if item is a component, if it is, tries to get a mutable
    // reference to itself by getting a mutable reference to the item that is its
    // parent
    //
    // It is safe to directly index into `inventory_items` with `index` as the
    // parent item of a component is loaded before its components, therefore the
    // index of a parent item should exist when loading the component.
    let parent_id = inventory_items[index].parent_container_item_id;
    if inventory_items[index].position.contains("component_") {
        if let Some(parent) = item_indices.get(&parent_id).map(move |i| {
            get_mutable_item(
                *i,
                inventory_items,
                item_indices,
                inventory,
                // slot,
                get_mut_item,
            )
        }) {
            // Parses component index
            let position = &inventory_items[index].position;
            let component_index = position
                .split('_')
                .nth(1)
                .and_then(|s| s.parse::<usize>().ok())
                .ok_or_else(|| {
                    PersistenceError::ConversionError(format!(
                        "Failed to parse position stored in database: {position}."
                    ))
                })?;
            // Returns mutable reference to component item by accessing the component
            // through its parent item item
            parent?
                .persistence_access_mutable_component(component_index)
                .ok_or_else(|| {
                    PersistenceError::ConversionError(format!(
                        "Component in position {component_index} doesn't exist on parent item \
                         {parent_id}."
                    ))
                })
        } else {
            Err(PersistenceError::ConversionError(format!(
                "Parent item with id {parent_id} does not exist in database."
            )))
        }
    } else {
        get_mut_item(inventory, &inventory_items[index].position).ok_or_else(|| {
            PersistenceError::ConversionError(format!(
                "Unable to retrieve parent veloren item {parent_id} of component from inventory."
            ))
        })
    }
}

/// Properly-recursive items (currently modular weapons) occupy the same
/// inventory slot as their parent. The caller is responsible for ensuring that
/// inventory_items and loadout_items are topologically sorted (i.e. forall i,
/// `items[i].parent_container_item_id == x` implies exists j < i satisfying
/// `items[j].item_id == x`)
pub fn convert_inventory_from_database_items(
    inventory_container_id: i64,
    inventory_items: &[Item],
    loadout_container_id: i64,
    loadout_items: &[Item],
) -> Result<Inventory, PersistenceError> {
    // Loadout items must be loaded before inventory items since loadout items
    // provide inventory slots. Since items stored inside loadout items actually
    // have their parent_container_item_id as the loadout pseudo-container we rely
    // on populating the loadout items first, and then inserting the items into the
    // inventory at the correct position.
    //
    let loadout = convert_loadout_from_database_items(loadout_container_id, loadout_items)?;
    let mut inventory = Inventory::with_loadout_humanoid(loadout);
    let mut item_indices = HashMap::new();

    let mut failed_inserts = HashMap::new();

    // In order to items with components to properly load, it is important that this
    // item iteration occurs in order so that any modular items are loaded before
    // its components.
    for (i, db_item) in inventory_items.iter().enumerate() {
        item_indices.insert(db_item.item_id, i);

        let mut item = get_item_from_asset(db_item.item_definition_id.as_str())?;
        let item_properties =
            serde_json::de::from_str::<DatabaseItemProperties>(&db_item.properties)?;
        json_models::apply_db_item_properties(&mut item, &item_properties);

        // NOTE: Since this is freshly loaded, the atomic is *unique.*
        let comp = item.get_item_id_for_database();

        // Item ID
        comp.store(Some(NonZeroU64::try_from(db_item.item_id as u64).map_err(
            |_| PersistenceError::ConversionError("Item with zero item_id".to_owned()),
        )?));

        // Stack Size
        if db_item.stack_size == 1 || item.is_stackable() {
            // FIXME: On failure, collect the set of items that don't fit and return them
            // (to be dropped next to the player) as this could be the result of
            // a change in the max amount for that item.
            item.set_amount(u32::try_from(db_item.stack_size).map_err(|_| {
                PersistenceError::ConversionError(format!(
                    "Invalid item stack size for stackable={}: {}",
                    item.is_stackable(),
                    &db_item.stack_size
                ))
            })?)
            .map_err(|_| {
                PersistenceError::ConversionError("Error setting amount for item".to_owned())
            })?;
        }

        // Insert item into inventory

        // Slot position
        let slot = |s: &str| {
            serde_json::from_str::<InvSlotId>(s).map_err(|_| {
                PersistenceError::ConversionError(format!(
                    "Failed to parse item position: {:?}",
                    &db_item.position
                ))
            })
        };

        if db_item.parent_container_item_id == inventory_container_id {
            if db_item.position.contains("overflow_item") {
                failed_inserts.insert(db_item.position.clone(), item);
            } else {
                match slot(&db_item.position) {
                    Ok(slot) => {
                        let insert_res = inventory.insert_at(slot, item);

                        match insert_res {
                            Ok(None) => {
                                // Insert successful
                            },
                            Ok(Some(_item)) => {
                                // If inventory.insert returns an item, it means it was swapped for
                                // an item that already occupied the
                                // slot. Multiple items being stored
                                // in the database for the same slot is
                                // an error.
                                return Err(PersistenceError::ConversionError(
                                    "Inserted an item into the same slot twice".to_string(),
                                ));
                            },
                            Err(item) => {
                                // If this happens there were too many items in the database for the
                                // current inventory size
                                failed_inserts.insert(db_item.position.clone(), item);
                            },
                        }
                    },
                    Err(err) => {
                        return Err(err);
                    },
                }
            }
        } else if let Some(&j) = item_indices.get(&db_item.parent_container_item_id) {
            get_mutable_item(
                j,
                inventory_items,
                &item_indices,
                &mut (&mut inventory, &mut failed_inserts),
                &|(inv, f_i): &mut (&mut Inventory, &mut HashMap<String, VelorenItem>), s| {
                    // Attempts first to access inventory if that slot exists there. If it does not
                    // it instead attempts to access failed inserts list.
                    slot(s)
                        .ok()
                        .and_then(|slot| inv.slot_mut(slot))
                        .and_then(|a| a.as_mut())
                        // .or_else(f_i.map.get(s).and_then(|i| f_i.items.get_mut(*i)))
                        .or_else(|| f_i.get_mut(s))
                },
            )?
            .persistence_access_add_component(item);
        } else {
            return Err(PersistenceError::ConversionError(format!(
                "Couldn't find parent item {} before item {} in inventory",
                db_item.parent_container_item_id, db_item.item_id
            )));
        }
    }

    // For failed inserts, attempt to push to inventory. If push fails, move to
    // overflow slots.
    if let Err(inv_error) = inventory.push_all(failed_inserts.into_values()) {
        inventory.persistence_push_overflow_items(inv_error.returned_items());
    }

    // Some items may have had components added, so update the item config of each
    // item to ensure that it correctly accounts for components that were added
    inventory.persistence_update_all_item_states(&ABILITY_MAP, &MATERIAL_STATS_MANIFEST);

    Ok(inventory)
}

pub fn convert_loadout_from_database_items(
    loadout_container_id: i64,
    database_items: &[Item],
) -> Result<Loadout, PersistenceError> {
    let loadout_builder = LoadoutBuilder::empty();
    let mut loadout = loadout_builder.build();
    let mut item_indices = HashMap::new();

    // In order to items with components to properly load, it is important that this
    // item iteration occurs in order so that any modular items are loaded before
    // its components.
    for (i, db_item) in database_items.iter().enumerate() {
        item_indices.insert(db_item.item_id, i);

        let mut item = get_item_from_asset(db_item.item_definition_id.as_str())?;
        let item_properties =
            serde_json::de::from_str::<DatabaseItemProperties>(&db_item.properties)?;
        json_models::apply_db_item_properties(&mut item, &item_properties);

        // NOTE: item id is currently *unique*, so we can store the ID safely.
        let comp = item.get_item_id_for_database();
        comp.store(Some(NonZeroU64::try_from(db_item.item_id as u64).map_err(
            |_| PersistenceError::ConversionError("Item with zero item_id".to_owned()),
        )?));

        let convert_error = |err| match err {
            LoadoutError::InvalidPersistenceKey => PersistenceError::ConversionError(format!(
                "Invalid persistence key: {}",
                &db_item.position
            )),
            LoadoutError::NoParentAtSlot => PersistenceError::ConversionError(format!(
                "No parent item at slot: {}",
                &db_item.position
            )),
        };

        if db_item.parent_container_item_id == loadout_container_id {
            loadout
                .set_item_at_slot_using_persistence_key(&db_item.position, item)
                .map_err(convert_error)?;
        } else if let Some(&j) = item_indices.get(&db_item.parent_container_item_id) {
            get_mutable_item(j, database_items, &item_indices, &mut loadout, &|l, s| {
                l.get_mut_item_at_slot_using_persistence_key(s).ok()
            })?
            .persistence_access_add_component(item);
        } else {
            return Err(PersistenceError::ConversionError(format!(
                "Couldn't find parent item {} before item {} in loadout",
                db_item.parent_container_item_id, db_item.item_id
            )));
        }
    }

    // Some items may have had components added, so update the item config of each
    // item to ensure that it correctly accounts for components that were added
    loadout.persistence_update_all_item_states(&ABILITY_MAP, &MATERIAL_STATS_MANIFEST);

    Ok(loadout)
}

fn get_item_from_asset(item_definition_id: &str) -> Result<common::comp::Item, PersistenceError> {
    common::comp::Item::new_from_asset(item_definition_id).map_err(|err| {
        PersistenceError::AssetError(format!(
            "Error loading item asset: {} - {}",
            item_definition_id, err
        ))
    })
}

/// Generates the code to deserialize a specific body variant from JSON
macro_rules! deserialize_body {
    ($body_data:expr, $body_variant:tt, $body_type:tt) => {{
        let json_model = serde_json::de::from_str::<GenericBody>($body_data)?;
        CompBody::$body_variant(common::comp::$body_type::Body {
            species: common::comp::$body_type::Species::from_str(&json_model.species)
                .map_err(|_| {
                    PersistenceError::ConversionError(format!(
                        "Missing species: {}",
                        json_model.species
                    ))
                })?
                .to_owned(),
            body_type: common::comp::$body_type::BodyType::from_str(&json_model.body_type)
                .map_err(|_| {
                    PersistenceError::ConversionError(format!(
                        "Missing body type: {}",
                        json_model.species
                    ))
                })?
                .to_owned(),
        })
    }};
}
pub fn convert_body_from_database(
    variant: &str,
    body_data: &str,
) -> Result<CompBody, PersistenceError> {
    Ok(match variant {
        // The humanoid variant doesn't use the body_variant! macro as it is unique in having
        // extra fields on its body struct
        "humanoid" => {
            let json_model = serde_json::de::from_str::<HumanoidBody>(body_data)?;
            CompBody::Humanoid(humanoid::Body {
                species: humanoid::ALL_SPECIES
                    .get(json_model.species as usize)
                    .ok_or_else(|| {
                        PersistenceError::ConversionError(format!(
                            "Missing species: {}",
                            json_model.species
                        ))
                    })?
                    .to_owned(),
                body_type: humanoid::ALL_BODY_TYPES
                    .get(json_model.body_type as usize)
                    .ok_or_else(|| {
                        PersistenceError::ConversionError(format!(
                            "Missing body_type: {}",
                            json_model.body_type
                        ))
                    })?
                    .to_owned(),
                hair_style: json_model.hair_style,
                beard: json_model.beard,
                eyes: json_model.eyes,
                accessory: json_model.accessory,
                hair_color: json_model.hair_color,
                skin: json_model.skin,
                eye_color: json_model.eye_color,
            })
        },
        "quadruped_low" => {
            deserialize_body!(body_data, QuadrupedLow, quadruped_low)
        },
        "quadruped_medium" => {
            deserialize_body!(body_data, QuadrupedMedium, quadruped_medium)
        },
        "quadruped_small" => {
            deserialize_body!(body_data, QuadrupedSmall, quadruped_small)
        },
        "bird_medium" => {
            deserialize_body!(body_data, BirdMedium, bird_medium)
        },
        "crustacean" => {
            deserialize_body!(body_data, Crustacean, crustacean)
        },
        _ => {
            return Err(PersistenceError::ConversionError(format!(
                "{} is not a supported body type for deserialization",
                variant
            )));
        },
    })
}

pub fn convert_character_from_database(character: &Character) -> common::character::Character {
    common::character::Character {
        id: Some(CharacterId(character.character_id)),
        alias: String::from(&character.alias),
    }
}

pub fn convert_stats_from_database(alias: String, body: Body) -> Stats {
    let mut new_stats = Stats::empty(body);
    new_stats.name = alias;
    new_stats
}

/// NOTE: This does *not* return an error on failure, since we can partially
/// recover from some failures.  Instead, it returns the error in the second
/// return value; make sure to handle it if present!
pub fn convert_skill_set_from_database(
    skill_groups: &[SkillGroup],
) -> (SkillSet, Option<skillset::SkillsPersistenceError>) {
    let (skillless_skill_groups, deserialized_skills) =
        convert_skill_groups_from_database(skill_groups);
    SkillSet::load_from_database(skillless_skill_groups, deserialized_skills)
}

#[allow(clippy::type_complexity)]
fn convert_skill_groups_from_database(
    skill_groups: &[SkillGroup],
) -> (
    // Skill groups in the vec do not contain skills, those are added later. The skill group only
    // contains fields related to experience and skill points
    HashMap<SkillGroupKind, skillset::SkillGroup>,
    //
    HashMap<SkillGroupKind, Result<Vec<Skill>, skillset::SkillsPersistenceError>>,
) {
    let mut new_skill_groups = HashMap::new();
    let mut deserialized_skills = HashMap::new();
    for skill_group in skill_groups.iter() {
        let skill_group_kind = json_models::db_string_to_skill_group(&skill_group.skill_group_kind);
        let mut new_skill_group = skillset::SkillGroup {
            skill_group_kind,
            // Available and earned exp and sp are reconstructed below
            earned_exp: 0,
            available_exp: 0,
            available_sp: 0,
            earned_sp: 0,
            // Ordered skills empty here as skills get inserted later as they are unlocked, so long
            // as there is not a respec.
            ordered_skills: Vec::new(),
        };

        // Add experience to skill group through method to ensure invariant of
        // (earned_exp >= available_exp) are maintained
        // Adding experience will automatically earn all possible skill points
        let skill_group_exp = skill_group.earned_exp.clamp(0, i64::from(u32::MAX)) as u32;
        new_skill_group.add_experience(skill_group_exp);

        use skillset::SkillsPersistenceError;

        let skills_result = if skill_group.spent_exp != i64::from(new_skill_group.spent_exp()) {
            // If persisted spent exp does not equal the spent exp after reacquiring skill
            // points, force a respec
            Err(SkillsPersistenceError::SpentExpMismatch)
        } else if Some(&skill_group.hash_val) != skillset::SKILL_GROUP_HASHES.get(&skill_group_kind)
        {
            // Else if persisted hash for skill group does not match current hash for skill
            // group, force a respec
            Err(SkillsPersistenceError::HashMismatch)
        } else {
            // Else attempt to deserialize skills from a json string
            match serde_json::from_str::<Vec<Skill>>(&skill_group.skills) {
                // If it correctly deserializes, return the persisted skills
                Ok(skills) => Ok(skills),
                // Else if doesn't deserialize correctly, force a respec
                Err(err) => {
                    warn!(
                        "Skills failed to correctly deserialized\nError: {:#?}\nRaw JSON: {:#?}",
                        err, &skill_group.skills
                    );
                    Err(SkillsPersistenceError::DeserializationFailure)
                },
            }
        };

        deserialized_skills.insert(skill_group_kind, skills_result);

        new_skill_groups.insert(skill_group_kind, new_skill_group);
    }
    (new_skill_groups, deserialized_skills)
}

pub fn convert_skill_groups_to_database<'a, I: Iterator<Item = &'a skillset::SkillGroup>>(
    entity_id: CharacterId,
    skill_groups: I,
) -> Vec<SkillGroup> {
    let skill_group_hashes = &skillset::SKILL_GROUP_HASHES;
    skill_groups
        .into_iter()
        .map(|sg| SkillGroup {
            entity_id: entity_id.0,
            skill_group_kind: json_models::skill_group_to_db_string(sg.skill_group_kind),
            earned_exp: i64::from(sg.earned_exp),
            spent_exp: i64::from(sg.spent_exp()),
            // If fails to convert, just forces a respec on next login
            skills: serde_json::to_string(&sg.ordered_skills).unwrap_or_else(|_| "".to_string()),
            hash_val: skill_group_hashes
                .get(&sg.skill_group_kind)
                .cloned()
                .unwrap_or_default(),
        })
        .collect()
}

pub fn convert_active_abilities_to_database(
    entity_id: CharacterId,
    active_abilities: &ActiveAbilities,
) -> AbilitySets {
    let ability_sets = json_models::active_abilities_to_db_model(active_abilities);
    AbilitySets {
        entity_id: entity_id.0,
        ability_sets: serde_json::to_string(&ability_sets).unwrap_or_default(),
    }
}

pub fn convert_active_abilities_from_database(ability_sets: &AbilitySets) -> ActiveAbilities {
    let ability_sets = serde_json::from_str::<Vec<DatabaseAbilitySet>>(&ability_sets.ability_sets)
        .unwrap_or_else(|err| {
            common_base::dev_panic!(format!(
                "Failed to parse ability sets. Error: {:#?}\nAbility sets:\n{:#?}",
                err, ability_sets.ability_sets
            ));
            Vec::new()
        });
    json_models::active_abilities_from_db_model(ability_sets)
}
