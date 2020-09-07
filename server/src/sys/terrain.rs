use super::SysTimer;
use crate::{chunk_generator::ChunkGenerator, client::Client, Tick};
use common::{
    assets::Asset,
    comp::{
        self, bird_medium,
        item::{self, ItemAsset},
        Alignment, CharacterAbility, ItemConfig, Player, Pos,
    },
    event::{EventBus, ServerEvent},
    generation::get_npc_name,
    msg::ServerMsg,
    npc::NPC_NAMES,
    span,
    state::TerrainChanges,
    terrain::TerrainGrid,
    LoadoutBuilder,
};
use rand::Rng;
use specs::{Join, Read, ReadStorage, System, Write, WriteExpect, WriteStorage};
use std::{sync::Arc, time::Duration};
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
                        client.notify(ServerMsg::TerrainChunkUpdate {
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

                let active_item =
                    if let Some(item::ItemKind::Tool(tool)) = main_tool.as_ref().map(|i| &i.kind) {
                        let mut abilities = tool.get_abilities();
                        let mut ability_drain = abilities.drain(..);

                        main_tool.map(|item| comp::ItemConfig {
                            item,
                            ability1: ability_drain.next(),
                            ability2: ability_drain.next(),
                            ability3: ability_drain.next(),
                            block_ability: None,
                            dodge_ability: Some(comp::CharacterAbility::Roll),
                        })
                    } else {
                        Some(ItemConfig {
                            // We need the empty item so npcs can attack
                            item: ItemAsset::load_expect_cloned("common.items.weapons.empty.empty"),
                            ability1: Some(CharacterAbility::BasicMelee {
                                energy_cost: 0,
                                buildup_duration: Duration::from_millis(0),
                                recover_duration: Duration::from_millis(400),
                                base_healthchange: -40,
                                range: 3.5,
                                max_angle: 15.0,
                            }),
                            ability2: None,
                            ability3: None,
                            block_ability: None,
                            dodge_ability: None,
                        })
                    };

                let mut loadout = match alignment {
                    comp::Alignment::Npc => comp::Loadout {
                        active_item,
                        second_item: None,
                        shoulder: None,
                        chest: Some(ItemAsset::load_expect_cloned(
                            match rand::thread_rng().gen_range(0, 10) {
                                0 => "common.items.npc_armor.chest.worker_green_0",
                                1 => "common.items.npc_armor.chest.worker_green_1",
                                2 => "common.items.npc_armor.chest.worker_red_0",
                                3 => "common.items.npc_armor.chest.worker_red_1",
                                4 => "common.items.npc_armor.chest.worker_purple_0",
                                5 => "common.items.npc_armor.chest.worker_purple_1",
                                6 => "common.items.npc_armor.chest.worker_yellow_0",
                                7 => "common.items.npc_armor.chest.worker_yellow_1",
                                8 => "common.items.npc_armor.chest.worker_orange_0",
                                _ => "common.items.npc_armor.chest.worker_orange_1",
                            },
                        )),
                        belt: Some(ItemAsset::load_expect_cloned(
                            "common.items.armor.belt.leather_0",
                        )),
                        hand: None,
                        pants: Some(ItemAsset::load_expect_cloned(
                            "common.items.armor.pants.worker_blue_0",
                        )),
                        foot: Some(ItemAsset::load_expect_cloned(
                            match rand::thread_rng().gen_range(0, 2) {
                                0 => "common.items.armor.foot.leather_0",
                                _ => "common.items.armor.starter.sandals_0",
                            },
                        )),
                        back: None,
                        ring: None,
                        neck: None,
                        lantern: None,
                        head: None,
                        tabard: None,
                    },
                    comp::Alignment::Enemy => comp::Loadout {
                        active_item,
                        second_item: None,
                        shoulder: Some(ItemAsset::load_expect_cloned(
                            "common.items.npc_armor.shoulder.cultist_shoulder_purple",
                        )),
                        chest: Some(ItemAsset::load_expect_cloned(
                            "common.items.npc_armor.chest.cultist_chest_purple",
                        )),
                        belt: Some(ItemAsset::load_expect_cloned(
                            "common.items.npc_armor.belt.cultist_belt",
                        )),
                        hand: Some(ItemAsset::load_expect_cloned(
                            "common.items.npc_armor.hand.cultist_hands_purple",
                        )),
                        pants: Some(ItemAsset::load_expect_cloned(
                            "common.items.npc_armor.pants.cultist_legs_purple",
                        )),
                        foot: Some(ItemAsset::load_expect_cloned(
                            "common.items.npc_armor.foot.cultist_boots",
                        )),
                        back: Some(ItemAsset::load_expect_cloned(
                            "common.items.npc_armor.back.dungeon_purple-0",
                        )),
                        ring: None,
                        neck: None,
                        lantern: Some(ItemAsset::load_expect_cloned(
                            "common.items.lantern.black_0",
                        )),
                        head: None,
                        tabard: None,
                    },
                    _ => LoadoutBuilder::animal(entity.body).build(),
                };

                loadout = match body {
                    comp::Body::Humanoid(_) => loadout,
                    _ => LoadoutBuilder::animal(entity.body).build(),
                };

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
                    if rand::random::<f32>() < 0.65 {
                        let body_new = comp::humanoid::Body::random();
                        body = comp::Body::Humanoid(body_new);
                        let adjective = if let Alignment::Enemy = entity.alignment {
                            "Angry"
                        } else {
                            "Gentle"
                        };
                        stats = comp::Stats::new(
                            format!(
                                "{} Giant {}",
                                adjective,
                                get_npc_name(&NPC_NAMES.humanoid, body_new.species)
                            ),
                            body,
                        );
                    }
                    loadout = comp::Loadout {
                        active_item: Some(comp::ItemConfig {
                            item: ItemAsset::load_expect_cloned(
                                "common.items.npc_weapons.sword.zweihander_sword_0",
                            ),
                            ability1: Some(CharacterAbility::BasicMelee {
                                energy_cost: 0,
                                buildup_duration: Duration::from_millis(800),
                                recover_duration: Duration::from_millis(200),
                                base_healthchange: -100,
                                range: 3.5,
                                max_angle: 60.0,
                            }),
                            ability2: None,
                            ability3: None,
                            block_ability: None,
                            dodge_ability: None,
                        }),
                        second_item: None,
                        shoulder: Some(ItemAsset::load_expect_cloned(
                            "common.items.armor.shoulder.plate_0",
                        )),
                        chest: Some(ItemAsset::load_expect_cloned(
                            "common.items.armor.chest.plate_green_0",
                        )),
                        belt: Some(ItemAsset::load_expect_cloned(
                            "common.items.armor.belt.plate_0",
                        )),
                        hand: Some(ItemAsset::load_expect_cloned(
                            "common.items.armor.hand.plate_0",
                        )),
                        pants: Some(ItemAsset::load_expect_cloned(
                            "common.items.armor.pants.plate_green_0",
                        )),
                        foot: Some(ItemAsset::load_expect_cloned(
                            "common.items.armor.foot.plate_0",
                        )),
                        back: None,
                        ring: None,
                        neck: None,
                        lantern: None,
                        head: None,
                        tabard: None,
                    };

                    stats.level.set_level(rand::thread_rng().gen_range(30, 35));
                    scale = 2.0 + rand::random::<f32>();
                }

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
