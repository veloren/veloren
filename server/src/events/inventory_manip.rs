use crate::{Server, StateExt};
use common::{
    comp::{self, Pos, MAX_PICKUP_RANGE_SQR},
    sync::WorldSyncExt,
    terrain::block::Block,
    vol::{ReadVol, Vox},
};
use log::error;
use rand::Rng;
use specs::{join::Join, world::WorldExt, Builder, Entity as EcsEntity};
use vek::Vec3;

pub fn handle_inventory(server: &mut Server, entity: EcsEntity, manip: comp::InventoryManip) {
    let state = server.state_mut();
    let mut dropped_items = Vec::new();

    match manip {
        comp::InventoryManip::Pickup(uid) => {
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
                if within_pickup_range(
                    state.ecs().read_storage::<comp::Pos>().get(entity),
                    state.ecs().read_storage::<comp::Pos>().get(item_entity),
                ) && inv.push(item).is_none()
                {
                    Some(item_entity)
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(item_entity) = item_entity {
                if let Err(err) = state.delete_entity_recorded(item_entity) {
                    error!("Failed to delete picked up item entity: {:?}", err);
                }
            }

            state.write_component(
                entity,
                comp::InventoryUpdate::new(comp::InventoryUpdateEvent::Collected),
            );
        },

        comp::InventoryManip::Collect(pos) => {
            let block = state.terrain().get(pos).ok().copied();

            if let Some(block) = block {
                if block.is_collectible()
                    && state
                        .ecs()
                        .read_storage::<comp::Inventory>()
                        .get(entity)
                        .map(|inv| !inv.is_full())
                        .unwrap_or(false)
                    && state.try_set_block(pos, Block::empty()).is_some()
                {
                    comp::Item::try_reclaim_from_block(block)
                        .map(|item| state.give_item(entity, item));
                }
            }
        },

        comp::InventoryManip::Use(slot) => {
            let item_opt = state
                .ecs()
                .write_storage::<comp::Inventory>()
                .get_mut(entity)
                .and_then(|inv| inv.remove(slot));

            let mut event = comp::InventoryUpdateEvent::Used;

            if let Some(item) = item_opt {
                match item.kind {
                    comp::ItemKind::Tool { .. } => {
                        if let Some(stats) =
                            state.ecs().write_storage::<comp::Stats>().get_mut(entity)
                        {
                            // Insert old item into inventory
                            if let Some(old_item) = stats.equipment.main.take() {
                                state
                                    .ecs()
                                    .write_storage::<comp::Inventory>()
                                    .get_mut(entity)
                                    .map(|inv| inv.insert(slot, old_item));
                            }

                            stats.equipment.main = Some(item);
                        }
                    },
                    comp::ItemKind::Consumable { kind, effect } => {
                        event = comp::InventoryUpdateEvent::Consumed(kind);
                        state.apply_effect(entity, effect);
                    },
                    comp::ItemKind::Utility { kind } => match kind {
                        comp::item::Utility::Collar => {
                            let reinsert = if let Some(pos) =
                                state.read_storage::<comp::Pos>().get(entity)
                            {
                                if (
                                    &state.read_storage::<comp::Alignment>(),
                                    &state.read_storage::<comp::Agent>(),
                                )
                                    .join()
                                    .filter(|(alignment, _)| {
                                        alignment == &&comp::Alignment::Owned(entity)
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
                                            wild_pos.0.distance_squared(pos.0) < 5.0f32.powf(2.0)
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
                                        .insert(tameable_entity, comp::Alignment::Owned(entity));
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
                                let _ = state
                                    .ecs()
                                    .write_storage::<comp::Inventory>()
                                    .get_mut(entity)
                                    .map(|inv| inv.insert(slot, item));
                            }
                        },
                    },
                    _ => {
                        let _ = state
                            .ecs()
                            .write_storage::<comp::Inventory>()
                            .get_mut(entity)
                            .map(|inv| inv.insert(slot, item));
                    },
                }
            }

            state.write_component(entity, comp::InventoryUpdate::new(event));
        },

        comp::InventoryManip::Swap(a, b) => {
            state
                .ecs()
                .write_storage::<comp::Inventory>()
                .get_mut(entity)
                .map(|inv| inv.swap_slots(a, b));
            state.write_component(
                entity,
                comp::InventoryUpdate::new(comp::InventoryUpdateEvent::Swapped),
            );
        },

        comp::InventoryManip::Drop(slot) => {
            let item = state
                .ecs()
                .write_storage::<comp::Inventory>()
                .get_mut(entity)
                .and_then(|inv| inv.remove(slot));

            if let (Some(item), Some(pos)) =
                (item, state.ecs().read_storage::<comp::Pos>().get(entity))
            {
                dropped_items.push((
                    *pos,
                    state
                        .ecs()
                        .read_storage::<comp::Ori>()
                        .get(entity)
                        .copied()
                        .unwrap_or(comp::Ori(Vec3::unit_y())),
                    item,
                ));
            }
            state.write_component(
                entity,
                comp::InventoryUpdate::new(comp::InventoryUpdateEvent::Dropped),
            );
        },
    }

    // Drop items
    for (pos, ori, item) in dropped_items {
        let vel = ori.0.normalized() * 5.0
            + Vec3::unit_z() * 10.0
            + Vec3::<f32>::zero().map(|_| rand::thread_rng().gen::<f32>() - 0.5) * 4.0;

        server
            .create_object(Default::default(), comp::object::Body::Pouch)
            .with(comp::Pos(pos.0 + Vec3::unit_z() * 0.25))
            .with(item)
            .with(comp::Vel(vel))
            .build();
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
