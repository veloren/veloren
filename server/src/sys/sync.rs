use crate::{
    client::{Client, RegionSubscription},
    Tick,
};
use common::{
    comp::{CharacterState, ForceUpdate, Inventory, InventoryUpdate, Last, Ori, Pos, Vel},
    msg::ServerMsg,
    region::{Event as RegionEvent, RegionMap},
    state::Uid,
};
use specs::{
    Entities, Entity as EcsEntity, Join, Read, ReadExpect, ReadStorage, System, WriteStorage,
};

/// This system will send physics updates to the client
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, Tick>,
        ReadExpect<'a, RegionMap>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Vel>,
        ReadStorage<'a, Ori>,
        ReadStorage<'a, CharacterState>,
        ReadStorage<'a, Inventory>,
        ReadStorage<'a, RegionSubscription>,
        WriteStorage<'a, Last<Pos>>,
        WriteStorage<'a, Last<Vel>>,
        WriteStorage<'a, Last<Ori>>,
        WriteStorage<'a, Last<CharacterState>>,
        WriteStorage<'a, Client>,
        WriteStorage<'a, ForceUpdate>,
        WriteStorage<'a, InventoryUpdate>,
    );

    fn run(
        &mut self,
        (
            entities,
            tick,
            region_map,
            uids,
            positions,
            velocities,
            orientations,
            character_states,
            inventories,
            subscriptions,
            mut last_pos,
            mut last_vel,
            mut last_ori,
            mut last_character_state,
            mut clients,
            mut force_updates,
            mut inventory_updates,
        ): Self::SystemData,
    ) {
        let tick = tick.0;
        // To send entity updates
        // 1. Iterate through regions
        // 2. Iterate through region subscribers (ie clients)
        //     - Collect a list of entity ids for clients who are subscribed to this region (hash calc to check each)
        // 3. Iterate through events from that region
        //     - For each entity left event, iterate through the client list and check if they are subscribed to the destination (hash calc per subscribed client per entity left event)
        //     - Do something with entity entered events when sphynx is removed??
        // 4. Iterate through entities in that region
        // 5. Inform clients of the component changes for that entity
        //     - Throttle update rate base on distance to each client

        // Sync physics
        // via iterating through regions
        for (key, region) in region_map.iter() {
            let mut subscribers = (&mut clients, &entities, &subscriptions, &positions)
                .join()
                .filter_map(|(client, entity, subscription, pos)| {
                    if client.is_ingame() && subscription.regions.contains(&key) {
                        Some((client, &subscription.regions, entity, *pos))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            for event in region.events() {
                match event {
                    RegionEvent::Entered(_, _) => {} // TODO use this
                    RegionEvent::Left(id, maybe_key) => {
                        // Lookup UID for entity
                        if let Some(&uid) = uids.get(entities.entity(*id)) {
                            for (client, regions, _, _) in &mut subscribers {
                                if !maybe_key
                                    .as_ref()
                                    .map(|key| regions.contains(key))
                                    .unwrap_or(false)
                                {
                                    client.notify(ServerMsg::DeleteEntity(uid.into()));
                                }
                            }
                        }
                    }
                }
            }

            let mut send_msg =
                |msg: ServerMsg, entity: EcsEntity, pos: Pos, force_update, throttle: bool| {
                    for (client, _, client_entity, client_pos) in &mut subscribers {
                        match force_update {
                            None if client_entity == &entity => {}
                            _ => {
                                let distance_sq = client_pos.0.distance_squared(pos.0);

                                // Throttle update rate based on distance to player
                                let update = if !throttle || distance_sq < 100.0f32.powi(2) {
                                    true // Closer than 100.0 blocks
                                } else if distance_sq < 150.0f32.powi(2) {
                                    (tick + entity.id() as u64) % 2 == 0
                                } else if distance_sq < 200.0f32.powi(2) {
                                    (tick + entity.id() as u64) % 4 == 0
                                } else if distance_sq < 250.0f32.powi(2) {
                                    (tick + entity.id() as u64) % 8 == 0
                                } else if distance_sq < 300.0f32.powi(2) {
                                    (tick + entity.id() as u64) % 16 == 0
                                } else {
                                    (tick + entity.id() as u64) % 32 == 0
                                };

                                if update {
                                    client.notify(msg.clone());
                                }
                            }
                        }
                    }
                };

            for (_, entity, &uid, &pos, maybe_vel, maybe_ori, character_state, force_update) in (
                region.entities(),
                &entities,
                &uids,
                &positions,
                velocities.maybe(),
                orientations.maybe(),
                character_states.maybe(),
                force_updates.maybe(),
            )
                .join()
            {
                if last_pos.get(entity).map(|&l| l.0 != pos).unwrap_or(true) {
                    let _ = last_pos.insert(entity, Last(pos));
                    send_msg(
                        ServerMsg::EntityPos {
                            entity: uid.into(),
                            pos,
                        },
                        entity,
                        pos,
                        force_update,
                        true,
                    );
                }

                if let Some(&vel) = maybe_vel {
                    if last_vel.get(entity).map(|&l| l.0 != vel).unwrap_or(true) {
                        let _ = last_vel.insert(entity, Last(vel));
                        send_msg(
                            ServerMsg::EntityVel {
                                entity: uid.into(),
                                vel,
                            },
                            entity,
                            pos,
                            force_update,
                            true,
                        );
                    }
                }

                if let Some(&ori) = maybe_ori {
                    if last_ori.get(entity).map(|&l| l.0 != ori).unwrap_or(true) {
                        let _ = last_ori.insert(entity, Last(ori));
                        send_msg(
                            ServerMsg::EntityOri {
                                entity: uid.into(),
                                ori,
                            },
                            entity,
                            pos,
                            force_update,
                            true,
                        );
                    }
                }

                if let Some(&character_state) = character_state {
                    if last_character_state
                        .get(entity)
                        .map(|&l| !character_state.is_same_state(&l.0))
                        .unwrap_or(true)
                    {
                        let _ = last_character_state.insert(entity, Last(character_state));
                        send_msg(
                            ServerMsg::EntityCharacterState {
                                entity: uid.into(),
                                character_state,
                            },
                            entity,
                            pos,
                            force_update,
                            false,
                        );
                    }
                }
            }
        }

        // Sync inventories
        for (client, inventory, _) in (&mut clients, &inventories, &inventory_updates).join() {
            client.notify(ServerMsg::InventoryUpdate(inventory.clone()));
        }

        // Remove all force flags.
        force_updates.clear();
        inventory_updates.clear();
    }
}
