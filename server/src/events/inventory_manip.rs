use rand::Rng;
use specs::{join::Join, world::WorldExt, Builder, Entity as EcsEntity, WriteStorage};
use tracing::{debug, error, warn};
use vek::{Rgb, Vec3};

use common::{
    comp::{
        self,
        item::{self, MaterialStatManifest},
        slot::{self, Slot},
    },
    consts::MAX_PICKUP_RANGE,
    recipe::default_recipe_book,
    trade::Trades,
    uid::Uid,
    util::find_dist::{self, FindDist},
    vol::ReadVol,
};
use common_net::{msg::ServerGeneral, sync::WorldSyncExt};
use common_sys::state::State;
use comp::LightEmitter;

use crate::{client::Client, Server, StateExt};

pub fn swap_lantern(
    storage: &mut WriteStorage<comp::LightEmitter>,
    entity: EcsEntity,
    lantern: &item::Lantern,
) {
    if let Some(mut light) = storage.get_mut(entity) {
        light.strength = lantern.strength();
        light.col = lantern.color();
    }
}

pub fn snuff_lantern(storage: &mut WriteStorage<comp::LightEmitter>, entity: EcsEntity) {
    storage.remove(entity);
}

#[allow(clippy::blocks_in_if_conditions)]
#[allow(clippy::same_item_push)] // TODO: Pending review in #587
pub fn handle_inventory(server: &mut Server, entity: EcsEntity, manip: comp::InventoryManip) {
    let state = server.state_mut();

    let uid = if let Some(uid) = state.ecs().uid_from_entity(entity) {
        uid
    } else {
        warn!(
            "Couldn't get uid for entity {:?} at start of handle_inventory",
            entity
        );
        return;
    };

    {
        let trades = state.ecs().read_resource::<Trades>();
        if trades.in_immutable_trade(&uid) {
            // manipulating the inventory can mutate the trade
            return;
        }
    }

    let mut dropped_items = Vec::new();
    let mut thrown_items = Vec::new();

    let get_cylinder = |state: &State, entity| {
        let ecs = state.ecs();
        let positions = ecs.read_storage::<comp::Pos>();
        let scales = ecs.read_storage::<comp::Scale>();
        let colliders = ecs.read_storage::<comp::Collider>();
        let char_states = ecs.read_storage::<comp::CharacterState>();

        positions.get(entity).map(|p| {
            find_dist::Cylinder::from_components(
                p.0,
                scales.get(entity).copied(),
                colliders.get(entity),
                char_states.get(entity),
            )
        })
    };

    match manip {
        comp::InventoryManip::Pickup(uid) => {
            let picked_up_item: Option<comp::Item>;
            let item_entity = if let (Some((item, item_entity)), Some(mut inv)) = (
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

                let entity_cylinder = get_cylinder(state, entity);
                if !within_pickup_range(entity_cylinder, || get_cylinder(state, item_entity)) {
                    debug!(
                        ?entity_cylinder,
                        "Failed to pick up item as not within range, Uid: {}", uid
                    );
                    return;
                };

                // Grab the health from the entity and check if the entity is dead.
                let healths = state.ecs().read_storage::<comp::Health>();
                if let Some(entity_health) = healths.get(entity) {
                    if entity_health.is_dead {
                        debug!("Failed to pick up item as the entity is dead");
                        return; // If dead, don't continue
                    }
                }

                // First try to equip the picked up item
                if let Err(returned_item) = inv.try_equip(item) {
                    // If we couldn't equip it (no empty slot for it or unequippable) then attempt
                    // to add the item to the entity's inventory
                    match inv.pickup_item(returned_item) {
                        Ok(_) => Some(item_entity),
                        Err(_) => None, // Inventory was full
                    }
                } else {
                    Some(item_entity)
                }
            } else {
                // Item entity/component could not be found - most likely because the entity
                // attempted to pick up the same item very quickly before its deletion of the
                // world from the first pickup attempt was processed.
                debug!("Failed to get entity/component for item Uid: {}", uid);
                return;
            };

            let event = if let Some(item_entity) = item_entity {
                if let Err(err) = state.delete_entity_recorded(item_entity) {
                    // If this occurs it means the item was duped as it's been pushed to the
                    // entity's inventory but also left on the ground
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
                if block.is_collectible() && state.can_set_block(pos) {
                    // Check if the block is within pickup range
                    let entity_cylinder = get_cylinder(state, entity);
                    if !within_pickup_range(entity_cylinder, || {
                        Some(find_dist::Cube {
                            min: pos.as_(),
                            side_length: 1.0,
                        })
                    }) {
                        debug!(
                            ?entity_cylinder,
                            "Failed to pick up block as not within range, block pos: {}", pos
                        );
                        return;
                    };

                    if let Some(item) = comp::Item::try_reclaim_from_block(block) {
                        let (event, item_was_added) = if let Some(mut inv) = state
                            .ecs()
                            .write_storage::<comp::Inventory>()
                            .get_mut(entity)
                        {
                            match inv.push(item.clone()) {
                                None => (
                                    Some(comp::InventoryUpdate::new(
                                        comp::InventoryUpdateEvent::Collected(item),
                                    )),
                                    true,
                                ),
                                Some(_) => (
                                    Some(comp::InventoryUpdate::new(
                                        comp::InventoryUpdateEvent::CollectFailed,
                                    )),
                                    false,
                                ),
                            }
                        } else {
                            debug!(
                                "Can't add item to inventory: entity has no inventory ({:?})",
                                entity
                            );
                            (None, false)
                        };
                        if let Some(event) = event {
                            state.write_component(entity, event);
                            if item_was_added {
                                // we made sure earlier the block was not already modified this tick
                                state.set_block(pos, block.into_vacant())
                            };
                        }
                    } else {
                        debug!(
                            "Failed to reclaim item from block at pos={} or entity had no \
                             inventory",
                            pos
                        )
                    }
                } else {
                    debug!(
                        "Can't reclaim item from block at pos={}: block is not collectable or was \
                         already set this tick.",
                        pos
                    );
                }
            }
        },
        comp::InventoryManip::Use(slot) => {
            let mut inventories = state.ecs().write_storage::<comp::Inventory>();
            let mut inventory = if let Some(inventory) = inventories.get_mut(entity) {
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
                        inventory.get(slot).map_or((false, None), |i| {
                            (i.kind().is_equippable(), match i.kind() {
                                ItemKind::Lantern(lantern) => Some(lantern),
                                _ => None,
                            })
                        });
                    if is_equippable {
                        if let Some(lantern) = lantern_opt {
                            swap_lantern(&mut state.ecs().write_storage(), entity, &lantern);
                        }
                        if let Some(pos) = state.ecs().read_storage::<comp::Pos>().get(entity) {
                            dropped_items.extend(inventory.equip(slot).into_iter().map(|x| {
                                (
                                    *pos,
                                    state
                                        .read_component_copied::<comp::Ori>(entity)
                                        .unwrap_or_default(),
                                    x,
                                )
                            }));
                        }
                        Some(comp::InventoryUpdateEvent::Used)
                    } else if let Some(item) = inventory.take(
                        slot,
                        &state.ecs().read_resource::<item::MaterialStatManifest>(),
                    ) {
                        match item.kind() {
                            ItemKind::Consumable { kind, effect, .. } => {
                                maybe_effect = Some(effect.clone());
                                Some(comp::InventoryUpdateEvent::Consumed(kind.clone()))
                            },
                            ItemKind::Throwable { kind, .. } => {
                                if let Some(pos) =
                                    state.ecs().read_storage::<comp::Pos>().get(entity)
                                {
                                    thrown_items.push((
                                        *pos,
                                        state
                                            .read_component_copied::<comp::Vel>(entity)
                                            .unwrap_or_default(),
                                        state
                                            .read_component_copied::<comp::Ori>(entity)
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
                                                wild_pos.0.distance_squared(pos.0) < 5.0f32.powi(2)
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

                                        // Add to group system
                                        let clients = state.ecs().read_storage::<Client>();
                                        let uids = state.ecs().read_storage::<Uid>();
                                        let mut group_manager = state
                                            .ecs()
                                            .write_resource::<comp::group::GroupManager>(
                                        );
                                        group_manager.new_pet(
                                            tameable_entity,
                                            entity,
                                            &mut state.ecs().write_storage(),
                                            &state.ecs().entities(),
                                            &state.ecs().read_storage(),
                                            &uids,
                                            &mut |entity, group_change| {
                                                clients
                                                    .get(entity)
                                                    .and_then(|c| {
                                                        group_change
                                                            .try_map(|e| uids.get(e).copied())
                                                            .map(|g| (g, c))
                                                    })
                                                    .map(|(g, c)| {
                                                        c.send(ServerGeneral::GroupUpdate(g))
                                                    });
                                            },
                                        );

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
                                    let _ = inventory.insert_or_stack_at(slot, item);
                                }

                                Some(comp::InventoryUpdateEvent::Used)
                            },
                            _ => {
                                inventory.insert_or_stack_at(slot, item).unwrap();
                                None
                            },
                        }
                    } else {
                        None
                    }
                },
                Slot::Equip(slot) => {
                    if slot == slot::EquipSlot::Lantern {
                        snuff_lantern(&mut state.ecs().write_storage(), entity);
                    }

                    if let Some(pos) = state.ecs().read_storage::<comp::Pos>().get(entity) {
                        // Unequip the item, any items that no longer fit within the inventory (due
                        // to unequipping a bag for example) will be dropped on the floor
                        if let Ok(Some(leftover_items)) = inventory.unequip(slot) {
                            dropped_items.extend(leftover_items.into_iter().map(|x| {
                                (
                                    *pos,
                                    state
                                        .read_component_copied::<comp::Ori>(entity)
                                        .unwrap_or_default(),
                                    x,
                                )
                            }));
                        }
                    }
                    Some(comp::InventoryUpdateEvent::Used)
                },
            };

            drop(inventories);

            if let Some(effects) = maybe_effect {
                for effect in effects {
                    state.apply_effect(entity, effect, None);
                }
            }
            if let Some(event) = event {
                state.write_component(entity, comp::InventoryUpdate::new(event));
            }
        },
        comp::InventoryManip::Swap(a, b) => {
            let ecs = state.ecs();

            if let Some(pos) = ecs.read_storage::<comp::Pos>().get(entity) {
                if let Some(mut inventory) = ecs.write_storage::<comp::Inventory>().get_mut(entity)
                {
                    let mut merged_stacks = false;

                    // If both slots have items and we're attemping to drag from one stack
                    // into another, stack the items.
                    if let (Slot::Inventory(slot_a), Slot::Inventory(slot_b)) = (a, b) {
                        merged_stacks |= inventory.merge_stack_into(slot_a, slot_b);
                    }

                    // If the stacks weren't mergable carry out a swap.
                    if !merged_stacks {
                        dropped_items.extend(inventory.swap(a, b).into_iter().map(|x| {
                            (
                                *pos,
                                state
                                    .read_component_copied::<comp::Ori>(entity)
                                    .unwrap_or_default(),
                                x,
                            )
                        }));
                    }
                }
            }

            state.write_component(
                entity,
                comp::InventoryUpdate::new(comp::InventoryUpdateEvent::Swapped),
            );
        },
        comp::InventoryManip::SplitSwap(slot, target) => {
            let msm = state.ecs().read_resource::<MaterialStatManifest>();
            let mut inventories = state.ecs().write_storage::<comp::Inventory>();
            let mut inventory = if let Some(inventory) = inventories.get_mut(entity) {
                inventory
            } else {
                error!(
                    ?entity,
                    "Can't manipulate inventory, entity doesn't have one"
                );
                return;
            };

            // If both slots have items and we're attemping to split from one stack
            // into another, ensure that they are the same type of item. If they are
            // the same type do nothing, as you don't want to overwrite the existing item.

            if let (Slot::Inventory(source_inv_slot_id), Slot::Inventory(target_inv_slot_id)) =
                (slot, target)
            {
                if let Some(source_item) = inventory.get(source_inv_slot_id) {
                    if let Some(target_item) = inventory.get(target_inv_slot_id) {
                        if source_item != target_item {
                            return;
                        }
                    }
                }
            }

            let item = match slot {
                Slot::Inventory(slot) => inventory.take_half(slot, &msm),
                Slot::Equip(_) => None,
            };

            if let Some(item) = item {
                if let Slot::Inventory(target) = target {
                    inventory.insert_or_stack_at(target, item).ok();
                }
            }
            drop(inventory);
            drop(inventories);
            drop(msm);

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
                    .and_then(|mut inv| inv.remove(slot)),
                Slot::Equip(slot) => state
                    .ecs()
                    .write_storage::<comp::Inventory>()
                    .get_mut(entity)
                    .and_then(|mut inv| inv.replace_loadout_item(slot, None)),
            };

            // FIXME: We should really require the drop and write to be atomic!
            if let (Some(mut item), Some(pos)) =
                (item, state.ecs().read_storage::<comp::Pos>().get(entity))
            {
                item.put_in_world();
                dropped_items.push((
                    *pos,
                    state
                        .read_component_copied::<comp::Ori>(entity)
                        .unwrap_or_default(),
                    item,
                ));
            }
            state.write_component(
                entity,
                comp::InventoryUpdate::new(comp::InventoryUpdateEvent::Dropped),
            );
        },
        comp::InventoryManip::SplitDrop(slot) => {
            let msm = state.ecs().read_resource::<MaterialStatManifest>();
            let item = match slot {
                Slot::Inventory(slot) => state
                    .ecs()
                    .write_storage::<comp::Inventory>()
                    .get_mut(entity)
                    .and_then(|mut inv| inv.take_half(slot, &msm)),
                Slot::Equip(_) => None,
            };

            // FIXME: We should really require the drop and write to be atomic!
            if let (Some(mut item), Some(pos)) =
                (item, state.ecs().read_storage::<comp::Pos>().get(entity))
            {
                item.put_in_world();
                dropped_items.push((
                    *pos,
                    state
                        .read_component_copied::<comp::Ori>(entity)
                        .unwrap_or_default(),
                    item,
                ));
            }
            drop(msm);
            state.write_component(
                entity,
                comp::InventoryUpdate::new(comp::InventoryUpdateEvent::Dropped),
            );
        },
        comp::InventoryManip::CraftRecipe(recipe) => {
            if let Some(mut inv) = state
                .ecs()
                .write_storage::<comp::Inventory>()
                .get_mut(entity)
            {
                let recipe_book = default_recipe_book().read();
                let craft_result = recipe_book.get(&recipe).and_then(|r| {
                    r.perform(
                        &mut inv,
                        &state.ecs().read_resource::<item::MaterialStatManifest>(),
                    )
                    .ok()
                });

                // FIXME: We should really require the drop and write to be atomic!
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
                                .read_component_copied::<comp::Pos>(entity)
                                .unwrap_or_default(),
                            state
                                .read_component_copied::<comp::Ori>(entity)
                                .unwrap_or_default(),
                            item.clone(),
                        ));
                    }
                }
            }
        },
    }
    // Drop items, Debug items should simply disappear when dropped
    for (pos, ori, item) in dropped_items
        .into_iter()
        .filter(|(_, _, i)| !matches!(i.quality(), item::Quality::Debug))
    {
        // hack: special case coins for now
        let body = match item.item_definition_id() {
            "common.items.utility.coins" => comp::object::Body::Coins,
            _ => comp::object::Body::Pouch,
        };
        state
            .create_object(Default::default(), body)
            .with(comp::Pos(pos.0 + *ori.look_dir() + Vec3::unit_z()))
            .with(item)
            .with(comp::Vel(Vec3::zero()))
            .build();
    }

    let mut rng = rand::thread_rng();

    // Throw items
    for (pos, vel, ori, kind) in thrown_items {
        let vel = match kind {
            item::Throwable::Firework(_) => Vec3::new(
                rng.gen_range(-15.0..15.0),
                rng.gen_range(-15.0..15.0),
                rng.gen_range(80.0..110.0),
            ),
            _ => {
                vel.0
                    + *ori.look_dir() * 20.0
                    + Vec3::unit_z() * 15.0
                    + Vec3::<f32>::zero().map(|_| rand::thread_rng().gen::<f32>() - 0.5) * 4.0
            },
        };

        let uid = state.read_component_copied::<Uid>(entity);

        let mut new_entity = state
            .create_object(Default::default(), match kind {
                item::Throwable::Bomb => comp::object::Body::Bomb,
                item::Throwable::Firework(reagent) => comp::object::Body::for_firework(reagent),
                item::Throwable::TrainingDummy => comp::object::Body::TrainingDummy,
            })
            .with(comp::Pos(pos.0 + Vec3::unit_z() * 0.25))
            .with(comp::Vel(vel));

        match kind {
            item::Throwable::Bomb => {
                new_entity = new_entity.with(comp::Object::Bomb { owner: uid });
            },
            item::Throwable::Firework(reagent) => {
                new_entity = new_entity
                    .with(comp::Object::Firework {
                        owner: uid,
                        reagent,
                    })
                    .with(LightEmitter {
                        animated: true,
                        flicker: 2.0,
                        strength: 2.0,
                        col: Rgb::new(1.0, 1.0, 0.0),
                    });
            },
            item::Throwable::TrainingDummy => {
                new_entity = new_entity.with(comp::Stats::new("Training Dummy".to_string()));
            },
        };

        new_entity.build();
    }

    let mut trades = state.ecs().write_resource::<Trades>();
    if trades.in_mutable_trade(&uid) {
        // manipulating the inventory mutated the trade, so reset the accept flags
        trades.implicit_mutation_occurred(&uid);
    }
}

fn within_pickup_range<S: FindDist<find_dist::Cylinder>>(
    entity_cylinder: Option<find_dist::Cylinder>,
    shape_fn: impl FnOnce() -> Option<S>,
) -> bool {
    entity_cylinder
        .and_then(|entity_cylinder| {
            shape_fn().map(|shape| shape.min_distance(entity_cylinder) < MAX_PICKUP_RANGE)
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use vek::Vec3;

    use common::comp::Pos;
    use find_dist::*;

    use super::*;

    // Helper function
    #[allow(clippy::unnecessary_wraps)]
    fn test_cylinder(pos: comp::Pos) -> Option<Cylinder> {
        Some(Cylinder::from_components(pos.0, None, None, None))
    }

    #[test]
    fn pickup_distance_within_range() {
        let position = Pos(Vec3::zero());
        let item_position = Pos(Vec3::one());

        assert_eq!(
            within_pickup_range(test_cylinder(position), || test_cylinder(item_position),),
            true
        );
    }

    #[test]
    fn pickup_distance_not_within_range() {
        let position = Pos(Vec3::zero());
        let item_position = Pos(Vec3::one() * 500.0);

        assert_eq!(
            within_pickup_range(test_cylinder(position), || test_cylinder(item_position),),
            false
        );
    }
}
