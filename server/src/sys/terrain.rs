use super::SysTimer;
use crate::{chunk_generator::ChunkGenerator, client::Client, Tick};
use common::{
    comp::{self, bird_medium, Alignment, Player, Pos},
    event::{EventBus, ServerEvent},
    generation::get_npc_name,
    msg::ServerInGame,
    npc::NPC_NAMES,
    span,
    state::TerrainChanges,
    terrain::TerrainGrid,
    LoadoutBuilder,
};
use rand::Rng;
use specs::{Join, Read, ReadStorage, System, Write, WriteExpect, WriteStorage};
use std::sync::Arc;
use vek::*;

/// This system will handle loading generated chunks and unloading
/// unneeded chunks.
///     1. Inserts newly generated chunks into the TerrainGrid
///     2. Sends new chunks to nearby clients
///     3. Handles the chunk's supplement (e.g. npcs)
///     4. Removes chunks outside the range of players
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)] // TODO: Pending review in #587
    type SystemData = (
        Read<'a, EventBus<ServerEvent>>,
        Read<'a, Tick>,
        Write<'a, SysTimer<Self>>,
        WriteExpect<'a, ChunkGenerator>,
        WriteExpect<'a, TerrainGrid>,
        Write<'a, TerrainChanges>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Player>,
        WriteStorage<'a, Client>,
    );

    fn run(
        &mut self,
        (
            server_event_bus,
            tick,
            mut timer,
            mut chunk_generator,
            mut terrain,
            mut terrain_changes,
            positions,
            players,
            mut clients,
        ): Self::SystemData,
    ) {
        span!(_guard, "run", "terrain::Sys::run");
        timer.start();

        let mut server_emitter = server_event_bus.emitter();

        // Fetch any generated `TerrainChunk`s and insert them into the terrain.
        // Also, send the chunk data to anybody that is close by.
        'insert_terrain_chunks: while let Some((key, res)) = chunk_generator.recv_new_chunk() {
            let (chunk, supplement) = match res {
                Ok((chunk, supplement)) => (chunk, supplement),
                Err(Some(entity)) => {
                    if let Some(client) = clients.get_mut(entity) {
                        client.send_msg(ServerInGame::TerrainChunkUpdate {
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
            // Send the chunk to all nearby players.
            for (view_distance, pos, client) in (&players, &positions, &mut clients)
                .join()
                .filter_map(|(player, pos, client)| {
                    player.view_distance.map(|vd| (vd, pos, client))
                })
            {
                let chunk_pos = terrain.pos_key(pos.0.map(|e| e as i32));
                // Subtract 2 from the offset before computing squared magnitude
                // 1 since chunks need neighbors to be meshed
                // 1 to act as a buffer if the player moves in that direction
                let adjusted_dist_sqr = (chunk_pos - key)
                    .map(|e: i32| (e.abs() as u32).saturating_sub(2))
                    .magnitude_squared();

                if adjusted_dist_sqr <= view_distance.pow(2) {
                    client.send_msg(ServerInGame::TerrainChunkUpdate {
                        key,
                        chunk: Ok(Box::new(chunk.clone())),
                    });
                }
            }

            // TODO: code duplication for chunk insertion between here and state.rs
            // Insert the chunk into terrain changes
            if terrain.insert(key, Arc::new(chunk)).is_some() {
                terrain_changes.modified_chunks.insert(key);
            } else {
                terrain_changes.new_chunks.insert(key);
            }

            // Handle chunk supplement
            for entity in supplement.entities {
                if entity.is_waypoint {
                    server_emitter.emit(ServerEvent::CreateWaypoint(entity.pos));
                    continue;
                }

                let mut body = entity.body;
                let name = entity.name.unwrap_or_else(|| "Unnamed".to_string());
                let alignment = entity.alignment;
                let main_tool = entity.main_tool;
                let mut stats = comp::Stats::new(name, body);
                // let damage = stats.level.level() as i32; TODO: Make NPC base damage
                // non-linearly depend on their level

                let mut scale = entity.scale;

                // TODO: Remove this and implement scaling or level depending on stuff like
                // species instead
                stats.level.set_level(
                    entity.level.unwrap_or_else(|| {
                        (rand::thread_rng().gen_range(1, 9) as f32 * scale) as u32
                    }),
                );

                // Replace stuff if it's a boss
                if entity.is_giant {
                    if rand::random::<f32>() < 0.65 && entity.alignment != Alignment::Enemy {
                        let body_new = comp::humanoid::Body::random();
                        body = comp::Body::Humanoid(body_new);
                        stats = comp::Stats::new(
                            format!(
                                "Gentle Giant {}",
                                get_npc_name(&NPC_NAMES.humanoid, body_new.species)
                            ),
                            body,
                        );
                    }
                    stats.level.set_level(rand::thread_rng().gen_range(30, 35));
                    scale = 2.0 + rand::random::<f32>();
                }

                let loadout =
                    LoadoutBuilder::build_loadout(body, alignment, main_tool, entity.is_giant)
                        .build();

                stats.update_max_hp(stats.body_type);

                stats
                    .health
                    .set_to(stats.health.maximum(), comp::HealthSource::Revive);

                let can_speak = match body {
                    comp::Body::Humanoid(_) => alignment == comp::Alignment::Npc,
                    comp::Body::BirdMedium(bird_medium) => match bird_medium.species {
                        // Parrots like to have a word in this, too...
                        bird_medium::Species::Parrot => alignment == comp::Alignment::Npc,
                        _ => false,
                    },
                    _ => false,
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
                    loadout,
                    agent: if entity.has_agency {
                        Some(comp::Agent::new(entity.pos, can_speak, &body))
                    } else {
                        None
                    },
                    body,
                    alignment,
                    scale: comp::Scale(scale),
                    drop_item: entity.loot_drop,
                })
            }
        }

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
                for (player, pos) in (&players, &positions).join() {
                    if player
                        .view_distance
                        .map(|vd| chunk_in_vd(pos.0, chunk_key, &terrain, vd))
                        .unwrap_or(false)
                    {
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
            }

            chunk_generator.cancel_if_pending(key);
        }

        timer.end()
    }
}

pub fn chunk_in_vd(
    player_pos: Vec3<f32>,
    chunk_pos: Vec2<i32>,
    terrain: &TerrainGrid,
    vd: u32,
) -> bool {
    let player_chunk_pos = terrain.pos_key(player_pos.map(|e| e as i32));

    let adjusted_dist_sqr = (player_chunk_pos - chunk_pos)
        .map(|e: i32| (e.abs() as u32).saturating_sub(2))
        .magnitude_squared();

    adjusted_dist_sqr <= vd.pow(2)
}
