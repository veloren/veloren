use crate::persistence::{
    character::EntityId,
    models::{Body, Character, Item, Stats},
};

use crate::persistence::{
    error::Error,
    json_models::{CharacterPosition, HumanoidBody},
};
use common::{
    character::CharacterId,
    comp::{item::tool::AbilityMap, Body as CompBody, *},
    loadout_builder,
};
use core::{convert::TryFrom, num::NonZeroU64};
use itertools::{Either, Itertools};
use std::sync::Arc;

pub struct ItemModelPair {
    pub comp: Arc<common::comp::item::ItemId>,
    pub model: Item,
}

/// The left vector contains all item rows to upsert; the right-hand vector
/// contains all item rows to delete (by parent ID and position).
pub fn convert_items_to_database_items(
    loadout: &Loadout,
    loadout_container_id: EntityId,
    inventory: &Inventory,
    inventory_container_id: EntityId,
    next_id: &mut i64,
) -> (Vec<ItemModelPair>, Vec<(EntityId, String)>) {
    // Loadout slots.
    let loadout = [
        ("active_item", loadout.active_item.as_ref().map(|x| &x.item)),
        ("second_item", loadout.second_item.as_ref().map(|x| &x.item)),
        ("lantern", loadout.lantern.as_ref()),
        ("shoulder", loadout.shoulder.as_ref()),
        ("chest", loadout.chest.as_ref()),
        ("belt", loadout.belt.as_ref()),
        ("hand", loadout.hand.as_ref()),
        ("pants", loadout.pants.as_ref()),
        ("foot", loadout.foot.as_ref()),
        ("back", loadout.back.as_ref()),
        ("ring", loadout.ring.as_ref()),
        ("neck", loadout.neck.as_ref()),
        ("head", loadout.head.as_ref()),
        ("tabard", loadout.tabard.as_ref()),
        ("glider", loadout.glider.as_ref()),
    ];

    let loadout = loadout
        .iter()
        .map(|&(slot, item)| (slot.to_string(), item, loadout_container_id));

    // Inventory slots.
    let inventory = inventory
        .slots()
        .iter()
        .enumerate()
        .map(|(slot, item)| (slot.to_string(), item.as_ref(), inventory_container_id));

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

pub fn convert_waypoint_to_database_json(waypoint: &Waypoint) -> Result<String, Error> {
    let charpos = CharacterPosition {
        waypoint: waypoint.get_pos(),
    };
    serde_json::to_string(&charpos).map_err(Error::SerializationError)
}

pub fn convert_stats_to_database(character_id: CharacterId, stats: &common::comp::Stats) -> Stats {
    Stats {
        stats_id: character_id,
        level: stats.level.level() as i32,
        exp: stats.exp.current() as i32,
        endurance: stats.endurance as i32,
        fitness: stats.fitness as i32,
        willpower: stats.willpower as i32,
    }
}

pub fn convert_inventory_from_database_items(database_items: &[Item]) -> Result<Inventory, Error> {
    let mut inventory = Inventory::new_empty();
    for db_item in database_items.iter() {
        let mut item = common::comp::Item::new_from_asset(db_item.item_definition_id.as_str())?;

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
        let slot = &db_item.position.parse::<usize>().map_err(|_| {
            Error::ConversionError(format!(
                "Failed to parse item position: {}",
                &db_item.position
            ))
        })?;

        let insert_res = inventory.insert(*slot, item).map_err(|_| {
            // If this happens there were too many items in the database for the current
            // inventory size
            Error::ConversionError("Error inserting item into inventory".to_string())
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

pub fn convert_loadout_from_database_items(
    database_items: &[Item],
    map: &AbilityMap,
) -> Result<Loadout, Error> {
    let mut loadout = loadout_builder::LoadoutBuilder::new();
    for db_item in database_items.iter() {
        let item = common::comp::Item::new_from_asset(db_item.item_definition_id.as_str())?;
        // NOTE: item id is currently *unique*, so we can store the ID safely.
        let comp = item.get_item_id_for_database();
        comp.store(Some(NonZeroU64::try_from(db_item.item_id as u64).map_err(
            |_| Error::ConversionError("Item with zero item_id".to_owned()),
        )?));

        match db_item.position.as_str() {
            "active_item" => loadout = loadout.active_item(Some(ItemConfig::from((item, map)))),
            "second_item" => loadout = loadout.second_item(Some(ItemConfig::from((item, map)))),
            "lantern" => loadout = loadout.lantern(Some(item)),
            "shoulder" => loadout = loadout.shoulder(Some(item)),
            "chest" => loadout = loadout.chest(Some(item)),
            "belt" => loadout = loadout.belt(Some(item)),
            "hand" => loadout = loadout.hand(Some(item)),
            "pants" => loadout = loadout.pants(Some(item)),
            "foot" => loadout = loadout.foot(Some(item)),
            "back" => loadout = loadout.back(Some(item)),
            "ring" => loadout = loadout.ring(Some(item)),
            "neck" => loadout = loadout.neck(Some(item)),
            "head" => loadout = loadout.head(Some(item)),
            "tabard" => loadout = loadout.tabard(Some(item)),
            "glider" => loadout = loadout.glider(Some(item)),
            _ => {
                return Err(Error::ConversionError(format!(
                    "Unknown loadout position on item: {}",
                    db_item.position.as_str()
                )));
            },
        }
    }

    Ok(loadout.build())
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

pub fn convert_stats_from_database(stats: &Stats, alias: String) -> common::comp::Stats {
    let mut new_stats = common::comp::Stats::empty();
    new_stats.name = alias;
    new_stats.level.set_level(stats.level as u32);
    new_stats.exp.update_maximum(stats.level as u32);
    new_stats.exp.set_current(stats.exp as u32);
    /*new_stats.update_max_hp(new_stats.body_type);
    new_stats.health.set_to(
        new_stats.health.maximum(),
        common::comp::HealthSource::Revive,
    );*/
    new_stats.endurance = stats.endurance as u32;
    new_stats.fitness = stats.fitness as u32;
    new_stats.willpower = stats.willpower as u32;

    new_stats
}
