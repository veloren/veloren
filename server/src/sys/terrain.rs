#[cfg(not(feature = "worldgen"))]
use crate::test_world::{IndexOwned, World};
#[cfg(feature = "persistent_world")]
use crate::TerrainPersistence;
#[cfg(feature = "worldgen")]
use world::{IndexOwned, World};

use crate::{
    chunk_generator::ChunkGenerator,
    client::Client,
    metrics::NetworkRequestMetrics,
    presence::{Presence, RepositionOnChunkLoad},
    rtsim::RtSim,
    settings::Settings,
    ChunkRequest, SpawnPoint, Tick,
};
use common::{
    calendar::Calendar,
    comp::{
        self, agent, bird_medium, skillset::skills, BehaviorCapability, ForceUpdate, Pos, Waypoint,
    },
    event::{EventBus, ServerEvent},
    generation::EntityInfo,
    lottery::LootSpec,
    resources::{Time, TimeOfDay},
    slowjob::SlowJobPool,
    terrain::TerrainGrid,
    SkillSetBuilder,
};

use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::{SerializedTerrainChunk, ServerGeneral};
use common_state::TerrainChanges;
use comp::Behavior;
use specs::{Entities, Join, Read, ReadExpect, ReadStorage, Write, WriteExpect, WriteStorage};
use std::sync::Arc;
use vek::*;

#[cfg(feature = "persistent_world")]
pub type TerrainPersistenceData<'a> = Option<Write<'a, TerrainPersistence>>;
#[cfg(not(feature = "persistent_world"))]
pub type TerrainPersistenceData<'a> = ();

pub(crate) struct LazyTerrainMessage {
    lazy_msg_lo: Option<crate::client::PreparedMsg>,
    lazy_msg_hi: Option<crate::client::PreparedMsg>,
}

pub const SAFE_ZONE_RADIUS: f32 = 200.0;

impl LazyTerrainMessage {
    pub(crate) fn new() -> Self {
        Self {
            lazy_msg_lo: None,
            lazy_msg_hi: None,
        }
    }

    pub(crate) fn prepare_and_send<
        'a,
        A,
        F: FnOnce() -> Result<&'a common::terrain::TerrainChunk, A>,
    >(
        &mut self,
        network_metrics: &NetworkRequestMetrics,
        client: &Client,
        presence: &Presence,
        chunk_key: &vek::Vec2<i32>,
        generate_chunk: F,
    ) -> Result<(), A> {
        let lazy_msg = if presence.lossy_terrain_compression {
            &mut self.lazy_msg_lo
        } else {
            &mut self.lazy_msg_hi
        };
        if lazy_msg.is_none() {
            *lazy_msg = Some(client.prepare(ServerGeneral::TerrainChunkUpdate {
                key: *chunk_key,
                chunk: Ok(match generate_chunk() {
                    Ok(chunk) => SerializedTerrainChunk::via_heuristic(
                        chunk,
                        presence.lossy_terrain_compression,
                    ),
                    Err(e) => return Err(e),
                }),
            }));
        }
        lazy_msg.as_ref().map(|msg| {
            let _ = client.send_prepared(msg);
            if presence.lossy_terrain_compression {
                network_metrics.chunks_served_lossy.inc();
            } else {
                network_metrics.chunks_served_lossless.inc();
            }
        });
        Ok(())
    }
}

/// This system will handle loading generated chunks and unloading
/// unneeded chunks.
///     1. Inserts newly generated chunks into the TerrainGrid
///     2. Sends new chunks to nearby clients
///     3. Handles the chunk's supplement (e.g. npcs)
///     4. Removes chunks outside the range of players
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Read<'a, EventBus<ServerEvent>>,
        Read<'a, Tick>,
        Read<'a, SpawnPoint>,
        Read<'a, Settings>,
        Read<'a, TimeOfDay>,
        Read<'a, Calendar>,
        ReadExpect<'a, SlowJobPool>,
        ReadExpect<'a, IndexOwned>,
        ReadExpect<'a, Arc<World>>,
        ReadExpect<'a, NetworkRequestMetrics>,
        WriteExpect<'a, ChunkGenerator>,
        WriteExpect<'a, TerrainGrid>,
        Write<'a, TerrainChanges>,
        Write<'a, Vec<ChunkRequest>>,
        WriteExpect<'a, RtSim>,
        TerrainPersistenceData<'a>,
        WriteStorage<'a, Pos>,
        ReadStorage<'a, Presence>,
        ReadStorage<'a, Client>,
        Entities<'a>,
        WriteStorage<'a, RepositionOnChunkLoad>,
        WriteStorage<'a, ForceUpdate>,
        WriteStorage<'a, Waypoint>,
        ReadExpect<'a, Time>,
    );

    const NAME: &'static str = "terrain";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (
            server_event_bus,
            tick,
            spawn_point,
            server_settings,
            time_of_day,
            calendar,
            slow_jobs,
            index,
            world,
            network_metrics,
            mut chunk_generator,
            mut terrain,
            mut terrain_changes,
            mut chunk_requests,
            mut rtsim,
            mut _terrain_persistence,
            mut positions,
            presences,
            clients,
            entities,
            mut reposition_on_load,
            mut force_update,
            mut waypoints,
            time,
        ): Self::SystemData,
    ) {
        let mut server_emitter = server_event_bus.emitter();

        // Generate requested chunks
        //
        // Submit requests for chunks right before receiving finished chunks so that we
        // don't create duplicate work for chunks that just finished but are not
        // yet added to the terrain.
        chunk_requests.drain(..).for_each(|request| {
            chunk_generator.generate_chunk(
                Some(request.entity),
                request.key,
                &slow_jobs,
                Arc::clone(&world),
                index.clone(),
                (*time_of_day, calendar.clone()),
            )
        });

        // Fetch any generated `TerrainChunk`s and insert them into the terrain.
        // Also, send the chunk data to anybody that is close by.
        let mut new_chunks = Vec::new();
        'insert_terrain_chunks: while let Some((key, res)) = chunk_generator.recv_new_chunk() {
            #[allow(unused_mut)]
            let (mut chunk, supplement) = match res {
                Ok((chunk, supplement)) => (chunk, supplement),
                Err(Some(entity)) => {
                    if let Some(client) = clients.get(entity) {
                        client.send_fallible(ServerGeneral::TerrainChunkUpdate {
                            key,
                            chunk: Err(()),
                        });
                    }
                    continue 'insert_terrain_chunks;
                },
                Err(None) => {
                    continue 'insert_terrain_chunks;
                },
            };

            // Apply changes from terrain persistence to this chunk
            #[cfg(feature = "persistent_world")]
            if let Some(terrain_persistence) = _terrain_persistence.as_mut() {
                terrain_persistence.apply_changes(key, &mut chunk);
            }

            // Arcify the chunk
            let chunk = Arc::new(chunk);

            // Add to list of chunks to send to nearby players.
            new_chunks.push((key, Arc::clone(&chunk)));

            // TODO: code duplication for chunk insertion between here and state.rs
            // Insert the chunk into terrain changes
            if terrain.insert(key, chunk).is_some() {
                terrain_changes.modified_chunks.insert(key);
            } else {
                terrain_changes.new_chunks.insert(key);
                rtsim.hook_load_chunk(key);
            }

            // Handle chunk supplement
            for entity in supplement.entities {
                // Check this because it's a common source of weird bugs
                assert!(
                    terrain
                        .pos_key(entity.pos.map(|e| e.floor() as i32))
                        .map2(key, |e, tgt| (e - tgt).abs() <= 1)
                        .reduce_and(),
                    "Chunk spawned entity that wasn't nearby",
                );

                let data = NpcData::from_entity_info(entity);
                match data {
                    NpcData::Waypoint(pos) => {
                        server_emitter.emit(ServerEvent::CreateWaypoint(pos));
                    },
                    NpcData::Data {
                        pos,
                        stats,
                        skill_set,
                        health,
                        poise,
                        inventory,
                        agent,
                        body,
                        alignment,
                        scale,
                        loot,
                    } => {
                        server_emitter.emit(ServerEvent::CreateNpc {
                            pos,
                            stats,
                            skill_set,
                            health,
                            poise,
                            inventory,
                            agent,
                            body,
                            alignment,
                            scale,
                            anchor: Some(comp::Anchor::Chunk(key)),
                            loot,
                            rtsim_entity: None,
                            projectile: None,
                        });
                    },
                }
            }

            // Insert a safezone if chunk contains the spawn position
            if server_settings.safe_spawn && is_spawn_chunk(key, *spawn_point, &terrain) {
                server_emitter.emit(ServerEvent::CreateSafezone {
                    range: Some(SAFE_ZONE_RADIUS),
                    pos: Pos(spawn_point.0),
                });
            }
        }

        let mut repositioned = Vec::new();
        for (entity, pos, _) in (&entities, &mut positions, &reposition_on_load).join() {
            // If an entity is marked as needing repositioning once the chunk loads (e.g.
            // from having just logged in), reposition them.

            let chunk_pos = terrain.pos_key(pos.0.map(|e| e as i32));
            if let Some(chunk) = terrain.get_key(chunk_pos) {
                pos.0 = terrain
                    .try_find_space(pos.0.as_::<i32>())
                    .map(|x| x.as_::<f32>())
                    .unwrap_or_else(|| chunk.find_accessible_pos(pos.0.xy().as_::<i32>(), false));
                repositioned.push(entity);
                let _ = force_update.insert(entity, ForceUpdate);
                let _ = waypoints.insert(entity, Waypoint::new(pos.0, *time));
            }
        }
        for entity in repositioned {
            reposition_on_load.remove(entity);
        }

        // Send the chunk to all nearby players.
        use rayon::iter::{IntoParallelIterator, ParallelIterator};
        new_chunks.into_par_iter().for_each(|(key, chunk)| {
            let mut lazy_msg = LazyTerrainMessage::new();

            (&presences, &positions, &clients)
                .join()
                .for_each(|(presence, pos, client)| {
                    let chunk_pos = terrain.pos_key(pos.0.map(|e| e as i32));
                    // Subtract 2 from the offset before computing squared magnitude
                    // 1 since chunks need neighbors to be meshed
                    // 1 to act as a buffer if the player moves in that direction
                    let adjusted_dist_sqr = (chunk_pos - key)
                        .map(|e: i32| (e.abs() as u32).saturating_sub(2))
                        .magnitude_squared();

                    if adjusted_dist_sqr <= presence.view_distance.pow(2) {
                        lazy_msg
                            .prepare_and_send::<!, _>(
                                &network_metrics,
                                client,
                                presence,
                                &key,
                                || Ok(&*chunk),
                            )
                            .into_ok();
                    }
                });
        });

        // Remove chunks that are too far from players.
        let mut chunks_to_remove = Vec::new();
        terrain
            .iter()
            .map(|(k, _)| k)
            // Don't check every chunk every tick (spread over 16 ticks)
            .filter(|k| k.x.abs() as u64 % 4 + (k.y.abs() as u64 % 4) * 4 == tick.0 % 16)
            // There shouldn't be to many pending chunks so we will just check them all
            .chain(chunk_generator.pending_chunks())
            .for_each(|chunk_key| {
                let mut should_drop = true;

                // For each player with a position, calculate the distance.
                for (presence, pos) in (&presences, &positions).join() {
                    if chunk_in_vd(pos.0, chunk_key, &terrain, presence.view_distance) {
                        should_drop = false;
                        break;
                    }
                }

                if should_drop {
                    chunks_to_remove.push(chunk_key);
                }
            });

        for key in chunks_to_remove {
            // Register the unloading of this chunk from terrain persistence
            #[cfg(feature = "persistent_world")]
            if let Some(terrain_persistence) = _terrain_persistence.as_mut() {
                terrain_persistence.unload_chunk(key);
            }

            // TODO: code duplication for chunk insertion between here and state.rs
            if terrain.remove(key).is_some() {
                terrain_changes.removed_chunks.insert(key);
                rtsim.hook_unload_chunk(key);
            }

            chunk_generator.cancel_if_pending(key);
        }
    }
}

/// Convinient structure to use when you need to create new npc
/// from EntityInfo
// TODO: better name?
// TODO: if this is send around network, optimize the large_enum_variant
#[allow(clippy::large_enum_variant)]
pub enum NpcData {
    Data {
        pos: Pos,
        stats: comp::Stats,
        skill_set: comp::SkillSet,
        health: Option<comp::Health>,
        poise: comp::Poise,
        inventory: comp::inventory::Inventory,
        agent: Option<comp::Agent>,
        body: comp::Body,
        alignment: comp::Alignment,
        scale: comp::Scale,
        loot: LootSpec<String>,
    },
    Waypoint(Vec3<f32>),
}

impl NpcData {
    pub fn from_entity_info(entity: EntityInfo) -> Self {
        let EntityInfo {
            // flags
            is_waypoint,
            has_agency,
            agent_mark,
            alignment,
            no_flee,
            // stats
            body,
            name,
            scale,
            pos,
            loot,
            // tools and skills
            skillset_asset,
            loadout: mut loadout_builder,
            inventory: items,
            make_loadout,
            trading_information: economy,
            // unused
            pet: _, // TODO: I had no idea we have this.
        } = entity;

        if is_waypoint {
            return Self::Waypoint(pos);
        }

        let name = name.unwrap_or_else(|| "Unnamed".to_string());
        let stats = comp::Stats::new(name);

        let skill_set = {
            let skillset_builder = SkillSetBuilder::default();
            if let Some(skillset_asset) = skillset_asset {
                skillset_builder.with_asset_expect(&skillset_asset).build()
            } else {
                skillset_builder.build()
            }
        };

        let inventory = {
            // Evaluate lazy function for loadout creation
            if let Some(make_loadout) = make_loadout {
                loadout_builder = loadout_builder.with_creator(make_loadout, economy.as_ref());
            }
            let loadout = loadout_builder.build();
            let mut inventory = comp::inventory::Inventory::new_with_loadout(loadout);
            for (num, mut item) in items {
                if let Err(e) = item.set_amount(num) {
                    tracing::warn!(
                        "error during creating inventory for {name} at {pos}: {e:?}",
                        name = &stats.name,
                    );
                }
                if let Err(e) = inventory.push(item) {
                    tracing::warn!(
                        "error during creating inventory for {name} at {pos}: {e:?}",
                        name = &stats.name,
                    );
                }
            }

            inventory
        };

        let health_level = skill_set
            .skill_level(skills::Skill::General(skills::GeneralSkill::HealthIncrease))
            .unwrap_or(0);
        let health = Some(comp::Health::new(body, health_level));
        let poise = comp::Poise::new(body);

        // Allow Humanoid, BirdMedium, and Parrot to speak
        let can_speak = match body {
            comp::Body::Humanoid(_) => true,
            comp::Body::BirdMedium(bird_medium) => match bird_medium.species {
                bird_medium::Species::Parrot => alignment == comp::Alignment::Npc,
                _ => false,
            },
            _ => false,
        };

        let trade_for_site = if matches!(agent_mark, Some(agent::Mark::Merchant)) {
            economy.map(|e| e.id)
        } else {
            None
        };

        let agent = has_agency.then(|| {
            comp::Agent::from_body(&body)
                .with_behavior(
                    Behavior::default()
                        .maybe_with_capabilities(can_speak.then(|| BehaviorCapability::SPEAK))
                        .with_trade_site(trade_for_site),
                )
                .with_patrol_origin(pos)
                .with_no_flee_if(matches!(agent_mark, Some(agent::Mark::Guard)) || no_flee)
        });

        let agent = if matches!(alignment, comp::Alignment::Enemy)
            && matches!(body, comp::Body::Humanoid(_))
        {
            agent.map(|a| a.with_aggro_no_warn().with_no_flee_if(true))
        } else {
            agent
        };

        NpcData::Data {
            pos: Pos(pos),
            stats,
            skill_set,
            health,
            poise,
            inventory,
            agent,
            body,
            alignment,
            scale: comp::Scale(scale),
            loot,
        }
    }
}

pub fn chunk_in_vd(
    player_pos: Vec3<f32>,
    chunk_pos: Vec2<i32>,
    terrain: &TerrainGrid,
    vd: u32,
) -> bool {
    // This fuzzy threshold prevents chunks rapidly unloading and reloading when
    // players move over a chunk border.
    const UNLOAD_THRESHOLD: u32 = 2;

    let player_chunk_pos = terrain.pos_key(player_pos.map(|e| e as i32));

    let adjusted_dist_sqr = (player_chunk_pos - chunk_pos)
        .map(|e: i32| e.abs() as u32)
        .magnitude_squared();

    adjusted_dist_sqr <= (vd.max(crate::MIN_VD) + UNLOAD_THRESHOLD).pow(2)
}

fn is_spawn_chunk(chunk_pos: Vec2<i32>, spawn_pos: SpawnPoint, terrain: &TerrainGrid) -> bool {
    let spawn_chunk_pos = terrain.pos_key(spawn_pos.0.map(|e| e as i32));
    chunk_pos == spawn_chunk_pos
}
