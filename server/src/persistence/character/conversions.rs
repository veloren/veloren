use crate::persistence::{
    character::EntityId,
    models::{Body, Character, Item, Skill, SkillGroup},
};

use crate::persistence::{
    error::Error,
    json_models::{self, CharacterPosition, HumanoidBody},
};
use common::{
    character::CharacterId,
    comp::{
        inventory::{
            loadout::{Loadout, LoadoutError},
            loadout_builder::LoadoutBuilder,
            slot::InvSlotId,
        },
        skills, Body as CompBody, Waypoint, *,
    },
    resources::Time,
};
use core::{convert::TryFrom, num::NonZeroU64};
use hashbrown::HashMap;
use itertools::{Either, Itertools};
use std::sync::Arc;

pub struct ItemModelPair {
    pub comp: Arc<common::comp::item::ItemId>,
    pub model: Item,
}

/// The left vector contains all item rows to upsert; the right-hand vector
/// contains all item rows to delete (by parent ID and position).
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
) -> (Vec<ItemModelPair>, Vec<(EntityId, String)>) {
    let loadout = inventory
        .loadout_items_with_persistence_key()
        .map(|(slot, item)| (slot.to_string(), item, loadout_container_id));

    // Inventory slots.
    let inventory = inventory.slots_with_id().map(|(pos, item)| {
        (
            serde_json::to_string(&pos).expect("failed to serialize InventorySlotPos"),
            item.as_ref(),
            inventory_container_id,
        )
    });

    // Construct new items.
    inventory.chain(loadout)
        .partition_map(|(position, item, parent_container_item_id)| {
            if let Some(item) = item {
                // Try using the next available id in the sequence as the default for new items.
                let new_item_id = NonZeroU64::new(u64::try_from(*next_id)
                    .expect("We are willing to crash if the next entity id overflows \
                (or is otherwise negative).")).expect("next_id should not be zero, either");

                let comp = item.get_item_id_for_database();
                Either::Left(ItemModelPair {
                    model: Item {
                        item_definition_id: item.item_definition_id().to_owned(),
                        position,
                        parent_container_item_id,
                        // Fast (kinda) path: acquire read for the common case where an id has
                        // already been assigned.
                        item_id: comp.load()
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
                            }),
                        stack_size: if item.is_stackable() {
                            item.amount() as i32
                        } else {
                            1
                        },
                    },
                    // Continue to remember the atomic, in case we detect an error later and want
                    // to roll back to preserve liveness.
                    comp,
                })
            } else {
                Either::Right((parent_container_item_id, position))
            }
        })
}

pub fn convert_body_to_database_json(body: &CompBody) -> Result<String, Error> {
    let json_model = match body {
        common::comp::Body::Humanoid(humanoid_body) => HumanoidBody::from(humanoid_body),
        _ => unimplemented!("Only humanoid bodies are currently supported for persistence"),
    };

    serde_json::to_string(&json_model).map_err(Error::SerializationError)
}

pub fn convert_waypoint_to_database_json(waypoint: Option<Waypoint>) -> Option<String> {
    match waypoint {
        Some(w) => {
            let charpos = CharacterPosition {
                waypoint: w.get_pos(),
            };
            Some(
                serde_json::to_string(&charpos)
                    .map_err(|err| {
                        Error::ConversionError(format!("Error encoding waypoint: {:?}", err))
                    })
                    .ok()?,
            )
        },
        None => None,
    }
}

pub fn convert_waypoint_from_database_json(position: &str) -> Result<Waypoint, Error> {
    let character_position =
        serde_json::de::from_str::<CharacterPosition>(position).map_err(|err| {
            Error::ConversionError(format!(
                "Error de-serializing waypoint: {} err: {}",
                position, err
            ))
        })?;
    Ok(Waypoint::new(character_position.waypoint, Time(0.0)))
}

pub fn convert_inventory_from_database_items(
    inventory_items: &[Item],
    loadout_items: &[Item],
) -> Result<Inventory, Error> {
    // Loadout items must be loaded before inventory items since loadout items
    // provide inventory slots. Since items stored inside loadout items actually
    // have their parent_container_item_id as the loadout pseudo-container we rely
    // on populating the loadout items first, and then inserting the items into the
    // inventory at the correct position. When we want to support items inside the
    // player's inventory containing other items (such as "right click to
    // unwrap" gifts perhaps) then we will need to refactor inventory/loadout
    // persistence to traverse the tree of items and load them from the root
    // down.
    let loadout = convert_loadout_from_database_items(loadout_items)?;
    let mut inventory = Inventory::new_with_loadout(loadout);

    for db_item in inventory_items.iter() {
        let mut item = get_item_from_asset(db_item.item_definition_id.as_str())?;

        // NOTE: Since this is freshly loaded, the atomic is *unique.*
        let comp = item.get_item_id_for_database();

        // Item ID
        comp.store(Some(NonZeroU64::try_from(db_item.item_id as u64).map_err(
            |_| Error::ConversionError("Item with zero item_id".to_owned()),
        )?));

        // Stack Size
        if db_item.stack_size == 1 || item.is_stackable() {
            item.set_amount(u32::try_from(db_item.stack_size).map_err(|_| {
                Error::ConversionError(format!(
                    "Invalid item stack size for stackable={}: {}",
                    item.is_stackable(),
                    &db_item.stack_size
                ))
            })?)
            .map_err(|_| Error::ConversionError("Error setting amount for item".to_owned()))?;
        }

        // Insert item into inventory

        // Slot position
        let slot: InvSlotId = serde_json::from_str(&db_item.position).map_err(|_| {
            Error::ConversionError(format!(
                "Failed to parse item position: {:?}",
                &db_item.position
            ))
        })?;

        let insert_res = inventory.insert_at(slot, item).map_err(|_| {
            // If this happens there were too many items in the database for the current
            // inventory size
            Error::ConversionError(format!(
                "Error inserting item into inventory, position: {:?}",
                slot
            ))
        })?;

        if insert_res.is_some() {
            // If inventory.insert returns an item, it means it was swapped for an item that
            // already occupied the slot. Multiple items being stored in the database for
            // the same slot is an error.
            return Err(Error::ConversionError(
                "Inserted an item into the same slot twice".to_string(),
            ));
        }
    }

    Ok(inventory)
}

pub fn convert_loadout_from_database_items(database_items: &[Item]) -> Result<Loadout, Error> {
    let loadout_builder = LoadoutBuilder::new();
    let mut loadout = loadout_builder.build();

    for db_item in database_items.iter() {
        let item = get_item_from_asset(db_item.item_definition_id.as_str())?;
        // NOTE: item id is currently *unique*, so we can store the ID safely.
        let comp = item.get_item_id_for_database();
        comp.store(Some(NonZeroU64::try_from(db_item.item_id as u64).map_err(
            |_| Error::ConversionError("Item with zero item_id".to_owned()),
        )?));

        loadout
            .set_item_at_slot_using_persistence_key(&db_item.position, item)
            .map_err(|err| match err {
                LoadoutError::InvalidPersistenceKey => Error::ConversionError(format!(
                    "Invalid persistence key: {}",
                    &db_item.position
                )),
            })?;
    }

    Ok(loadout)
}

pub fn convert_body_from_database(body: &Body) -> Result<CompBody, Error> {
    Ok(match body.variant.as_str() {
        "humanoid" => {
            let json_model = serde_json::de::from_str::<HumanoidBody>(&body.body_data)?;
            CompBody::Humanoid(common::comp::humanoid::Body {
                species: common::comp::humanoid::ALL_SPECIES
                    .get(json_model.species as usize)
                    .ok_or_else(|| {
                        Error::ConversionError(format!("Missing species: {}", json_model.species))
                    })?
                    .to_owned(),
                body_type: common::comp::humanoid::ALL_BODY_TYPES
                    .get(json_model.body_type as usize)
                    .ok_or_else(|| {
                        Error::ConversionError(format!(
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
        _ => {
            return Err(Error::ConversionError(
                "Only humanoid bodies are supported for characters".to_string(),
            ));
        },
    })
}

pub fn convert_character_from_database(character: &Character) -> common::character::Character {
    common::character::Character {
        id: Some(character.character_id),
        alias: String::from(&character.alias),
    }
}

pub fn convert_stats_from_database(
    alias: String,
    skills: &[Skill],
    skill_groups: &[SkillGroup],
) -> common::comp::Stats {
    let mut new_stats = common::comp::Stats::empty();
    new_stats.name = alias;
    /*new_stats.update_max_hp(new_stats.body_type);
    new_stats.health.set_to(
        new_stats.health.maximum(),
        common::comp::HealthSource::Revive,
    );*/
    new_stats.skill_set = skills::SkillSet {
        skill_groups: convert_skill_groups_from_database(skill_groups),
        skills: convert_skills_from_database(skills),
        modify_health: true,
        modify_energy: true,
    };

    new_stats
}

fn get_item_from_asset(item_definition_id: &str) -> Result<common::comp::Item, Error> {
    common::comp::Item::new_from_asset(item_definition_id).map_err(|err| {
        Error::AssetError(format!(
            "Error loading item asset: {} - {}",
            item_definition_id,
            err.to_string()
        ))
    })
}

fn convert_skill_groups_from_database(skill_groups: &[SkillGroup]) -> Vec<skills::SkillGroup> {
    let mut new_skill_groups = Vec::new();
    for skill_group in skill_groups.iter() {
        let skill_group_type = json_models::db_string_to_skill_group(&skill_group.skill_group_type);
        let new_skill_group = skills::SkillGroup {
            skill_group_type,
            exp: skill_group.exp as u16,
            available_sp: skill_group.available_sp as u16,
            earned_sp: skill_group.earned_sp as u16,
        };
        new_skill_groups.push(new_skill_group);
    }
    new_skill_groups
}

fn convert_skills_from_database(skills: &[Skill]) -> HashMap<skills::Skill, Option<u16>> {
    let mut new_skills = HashMap::new();
    for skill in skills.iter() {
        let new_skill = json_models::db_string_to_skill(&skill.skill_type);
        new_skills.insert(new_skill, skill.level.map(|l| l as u16));
    }
    new_skills
}

pub fn convert_skill_groups_to_database(
    entity_id: CharacterId,
    skill_groups: Vec<skills::SkillGroup>,
) -> Vec<SkillGroup> {
    skill_groups
        .into_iter()
        .map(|sg| SkillGroup {
            entity_id,
            skill_group_type: json_models::skill_group_to_db_string(sg.skill_group_type),
            exp: sg.exp as i32,
            available_sp: sg.available_sp as i32,
            earned_sp: sg.earned_sp as i32,
        })
        .collect()
}

pub fn convert_skills_to_database(
    entity_id: CharacterId,
    skills: HashMap<skills::Skill, Option<u16>>,
) -> Vec<Skill> {
    skills
        .iter()
        .map(|(s, l)| Skill {
            entity_id,
            skill_type: json_models::skill_to_db_string(*s),
            level: l.map(|l| l as i32),
        })
        .collect()
}
