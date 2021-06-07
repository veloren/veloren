use crate::{
    chunk_generator::ChunkGenerator, client::Client, metrics::NetworkRequestMetrics,
    presence::Presence, rtsim::RtSim, settings::Settings, SpawnPoint, Tick,
};
use common::{
    comp::{self, agent, bird_medium, Alignment, BehaviorCapability, Pos},
    event::{EventBus, ServerEvent},
    generation::{get_npc_name, EntityInfo},
    npc::NPC_NAMES,
    terrain::TerrainGrid,
    LoadoutBuilder, SkillSetBuilder,
};
use common_ecs::{Job, Origin, Phase, System};
use common_net::msg::{SerializedTerrainChunk, ServerGeneral};
use common_state::TerrainChanges;
use comp::Behavior;
use specs::{Join, Read, ReadExpect, ReadStorage, Write, WriteExpect};
use std::sync::Arc;
use vek::*;

pub(crate) struct LazyTerrainMessage {
    lazy_msg_lo: Option<crate::client::PreparedMsg>,
    lazy_msg_hi: Option<crate::client::PreparedMsg>,
}

impl LazyTerrainMessage {
    #[allow(clippy::new_without_default)]
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
                        &chunk,
                        presence.lossy_terrain_compression,
                    ),
                    Err(e) => return Err(e),
                }),
            }));
        }
        lazy_msg.as_ref().map(|ref msg| {
            let _ = client.send_prepared(&msg);
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
    #[allow(clippy::type_complexity)] // TODO: Pending review in #587
    type SystemData = (
        Read<'a, EventBus<ServerEvent>>,
        Read<'a, Tick>,
        Read<'a, SpawnPoint>,
        Read<'a, Settings>,
        ReadExpect<'a, NetworkRequestMetrics>,
        WriteExpect<'a, ChunkGenerator>,
        WriteExpect<'a, TerrainGrid>,
        Write<'a, TerrainChanges>,
        WriteExpect<'a, RtSim>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Presence>,
        ReadStorage<'a, Client>,
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
            network_metrics,
            mut chunk_generator,
            mut terrain,
            mut terrain_changes,
            mut rtsim,
            positions,
            presences,
            clients,
        ): Self::SystemData,
    ) {
        let mut server_emitter = server_event_bus.emitter();

        // Fetch any generated `TerrainChunk`s and insert them into the terrain.
        // Also, send the chunk data to anybody that is close by.
        let mut new_chunks = Vec::new();
        'insert_terrain_chunks: while let Some((key, res)) = chunk_generator.recv_new_chunk() {
            let (chunk, supplement) = match res {
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

                if entity.is_waypoint {
                    server_emitter.emit(ServerEvent::CreateWaypoint(entity.pos));
                    continue;
                }

                let mut body = entity.body;
                let name = entity.name.unwrap_or_else(|| "Unnamed".to_string());
                let alignment = entity.alignment;
                let mut stats = comp::Stats::new(name);

                let mut scale = entity.scale;

                // Replace stuff if it's a boss
                if entity.is_giant {
                    if rand::random::<f32>() < 0.65 && entity.alignment != Alignment::Enemy {
                        let body_new = comp::humanoid::Body::random();
                        let npc_names = NPC_NAMES.read();

                        body = comp::Body::Humanoid(body_new);
                        stats = comp::Stats::new(format!(
                            "Gentle Giant {}",
                            get_npc_name(&npc_names.humanoid, body_new.species)
                        ));
                    }
                    scale = 2.0 + rand::random::<f32>();
                }

                let EntityInfo {
                    skillset_asset,
                    main_tool,
                    loadout_asset,
                    make_loadout,
                    trading_information: economy,
                    ..
                } = entity;

                let skill_set = {
                    let skillset_builder = SkillSetBuilder::default();
                    if let Some(skillset_asset) = skillset_asset {
                        skillset_builder.with_asset_expect(&skillset_asset).build()
                    } else {
                        skillset_builder.build()
                    }
                };

                let loadout = {
                    let mut loadout_builder = LoadoutBuilder::new();
                    let rng = &mut rand::thread_rng();

                    // If main tool is passed, use it. Otherwise fallback to default tool
                    if let Some(main_tool) = main_tool {
                        loadout_builder = loadout_builder.active_mainhand(Some(main_tool));
                    } else {
                        loadout_builder = loadout_builder.with_default_maintool(&body);
                    }

                    // If there is config, apply it.
                    // If not, use default equipement for this body.
                    if let Some(asset) = loadout_asset {
                        loadout_builder = loadout_builder.with_asset_expect(&asset, rng);
                    } else {
                        loadout_builder = loadout_builder.with_default_equipment(&body);
                    }

                    // Evaluate lazy function for loadout creation
                    if let Some(make_loadout) = make_loadout {
                        loadout_builder =
                            loadout_builder.with_creator(make_loadout, economy.as_ref());
                    }
                    loadout_builder.build()
                };

                let health = comp::Health::new(body, entity.level.unwrap_or(0));
                let poise = comp::Poise::new(body);

                let can_speak = match body {
                    comp::Body::Humanoid(_) => alignment == comp::Alignment::Npc,
                    comp::Body::BirdMedium(bird_medium) => match bird_medium.species {
                        // Parrots like to have a word in this, too...
                        bird_medium::Species::Parrot => alignment == comp::Alignment::Npc,
                        _ => false,
                    },
                    _ => false,
                };
                let trade_for_site = if matches!(entity.agent_mark, Some(agent::Mark::Merchant)) {
                    economy.map(|e| e.id)
                } else {
                    None
                };

                // TODO: This code sets an appropriate base_damage for the enemy. This doesn't
                // work because the damage is now saved in an ability
                /*
                if let Some(item::ItemKind::Tool(item::ToolData { base_damage, .. })) =
                    &mut loadout.active_item.map(|i| i.item.kind)
                {
                    *base_damage = stats.level.level() as u32 * 3;
                }
                */
                server_emitter.emit(ServerEvent::CreateNpc {
                    pos: Pos(entity.pos),
                    stats,
                    skill_set,
                    health,
                    poise,
                    loadout,
                    agent: if entity.has_agency {
                        Some(comp::Agent::new(
                            Some(entity.pos),
                            &body,
                            Behavior::default()
                                .maybe_with_capabilities(
                                    can_speak.then(|| BehaviorCapability::SPEAK),
                                )
                                .with_trade_site(trade_for_site),
                            matches!(entity.agent_mark, Some(agent::Mark::Guard)),
                        ))
                    } else {
                        None
                    },
                    body,
                    alignment,
                    scale: comp::Scale(scale),
                    home_chunk: Some(comp::HomeChunk(key)),
                    drop_item: entity.loot_drop,
                    rtsim_entity: None,
                })
            }

            // Insert a safezone if chunk contains the spawn position
            if server_settings.safe_spawn && is_spawn_chunk(key, *spawn_point, &terrain) {
                server_emitter.emit(ServerEvent::CreateSafezone {
                    range: Some(100.0),
                    pos: Pos(spawn_point.0),
                });
            }
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
                                &client,
                                &presence,
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
            // TODO: code duplication for chunk insertion between here and state.rs
            if terrain.remove(key).is_some() {
                terrain_changes.removed_chunks.insert(key);
                rtsim.hook_unload_chunk(key);
            }

            chunk_generator.cancel_if_pending(key);
        }
    }
}

pub fn chunk_in_vd(
    player_pos: Vec3<f32>,
    chunk_pos: Vec2<i32>,
    terrain: &TerrainGrid,
    vd: u32,
) -> bool {
    const MINIMUM_UNLOAD_DIST: u32 = 4;

    let player_chunk_pos = terrain.pos_key(player_pos.map(|e| e as i32));

    let adjusted_dist_sqr = (player_chunk_pos - chunk_pos)
        .map(|e: i32| (e.abs() as u32).saturating_sub(2))
        .magnitude_squared();

    adjusted_dist_sqr <= vd.max(MINIMUM_UNLOAD_DIST).pow(2)
}

fn is_spawn_chunk(chunk_pos: Vec2<i32>, spawn_pos: SpawnPoint, terrain: &TerrainGrid) -> bool {
    let spawn_chunk_pos = terrain.pos_key(spawn_pos.0.map(|e| e as i32));
    chunk_pos == spawn_chunk_pos
}
