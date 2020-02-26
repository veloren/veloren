use super::SysTimer;
use crate::{chunk_generator::ChunkGenerator, client::Client, Tick};
use common::{
    assets,
    comp::{self, item, Player, Pos},
    event::{EventBus, ServerEvent},
    generation::EntityKind,
    msg::ServerMsg,
    npc::{self, NPC_NAMES},
    state::TerrainChanges,
    terrain::TerrainGrid,
};
use rand::{seq::SliceRandom, Rng};
use specs::{Join, Read, ReadStorage, System, Write, WriteExpect, WriteStorage};
use std::sync::Arc;
use vek::*;

/// This system will handle loading generated chunks and unloading
/// uneeded chunks.
///     1. Inserts newly generated chunks into the TerrainGrid
///     2. Sends new chunks to neaby clients
///     3. Handles the chunk's supplement (e.g. npcs)
///     4. Removes chunks outside the range of players
pub struct Sys;
impl<'a> System<'a> for Sys {
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
            server_emitter,
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
        timer.start();

        // Fetch any generated `TerrainChunk`s and insert them into the terrain.
        // Also, send the chunk data to anybody that is close by.
        'insert_terrain_chunks: while let Some((key, res)) = chunk_generator.recv_new_chunk() {
            let (chunk, supplement) = match res {
                Ok((chunk, supplement)) => (chunk, supplement),
                Err(entity) => {
                    if let Some(client) = clients.get_mut(entity) {
                        client.notify(ServerMsg::TerrainChunkUpdate {
                            key,
                            chunk: Err(()),
                        });
                    }
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
                let adjusted_dist_sqr = (Vec2::from(chunk_pos) - Vec2::from(key))
                    .map(|e: i32| (e.abs() as u32).checked_sub(2).unwrap_or(0))
                    .magnitude_squared();

                if adjusted_dist_sqr <= view_distance.pow(2) {
                    client.notify(ServerMsg::TerrainChunkUpdate {
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
                if let EntityKind::Waypoint = entity.kind {
                    server_emitter.emit(ServerEvent::CreateWaypoint(entity.pos));
                } else {
                    fn get_npc_name<
                        'a,
                        Species,
                        SpeciesData: for<'b> core::ops::Index<&'b Species, Output = npc::SpeciesNames>,
                    >(
                        body_data: &'a comp::BodyData<npc::BodyNames, SpeciesData>,
                        species: Species,
                    ) -> &'a str {
                        &body_data.species[&species].generic
                    }
                    const SPAWN_NPCS: &'static [fn() -> (
                        String,
                        comp::Body,
                        Option<comp::Item>,
                        comp::Alignment,
                    )] = &[
                        (|| {
                            let body = comp::humanoid::Body::random();
                            (
                                format!(
                                    "{} Traveler",
                                    get_npc_name(&NPC_NAMES.humanoid, body.race)
                                ),
                                comp::Body::Humanoid(body),
                                Some(assets::load_expect_cloned("common.items.weapons.staff_1")),
                                comp::Alignment::Npc,
                            )
                        }) as _,
                        (|| {
                            let body = comp::humanoid::Body::random();
                            (
                                format!("{} Bandit", get_npc_name(&NPC_NAMES.humanoid, body.race)),
                                comp::Body::Humanoid(body),
                                Some(assets::load_expect_cloned("common.items.weapons.staff_1")),
                                comp::Alignment::Enemy,
                            )
                        }) as _,
                        (|| {
                            let body = comp::quadruped_medium::Body::random();
                            (
                                get_npc_name(&NPC_NAMES.quadruped_medium, body.species).into(),
                                comp::Body::QuadrupedMedium(body),
                                None,
                                comp::Alignment::Enemy,
                            )
                        }) as _,
                        (|| {
                            let body = comp::bird_medium::Body::random();
                            (
                                get_npc_name(&NPC_NAMES.bird_medium, body.species).into(),
                                comp::Body::BirdMedium(body),
                                None,
                                comp::Alignment::Wild,
                            )
                        }) as _,
                        (|| {
                            let body = comp::critter::Body::random();
                            (
                                get_npc_name(&NPC_NAMES.critter, body.species).into(),
                                comp::Body::Critter(body),
                                None,
                                comp::Alignment::Wild,
                            )
                        }) as _,
                        (|| {
                            let body = comp::quadruped_small::Body::random();
                            (
                                get_npc_name(&NPC_NAMES.quadruped_small, body.species).into(),
                                comp::Body::QuadrupedSmall(body),
                                None,
                                comp::Alignment::Wild,
                            )
                        }),
                    ];
                    let (name, mut body, main, mut alignment) = SPAWN_NPCS
                        .choose(&mut rand::thread_rng())
                        .expect("SPAWN_NPCS is nonempty")(
                    );
                    let mut stats = comp::Stats::new(name, body);
                    let mut loadout = comp::Loadout {
                        active_item: main.map(|item| comp::ItemConfig {
                            item,
                            primary_ability: None,
                            secondary_ability: None,
                            block_ability: None,
                            dodge_ability: None,
                        }),
                        second_item: None,
                        shoulder: None,
                        chest: None,
                        belt: None,
                        hand: None,
                        pants: None,
                        foot: None,
                    };

                    let mut scale = 1.0;

                    // TODO: Remove this and implement scaling or level depending on stuff like
                    // species instead
                    stats.level.set_level(rand::thread_rng().gen_range(1, 4));

                    if let EntityKind::Boss = entity.kind {
                        if rand::random::<f32>() < 0.65 {
                            let body_new = comp::humanoid::Body::random();
                            body = comp::Body::Humanoid(body_new);
                            alignment = comp::Alignment::Npc;
                            stats = comp::Stats::new(
                                format!(
                                    "Fearless Giant {}",
                                    get_npc_name(&NPC_NAMES.humanoid, body_new.race)
                                ),
                                body,
                            );
                        }
                        loadout = comp::Loadout {
                            active_item: Some(comp::ItemConfig {
                                item: assets::load_expect_cloned("common.items.weapons.hammer_1"),
                                primary_ability: None, /* TODO: when implementing this, make sure
                                                        * to adjust the base damage (see todo
                                                        * below) */
                                secondary_ability: None,
                                block_ability: None,
                                dodge_ability: None,
                            }),
                            second_item: None,
                            shoulder: None,
                            chest: None,
                            belt: None,
                            hand: None,
                            pants: None,
                            foot: None,
                        };

                        stats.level.set_level(rand::thread_rng().gen_range(8, 15));
                        scale = 2.0 + rand::random::<f32>();
                    }

                    stats.update_max_hp();

                    stats
                        .health
                        .set_to(stats.health.maximum(), comp::HealthSource::Revive);

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
                        body,
                        alignment,
                        agent: comp::Agent::default().with_patrol_origin(entity.pos),
                        scale: comp::Scale(scale),
                    })
                }
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

    let adjusted_dist_sqr = Vec2::from(player_chunk_pos - chunk_pos)
        .map(|e: i32| (e.abs() as u32).checked_sub(2).unwrap_or(0))
        .magnitude_squared();

    adjusted_dist_sqr <= vd.pow(2)
}
