use super::sentinel::{DeletedEntities, TrackedStorages};
use crate::{
    client::Client,
    presence::{self, Presence, RegionSubscription},
};
use common::{
    comp::{Ori, Pos, Vel},
    region::{region_in_vd, regions_in_vd, Event as RegionEvent, RegionMap},
    terrain::{CoordinateConversions, TerrainChunkSize},
    uid::Uid,
    vol::RectVolSize,
};
use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::ServerGeneral;
use specs::{
    Entities, Join, Read, ReadExpect, ReadStorage, SystemData, World, WorldExt, WriteStorage,
};
use tracing::{debug, error};
use vek::*;

/// This system will update region subscriptions based on client positions
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        ReadExpect<'a, RegionMap>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Vel>,
        ReadStorage<'a, Ori>,
        ReadStorage<'a, Presence>,
        ReadStorage<'a, Client>,
        WriteStorage<'a, RegionSubscription>,
        Read<'a, DeletedEntities>,
        TrackedStorages<'a>,
    );

    const NAME: &'static str = "subscription";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    #[allow(clippy::blocks_in_if_conditions)] // TODO: Pending review in #587
    fn run(
        _job: &mut Job<Self>,
        (
            entities,
            region_map,
            uids,
            positions,
            velocities,
            orientations,
            presences,
            clients,
            mut subscriptions,
            deleted_entities,
            tracked_comps,
        ): Self::SystemData,
    ) {
        // To update subscriptions
        // 1. Iterate through clients
        // 2. Calculate current chunk position
        // 3. If chunk is different (use fuzziness) or the client view distance
        //    has changed continue, otherwise return
        // 4. Iterate through subscribed regions
        // 5. Check if region is still in range (use fuzziness)
        // 6. If not in range
        //     - remove from hashset
        //     - inform client of which entities to remove
        // 7. Determine list of regions that are in range and iterate through it
        //    - check if in hashset (hash calc) if not add it
        let mut regions_to_remove = Vec::new();
        for (mut subscription, pos, presence, client_entity, client) in (
            &mut subscriptions,
            &positions,
            &presences,
            &entities,
            &clients,
        )
            .join()
        {
            let vd = presence.entity_view_distance.current();
            // Calculate current chunk
            let chunk = (Vec2::<f32>::from(pos.0)).as_::<i32>().wpos_to_cpos();
            // Only update regions when moving to a new chunk or if view distance has
            // changed.
            //
            // Uses a fuzzy border to prevent rapid triggering when moving along chunk
            // boundaries.
            if chunk != subscription.fuzzy_chunk
                && (subscription
                    .fuzzy_chunk
                    .map2(TerrainChunkSize::RECT_SIZE, |e, sz| {
                        (e as f32 + 0.5) * sz as f32
                    })
                    - Vec2::from(pos.0))
                .map2(TerrainChunkSize::RECT_SIZE, |e, sz| {
                    e.abs() > (sz / 2 + presence::CHUNK_FUZZ) as f32
                })
                .reduce_or()
                || subscription.last_entity_view_distance != vd
            {
                // Update the view distance
                subscription.last_entity_view_distance = vd;
                // Update current chunk
                subscription.fuzzy_chunk = Vec2::<f32>::from(pos.0).as_::<i32>().wpos_to_cpos();
                // Use the largest side length as our chunk size
                let chunk_size = TerrainChunkSize::RECT_SIZE.reduce_max() as f32;
                // Iterate through currently subscribed regions
                for key in &subscription.regions {
                    // Check if the region is not within range anymore
                    if !region_in_vd(
                        *key,
                        pos.0,
                        (vd as f32 * chunk_size)
                            + (presence::CHUNK_FUZZ as f32
                                + presence::REGION_FUZZ as f32
                                + chunk_size)
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
                    // Tell the client to delete the entities in that region if it exists in the
                    // RegionMap
                    if let Some(region) = region_map.get(key) {
                        // Process entity left events since they won't be processed during entity
                        // sync because this region is no longer subscribed to
                        // TODO: consider changing system ordering??
                        for event in region.events() {
                            match event {
                                RegionEvent::Entered(_, _) => {}, /* These don't need to be */
                                // processed because this
                                // region is being thrown out
                                // anyway
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
                                            client.send_fallible(ServerGeneral::DeleteEntity(uid));
                                        }
                                    }
                                },
                            }
                        }
                        // Tell client to delete entities in the region
                        for (&uid, _) in (&uids, region.entities()).join() {
                            client.send_fallible(ServerGeneral::DeleteEntity(uid));
                        }
                    }
                    // Send deleted entities since they won't be processed for this client in entity
                    // sync
                    for uid in deleted_entities.get_deleted_in_region(key).iter() {
                        client.send_fallible(ServerGeneral::DeleteEntity(Uid(*uid)));
                    }
                }

                for key in regions_in_vd(
                    pos.0,
                    (vd as f32 * chunk_size)
                        + (presence::CHUNK_FUZZ as f32 + chunk_size) * 2.0f32.sqrt(),
                ) {
                    // Send client initial info about the entities in this region if it was not
                    // already within the set of subscribed regions
                    if subscription.regions.insert(key) {
                        if let Some(region) = region_map.get(key) {
                            (
                                &positions,
                                velocities.maybe(),
                                orientations.maybe(),
                                region.entities(),
                                &entities,
                            )
                                .join()
                                .filter(|(_, _, _, _, e)| *e != client_entity)
                                .filter_map(|(pos, vel, ori, _, entity)| {
                                    tracked_comps.create_entity_package(
                                        entity,
                                        Some(*pos),
                                        vel.copied(),
                                        ori.copied(),
                                    )
                                })
                                .for_each(|msg| {
                                    // Send message to create entity and tracked components and
                                    // physics components
                                    client.send_fallible(ServerGeneral::CreateEntity(msg));
                                })
                        }
                    }
                }
            }
        }
    }
}

/// Initialize region subscription
pub fn initialize_region_subscription(world: &World, entity: specs::Entity) {
    if let (Some(client_pos), Some(presence), Some(client)) = (
        world.read_storage::<Pos>().get(entity),
        world.read_storage::<Presence>().get(entity),
        world.write_storage::<Client>().get(entity),
    ) {
        let fuzzy_chunk = (Vec2::<f32>::from(client_pos.0))
            .as_::<i32>()
            .wpos_to_cpos();
        let chunk_size = TerrainChunkSize::RECT_SIZE.reduce_max() as f32;
        let regions = regions_in_vd(
            client_pos.0,
            (presence.entity_view_distance.current() as f32 * chunk_size)
                + (presence::CHUNK_FUZZ as f32 + chunk_size) * 2.0f32.sqrt(),
        );

        let region_map = world.read_resource::<RegionMap>();
        let tracked_comps = TrackedStorages::fetch(world);
        for key in &regions {
            if let Some(region) = region_map.get(*key) {
                (
                    &world.read_storage::<Pos>(), // We assume all these entities have a position
                    world.read_storage::<Vel>().maybe(),
                    world.read_storage::<Ori>().maybe(),
                    region.entities(),
                    &world.entities(),
                )
                .join()
                // Don't send client its own components because we do that below
                .filter(|t| t.4 != entity)
                .filter_map(|(pos, vel, ori, _, entity)|
                    tracked_comps.create_entity_package(
                        entity,
                        Some(*pos),
                        vel.copied(),
                        ori.copied(),
                    )
                )
                .for_each(|msg| {
                    // Send message to create entity and tracked components and physics components
                    client.send_fallible(ServerGeneral::CreateEntity(msg));
                });
            }
        }
        // If client position was modified it might not be updated in the region system
        // so we send its components here
        if let Some(pkg) = tracked_comps.create_entity_package(
            entity,
            Some(*client_pos),
            world.read_storage().get(entity).copied(),
            world.read_storage().get(entity).copied(),
        ) {
            client.send_fallible(ServerGeneral::CreateEntity(pkg));
        }

        if let Err(e) = world.write_storage().insert(entity, RegionSubscription {
            fuzzy_chunk,
            last_entity_view_distance: presence.entity_view_distance.current(),
            regions,
        }) {
            error!(?e, "Failed to insert region subscription component");
        }
    } else {
        debug!(
            ?entity,
            "Failed to initialize region subscription. Couldn't retrieve all the neccesary \
             components on the provided entity"
        );
    }
}
