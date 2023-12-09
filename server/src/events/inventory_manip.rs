use hashbrown::HashSet;
use rand::{seq::IteratorRandom, Rng};
use specs::{
    join::Join, shred, DispatcherBuilder, Entities, Entity as EcsEntity, Read, ReadExpect,
    ReadStorage, SystemData, Write, WriteStorage,
};
use tracing::{debug, error, warn};
use vek::{Rgb, Vec3};

use common::{
    comp::{
        self,
        group::members,
        item::{self, flatten_counted_items, tool::AbilityMap, MaterialStatManifest},
        loot_owner::LootOwnerKind,
        slot::{self, Slot},
        InventoryUpdate, LootOwner,
    },
    consts::MAX_PICKUP_RANGE,
    event::{
        BuffEvent, CreateItemDropEvent, CreateObjectEvent, DeleteEvent, EmitExt, HealthChangeEvent,
        InventoryManipEvent, PoiseChangeEvent, TamePetEvent,
    },
    event_emitters,
    mounting::VolumePos,
    recipe::{
        self, default_component_recipe_book, default_recipe_book, default_repair_recipe_book,
    },
    resources::Time,
    terrain::{Block, SpriteKind},
    trade::Trades,
    uid::{IdMaps, Uid},
    util::find_dist::{self, FindDist},
    vol::ReadVol,
};
use comp::LightEmitter;

use crate::client::Client;
use common::comp::{
    pet::is_tameable, Alignment, Body, CollectFailedReason, Group, InventoryUpdateEvent,
};
use common_net::msg::ServerGeneral;

use super::{entity_manipulation::emit_effect_events, event_dispatch, ServerEvent};

pub(super) fn register_event_systems(builder: &mut DispatcherBuilder) {
    event_dispatch::<InventoryManipEvent>(builder);
}

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

event_emitters! {
    struct Events[Emitters] {
        tame_pet: TamePetEvent,
        delete: DeleteEvent,
        create_item_drop: CreateItemDropEvent,
        create_object: CreateObjectEvent,
        health_change: HealthChangeEvent,
        poise_change: PoiseChangeEvent,
        buff: BuffEvent,
    }
}
#[derive(SystemData)]
pub struct InventoryManipData<'a> {
    entities: Entities<'a>,
    events: Events<'a>,
    block_change: Write<'a, common_state::BlockChange>,
    trades: Write<'a, Trades>,
    terrain: ReadExpect<'a, common::terrain::TerrainGrid>,
    id_maps: Read<'a, IdMaps>,
    time: Read<'a, Time>,
    ability_map: ReadExpect<'a, AbilityMap>,
    msm: ReadExpect<'a, MaterialStatManifest>,
    inventories: WriteStorage<'a, comp::Inventory>,
    items: WriteStorage<'a, comp::Item>,
    inventory_updates: WriteStorage<'a, comp::InventoryUpdate>,
    light_emitters: WriteStorage<'a, comp::LightEmitter>,
    positions: ReadStorage<'a, comp::Pos>,
    scales: ReadStorage<'a, comp::Scale>,
    colliders: ReadStorage<'a, comp::Collider>,
    character_states: ReadStorage<'a, comp::CharacterState>,
    healths: ReadStorage<'a, comp::Health>,
    uids: ReadStorage<'a, Uid>,
    loot_owners: ReadStorage<'a, comp::LootOwner>,
    alignments: ReadStorage<'a, comp::Alignment>,
    bodies: ReadStorage<'a, comp::Body>,
    players: ReadStorage<'a, comp::Player>,
    groups: ReadStorage<'a, comp::Group>,
    stats: ReadStorage<'a, comp::Stats>,
    clients: ReadStorage<'a, Client>,
    orientations: ReadStorage<'a, comp::Ori>,
    controllers: ReadStorage<'a, comp::Controller>,
    agents: ReadStorage<'a, comp::Agent>,
    velocities: ReadStorage<'a, comp::Vel>,
}

impl ServerEvent for InventoryManipEvent {
    type SystemData<'a> = InventoryManipData<'a>;

    fn handle(events: impl ExactSizeIterator<Item = Self>, mut data: Self::SystemData<'_>) {
        let mut emitters = data.events.get_emitters();
        let get_cylinder = |entity| {
            data.positions.get(entity).map(|p| {
                find_dist::Cylinder::from_components(
                    p.0,
                    data.scales.get(entity).copied(),
                    data.colliders.get(entity),
                    data.character_states.get(entity),
                )
            })
        };
        let mut rng = rand::thread_rng();

        let mut dropped_items = Vec::new();
        let mut thrown_items = Vec::new();

        for InventoryManipEvent(entity, manip) in events {
            let uid = if let Some(uid) = data.uids.get(entity) {
                uid
            } else {
                warn!(
                    "Couldn't get uid for entity {:?} at start of handle_inventory",
                    entity
                );
                continue;
            };
            if data.trades.in_immutable_trade(uid) {
                // manipulating the inventory can mutate the trade
                continue;
            }

            let mut inventory = if let Some(inventory) = data.inventories.get_mut(entity) {
                inventory
            } else {
                error!(
                    ?entity,
                    "Can't manipulate inventory, entity doesn't have one"
                );
                continue;
            };
            // Disallow inventory manipulation while dead
            if data.healths.get(entity).map_or(false, |h| h.is_dead) {
                debug!("Can't manipulate inventory; entity is dead");
                continue;
            }
            match manip {
                comp::InventoryManip::Pickup(pickup_uid) => {
                    let item_entity = if let Some(item_entity) = data.id_maps.uid_entity(pickup_uid)
                    {
                        item_entity
                    } else {
                        // Item entity could not be found - most likely because the entity
                        // attempted to pick up the same item very quickly before its deletion
                        // of the world from the first pickup
                        // attempt was processed.
                        debug!("Failed to get entity for item Uid: {}", pickup_uid);
                        return;
                    };
                    let entity_cylinder = get_cylinder(entity);

                    // FIXME: Raycast so we can't pick up items through walls.
                    if !within_pickup_range(entity_cylinder, || get_cylinder(item_entity)) {
                        debug!(
                            ?entity_cylinder,
                            "Failed to pick up item as not within range, Uid: {}", pickup_uid
                        );
                        return;
                    }

                    // If there's a loot owner for the item being picked up, then
                    // determine whether the pickup should be rejected.
                    let ownership_check_passed =
                        data.loot_owners
                            .get(item_entity)
                            .map_or(true, |loot_owner| {
                                let can_pickup = loot_owner.can_pickup(
                                    *uid,
                                    data.groups.get(entity),
                                    data.alignments.get(entity),
                                    data.stats
                                        .get(entity)
                                        .map(|stats| &stats.original_body)
                                        .or_else(|| data.bodies.get(entity)),
                                    data.players.get(entity),
                                );
                                if !can_pickup {
                                    let event = comp::InventoryUpdate::new(
                                        InventoryUpdateEvent::EntityCollectFailed {
                                            entity: pickup_uid,
                                            reason: CollectFailedReason::LootOwned {
                                                owner: loot_owner.owner(),
                                                expiry_secs: loot_owner
                                                    .time_until_expiration()
                                                    .as_secs(),
                                            },
                                        },
                                    );
                                    data.inventory_updates.insert(entity, event).unwrap();
                                }
                                can_pickup
                            });

                    if !ownership_check_passed {
                        return;
                    }

                    // First, we remove the item, assuming picking it up will succeed (we do this to
                    // avoid cloning the item, as we should not call Item::clone and it
                    // may be removed!).
                    let item = if let Some(item) = data.items.remove(item_entity) {
                        item
                    } else {
                        // Item component could not be found - most likely because the entity
                        // attempted to pick up the same item very quickly before its deletion of
                        // the world from the first pickup attempt was
                        // processed.
                        debug!(
                            "Failed to delete item component for entity, Uid: {}",
                            pickup_uid
                        );
                        return;
                    };

                    // NOTE: We dup the item for message purposes.
                    let item_msg = item.duplicate(&data.ability_map, &data.msm);

                    // Next, we try to equip the picked up item
                    let event = match inventory.try_equip(item).or_else(|returned_item| {
                        // If we couldn't equip it (no empty slot for it or unequippable) then
                        // attempt to add the item to the entity's inventory
                        inventory.pickup_item(returned_item)
                    }) {
                        Err(returned_item) => {
                            // Inventory was full, so we need to put back the item (note that we
                            // know there was no old item component for
                            // this entity).
                            data.items.insert(item_entity, returned_item).expect(
                                "We know item_entity exists since we just successfully removed \
                                 its Item component.",
                            );
                            comp::InventoryUpdate::new(InventoryUpdateEvent::EntityCollectFailed {
                                entity: pickup_uid,
                                reason: CollectFailedReason::InventoryFull,
                            })
                        },
                        Ok(_) => {
                            // We succeeded in picking up the item, so we may now delete its old
                            // entity entirely.
                            emitters.emit(DeleteEvent(item_entity));
                            if let Some(group_id) = data.groups.get(entity) {
                                announce_loot_to_group(
                                    group_id,
                                    entity,
                                    item_msg.duplicate(&data.ability_map, &data.msm),
                                    &data.clients,
                                    &data.uids,
                                    &data.groups,
                                    &data.alignments,
                                    &data.stats,
                                    &data.entities,
                                    &data.ability_map,
                                    &data.msm,
                                );
                            }
                            comp::InventoryUpdate::new(InventoryUpdateEvent::Collected(item_msg))
                        },
                    };

                    data.inventory_updates
                        .insert(entity, event)
                        .expect("We know entity exists since we got its inventory.");
                },
                comp::InventoryManip::Collect {
                    sprite_pos,
                    required_item,
                } => {
                    let block = data.terrain.get(sprite_pos).ok().copied();
                    let mut drop_items = Vec::new();
                    let inventory_update = data
                        .inventory_updates
                        .entry(entity)
                        .expect("We know entity exists since we got its inventory.")
                        .or_insert_with(InventoryUpdate::default);

                    if let Some(block) = block {
                        if block.is_collectible() && data.block_change.can_set_block(sprite_pos) {
                            // If an item was required to collect the sprite, consume it now
                            if let Some((inv_slot, true)) = required_item {
                                inventory.take(inv_slot, &data.ability_map, &data.msm);
                            }

                            // If there are items to be reclaimed from the block, add it to the
                            // inventory
                            if let Some(items) = comp::Item::try_reclaim_from_block(block) {
                                let msm = &MaterialStatManifest::load().read();
                                let ability_map = &AbilityMap::load().read();
                                for item in flatten_counted_items(&items, ability_map, msm) {
                                    // NOTE: We dup the item for message purposes.
                                    let item_msg = item.duplicate(ability_map, msm);
                                    match inventory.push(item) {
                                        Ok(_) => {
                                            if let Some(group_id) = data.groups.get(entity) {
                                                announce_loot_to_group(
                                                    group_id,
                                                    entity,
                                                    item_msg.duplicate(ability_map, msm),
                                                    &data.clients,
                                                    &data.uids,
                                                    &data.groups,
                                                    &data.alignments,
                                                    &data.stats,
                                                    &data.entities,
                                                    &data.ability_map,
                                                    &data.msm,
                                                );
                                            }
                                            inventory_update
                                                .push(InventoryUpdateEvent::Collected(item_msg));
                                        },
                                        // The item we created was in some sense "fake" so it's safe
                                        // to drop it.
                                        Err(_) => {
                                            drop_items.push(item_msg);
                                        },
                                    }
                                }
                            }

                            // We made sure earlier the block was not already modified this tick
                            data.block_change.set(sprite_pos, block.into_vacant());

                            // If the block was a keyhole, remove nearby door blocks
                            // TODO: Abstract this code into a generalised way to do block updates?
                            if let Some(kind_to_destroy) = match block.get_sprite() {
                                Some(SpriteKind::Keyhole) => Some(SpriteKind::KeyDoor),
                                Some(SpriteKind::BoneKeyhole) => Some(SpriteKind::BoneKeyDoor),
                                Some(SpriteKind::HaniwaKeyhole) => Some(SpriteKind::HaniwaKeyDoor),
                                Some(SpriteKind::GlassKeyhole) => Some(SpriteKind::GlassBarrier),
                                Some(SpriteKind::KeyholeBars) => Some(SpriteKind::DoorBars),
                                _ => None,
                            } {
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
                                // TODO: Replace with `entry` eventually
                                while destroyed.len() < 450 {
                                    if let Some(pos) = pending.iter().next().copied() {
                                        pending.remove(&pos);

                                        if !destroyed.contains(&pos)
                                            && data
                                                .terrain
                                                .get(pos)
                                                .ok()
                                                .and_then(|b| b.get_sprite())
                                                == Some(kind_to_destroy)
                                        {
                                            data.block_change.try_set(pos, Block::empty());
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
                                "Can't reclaim item from block at pos={}: block is not \
                                 collectable or was already set this tick.",
                                sprite_pos
                            );
                        }
                    }
                    if !drop_items.is_empty() {
                        inventory_update.push(InventoryUpdateEvent::BlockCollectFailed {
                            pos: sprite_pos,
                            reason: CollectFailedReason::InventoryFull,
                        })
                    }

                    for item in drop_items {
                        emitters.emit(CreateItemDropEvent {
                            pos: comp::Pos(
                                Vec3::new(
                                    sprite_pos.x as f32,
                                    sprite_pos.y as f32,
                                    sprite_pos.z as f32,
                                ) + Vec3::one().with_z(0.0) * 0.5,
                            ),
                            vel: comp::Vel(Vec3::zero()),
                            // TODO: Randomize
                            ori: data.orientations.get(entity).copied().unwrap_or_default(),
                            item,
                            loot_owner: Some(LootOwner::new(LootOwnerKind::Player(*uid), false)),
                        });
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
                                    swap_lantern(&mut data.light_emitters, entity, lantern_info);
                                }
                                if let Some(pos) = data.positions.get(entity) {
                                    dropped_items.extend(
                                        inventory.equip(slot, *data.time).into_iter().map(|x| {
                                            (
                                                *pos,
                                                data.orientations
                                                    .get(entity)
                                                    .copied()
                                                    .unwrap_or_default(),
                                                x,
                                                *uid,
                                            )
                                        }),
                                    );
                                }
                                Some(InventoryUpdateEvent::Used)
                            } else if let Some(item) =
                                inventory.take(slot, &data.ability_map, &data.msm)
                            {
                                match &*item.kind() {
                                    ItemKind::Consumable { effects, .. } => {
                                        maybe_effect = Some(effects.clone());
                                        Some(InventoryUpdateEvent::Consumed((&item).into()))
                                    },
                                    ItemKind::Throwable { kind, .. } => {
                                        if let Some(pos) = data.positions.get(entity) {
                                            let controller = data.controllers.get(entity);
                                            let look_dir = controller
                                                .map_or_else(Vec3::zero, |c| {
                                                    c.inputs.look_dir.to_vec()
                                                });
                                            thrown_items.push((
                                                *pos,
                                                data.velocities
                                                    .get(entity)
                                                    .copied()
                                                    .unwrap_or_default(),
                                                look_dir,
                                                *kind,
                                                *uid,
                                            ));
                                        }
                                        Some(InventoryUpdateEvent::Used)
                                    },
                                    ItemKind::Utility {
                                        kind: item::Utility::Collar,
                                        ..
                                    } => {
                                        const MAX_PETS: usize = 3;
                                        let reinsert = if let Some(pos) = data.positions.get(entity)
                                        {
                                            if (&data.alignments, &data.agents)
                                                .join()
                                                .filter(|(alignment, _)| {
                                                    alignment == &&comp::Alignment::Owned(*uid)
                                                })
                                                .count()
                                                >= MAX_PETS
                                            {
                                                true
                                            } else if let Some(tameable_entity) = {
                                                (
                                                    &data.entities,
                                                    &data.bodies,
                                                    &data.positions,
                                                    &data.alignments,
                                                )
                                                    .join()
                                                    .filter(|(_, _, wild_pos, _)| {
                                                        wild_pos.0.distance_squared(pos.0)
                                                            < 5.0f32.powi(2)
                                                    })
                                                    .filter(|(_, body, _, alignment)| {
                                                        alignment == &&Alignment::Wild
                                                            && is_tameable(body)
                                                    })
                                                    .min_by_key(|(_, _, wild_pos, _)| {
                                                        (wild_pos.0.distance_squared(pos.0) * 100.0)
                                                            as i32
                                                    })
                                                    .map(|(entity, _, _, _)| entity)
                                            } {
                                                emitters.emit(TamePetEvent {
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
                                            "slot was just vacated of item, so it definitely fits \
                                             there.",
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
                                snuff_lantern(&mut data.light_emitters, entity);
                            }

                            if let Some(pos) = data.positions.get(entity) {
                                // Unequip the item, any items that no longer fit within the
                                // inventory (due to unequipping a
                                // bag for example) will be dropped on the floor
                                if let Ok(Some(leftover_items)) =
                                    inventory.unequip(slot, *data.time)
                                {
                                    dropped_items.extend(leftover_items.into_iter().map(|x| {
                                        (
                                            *pos,
                                            data.orientations
                                                .get(entity)
                                                .copied()
                                                .unwrap_or_default(),
                                            x,
                                            *uid,
                                        )
                                    }));
                                }
                            }
                            Some(InventoryUpdateEvent::Used)
                        },
                        // Items in overflow slots cannot be used
                        Slot::Overflow(_) => None,
                    };

                    if let Some(effects) = maybe_effect {
                        match effects {
                            item::Effects::Any(effects) => {
                                if let Some(effect) = effects.into_iter().choose(&mut rng) {
                                    emit_effect_events(
                                        &mut emitters,
                                        *data.time,
                                        entity,
                                        effect,
                                        None,
                                        data.inventories.get(entity),
                                        &data.msm,
                                        data.character_states.get(entity),
                                        data.stats.get(entity),
                                    );
                                }
                            },
                            item::Effects::All(effects) => {
                                for effect in effects {
                                    emit_effect_events(
                                        &mut emitters,
                                        *data.time,
                                        entity,
                                        effect,
                                        None,
                                        data.inventories.get(entity),
                                        &data.msm,
                                        data.character_states.get(entity),
                                        data.stats.get(entity),
                                    );
                                }
                            },
                            item::Effects::One(effect) => {
                                emit_effect_events(
                                    &mut emitters,
                                    *data.time,
                                    entity,
                                    effect,
                                    None,
                                    data.inventories.get(entity),
                                    &data.msm,
                                    data.character_states.get(entity),
                                    data.stats.get(entity),
                                );
                            },
                        }
                    }
                    if let Some(event) = event {
                        data.inventory_updates
                            .insert(entity, comp::InventoryUpdate::new(event))
                            .expect("We know entity exists since we got its inventory.");
                    }
                },
                comp::InventoryManip::Swap(a, b) => {
                    use item::ItemKind;

                    if let Some(lantern_info) = match (a, b) {
                        // Only current possible lantern swap is between Slot::Inventory and
                        // Slot::Equip add more cases if needed
                        (Slot::Equip(slot::EquipSlot::Lantern), Slot::Inventory(slot))
                        | (Slot::Inventory(slot), Slot::Equip(slot::EquipSlot::Lantern)) => {
                            inventory.get(slot).and_then(|i| match &*i.kind() {
                                ItemKind::Lantern(lantern) => {
                                    Some((lantern.color(), lantern.strength()))
                                },
                                _ => None,
                            })
                        },
                        _ => None,
                    } {
                        swap_lantern(&mut data.light_emitters, entity, lantern_info);
                    }

                    if let Some(pos) = data.positions.get(entity) {
                        let mut merged_stacks = false;

                        // If both slots have items and we're attempting to drag from one stack
                        // into another, stack the items.
                        if let (Slot::Inventory(slot_a), Slot::Inventory(slot_b)) = (a, b) {
                            merged_stacks |= inventory.merge_stack_into(slot_a, slot_b);
                        }

                        // If the stacks weren't mergable carry out a swap.
                        if !merged_stacks {
                            dropped_items.extend(inventory.swap(a, b, *data.time).into_iter().map(
                                |x| {
                                    (
                                        *pos,
                                        data.orientations.get(entity).copied().unwrap_or_default(),
                                        x,
                                        *uid,
                                    )
                                },
                            ));
                        }
                    }

                    data.inventory_updates
                        .insert(
                            entity,
                            comp::InventoryUpdate::new(InventoryUpdateEvent::Swapped),
                        )
                        .expect("We know entity exists since we got its inventory.");
                },
                comp::InventoryManip::SplitSwap(slot, target) => {
                    // If both slots have items and we're attempting to split from one stack
                    // into another, ensure that they are the same type of item. If they are
                    // the same type do nothing, as you don't want to overwrite the existing item.

                    if let (
                        Slot::Inventory(source_inv_slot_id),
                        Slot::Inventory(target_inv_slot_id),
                    ) = (slot, target)
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
                        Slot::Inventory(slot) => {
                            inventory.take_half(slot, &data.ability_map, &data.msm)
                        },
                        Slot::Equip(_) => None,
                        Slot::Overflow(_) => None,
                    };

                    if let Some(item) = item {
                        if let Slot::Inventory(target) = target {
                            inventory.insert_or_stack_at(target, item).ok();
                        }
                    }

                    data.inventory_updates
                        .insert(
                            entity,
                            comp::InventoryUpdate::new(InventoryUpdateEvent::Swapped),
                        )
                        .expect("We know entity exists since we got its inventory.");
                },
                comp::InventoryManip::Drop(slot) => {
                    let item = match slot {
                        Slot::Inventory(slot) => inventory.remove(slot),
                        Slot::Equip(slot) => inventory.replace_loadout_item(slot, None, *data.time),
                        Slot::Overflow(slot) => inventory.overflow_remove(slot),
                    };

                    // FIXME: We should really require the drop and write to be atomic!
                    if let (Some(mut item), Some(pos)) = (item, data.positions.get(entity)) {
                        item.put_in_world();
                        dropped_items.push((
                            *pos,
                            data.orientations.get(entity).copied().unwrap_or_default(),
                            item,
                            *uid,
                        ));
                    }
                    data.inventory_updates
                        .insert(
                            entity,
                            comp::InventoryUpdate::new(InventoryUpdateEvent::Dropped),
                        )
                        .expect("We know entity exists since we got its inventory.");
                },
                comp::InventoryManip::SplitDrop(slot) => {
                    let item = match slot {
                        Slot::Inventory(slot) => {
                            inventory.take_half(slot, &data.ability_map, &data.msm)
                        },
                        Slot::Equip(_) => None,
                        Slot::Overflow(o) => {
                            inventory.overflow_take_half(o, &data.ability_map, &data.msm)
                        },
                    };

                    // FIXME: We should really require the drop and write to be atomic!
                    if let (Some(mut item), Some(pos)) = (item, data.positions.get(entity)) {
                        item.put_in_world();
                        dropped_items.push((
                            *pos,
                            data.orientations.get(entity).copied().unwrap_or_default(),
                            item,
                            *uid,
                        ));
                    }
                    data.inventory_updates
                        .insert(
                            entity,
                            comp::InventoryUpdate::new(InventoryUpdateEvent::Dropped),
                        )
                        .expect("We know entity exists since we got its inventory.");
                },
                comp::InventoryManip::CraftRecipe {
                    craft_event,
                    craft_sprite,
                } => {
                    use comp::controller::CraftEvent;
                    use recipe::ComponentKey;
                    let recipe_book = default_recipe_book().read();

                    let get_craft_sprite = |sprite_pos: Option<VolumePos>| {
                        sprite_pos
                            .filter(|pos| {
                                let entity_cylinder = get_cylinder(entity);
                                let in_range = within_pickup_range(entity_cylinder, || {
                                    pos.get_block_and_transform(
                                        &data.terrain,
                                        &data.id_maps,
                                        |e| {
                                            data.positions
                                                .get(e)
                                                .copied()
                                                .zip(data.orientations.get(e).copied())
                                        },
                                        &data.colliders,
                                    )
                                    .map(|(mat, _, _)| mat.mul_point(Vec3::broadcast(0.5)))
                                });
                                if !in_range {
                                    debug!(
                                        ?entity_cylinder,
                                        "Failed to craft recipe as not within range of required \
                                         sprite, sprite pos: {:?}",
                                        pos
                                    );
                                }
                                in_range
                            })
                            .and_then(|pos| {
                                pos.get_block(&data.terrain, &data.id_maps, &data.colliders)
                            })
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
                                    let sprite = get_craft_sprite(craft_sprite);
                                    Some(needed_sprite) == sprite
                                } else {
                                    true
                                }
                            })
                            .and_then(|r| {
                                let items = (0..amount)
                                    .filter_map(|_| {
                                        r.craft_simple(
                                            &mut inventory,
                                            slots.clone(),
                                            &data.ability_map,
                                            &data.msm,
                                        )
                                        .ok()
                                    })
                                    .flatten()
                                    .collect::<Vec<_>>();

                                if items.is_empty() { None } else { Some(items) }
                            }),
                        CraftEvent::Salvage(slot) => {
                            let sprite = get_craft_sprite(craft_sprite);
                            if matches!(sprite, Some(SpriteKind::DismantlingBench)) {
                                recipe::try_salvage(
                                    &mut inventory,
                                    slot,
                                    &data.ability_map,
                                    &data.msm,
                                )
                                .ok()
                            } else {
                                None
                            }
                        },
                        CraftEvent::ModularWeapon {
                            primary_component,
                            secondary_component,
                        } => {
                            let sprite = get_craft_sprite(craft_sprite);
                            if matches!(sprite, Some(SpriteKind::CraftingBench)) {
                                recipe::modular_weapon(
                                    &mut inventory,
                                    primary_component,
                                    secondary_component,
                                    &data.ability_map,
                                    &data.msm,
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
                                            let sprite = get_craft_sprite(craft_sprite);
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
                                            &data.ability_map,
                                            &data.msm,
                                        )
                                        .ok()
                                    })
                            } else {
                                None
                            }
                        },
                        CraftEvent::Repair { item, slots } => {
                            let repair_recipes = default_repair_recipe_book().read();
                            let sprite = get_craft_sprite(craft_sprite);
                            if matches!(sprite, Some(SpriteKind::RepairBench)) {
                                let _ = repair_recipes.repair_item(
                                    &mut inventory,
                                    item,
                                    slots,
                                    &data.ability_map,
                                    &data.msm,
                                );
                            }
                            None
                        },
                    };

                    // Attempt to insert items into inventory, dropping them if there is not enough
                    // space
                    let items_were_crafted = if let Some(crafted_items) = crafted_items {
                        for item in crafted_items {
                            if let Err(item) = inventory.push(item) {
                                if let Some(pos) = data.positions.get(entity) {
                                    dropped_items.push((
                                        *pos,
                                        data.orientations.get(entity).copied().unwrap_or_default(),
                                        item.duplicate(&data.ability_map, &data.msm),
                                        *uid,
                                    ));
                                }
                            }
                        }
                        true
                    } else {
                        false
                    };

                    // FIXME: We should really require the drop and write to be atomic!
                    if items_were_crafted {
                        let _ = data.inventory_updates.insert(
                            entity,
                            comp::InventoryUpdate::new(InventoryUpdateEvent::Craft),
                        );
                    }
                },
                comp::InventoryManip::Sort => {
                    inventory.sort();
                },
                comp::InventoryManip::SwapEquippedWeapons => {
                    inventory.swap_equipped_weapons(*data.time);
                },
            }
            if data.trades.in_mutable_trade(uid) {
                // manipulating the inventory mutated the trade, so reset the accept flags
                data.trades.implicit_mutation_occurred(uid);
            }
        }

        // Drop items, Debug items should simply disappear when dropped
        for (pos, ori, mut item, owner) in dropped_items
            .into_iter()
            .filter(|(_, _, i, _)| !matches!(i.quality(), item::Quality::Debug))
        {
            // If item is a container check inside of it for Debug items and remove them
            item.slots_mut().iter_mut().for_each(|x| {
                if let Some(contained_item) = &x {
                    if matches!(contained_item.quality(), item::Quality::Debug) {
                        std::mem::take(x);
                    }
                }
            });

            emitters.emit(CreateItemDropEvent {
                pos,
                vel: comp::Vel::default(),
                ori,
                item,
                loot_owner: Some(LootOwner::new(LootOwnerKind::Player(owner), true)),
            })
        }

        let mut rng = rand::thread_rng();

        // Throw items
        for (pos, vel, look_dir, kind, owner) in thrown_items {
            let vel = match kind {
                item::Throwable::Firework(_) => Vec3::new(
                    rng.gen_range(-15.0..15.0),
                    rng.gen_range(-15.0..15.0),
                    rng.gen_range(80.0..110.0),
                ),
                _ => vel.0 + look_dir * 20.0,
            };

            emitters.emit(CreateObjectEvent {
                pos: comp::Pos(pos.0 + Vec3::unit_z() * 0.25),
                vel: comp::Vel(vel),
                body: match kind {
                    item::Throwable::Bomb => comp::object::Body::Bomb,
                    item::Throwable::Mine => comp::object::Body::Mine,
                    item::Throwable::Firework(reagent) => comp::object::Body::for_firework(reagent),
                    item::Throwable::TrainingDummy => comp::object::Body::TrainingDummy,
                },
                object: match kind {
                    item::Throwable::Bomb => Some(comp::Object::Bomb { owner: Some(owner) }),
                    item::Throwable::Firework(reagent) => Some(comp::Object::Firework {
                        owner: Some(owner),
                        reagent,
                    }),
                    item::Throwable::Mine => Some(comp::Object::Bomb { owner: Some(owner) }),
                    item::Throwable::TrainingDummy => None,
                },
                light_emitter: match kind {
                    item::Throwable::Firework(_) => Some(LightEmitter {
                        animated: true,
                        flicker: 2.0,
                        strength: 2.0,
                        col: Rgb::new(1.0, 1.0, 0.0),
                    }),
                    _ => None,
                },
                stats: match kind {
                    item::Throwable::TrainingDummy => Some(comp::Stats::new(
                        "Training Dummy".to_string(),
                        Body::Object(common::comp::object::Body::TrainingDummy),
                    )),
                    _ => None,
                },
                item: None,
            });
        }
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
    entity: EcsEntity,
    item: comp::Item,
    clients: &ReadStorage<Client>,
    uids: &ReadStorage<Uid>,
    groups: &ReadStorage<comp::Group>,
    alignments: &ReadStorage<comp::Alignment>,
    stats: &ReadStorage<comp::Stats>,
    entities: &Entities,
    ability_map: &AbilityMap,
    msm: &MaterialStatManifest,
) {
    if let Some(uid) = uids.get(entity) {
        members(*group_id, groups, entities, alignments, uids)
            .filter(|(member_e, _)| member_e != &entity)
            .for_each(|(e, _)| {
                clients.get(e).and_then(|c| {
                    stats.get(entity).map(|stats| {
                        c.send_fallible(ServerGeneral::GroupInventoryUpdate(
                            item.duplicate(ability_map, msm),
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
