use super::sentinel::{DeletedEntities, TrackedStorages, UpdateTrackers};
use crate::{
    client::Client,
    presence::{Presence, RegionSubscription},
    Tick,
};
use common::{
    calendar::Calendar,
    comp::{Collider, ForceUpdate, InventoryUpdate, Last, Ori, Player, Pos, Vel},
    event::EventBus,
    outcome::Outcome,
    region::{Event as RegionEvent, RegionMap},
    resources::{PlayerPhysicsSettings, Time, TimeOfDay},
    terrain::TerrainChunkSize,
    uid::Uid,
    vol::RectVolSize,
};
use common_ecs::{Job, Origin, Phase, System};
use common_net::{msg::ServerGeneral, sync::CompSyncPackage};
use itertools::Either;
use specs::{Entities, Join, Read, ReadExpect, ReadStorage, Write, WriteStorage};
use vek::*;

/// This system will send physics updates to the client
#[derive(Default)]
pub struct Sys;

impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Entities<'a>,
        Read<'a, Tick>,
        Read<'a, PlayerPhysicsSettings>,
        TrackedStorages<'a>,
        ReadExpect<'a, TimeOfDay>,
        ReadExpect<'a, Time>,
        ReadExpect<'a, Calendar>,
        ReadExpect<'a, RegionMap>,
        ReadExpect<'a, UpdateTrackers>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Vel>,
        ReadStorage<'a, Ori>,
        ReadStorage<'a, RegionSubscription>,
        ReadStorage<'a, Player>,
        ReadStorage<'a, Presence>,
        ReadStorage<'a, Client>,
        WriteStorage<'a, Last<Pos>>,
        WriteStorage<'a, Last<Vel>>,
        WriteStorage<'a, Last<Ori>>,
        WriteStorage<'a, ForceUpdate>,
        WriteStorage<'a, InventoryUpdate>,
        Write<'a, DeletedEntities>,
        Read<'a, EventBus<Outcome>>,
    );

    const NAME: &'static str = "entity_sync";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        job: &mut Job<Self>,
        (
            entities,
            tick,
            player_physics_settings,
            tracked_storages,
            time_of_day,
            time,
            calendar,
            region_map,
            trackers,
            positions,
            velocities,
            orientations,
            subscriptions,
            players,
            presences,
            clients,
            mut last_pos,
            mut last_vel,
            mut last_ori,
            mut force_updates,
            mut inventory_updates,
            mut deleted_entities,
            outcomes,
        ): Self::SystemData,
    ) {
        let tick = tick.0;

        // Storages already provided in `TrackedStorages` that we need to use
        // for other things besides change detection.
        let uids = &tracked_storages.uid;
        let colliders = &tracked_storages.collider;
        let inventories = &tracked_storages.inventory;
        let is_rider = &tracked_storages.is_rider;

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

        // Sync physics and other components
        // via iterating through regions (in parallel)

        // Pre-collect regions paired with deleted entity list so we can iterate over
        // them in parallel below
        let regions_and_deleted_entities = region_map
            .iter()
            .map(|(key, region)| (key, region, deleted_entities.take_deleted_in_region(key)))
            .collect::<Vec<_>>();

        use rayon::iter::{IntoParallelIterator, ParallelIterator};
        job.cpu_stats.measure(common_ecs::ParMode::Rayon);
        common_base::prof_span!(guard, "regions");
        regions_and_deleted_entities.into_par_iter().for_each_init(
            || {
                common_base::prof_span!(guard, "entity sync rayon job");
                guard
            },
            |_guard, (key, region, deleted_entities_in_region)| {
                // Assemble subscriber list for this region by iterating through clients and
                // checking if they are subscribed to this region
                let mut subscribers = (
                    &clients,
                    &entities,
                    presences.maybe(),
                    &subscriptions,
                    &positions,
                )
                    .join()
                    .filter_map(|(client, entity, presence, subscription, pos)| {
                        if presence.is_some() && subscription.regions.contains(&key) {
                            Some((client, &subscription.regions, entity, *pos))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();

                for event in region.events() {
                    match event {
                        RegionEvent::Entered(id, maybe_key) => {
                            // Don't process newly created entities here (redundant network
                            // messages)
                            if trackers.uid.inserted().contains(*id) {
                                continue;
                            }
                            let entity = entities.entity(*id);
                            if let Some(pkg) = positions
                                .get(entity)
                                .map(|pos| (pos, velocities.get(entity), orientations.get(entity)))
                                .and_then(|(pos, vel, ori)| {
                                    tracked_storages.create_entity_package(
                                        entity,
                                        Some(*pos),
                                        vel.copied(),
                                        ori.copied(),
                                    )
                                })
                            {
                                let create_msg = ServerGeneral::CreateEntity(pkg);
                                for (client, regions, client_entity, _) in &mut subscribers {
                                    if maybe_key
                                    .as_ref()
                                    .map(|key| !regions.contains(key))
                                    .unwrap_or(true)
                                    // Client doesn't need to know about itself
                                    && *client_entity != entity
                                    {
                                        client.send_fallible(create_msg.clone());
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
                                        client.send_fallible(ServerGeneral::DeleteEntity(uid));
                                    }
                                }
                            }
                        },
                    }
                }

                // Sync tracked components
                // Get deleted entities in this region from DeletedEntities
                let (entity_sync_package, comp_sync_package) = trackers.create_sync_packages(
                    &tracked_storages,
                    region.entities(),
                    deleted_entities_in_region,
                );
                // We lazily initialize the the synchronization messages in case there are no
                // clients.
                let mut entity_comp_sync = Either::Left((entity_sync_package, comp_sync_package));
                for (client, _, client_entity, _) in &mut subscribers {
                    let msg = entity_comp_sync.right_or_else(
                        |(entity_sync_package, comp_sync_package)| {
                            (
                                client.prepare(ServerGeneral::EntitySync(entity_sync_package)),
                                client.prepare(ServerGeneral::CompSync(
                                    comp_sync_package,
                                    force_updates.get(*client_entity).map_or(0, |f| f.counter()),
                                )),
                            )
                        },
                    );
                    // We don't care much about stream errors here since they could just represent
                    // network disconnection, which is handled elsewhere.
                    let _ = client.send_prepared(&msg.0);
                    let _ = client.send_prepared(&msg.1);
                    entity_comp_sync = Either::Right(msg);
                }

                for (client, _, client_entity, client_pos) in &mut subscribers {
                    let mut comp_sync_package = CompSyncPackage::new();

                    for (_, entity, &uid, (&pos, last_pos), vel, ori, force_update, collider) in (
                        region.entities(),
                        &entities,
                        uids,
                        (&positions, last_pos.mask().maybe()),
                        (&velocities, last_vel.mask().maybe()).maybe(),
                        (&orientations, last_vel.mask().maybe()).maybe(),
                        force_updates.maybe(),
                        colliders.maybe(),
                    )
                        .join()
                    {
                        // Decide how regularly to send physics updates.
                        let send_now = if client_entity == &entity {
                            let player_physics_setting = players
                                .get(entity)
                                .and_then(|p| {
                                    player_physics_settings.settings.get(&p.uuid()).copied()
                                })
                                .unwrap_or_default();
                            // Don't send client physics updates about itself unless force update is
                            // set or the client is subject to
                            // server-authoritative physics
                            force_update.map_or(false, |f| f.is_forced())
                                || player_physics_setting.server_authoritative()
                                || is_rider.get(entity).is_some()
                        } else if matches!(collider, Some(Collider::Voxel { .. })) {
                            // Things with a voxel collider (airships, etc.) need to have very
                            // stable physics so we always send updated
                            // for these where we can.
                            true
                        } else {
                            // Throttle update rates for all other entities based on distance to
                            // client
                            let distance_sq = client_pos.0.distance_squared(pos.0);
                            let id_staggered_tick = tick + entity.id() as u64;

                            // More entities farther away so checks start there
                            if distance_sq > 500.0f32.powi(2) {
                                id_staggered_tick % 32 == 0
                            } else if distance_sq > 300.0f32.powi(2) {
                                id_staggered_tick % 16 == 0
                            } else if distance_sq > 200.0f32.powi(2) {
                                id_staggered_tick % 8 == 0
                            } else if distance_sq > 120.0f32.powi(2) {
                                id_staggered_tick % 6 == 0
                            } else if distance_sq > 64.0f32.powi(2) {
                                id_staggered_tick % 3 == 0
                            } else if distance_sq > 24.0f32.powi(2) {
                                id_staggered_tick % 2 == 0
                            } else {
                                true
                            }
                        };

                        if last_pos.is_none() {
                            comp_sync_package.comp_inserted(uid, pos);
                        } else if send_now {
                            comp_sync_package.comp_modified(uid, pos);
                        }

                        if let Some((v, last_vel)) = vel {
                            if last_vel.is_none() {
                                comp_sync_package.comp_inserted(uid, *v);
                            } else if send_now {
                                comp_sync_package.comp_modified(uid, *v);
                            }
                        }

                        if let Some((o, last_ori)) = ori {
                            if last_ori.is_none() {
                                comp_sync_package.comp_inserted(uid, *o);
                            } else if send_now {
                                comp_sync_package.comp_modified(uid, *o);
                            }
                        }
                    }

                    client.send_fallible(ServerGeneral::CompSync(
                        comp_sync_package,
                        force_updates.get(*client_entity).map_or(0, |f| f.counter()),
                    ));
                }
            },
        );
        drop(guard);
        job.cpu_stats.measure(common_ecs::ParMode::Single);

        // Update the last physics components for each entity
        for (_, &pos, vel, ori, last_pos, last_vel, last_ori) in (
            &entities,
            &positions,
            velocities.maybe(),
            orientations.maybe(),
            last_pos.entries(),
            last_vel.entries(),
            last_ori.entries(),
        )
            .join()
        {
            last_pos.replace(Last(pos));
            vel.and_then(|&v| last_vel.replace(Last(v)));
            ori.and_then(|&o| last_ori.replace(Last(o)));
        }

        // Handle entity deletion in regions that don't exist in RegionMap
        // (theoretically none)
        for (region_key, deleted) in deleted_entities.take_remaining_deleted() {
            for client in (presences.maybe(), &subscriptions, &clients)
                .join()
                .filter_map(|(presence, subscription, client)| {
                    if presence.is_some() && subscription.regions.contains(&region_key) {
                        Some(client)
                    } else {
                        None
                    }
                })
            {
                for uid in &deleted {
                    client.send_fallible(ServerGeneral::DeleteEntity(Uid(*uid)));
                }
            }
        }

        // TODO: Sync clients that don't have a position?

        // Sync inventories
        for (inventory, update, client) in (inventories, &inventory_updates, &clients).join() {
            client.send_fallible(ServerGeneral::InventoryUpdate(
                inventory.clone(),
                update.event(),
            ));
        }

        // Sync components that are only synced for the client's own entity.
        for (entity, client) in (&entities, &clients).join() {
            let comp_sync_package =
                trackers.create_sync_from_client_package(&tracked_storages, entity);
            if !comp_sync_package.is_empty() {
                client.send_fallible(ServerGeneral::CompSync(
                    comp_sync_package,
                    force_updates.get(entity).map_or(0, |f| f.counter()),
                ));
            }
        }

        // Consume/clear the current outcomes and convert them to a vec
        let outcomes = outcomes.recv_all().collect::<Vec<_>>();

        // Sync outcomes
        for (presence, pos, client) in (presences.maybe(), positions.maybe(), &clients).join() {
            let is_near = |o_pos: Vec3<f32>| {
                pos.zip_with(presence, |pos, presence| {
                    pos.0.xy().distance_squared(o_pos.xy())
                        < (presence.entity_view_distance.current() as f32
                            * TerrainChunkSize::RECT_SIZE.x as f32)
                            .powi(2)
                })
            };

            let outcomes = outcomes
                .iter()
                .filter(|o| o.get_pos().and_then(is_near).unwrap_or(true))
                .cloned()
                .collect::<Vec<_>>();

            if !outcomes.is_empty() {
                client.send_fallible(ServerGeneral::Outcomes(outcomes));
            }
        }

        // Remove all force flags.
        for force_update in (&mut force_updates).join() {
            force_update.clear();
        }
        inventory_updates.clear();

        // Sync resources
        // TODO: doesn't really belong in this system (rename system or create another
        // system?)
        const TOD_SYNC_FREQ: u64 = 100;
        if tick % TOD_SYNC_FREQ == 0 {
            let mut tod_lazymsg = None;
            for client in (&clients).join() {
                let msg = tod_lazymsg.unwrap_or_else(|| {
                    client.prepare(ServerGeneral::TimeOfDay(
                        *time_of_day,
                        (*calendar).clone(),
                        *time,
                    ))
                });
                // We don't care much about stream errors here since they could just represent
                // network disconnection, which is handled elsewhere.
                let _ = client.send_prepared(&msg);
                tod_lazymsg = Some(msg);
            }
        }
    }
}
