use super::{
    sentinel::{DeletedEntities, ReadTrackers, TrackedComps},
    SysTimer,
};
use crate::{
    client::{Client, RegionSubscription},
    Tick,
};
use common::{
    comp::{CharacterState, ForceUpdate, Inventory, InventoryUpdate, Last, Ori, Pos, Vel},
    msg::ServerMsg,
    region::{Event as RegionEvent, RegionMap},
    state::TimeOfDay,
    sync::Uid,
};
use specs::{
    Entities, Entity as EcsEntity, Join, Read, ReadExpect, ReadStorage, System, Write, WriteStorage,
};

/// This system will send physics updates to the client
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, Tick>,
        ReadExpect<'a, TimeOfDay>,
        ReadExpect<'a, RegionMap>,
        Write<'a, SysTimer<Self>>,
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
        Write<'a, DeletedEntities>,
        TrackedComps<'a>,
        ReadTrackers<'a>,
    );

    fn run(
        &mut self,
        (
            entities,
            tick,
            time_of_day,
            region_map,
            mut timer,
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
            mut deleted_entities,
            tracked_comps,
            trackers,
        ): Self::SystemData,
    ) {
        timer.start();

        let tick = tick.0;
        // To send entity updates
        // 1. Iterate through regions
        // 2. Iterate through region subscribers (ie clients)
        //     - Collect a list of entity ids for clients who are subscribed to this
        //       region (hash calc to check each)
        // 3. Iterate through events from that region
        //     - For each entity entered event, iterate through the client list and
        //       check if they are subscribed to the source (hash calc per subscribed
        //       client per entity event), if not subscribed to the source send a entity
        //       creation message to that client
        //     - For each entity left event, iterate through the client list and check
        //       if they are subscribed to the destination (hash calc per subscribed
        //       client per entity event)
        // 4. Iterate through entities in that region
        // 5. Inform clients of the component changes for that entity
        //     - Throttle update rate base on distance to each client

        // Sync physics
        // via iterating through regions
        for (key, region) in region_map.iter() {
            // Assemble subscriber list for this region by iterating through clients and
            // checking if they are subscribed to this region
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
                    RegionEvent::Entered(id, maybe_key) => {
                        // Don't process newly created entities here (redundant network messages)
                        if trackers.uid.inserted().contains(*id) {
                            continue;
                        }
                        let entity = entities.entity(*id);
                        if let Some((uid, pos, vel, ori, character_state)) =
                            uids.get(entity).and_then(|uid| {
                                positions.get(entity).map(|pos| {
                                    (
                                        uid,
                                        pos,
                                        velocities.get(entity),
                                        orientations.get(entity),
                                        character_states.get(entity),
                                    )
                                })
                            })
                        {
                            let create_msg = ServerMsg::CreateEntity(
                                tracked_comps.create_entity_package(entity),
                            );
                            for (client, regions, client_entity, _) in &mut subscribers {
                                if maybe_key
                                    .as_ref()
                                    .map(|key| !regions.contains(key))
                                    .unwrap_or(true)
                                    // Client doesn't need to know about itself
                                    && *client_entity != entity
                                {
                                    client.notify(create_msg.clone());
                                    send_initial_unsynced_components(
                                        client,
                                        uid,
                                        pos,
                                        vel,
                                        ori,
                                        character_state,
                                    );
                                }
                            }
                        }
                    },
                    RegionEvent::Left(id, maybe_key) => {
                        // Lookup UID for entity
                        if let Some(&uid) = uids.get(entities.entity(*id)) {
                            for (client, regions, _, _) in &mut subscribers {
                                if maybe_key
                                    .as_ref()
                                    .map(|key| !regions.contains(key))
                                    .unwrap_or(true)
                                {
                                    client.notify(ServerMsg::DeleteEntity(uid.into()));
                                }
                            }
                        }
                    },
                }
            }

            // Sync tracked components
            // Get deleted entities in this region from DeletedEntities
            let sync_msg = ServerMsg::EcsSync(
                trackers.create_sync_package(
                    &tracked_comps,
                    region.entities(),
                    deleted_entities
                        .take_deleted_in_region(key)
                        .unwrap_or_else(|| Vec::new()),
                ),
            );
            for (client, _, _, _) in &mut subscribers {
                client.notify(sync_msg.clone());
            }

            let mut send_msg = |msg: ServerMsg,
                                entity: EcsEntity,
                                pos: Pos,
                                force_update: Option<&ForceUpdate>,
                                throttle: bool| {
                for (client, _, client_entity, client_pos) in &mut subscribers {
                    let update = if client_entity == &entity {
                        // Don't send client physics updates about itself unless force update is set
                        force_update.is_some()
                    } else if !throttle {
                        // Update rate not thottled by distance
                        true
                    } else {
                        // Throttle update rate based on distance to client
                        let distance_sq = client_pos.0.distance_squared(pos.0);
                        // More entities farther away so checks start there
                        if distance_sq > 300.0f32.powi(2) {
                            (tick + entity.id() as u64) % 32 == 0
                        } else if distance_sq > 250.0f32.powi(2) {
                            (tick + entity.id() as u64) % 16 == 0
                        } else if distance_sq > 200.0f32.powi(2) {
                            (tick + entity.id() as u64) % 8 == 0
                        } else if distance_sq > 150.0f32.powi(2) {
                            (tick + entity.id() as u64) % 4 == 0
                        } else if distance_sq > 100.0f32.powi(2) {
                            (tick + entity.id() as u64) % 2 == 0
                        } else {
                            true // Closer than 100 blocks
                        }
                    };

                    if update {
                        client.notify(msg.clone());
                    }
                }
            };

            // Sync physics components
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
                // TODO: An entity that stoppped moving on a tick that it wasn't sent to the
                // player will never have it's position updated
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
                        .map(|&l| !character_state.equals(&l.0))
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

        // Handle entity deletion in regions that don't exist in RegionMap
        // (theoretically none)
        for (region_key, deleted) in deleted_entities.take_remaining_deleted() {
            for client in
                (&mut clients, &subscriptions)
                    .join()
                    .filter_map(|(client, subscription)| {
                        if client.is_ingame() && subscription.regions.contains(&region_key) {
                            Some(client)
                        } else {
                            None
                        }
                    })
            {
                for uid in &deleted {
                    client.notify(ServerMsg::DeleteEntity(*uid));
                }
            }
        }

        // TODO: Sync clients that don't have a position?

        // Sync inventories
        for (client, inventory, update) in (&mut clients, &inventories, &inventory_updates).join() {
            client.notify(ServerMsg::InventoryUpdate(
                inventory.clone(),
                update.event(),
            ));
        }

        // Remove all force flags.
        force_updates.clear();
        inventory_updates.clear();

        // Sync resources
        // TODO: doesn't really belong in this system (rename system or create another
        // system?)
        let tof_msg = ServerMsg::TimeOfDay(*time_of_day);
        for client in (&mut clients).join() {
            client.notify(tof_msg.clone());
        }

        timer.end();
    }
}

pub fn send_initial_unsynced_components(
    client: &mut Client,
    uid: &Uid,
    pos: &Pos,
    vel: Option<&Vel>,
    ori: Option<&Ori>,
    character_state: Option<&CharacterState>,
) {
    let entity = (*uid).into();
    client.notify(ServerMsg::EntityPos { entity, pos: *pos });
    if let Some(&vel) = vel {
        client.notify(ServerMsg::EntityVel { entity, vel });
    }
    if let Some(&ori) = ori {
        client.notify(ServerMsg::EntityOri { entity, ori });
    }
    if let Some(&character_state) = character_state {
        client.notify(ServerMsg::EntityCharacterState {
            entity,
            character_state,
        });
    }
}
