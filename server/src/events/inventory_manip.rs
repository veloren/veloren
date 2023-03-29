use hashbrown::HashSet;
use rand::{seq::IteratorRandom, Rng};
use specs::{join::Join, world::WorldExt, Builder, Entity as EcsEntity, WriteStorage};
use tracing::{debug, error, warn};
use vek::{Rgb, Vec3};

use common::{
    comp::{
        self,
        group::members,
        item::{self, tool::AbilityMap, MaterialStatManifest},
        slot::{self, Slot},
    },
    consts::MAX_PICKUP_RANGE,
    recipe::{self, default_component_recipe_book, default_recipe_book},
    terrain::{Block, SpriteKind},
    trade::Trades,
    uid::Uid,
    util::find_dist::{self, FindDist},
    vol::ReadVol,
};
use common_net::sync::WorldSyncExt;
use common_state::State;
use comp::LightEmitter;

use crate::{client::Client, Server, StateExt};
use common::{
    comp::{
        pet::is_tameable, Alignment, Body, CollectFailedReason, Group, InventoryUpdateEvent, Player,
    },
    event::{EventBus, ServerEvent},
};
use common_net::msg::ServerGeneral;

pub fn swap_lantern(
    storage: &mut WriteStorage<LightEmitter>,
    entity: EcsEntity,
    (lantern_color, lantern_strength): (Rgb<f32>, f32),
) {
    if let Some(mut light) = storage.get_mut(entity) {
        light.strength = lantern_strength;
        light.col = lantern_color;
    }
}

pub fn snuff_lantern(storage: &mut WriteStorage<LightEmitter>, entity: EcsEntity) {
    storage.remove(entity);
}

#[allow(clippy::blocks_in_if_conditions)]
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

    // Disallow inventory manipulation while dead
    if state
        .ecs()
        .read_storage::<comp::Health>()
        .get(entity)
        .map_or(false, |h| h.is_dead)
    {
        debug!("Can't manipulate inventory; entity is dead");
        return;
    }

    match manip {
        comp::InventoryManip::Pickup(pickup_uid) => {
            let item_entity =
                if let Some(item_entity) = state.ecs().entity_from_uid(pickup_uid.into()) {
                    item_entity
                } else {
                    // Item entity could not be found - most likely because the entity
                    // attempted to pick up the same item very quickly before its deletion of the
                    // world from the first pickup attempt was processed.
                    debug!("Failed to get entity for item Uid: {}", pickup_uid);
                    return;
                };
            let entity_cylinder = get_cylinder(state, entity);

            // FIXME: Raycast so we can't pick up items through walls.
            if !within_pickup_range(entity_cylinder, || get_cylinder(state, item_entity)) {
                debug!(
                    ?entity_cylinder,
                    "Failed to pick up item as not within range, Uid: {}", pickup_uid
                );
                return;
            }

            let loot_owner_storage = state.ecs().read_storage::<comp::LootOwner>();

            // If there's a loot owner for the item being picked up, then
            // determine whether the pickup should be rejected.
            let ownership_check_passed = state
                .ecs()
                .read_storage::<comp::LootOwner>()
                .get(item_entity)
                .map_or(true, |loot_owner| {
                    let alignments = state.ecs().read_storage::<Alignment>();
                    let bodies = state.ecs().read_storage::<Body>();
                    let players = state.ecs().read_storage::<Player>();
                    let groups = state.ecs().read_storage::<Group>();
                    let can_pickup = loot_owner.can_pickup(
                        uid,
                        groups.get(entity),
                        alignments.get(entity),
                        bodies.get(entity),
                        players.get(entity),
                    );
                    if !can_pickup {
                        let event =
                            comp::InventoryUpdate::new(InventoryUpdateEvent::EntityCollectFailed {
                                entity: pickup_uid,
                                reason: CollectFailedReason::LootOwned {
                                    owner: loot_owner.owner(),
                                    expiry_secs: loot_owner.time_until_expiration().as_secs(),
                                },
                            });
                        state.ecs().write_storage().insert(entity, event).unwrap();
                    }
                    can_pickup
                });

            if !ownership_check_passed {
                return;
            }

            drop(loot_owner_storage);

            // First, we remove the item, assuming picking it up will succeed (we do this to
            // avoid cloning the item, as we should not call Item::clone and it
            // may be removed!).
            let mut item_storage = state.ecs().write_storage::<comp::Item>();
            let item = if let Some(item) = item_storage.remove(item_entity) {
                item
            } else {
                // Item component could not be found - most likely because the entity
                // attempted to pick up the same item very quickly before its deletion of the
                // world from the first pickup attempt was processed.
                debug!(
                    "Failed to delete item component for entity, Uid: {}",
                    pickup_uid
                );
                return;
            };

            // NOTE: We dup the item for message purposes.
            let item_msg = item.duplicate(
                &state.ecs().read_resource::<AbilityMap>(),
                &state.ecs().read_resource::<MaterialStatManifest>(),
            );

            // Next, we try to equip the picked up item
            let event = match inventory.try_equip(item).or_else(|returned_item| {
                // If we couldn't equip it (no empty slot for it or unequippable) then attempt
                // to add the item to the entity's inventory
                inventory.pickup_item(returned_item)
            }) {
                Err(returned_item) => {
                    // Inventory was full, so we need to put back the item (note that we know there
                    // was no old item component for this entity).
                    item_storage.insert(item_entity, returned_item).expect(
                        "We know item_entity exists since we just successfully removed its Item \
                         component.",
                    );
                    drop(item_storage);
                    drop(inventories);
                    comp::InventoryUpdate::new(InventoryUpdateEvent::EntityCollectFailed {
                        entity: pickup_uid,
                        reason: CollectFailedReason::InventoryFull,
                    })
                },
                Ok(_) => {
                    // We succeeded in picking up the item, so we may now delete its old entity
                    // entirely.
                    drop(item_storage);
                    drop(inventories);
                    state.delete_entity_recorded(item_entity).expect(
                        "We knew item_entity existed since we just successfully removed its Item \
                         component.",
                    );
                    let ecs = state.ecs();
                    if let Some(group_id) = ecs.read_storage::<Group>().get(entity) {
                        announce_loot_to_group(
                            group_id,
                            ecs,
                            entity,
                            item_msg.duplicate(
                                &state.ecs().read_resource::<AbilityMap>(),
                                &state.ecs().read_resource::<MaterialStatManifest>(),
                            ),
                        );
                    }
                    comp::InventoryUpdate::new(InventoryUpdateEvent::Collected(item_msg))
                },
            };

            state
                .ecs()
                .write_storage()
                .insert(entity, event)
                .expect("We know entity exists since we got its inventory.");
        },
        comp::InventoryManip::Collect {
            sprite_pos,
            required_item,
        } => {
            let ecs = state.ecs();
            let terrain = ecs.read_resource::<common::terrain::TerrainGrid>();
            let mut block_change = ecs.write_resource::<common_state::BlockChange>();

            let block = terrain.get(sprite_pos).ok().copied();
            let mut drop_item = None;

            if let Some(block) = block {
                if block.is_collectible() && block_change.can_set_block(sprite_pos) {
                    // If an item was required to collect the sprite, consume it now
                    if let Some((inv_slot, true)) = required_item {
                        inventory.take(
                            inv_slot,
                            &ecs.read_resource::<AbilityMap>(),
                            &ecs.read_resource::<MaterialStatManifest>(),
                        );
                    }

                    // If there's an item to be reclaimed from the block, add it to the inventory
                    if let Some(item) = comp::Item::try_reclaim_from_block(block) {
                        // NOTE: We dup the item for message purposes.
                        let item_msg = item.duplicate(
                            &ecs.read_resource::<AbilityMap>(),
                            &ecs.read_resource::<MaterialStatManifest>(),
                        );
                        let event = match inventory.push(item) {
                            Ok(_) => {
                                if let Some(group_id) = ecs.read_storage::<Group>().get(entity) {
                                    announce_loot_to_group(
                                        group_id,
                                        ecs,
                                        entity,
                                        item_msg.duplicate(
                                            &ecs.read_resource::<AbilityMap>(),
                                            &ecs.read_resource::<MaterialStatManifest>(),
                                        ),
                                    );
                                }
                                comp::InventoryUpdate::new(InventoryUpdateEvent::Collected(
                                    item_msg,
                                ))
                            },
                            // The item we created was in some sense "fake" so it's safe to
                            // drop it.
                            Err(_) => {
                                drop_item = Some(item_msg);
                                comp::InventoryUpdate::new(
                                    InventoryUpdateEvent::BlockCollectFailed {
                                        pos: sprite_pos,
                                        reason: CollectFailedReason::InventoryFull,
                                    },
                                )
                            },
                        };
                        ecs.write_storage()
                            .insert(entity, event)
                            .expect("We know entity exists since we got its inventory.");
                    }

                    // We made sure earlier the block was not already modified this tick
                    block_change.set(sprite_pos, block.into_vacant());

                    // If the block was a keyhole, remove nearby door blocks
                    // TODO: Abstract this code into a generalised way to do block updates?
                    if matches!(block.get_sprite(), Some(SpriteKind::Keyhole)) {
                        let dirs = [
                            Vec3::unit_x(),
                            -Vec3::unit_x(),
                            Vec3::unit_y(),
                            -Vec3::unit_y(),
                            Vec3::unit_z(),
                            -Vec3::unit_z(),
                        ];

                        let mut destroyed = HashSet::<Vec3<i32>>::default();
                        let mut pending = dirs
                            .into_iter()
                            .map(|dir| sprite_pos + dir)
                            .collect::<HashSet<_>>();

                        // Limit the number of blocks we destroy
                        for _ in 0..250 {
                            // TODO: Replace with `entry` eventually
                            let next_pending = pending.iter().next().copied();
                            if let Some(pos) = next_pending {
                                pending.remove(&pos);
                                if !destroyed.contains(&pos)
                                    && matches!(
                                        terrain.get(pos).ok().and_then(|b| b.get_sprite()),
                                        Some(SpriteKind::KeyDoor)
                                    )
                                {
                                    block_change.try_set(pos, Block::empty());
                                    destroyed.insert(pos);

                                    pending.extend(dirs.into_iter().map(|dir| pos + dir));
                                }
                            } else {
                                break;
                            }
                        }
                    }
                } else {
                    debug!(
                        "Can't reclaim item from block at pos={}: block is not collectable or was \
                         already set this tick.",
                        sprite_pos
                    );
                }
            }
            drop(inventories);
            drop(terrain);
            drop(block_change);
            if let Some(item) = drop_item {
                state
                    .create_item_drop(Default::default(), item)
                    .with(comp::Pos(
                        Vec3::new(
                            sprite_pos.x as f32,
                            sprite_pos.y as f32,
                            sprite_pos.z as f32,
                        ) + Vec3::one().with_z(0.0) * 0.5,
                    ))
                    .with(comp::Vel(Vec3::zero()))
                    .build();
            }
        },
        comp::InventoryManip::Use(slot) => {
            let mut maybe_effect = None;

            let event = match slot {
                Slot::Inventory(slot) => {
                    use item::ItemKind;

                    let (is_equippable, lantern_info) =
                        inventory.get(slot).map_or((false, None), |i| {
                            let kind = i.kind();
                            let is_equippable = kind.is_equippable();
                            let lantern_info = match &*kind {
                                ItemKind::Lantern(lantern) => {
                                    Some((lantern.color(), lantern.strength()))
                                },
                                _ => None,
                            };
                            (is_equippable, lantern_info)
                        });
                    if is_equippable {
                        if let Some(lantern_info) = lantern_info {
                            swap_lantern(&mut state.ecs().write_storage(), entity, lantern_info);
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
                        Some(InventoryUpdateEvent::Used)
                    } else if let Some(item) = inventory.take(
                        slot,
                        &state.ecs().read_resource::<AbilityMap>(),
                        &state.ecs().read_resource::<MaterialStatManifest>(),
                    ) {
                        match &*item.kind() {
                            ItemKind::Consumable { effects, .. } => {
                                maybe_effect = Some(effects.clone());
                                Some(InventoryUpdateEvent::Consumed(item.name().into_owned()))
                            },
                            ItemKind::Throwable { kind, .. } => {
                                if let Some(pos) =
                                    state.ecs().read_storage::<comp::Pos>().get(entity)
                                {
                                    let controllers =
                                        state.ecs().read_storage::<comp::Controller>();
                                    let controller = controllers.get(entity);
                                    let look_dir = controller
                                        .map_or_else(Vec3::zero, |c| c.inputs.look_dir.to_vec());
                                    thrown_items.push((
                                        *pos,
                                        state
                                            .read_component_copied::<comp::Vel>(entity)
                                            .unwrap_or_default(),
                                        look_dir,
                                        *kind,
                                    ));
                                }
                                Some(InventoryUpdateEvent::Used)
                            },
                            ItemKind::Utility {
                                kind: item::Utility::Collar,
                                ..
                            } => {
                                const MAX_PETS: usize = 3;
                                let reinsert = if let Some(pos) =
                                    state.read_storage::<comp::Pos>().get(entity)
                                {
                                    if (
                                        &state.read_storage::<Alignment>(),
                                        &state.read_storage::<comp::Agent>(),
                                    )
                                        .join()
                                        .filter(|(alignment, _)| {
                                            alignment == &&comp::Alignment::Owned(uid)
                                        })
                                        .count()
                                        >= MAX_PETS
                                    {
                                        true
                                    } else if let Some(tameable_entity) = {
                                        let nearest_tameable = (
                                            &state.ecs().entities(),
                                            &state.ecs().read_storage::<Body>(),
                                            &state.ecs().read_storage::<comp::Pos>(),
                                            &state.ecs().read_storage::<Alignment>(),
                                        )
                                            .join()
                                            .filter(|(_, _, wild_pos, _)| {
                                                wild_pos.0.distance_squared(pos.0) < 5.0f32.powi(2)
                                            })
                                            .filter(|(_, body, _, alignment)| {
                                                alignment == &&Alignment::Wild && is_tameable(body)
                                            })
                                            .min_by_key(|(_, _, wild_pos, _)| {
                                                (wild_pos.0.distance_squared(pos.0) * 100.0) as i32
                                            })
                                            .map(|(entity, _, _, _)| entity);
                                        nearest_tameable
                                    } {
                                        let server_eventbus =
                                            state.ecs().read_resource::<EventBus<ServerEvent>>();
                                        server_eventbus.emit_now(ServerEvent::TamePet {
                                            owner_entity: entity,
                                            pet_entity: tameable_entity,
                                        });
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

                                Some(InventoryUpdateEvent::Used)
                            },
                            _ => {
                                inventory.insert_or_stack_at(slot, item).expect(
                                    "slot was just vacated of item, so it definitely fits there.",
                                );
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
                    Some(InventoryUpdateEvent::Used)
                },
            };

            drop(inventories);

            if let Some(effects) = maybe_effect {
                match effects {
                    item::Effects::Any(effects) => {
                        if let Some(effect) = effects.into_iter().choose(&mut rand::thread_rng()) {
                            state.apply_effect(entity, effect, None);
                        }
                    },
                    item::Effects::All(effects) => {
                        for effect in effects {
                            state.apply_effect(entity, effect, None);
                        }
                    },
                    item::Effects::One(effect) => {
                        state.apply_effect(entity, effect, None);
                    },
                }
            }
            if let Some(event) = event {
                state
                    .ecs()
                    .write_storage()
                    .insert(entity, comp::InventoryUpdate::new(event))
                    .expect("We know entity exists since we got its inventory.");
            }
        },
        comp::InventoryManip::Swap(a, b) => {
            use item::ItemKind;
            let ecs = state.ecs();

            if let Some(lantern_info) = match (a, b) {
                // Only current possible lantern swap is between Slot::Inventory and Slot::Equip
                // add more cases if needed
                (Slot::Equip(slot::EquipSlot::Lantern), Slot::Inventory(slot))
                | (Slot::Inventory(slot), Slot::Equip(slot::EquipSlot::Lantern)) => {
                    inventory.get(slot).and_then(|i| match &*i.kind() {
                        ItemKind::Lantern(lantern) => Some((lantern.color(), lantern.strength())),
                        _ => None,
                    })
                },
                _ => None,
            } {
                swap_lantern(&mut ecs.write_storage(), entity, lantern_info);
            }

            if let Some(pos) = ecs.read_storage::<comp::Pos>().get(entity) {
                let mut merged_stacks = false;

                // If both slots have items and we're attempting to drag from one stack
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
            drop(inventories);

            state
                .ecs()
                .write_storage()
                .insert(
                    entity,
                    comp::InventoryUpdate::new(InventoryUpdateEvent::Swapped),
                )
                .expect("We know entity exists since we got its inventory.");
        },
        comp::InventoryManip::SplitSwap(slot, target) => {
            let ability_map = state.ecs().read_resource::<AbilityMap>();
            let msm = state.ecs().read_resource::<MaterialStatManifest>();

            // If both slots have items and we're attempting to split from one stack
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
                Slot::Inventory(slot) => inventory.take_half(slot, &ability_map, &msm),
                Slot::Equip(_) => None,
            };

            if let Some(item) = item {
                if let Slot::Inventory(target) = target {
                    inventory.insert_or_stack_at(target, item).ok();
                }
            }
            drop(msm);

            state
                .ecs()
                .write_storage()
                .insert(
                    entity,
                    comp::InventoryUpdate::new(InventoryUpdateEvent::Swapped),
                )
                .expect("We know entity exists since we got its inventory.");
            drop(inventories);
        },
        comp::InventoryManip::Drop(slot) => {
            let item = match slot {
                Slot::Inventory(slot) => inventory.remove(slot),
                Slot::Equip(slot) => inventory.replace_loadout_item(slot, None),
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
            state
                .ecs()
                .write_storage()
                .insert(
                    entity,
                    comp::InventoryUpdate::new(InventoryUpdateEvent::Dropped),
                )
                .expect("We know entity exists since we got its inventory.");
            drop(inventories);
        },
        comp::InventoryManip::SplitDrop(slot) => {
            let ability_map = &state.ecs().read_resource::<AbilityMap>();
            let msm = state.ecs().read_resource::<MaterialStatManifest>();
            let item = match slot {
                Slot::Inventory(slot) => inventory.take_half(slot, ability_map, &msm),
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
            state
                .ecs()
                .write_storage()
                .insert(
                    entity,
                    comp::InventoryUpdate::new(InventoryUpdateEvent::Dropped),
                )
                .expect("We know entity exists since we got its inventory.");
            drop(inventories);
        },
        comp::InventoryManip::CraftRecipe {
            craft_event,
            craft_sprite,
        } => {
            use comp::controller::CraftEvent;
            use recipe::ComponentKey;
            let recipe_book = default_recipe_book().read();
            let ability_map = &state.ecs().read_resource::<AbilityMap>();
            let msm = state.ecs().read_resource::<MaterialStatManifest>();

            let get_craft_sprite = |state, sprite_pos: Option<Vec3<i32>>| {
                sprite_pos
                    .filter(|pos| {
                        let entity_cylinder = get_cylinder(state, entity);
                        let in_range = within_pickup_range(entity_cylinder, || {
                            Some(find_dist::Cube {
                                min: pos.as_(),
                                side_length: 1.0,
                            })
                        });
                        if !in_range {
                            debug!(
                                ?entity_cylinder,
                                "Failed to craft recipe as not within range of required sprite, \
                                 sprite pos: {}",
                                pos
                            );
                        }
                        in_range
                    })
                    .and_then(|pos| state.terrain().get(pos).ok().copied())
                    .and_then(|block| block.get_sprite())
            };

            let crafted_items = match craft_event {
                CraftEvent::Simple {
                    recipe,
                    slots,
                    amount,
                } => recipe_book
                    .get(&recipe)
                    .filter(|r| {
                        if let Some(needed_sprite) = r.craft_sprite {
                            let sprite = get_craft_sprite(state, craft_sprite);
                            Some(needed_sprite) == sprite
                        } else {
                            true
                        }
                    })
                    .and_then(|r| {
                        let items = (0..amount)
                            .into_iter()
                            .filter_map(|_| {
                                r.craft_simple(
                                    &mut inventory,
                                    slots.clone(),
                                    &state.ecs().read_resource::<AbilityMap>(),
                                    &state.ecs().read_resource::<MaterialStatManifest>(),
                                )
                                .ok()
                            })
                            .flatten()
                            .collect::<Vec<_>>();

                        if items.is_empty() { None } else { Some(items) }
                    }),
                CraftEvent::Salvage(slot) => {
                    let sprite = get_craft_sprite(state, craft_sprite);
                    if matches!(sprite, Some(SpriteKind::DismantlingBench)) {
                        recipe::try_salvage(&mut inventory, slot, ability_map, &msm).ok()
                    } else {
                        None
                    }
                },
                CraftEvent::ModularWeapon {
                    primary_component,
                    secondary_component,
                } => {
                    let sprite = get_craft_sprite(state, craft_sprite);
                    if matches!(sprite, Some(SpriteKind::CraftingBench)) {
                        recipe::modular_weapon(
                            &mut inventory,
                            primary_component,
                            secondary_component,
                            ability_map,
                            &msm,
                        )
                        .ok()
                        .map(|item| vec![item])
                    } else {
                        None
                    }
                },
                CraftEvent::ModularWeaponPrimaryComponent {
                    toolkind,
                    material,
                    modifier,
                    slots,
                } => {
                    let component_recipes = default_component_recipe_book().read();
                    let item_id = |slot| {
                        inventory.get(slot).and_then(|item| {
                            item.item_definition_id().itemdef_id().map(String::from)
                        })
                    };
                    if let Some(material_item_id) = item_id(material) {
                        component_recipes
                            .get(&ComponentKey {
                                toolkind,
                                material: material_item_id,
                                modifier: modifier.and_then(item_id),
                            })
                            .filter(|r| {
                                if let Some(needed_sprite) = r.craft_sprite {
                                    let sprite = get_craft_sprite(state, craft_sprite);
                                    Some(needed_sprite) == sprite
                                } else {
                                    true
                                }
                            })
                            .and_then(|r| {
                                r.craft_component(
                                    &mut inventory,
                                    material,
                                    modifier,
                                    slots,
                                    &state.ecs().read_resource::<AbilityMap>(),
                                    &state.ecs().read_resource::<MaterialStatManifest>(),
                                )
                                .ok()
                            })
                    } else {
                        None
                    }
                },
            };

            // Attempt to insert items into inventory, dropping them if there is not enough
            // space
            let items_were_crafted = if let Some(crafted_items) = crafted_items {
                for item in crafted_items {
                    if let Err(item) = inventory.push(item) {
                        dropped_items.push((
                            state
                                .read_component_copied::<comp::Pos>(entity)
                                .unwrap_or_default(),
                            state
                                .read_component_copied::<comp::Ori>(entity)
                                .unwrap_or_default(),
                            item.duplicate(ability_map, &msm),
                        ));
                    }
                }
                true
            } else {
                false
            };

            drop(inventories);

            // FIXME: We should really require the drop and write to be atomic!
            if items_were_crafted {
                let _ = state.ecs().write_storage().insert(
                    entity,
                    comp::InventoryUpdate::new(InventoryUpdateEvent::Craft),
                );
            }
        },
        comp::InventoryManip::Sort => {
            inventory.sort();
            drop(inventories);
        },
        comp::InventoryManip::SwapEquippedWeapons => {
            inventory.swap_equipped_weapons();
            drop(inventories);
        },
    }

    // Drop items, Debug items should simply disappear when dropped
    for (pos, ori, mut item) in dropped_items
        .into_iter()
        .filter(|(_, _, i)| !matches!(i.quality(), item::Quality::Debug))
    {
        // If item is a container check inside of it for Debug items and remove them
        item.slots_mut().iter_mut().for_each(|x| {
            if let Some(contained_item) = &x {
                if matches!(contained_item.quality(), item::Quality::Debug) {
                    std::mem::take(x);
                }
            }
        });

        state
            .create_item_drop(Default::default(), item)
            .with(comp::Pos(pos.0 + *ori.look_dir() + Vec3::unit_z()))
            .with(comp::Vel(Vec3::zero()))
            .build();
    }

    let mut rng = rand::thread_rng();

    // Throw items
    for (pos, vel, look_dir, kind) in thrown_items {
        let vel = match kind {
            item::Throwable::Firework(_) => Vec3::new(
                rng.gen_range(-15.0..15.0),
                rng.gen_range(-15.0..15.0),
                rng.gen_range(80.0..110.0),
            ),
            _ => vel.0 + look_dir * 20.0,
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
                new_entity = new_entity.with(comp::Stats::new(
                    "Training Dummy".to_string(),
                    Body::Object(common::comp::object::Body::TrainingDummy),
                ));
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

fn announce_loot_to_group(
    group_id: &Group,
    ecs: &specs::World,
    entity: EcsEntity,
    item: comp::Item,
) {
    let clients = ecs.read_storage::<Client>();
    let uids = ecs.read_storage();
    if let Some(uid) = uids.get(entity) {
        members(
            *group_id,
            &ecs.read_storage(),
            &ecs.entities(),
            &ecs.read_storage(),
            &ecs.read_storage::<Uid>(),
        )
        .filter(|(member_e, _)| member_e != &entity)
        .for_each(|(e, _)| {
            clients.get(e).and_then(|c| {
                ecs.read_storage::<comp::Stats>().get(entity).map(|stats| {
                    c.send_fallible(ServerGeneral::GroupInventoryUpdate(
                        item.duplicate(
                            &ecs.read_resource::<AbilityMap>(),
                            &ecs.read_resource::<MaterialStatManifest>(),
                        ),
                        stats.name.to_string(),
                        *uid,
                    ))
                })
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use vek::Vec3;

    use common::comp::Pos;
    use find_dist::*;

    use super::*;

    // Helper function
    fn test_cylinder(pos: Pos) -> Option<Cylinder> {
        Some(Cylinder::from_components(pos.0, None, None, None))
    }

    #[test]
    fn pickup_distance_within_range() {
        let position = Pos(Vec3::zero());
        let item_position = Pos(Vec3::one());

        assert!(within_pickup_range(test_cylinder(position), || {
            test_cylinder(item_position)
        },),);
    }

    #[test]
    fn pickup_distance_not_within_range() {
        let position = Pos(Vec3::zero());
        let item_position = Pos(Vec3::one() * 500.0);

        assert!(!within_pickup_range(test_cylinder(position), || {
            test_cylinder(item_position)
        },),);
    }
}
