use super::{
    sentinel::{DeletedEntities, TrackedComps},
    SysTimer,
};
use crate::client::{self, Client, RegionSubscription};
use common::{
    comp::{CharacterState, Ori, Player, Pos, Vel},
    msg::ServerMsg,
    region::{region_in_vd, regions_in_vd, Event as RegionEvent, RegionMap},
    sync::Uid,
    terrain::TerrainChunkSize,
    vol::RectVolSize,
};
use log::{debug, error};
use specs::{
    Entities, Join, ReadExpect, ReadStorage, System, SystemData, World, Write, WriteStorage,
};
use vek::*;

/// This system will update region subscriptions based on client positions
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        ReadExpect<'a, RegionMap>,
        Write<'a, SysTimer<Self>>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Vel>,
        ReadStorage<'a, Ori>,
        ReadStorage<'a, CharacterState>,
        ReadStorage<'a, Player>,
        WriteStorage<'a, Client>,
        WriteStorage<'a, RegionSubscription>,
        Write<'a, DeletedEntities>,
        TrackedComps<'a>,
    );

    fn run(
        &mut self,
        (
            entities,
            region_map,
            mut timer,
            uids,
            positions,
            velocities,
            orientations,
            character_states,
            players,
            mut clients,
            mut subscriptions,
            mut deleted_entities,
            tracked_comps,
        ): Self::SystemData,
    ) {
        timer.start();

        // To update subscriptions
        // 1. Iterate through clients
        // 2. Calculate current chunk position
        // 3. If chunk is the same return, otherwise continue (use fuzzyiness)
        // 4. Iterate through subscribed regions
        // 5. Check if region is still in range (use fuzzyiness)
        // 6. If not in range
        //     - remove from hashset
        //     - inform client of which entities to remove
        // 7. Determine list of regions that are in range and iterate through it
        //    - check if in hashset (hash calc) if not add it
        let mut regions_to_remove = Vec::new();
        for (client, subscription, pos, vd, client_entity) in (
            &mut clients,
            &mut subscriptions,
            &positions,
            &players,
            &entities,
        )
            .join()
            .filter_map(|(client, s, pos, player, e)| {
                if client.is_ingame() {
                    player.view_distance.map(|v| (client, s, pos, v, e))
                } else {
                    None
                }
            })
        {
            // Calculate current chunk
            let chunk = (Vec2::<f32>::from(pos.0))
                .map2(TerrainChunkSize::RECT_SIZE, |e, sz| e as i32 / sz as i32);
            // Only update regions when moving to a new chunk
            // uses a fuzzy border to prevent rapid triggering when moving along chunk boundaries
            if chunk != subscription.fuzzy_chunk
                && (subscription
                    .fuzzy_chunk
                    .map2(TerrainChunkSize::RECT_SIZE, |e, sz| {
                        (e as f32 + 0.5) * sz as f32
                    })
                    - Vec2::from(pos.0))
                .map2(TerrainChunkSize::RECT_SIZE, |e, sz| {
                    e.abs() > (sz / 2 + client::CHUNK_FUZZ) as f32
                })
                .reduce_or()
            {
                // Update current chunk
                subscription.fuzzy_chunk = Vec2::<f32>::from(pos.0)
                    .map2(TerrainChunkSize::RECT_SIZE, |e, sz| e as i32 / sz as i32);
                // Use the largest side length as our chunk size
                let chunk_size = TerrainChunkSize::RECT_SIZE.reduce_max() as f32;
                // Iterate through currently subscribed regions
                for key in &subscription.regions {
                    // Check if the region is not within range anymore
                    if !region_in_vd(
                        *key,
                        pos.0,
                        (vd as f32 * chunk_size)
                            + (client::CHUNK_FUZZ as f32 + client::REGION_FUZZ as f32 + chunk_size)
                                * 2.0f32.sqrt(),
                    ) {
                        // Add to the list of regions to remove
                        regions_to_remove.push(*key);
                    }
                }

                // Iterate through regions to remove
                for key in regions_to_remove.drain(..) {
                    // Remove region from this client's set of subscribed regions
                    subscription.regions.remove(&key);
                    // Tell the client to delete the entities in that region if it exists in the RegionMap
                    if let Some(region) = region_map.get(key) {
                        // Process entity left events since they won't be processed during entity sync because this region is no longer subscribed to
                        // TODO: consider changing system ordering??
                        for event in region.events() {
                            match event {
                                RegionEvent::Entered(_, _) => {} // These don't need to be processed because this region is being thrown out anyway
                                RegionEvent::Left(id, maybe_key) => {
                                    // Lookup UID for entity
                                    // Doesn't overlap with entity deletion in sync packages
                                    // because the uid would not be available if the entity was
                                    // deleted
                                    if let Some(&uid) = uids.get(entities.entity(*id)) {
                                        if !maybe_key
                                            .as_ref()
                                            // Don't need to check that this isn't also in the regions to remove since the entity will be removed when we get to that one
                                            .map(|key| subscription.regions.contains(key))
                                            .unwrap_or(false)
                                        {
                                            client.notify(ServerMsg::DeleteEntity(uid.into()));
                                        }
                                    }
                                }
                            }
                        }
                        // Tell client to delete entities in the region
                        for (&uid, _) in (&uids, region.entities()).join() {
                            client.notify(ServerMsg::DeleteEntity(uid.into()));
                        }
                    }
                    // Send deleted entities since they won't be processed for this client in entity sync
                    for uid in deleted_entities
                        .get_deleted_in_region(key)
                        .iter()
                        .flat_map(|v| v.iter())
                    {
                        client.notify(ServerMsg::DeleteEntity(*uid));
                    }
                }

                for key in regions_in_vd(
                    pos.0,
                    (vd as f32 * chunk_size)
                        + (client::CHUNK_FUZZ as f32 + chunk_size) * 2.0f32.sqrt(),
                ) {
                    // Send client intial info about the entities in this region if it was not
                    // already within the set of subscribed regions
                    if subscription.regions.insert(key.clone()) {
                        if let Some(region) = region_map.get(key) {
                            for (uid, pos, vel, ori, character_state, _, entity) in (
                                &uids,
                                &positions,
                                velocities.maybe(),
                                orientations.maybe(),
                                character_states.maybe(),
                                region.entities(),
                                &entities,
                            )
                                .join()
                                .filter(|(_, _, _, _, _, _, e)| *e != client_entity)
                            {
                                // Send message to create entity and tracked components
                                client.notify(ServerMsg::CreateEntity(
                                    tracked_comps.create_entity_package(entity),
                                ));
                                // Send message to create physics components
                                super::entity_sync::send_initial_unsynced_components(
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
                }
            }
        }

        timer.end();
    }
}

/// Initialize region subscription
pub fn initialize_region_subscription(world: &World, entity: specs::Entity) {
    if let (Some(client_pos), Some(client_vd), Some(client)) = (
        world.read_storage::<Pos>().get(entity),
        world
            .read_storage::<Player>()
            .get(entity)
            .map(|pl| pl.view_distance)
            .and_then(|v| v),
        world.write_storage::<Client>().get_mut(entity),
    ) {
        let fuzzy_chunk = (Vec2::<f32>::from(client_pos.0))
            .map2(TerrainChunkSize::RECT_SIZE, |e, sz| e as i32 / sz as i32);
        let chunk_size = TerrainChunkSize::RECT_SIZE.reduce_max() as f32;
        let regions = common::region::regions_in_vd(
            client_pos.0,
            (client_vd as f32 * chunk_size) as f32
                + (client::CHUNK_FUZZ as f32 + chunk_size) * 2.0f32.sqrt(),
        );

        let region_map = world.read_resource::<RegionMap>();
        let tracked_comps = TrackedComps::fetch(&world.res);
        for key in &regions {
            if let Some(region) = region_map.get(*key) {
                for (uid, pos, vel, ori, character_state, _, entity) in (
                    &tracked_comps.uid,
                    &world.read_storage::<Pos>(), // We assume all these entities have a position
                    world.read_storage::<Vel>().maybe(),
                    world.read_storage::<Ori>().maybe(),
                    world.read_storage::<CharacterState>().maybe(),
                    region.entities(),
                    &world.entities(),
                )
                    .join()
                {
                    // Send message to create entity and tracked components
                    client.notify(ServerMsg::CreateEntity(
                        tracked_comps.create_entity_package(entity),
                    ));
                    // Send message to create physics components
                    super::entity_sync::send_initial_unsynced_components(
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

        if let Err(err) = world.write_storage().insert(
            entity,
            RegionSubscription {
                fuzzy_chunk,
                regions,
            },
        ) {
            error!("Failed to insert region subscription component: {:?}", err);
        }
    } else {
        debug!("Failed to initialize region subcription. Couldn't retrieve all the neccesary components on the provided entity");
    }
}
