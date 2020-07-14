use crate::{Server, StateExt};
use common::{
    comp::{
        self, item,
        slot::{self, Slot},
        Pos, MAX_PICKUP_RANGE_SQR,
    },
    recipe::default_recipe_book,
    sync::{Uid, WorldSyncExt},
    terrain::block::Block,
    vol::{ReadVol, Vox},
};
use rand::Rng;
use specs::{join::Join, world::WorldExt, Builder, Entity as EcsEntity, WriteStorage};
use tracing::{debug, error};
use vek::Vec3;

pub fn swap_lantern(
    storage: &mut WriteStorage<comp::LightEmitter>,
    entity: EcsEntity,
    lantern: &item::Lantern,
) {
    if let Some(light) = storage.get_mut(entity) {
        light.strength = lantern.strength();
        light.col = lantern.color();
    }
}

pub fn snuff_lantern(storage: &mut WriteStorage<comp::LightEmitter>, entity: EcsEntity) {
    storage.remove(entity);
}

#[allow(clippy::blocks_in_if_conditions)]
pub fn handle_inventory(server: &mut Server, entity: EcsEntity, manip: comp::InventoryManip) {
    let state = server.state_mut();
    let mut dropped_items = Vec::new();
    let mut thrown_items = Vec::new();

    match manip {
        comp::InventoryManip::Pickup(uid) => {
            let picked_up_item: Option<comp::Item>;
            let item_entity = if let (Some((item, item_entity)), Some(inv)) = (
                state
                    .ecs()
                    .entity_from_uid(uid.into())
                    .and_then(|item_entity| {
                        state
                            .ecs()
                            .write_storage::<comp::Item>()
                            .get_mut(item_entity)
                            .map(|item| (item.clone(), item_entity))
                    }),
                state
                    .ecs()
                    .write_storage::<comp::Inventory>()
                    .get_mut(entity),
            ) {
                picked_up_item = Some(item.clone());
                if !within_pickup_range(
                    state.ecs().read_storage::<comp::Pos>().get(entity),
                    state.ecs().read_storage::<comp::Pos>().get(item_entity),
                ) {
                    debug!("Failed to pick up item as not within range, Uid: {}", uid);
                    return;
                };

                // Attempt to add the item to the player's inventory
                match inv.push(item) {
                    None => Some(item_entity),
                    Some(_) => None, // Inventory was full
                }
            } else {
                // Item entity/component could not be found - most likely because the player
                // attempted to pick up the same item very quickly before its deletion of the
                // world from the first pickup attempt was processed.
                debug!("Failed to get entity/component for item Uid: {}", uid);
                return;
            };

            let event = if let Some(item_entity) = item_entity {
                if let Err(err) = state.delete_entity_recorded(item_entity) {
                    // If this occurs it means the item was duped as it's been pushed to the
                    // player's inventory but also left on the ground
                    panic!("Failed to delete picked up item entity: {:?}", err);
                }
                comp::InventoryUpdate::new(comp::InventoryUpdateEvent::Collected(
                    picked_up_item.unwrap(),
                ))
            } else {
                comp::InventoryUpdate::new(comp::InventoryUpdateEvent::CollectFailed)
            };

            state.write_component(entity, event);
        },

        comp::InventoryManip::Collect(pos) => {
            let block = state.terrain().get(pos).ok().copied();

            if let Some(block) = block {
                let has_inv_space = state
                    .ecs()
                    .read_storage::<comp::Inventory>()
                    .get(entity)
                    .map(|inv| !inv.is_full())
                    .unwrap_or(false);

                if !has_inv_space {
                    state.write_component(
                        entity,
                        comp::InventoryUpdate::new(comp::InventoryUpdateEvent::CollectFailed),
                    );
                } else if block.is_collectible()
                    && state.try_set_block(pos, Block::empty()).is_some()
                {
                    comp::Item::try_reclaim_from_block(block)
                        .map(|item| state.give_item(entity, item));
                }
            }
        },

        comp::InventoryManip::Use(slot) => {
            let mut inventories = state.ecs().write_storage::<comp::Inventory>();
            let inventory = if let Some(inventory) = inventories.get_mut(entity) {
                inventory
            } else {
                error!(
                    ?entity,
                    "Can't manipulate inventory, entity doesn't have one"
                );
                return;
            };

            let mut maybe_effect = None;

            let event = match slot {
                Slot::Inventory(slot) => {
                    use item::ItemKind;
                    let (is_equippable, lantern_opt) =
                        inventory
                            .get(slot)
                            .map_or((false, None), |i| match &i.kind {
                                ItemKind::Tool(_) | ItemKind::Armor { .. } => (true, None),
                                ItemKind::Lantern(lantern) => (true, Some(lantern)),
                                _ => (false, None),
                            });
                    if is_equippable {
                        if let Some(loadout) = state.ecs().write_storage().get_mut(entity) {
                            if let Some(lantern) = lantern_opt {
                                swap_lantern(&mut state.ecs().write_storage(), entity, lantern);
                            }
                            slot::equip(slot, inventory, loadout);
                            Some(comp::InventoryUpdateEvent::Used)
                        } else {
                            None
                        }
                    } else if let Some(item) = inventory.take(slot) {
                        match &item.kind {
                            ItemKind::Consumable { kind, effect, .. } => {
                                maybe_effect = Some(*effect);
                                Some(comp::InventoryUpdateEvent::Consumed(*kind))
                            },
                            ItemKind::Throwable { kind, .. } => {
                                if let Some(pos) =
                                    state.ecs().read_storage::<comp::Pos>().get(entity)
                                {
                                    thrown_items.push((
                                        *pos,
                                        state
                                            .read_component_cloned::<comp::Vel>(entity)
                                            .unwrap_or_default(),
                                        state
                                            .read_component_cloned::<comp::Ori>(entity)
                                            .unwrap_or_default(),
                                        *kind,
                                    ));
                                }
                                Some(comp::InventoryUpdateEvent::Used)
                            },
                            ItemKind::Utility {
                                kind: comp::item::Utility::Collar,
                                ..
                            } => {
                                let reinsert = if let Some(pos) =
                                    state.read_storage::<comp::Pos>().get(entity)
                                {
                                    let uid = state
                                        .read_component_cloned(entity)
                                        .expect("Expected player to have a UID");
                                    if (
                                        &state.read_storage::<comp::Alignment>(),
                                        &state.read_storage::<comp::Agent>(),
                                    )
                                        .join()
                                        .filter(|(alignment, _)| {
                                            alignment == &&comp::Alignment::Owned(uid)
                                        })
                                        .count()
                                        >= 3
                                    {
                                        true
                                    } else if let Some(tameable_entity) = {
                                        let nearest_tameable = (
                                            &state.ecs().entities(),
                                            &state.ecs().read_storage::<comp::Pos>(),
                                            &state.ecs().read_storage::<comp::Alignment>(),
                                        )
                                            .join()
                                            .filter(|(_, wild_pos, _)| {
                                                wild_pos.0.distance_squared(pos.0)
                                                    < 5.0f32.powf(2.0)
                                            })
                                            .filter(|(_, _, alignment)| {
                                                alignment == &&comp::Alignment::Wild
                                            })
                                            .min_by_key(|(_, wild_pos, _)| {
                                                (wild_pos.0.distance_squared(pos.0) * 100.0) as i32
                                            })
                                            .map(|(entity, _, _)| entity);
                                        nearest_tameable
                                    } {
                                        let _ = state
                                            .ecs()
                                            .write_storage()
                                            .insert(tameable_entity, comp::Alignment::Owned(uid));
                                        let _ = state
                                            .ecs()
                                            .write_storage()
                                            .insert(tameable_entity, comp::Agent::default());
                                        false
                                    } else {
                                        true
                                    }
                                } else {
                                    true
                                };

                                if reinsert {
                                    let _ = inventory.insert(slot, item);
                                }

                                Some(comp::InventoryUpdateEvent::Used)
                            },
                            _ => {
                                // TODO: this doesn't work for stackable items
                                inventory.insert(slot, item).unwrap();
                                None
                            },
                        }
                    } else {
                        None
                    }
                },
                Slot::Equip(slot) => {
                    if let Some(loadout) = state.ecs().write_storage().get_mut(entity) {
                        if slot == slot::EquipSlot::Lantern {
                            snuff_lantern(&mut state.ecs().write_storage(), entity);
                        }
                        slot::unequip(slot, inventory, loadout);
                        Some(comp::InventoryUpdateEvent::Used)
                    } else {
                        error!(?entity, "Entity doesn't have a loadout, can't unequip...");
                        None
                    }
                },
            };

            drop(inventories);
            if let Some(effect) = maybe_effect {
                state.apply_effect(entity, effect);
            }
            if let Some(event) = event {
                state.write_component(entity, comp::InventoryUpdate::new(event));
            }
        },

        comp::InventoryManip::Swap(a, b) => {
            let ecs = state.ecs();
            let mut inventories = ecs.write_storage();
            let mut loadouts = ecs.write_storage();
            let inventory = inventories.get_mut(entity);
            let loadout = loadouts.get_mut(entity);

            slot::swap(a, b, inventory, loadout);

            // :/
            drop(loadouts);
            drop(inventories);

            state.write_component(
                entity,
                comp::InventoryUpdate::new(comp::InventoryUpdateEvent::Swapped),
            );
        },

        comp::InventoryManip::Drop(slot) => {
            let item = match slot {
                Slot::Inventory(slot) => state
                    .ecs()
                    .write_storage::<comp::Inventory>()
                    .get_mut(entity)
                    .and_then(|inv| inv.remove(slot)),
                Slot::Equip(slot) => state
                    .ecs()
                    .write_storage()
                    .get_mut(entity)
                    .and_then(|ldt| slot::loadout_remove(slot, ldt)),
            };

            if let (Some(item), Some(pos)) =
                (item, state.ecs().read_storage::<comp::Pos>().get(entity))
            {
                dropped_items.push((
                    *pos,
                    state
                        .read_component_cloned::<comp::Ori>(entity)
                        .unwrap_or_default(),
                    item,
                ));
            }
            state.write_component(
                entity,
                comp::InventoryUpdate::new(comp::InventoryUpdateEvent::Dropped),
            );
        },

        comp::InventoryManip::CraftRecipe(recipe) => {
            if let Some(inv) = state
                .ecs()
                .write_storage::<comp::Inventory>()
                .get_mut(entity)
            {
                let recipe_book = default_recipe_book();
                let craft_result = recipe_book.get(&recipe).and_then(|r| r.perform(inv).ok());

                if craft_result.is_some() {
                    let _ = state.ecs().write_storage().insert(
                        entity,
                        comp::InventoryUpdate::new(comp::InventoryUpdateEvent::Craft),
                    );
                }

                // Drop the item if there wasn't enough space
                if let Some(Some((item, amount))) = craft_result {
                    for _ in 0..amount {
                        dropped_items.push((
                            state
                                .read_component_cloned::<comp::Pos>(entity)
                                .unwrap_or_default(),
                            state
                                .read_component_cloned::<comp::Ori>(entity)
                                .unwrap_or_default(),
                            item.clone(),
                        ));
                    }
                }
            }
        },
    }

    // Drop items
    for (pos, ori, item) in dropped_items {
        let vel = *ori.0 * 5.0
            + Vec3::unit_z() * 10.0
            + Vec3::<f32>::zero().map(|_| rand::thread_rng().gen::<f32>() - 0.5) * 4.0;

        state
            .create_object(Default::default(), comp::object::Body::Pouch)
            .with(comp::Pos(pos.0 + Vec3::unit_z() * 0.25))
            .with(item)
            .with(comp::Vel(vel))
            .build();
    }

    // Throw items
    for (pos, vel, ori, kind) in thrown_items {
        let vel = vel.0
            + *ori.0 * 20.0
            + Vec3::unit_z() * 15.0
            + Vec3::<f32>::zero().map(|_| rand::thread_rng().gen::<f32>() - 0.5) * 4.0;

        let uid = state.read_component_cloned::<Uid>(entity);

        let mut new_entity = state
            .create_object(Default::default(), match kind {
                item::Throwable::Bomb => comp::object::Body::Bomb,
                item::Throwable::TrainingDummy => comp::object::Body::TrainingDummy,
            })
            .with(comp::Pos(pos.0 + Vec3::unit_z() * 0.25))
            .with(comp::Vel(vel));

        match kind {
            item::Throwable::Bomb => {
                new_entity = new_entity.with(comp::Object::Bomb { owner: uid });
            },
            item::Throwable::TrainingDummy => {
                new_entity = new_entity.with(comp::Stats::new(
                    "Training Dummy".to_string(),
                    comp::object::Body::TrainingDummy.into(),
                ));
            },
        };

        new_entity.build();
    }
}

fn within_pickup_range(player_position: Option<&Pos>, item_position: Option<&Pos>) -> bool {
    match (player_position, item_position) {
        (Some(ppos), Some(ipos)) => ppos.0.distance_squared(ipos.0) < MAX_PICKUP_RANGE_SQR,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::comp::Pos;
    use vek::Vec3;

    #[test]
    fn pickup_distance_within_range() {
        let player_position = Pos(Vec3::zero());
        let item_position = Pos(Vec3::one());

        assert_eq!(
            within_pickup_range(Some(&player_position), Some(&item_position)),
            true
        );
    }

    #[test]
    fn pickup_distance_not_within_range() {
        let player_position = Pos(Vec3::zero());
        let item_position = Pos(Vec3::one() * 500.0);

        assert_eq!(
            within_pickup_range(Some(&player_position), Some(&item_position)),
            false
        );
    }
}
