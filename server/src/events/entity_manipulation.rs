use crate::{client::Client, comp::quadruped_small, Server, SpawnPoint, StateExt};
use common::{
    assets,
    comp::{
        self, object, Alignment, Body, Damage, DamageSource, Group, HealthChange, HealthSource,
        Player, Pos, Stats,
    },
    lottery::Lottery,
    msg::{PlayerListUpdate, ServerMsg},
    outcome::Outcome,
    state::BlockChange,
    sync::{Uid, UidAllocator, WorldSyncExt},
    sys::combat::BLOCK_ANGLE,
    terrain::{Block, TerrainGrid},
    vol::{ReadVol, Vox},
};
use comp::item::Reagent;
use rand::prelude::*;
use specs::{join::Join, saveload::MarkerAllocator, Entity as EcsEntity, WorldExt};
use tracing::error;
use vek::Vec3;

pub fn handle_damage(server: &Server, uid: Uid, change: HealthChange) {
    let state = &server.state;
    let ecs = state.ecs();
    if let Some(entity) = ecs.entity_from_uid(uid.into()) {
        if let Some(stats) = ecs.write_storage::<Stats>().get_mut(entity) {
            stats.health.change_by(change);
        }
    }
}

/// Handle an entity dying. If it is a player, it will send a message to all
/// other players. If the entity that killed it had stats, then give it exp for
/// the kill. Experience given is equal to the level of the entity that was
/// killed times 10.
// NOTE: Clippy incorrectly warns about a needless collect here because it does not
// understand that the pet count (which is computed during the first iteration over the
// members in range) is actually used by the second iteration over the members in range;
// since we have no way of knowing the pet count before the first loop finishes, we
// definitely need at least two loops.   Then (currently) our only options are to store
// the member list in temporary space (e.g. by collecting to a vector), or to repeat
// the loop; but repeating the loop would currently be very inefficient since it has to
// rescan every entity on the server again.
#[allow(clippy::needless_collect)]
pub fn handle_destroy(server: &mut Server, entity: EcsEntity, cause: HealthSource) {
    let state = server.state_mut();

    // Chat message
    if let Some(player) = state.ecs().read_storage::<Player>().get(entity) {
        let msg = if let HealthSource::Attack { by }
        | HealthSource::Projectile { owner: Some(by) } = cause
        {
            state.ecs().entity_from_uid(by.into()).and_then(|attacker| {
                state
                    .ecs()
                    .read_storage::<Player>()
                    .get(attacker)
                    .map(|attacker_alias| {
                        format!("{} was killed by {}", &player.alias, &attacker_alias.alias)
                    })
            })
        } else {
            None
        }
        .unwrap_or(format!("{} died", &player.alias));

        state.notify_registered_clients(comp::ChatType::Kill.server_msg(msg));
    }

    // Give EXP to the killer if entity had stats
    (|| {
        let mut stats = state.ecs().write_storage::<Stats>();
        let by = if let HealthSource::Attack { by } | HealthSource::Projectile { owner: Some(by) } =
            cause
        {
            by
        } else {
            return;
        };
        let attacker = if let Some(attacker) = state.ecs().entity_from_uid(by.into()) {
            attacker
        } else {
            return;
        };
        let entity_stats = if let Some(entity_stats) = stats.get(entity) {
            entity_stats
        } else {
            return;
        };

        let groups = state.ecs().read_storage::<Group>();
        let attacker_group = groups.get(attacker);
        let destroyed_group = groups.get(entity);
        // Don't give exp if attacker destroyed themselves or one of their group members
        if (attacker_group.is_some() && attacker_group == destroyed_group) || attacker == entity {
            return;
        }

        // Maximum distance for other group members to receive exp
        const MAX_EXP_DIST: f32 = 150.0;
        // Attacker gets same as exp of everyone else
        const ATTACKER_EXP_WEIGHT: f32 = 1.0;
        let mut exp_reward = (entity_stats.body_type.base_exp()
            + entity_stats.level.level() * entity_stats.body_type.base_exp_increase())
            as f32;

        // Distribute EXP to group
        let positions = state.ecs().read_storage::<Pos>();
        let alignments = state.ecs().read_storage::<Alignment>();
        let uids = state.ecs().read_storage::<Uid>();
        if let (Some(attacker_group), Some(pos)) = (attacker_group, positions.get(entity)) {
            // TODO: rework if change to groups makes it easier to iterate entities in a
            // group
            let mut num_not_pets_in_range = 0;
            let members_in_range = (
                &state.ecs().entities(),
                &groups,
                &positions,
                alignments.maybe(),
                &uids,
            )
                .join()
                .filter(|(entity, group, member_pos, _, _)| {
                    // Check if: in group, not main attacker, and in range
                    *group == attacker_group
                        && *entity != attacker
                        && pos.0.distance_squared(member_pos.0) < MAX_EXP_DIST.powi(2)
                })
                .map(|(entity, _, _, alignment, uid)| {
                    if !matches!(alignment, Some(Alignment::Owned(owner)) if owner != uid) {
                        num_not_pets_in_range += 1;
                    }

                    entity
                })
                .collect::<Vec<_>>();
            let exp = exp_reward / (num_not_pets_in_range as f32 + ATTACKER_EXP_WEIGHT);
            exp_reward = exp * ATTACKER_EXP_WEIGHT;
            members_in_range.into_iter().for_each(|e| {
                if let Some(stats) = stats.get_mut(e) {
                    stats.exp.change_by(exp.ceil() as i64);
                }
            });
        }

        if let Some(attacker_stats) = stats.get_mut(attacker) {
            // TODO: Discuss whether we should give EXP by Player
            // Killing or not.
            attacker_stats.exp.change_by(exp_reward.ceil() as i64);
        }
    })();

    if state
        .ecs()
        .write_storage::<Client>()
        .get_mut(entity)
        .is_some()
    {
        state
            .ecs()
            .write_storage()
            .insert(entity, comp::Vel(Vec3::zero()))
            .err()
            .map(|e| error!(?e, ?entity, "Failed to set zero vel on dead client"));
        state
            .ecs()
            .write_storage()
            .insert(entity, comp::ForceUpdate)
            .err()
            .map(|e| error!(?e, ?entity, "Failed to insert ForceUpdate on dead client"));
        state
            .ecs()
            .write_storage::<comp::LightEmitter>()
            .remove(entity);
        state
            .ecs()
            .write_storage::<comp::Energy>()
            .get_mut(entity)
            .map(|energy| energy.set_to(energy.maximum(), comp::EnergySource::Revive));
        let _ = state
            .ecs()
            .write_storage::<comp::CharacterState>()
            .insert(entity, comp::CharacterState::default());
    } else if state.ecs().read_storage::<comp::Agent>().contains(entity) {
        // Decide for a loot drop before turning into a lootbag
        let old_body = state.ecs().write_storage::<Body>().remove(entity);
        let mut rng = rand::thread_rng();
        let drop = match old_body {
            Some(common::comp::Body::Humanoid(_)) => match rng.gen_range(0, 4) {
                0 => assets::load_expect::<Lottery<String>>(
                    "common.loot_tables.loot_table_humanoids",
                ),
                1 => assets::load_expect::<Lottery<String>>(
                    "common.loot_tables.loot_table_armor_light",
                ),
                2 => assets::load_expect::<Lottery<String>>(
                    "common.loot_tables.loot_table_armor_cloth",
                ),
                3 => assets::load_expect::<Lottery<String>>(
                    "common.loot_tables.loot_table_weapon_common",
                ),
                _ => assets::load_expect::<Lottery<String>>(
                    "common.loot_tables.loot_table_humanoids",
                ),
            },
            Some(common::comp::Body::QuadrupedSmall(quadruped_small)) => {
                match quadruped_small.species {
                    quadruped_small::Species::Dodarock => match rng.gen_range(0, 6) {
                        0 => assets::load_expect::<Lottery<String>>(
                            "common.loot_tables.loot_table_armor_misc",
                        ),
                        1 => assets::load_expect::<Lottery<String>>(
                            "common.loot_tables.loot_table_rocks",
                        ),
                        _ => assets::load_expect::<Lottery<String>>(
                            "common.loot_tables.loot_table_rocks",
                        ),
                    },
                    _ => match rng.gen_range(0, 4) {
                        0 => assets::load_expect::<Lottery<String>>(
                            "common.loot_tables.loot_table_food",
                        ),
                        1 => assets::load_expect::<Lottery<String>>(
                            "common.loot_tables.loot_table_armor_misc",
                        ),
                        2 => assets::load_expect::<Lottery<String>>(
                            "common.loot_tables.loot_table_animal_parts",
                        ),
                        _ => assets::load_expect::<Lottery<String>>(
                            "common.loot_tables.loot_table_animal_parts",
                        ),
                    },
                }
            },
            Some(common::comp::Body::QuadrupedMedium(quadruped_medium)) => {
                match quadruped_medium.species {
                    _ => match rng.gen_range(0, 4) {
                        0 => assets::load_expect::<Lottery<String>>(
                            "common.loot_tables.loot_table_food",
                        ),
                        1 => assets::load_expect::<Lottery<String>>(
                            "common.loot_tables.loot_table_armor_misc",
                        ),
                        2 => assets::load_expect::<Lottery<String>>(
                            "common.loot_tables.loot_table_animal_parts",
                        ),
                        _ => assets::load_expect::<Lottery<String>>(
                            "common.loot_tables.loot_table_animal_parts",
                        ),
                    },
                }
            },
            Some(common::comp::Body::BirdMedium(bird_medium)) => match bird_medium.species {
                _ => match rng.gen_range(0, 3) {
                    0 => {
                        assets::load_expect::<Lottery<String>>("common.loot_tables.loot_table_food")
                    },
                    1 => assets::load_expect::<Lottery<String>>(
                        "common.loot_tables.loot_table_armor_misc",
                    ),
                    _ => assets::load_expect::<Lottery<String>>("common.loot_tables.loot_table"),
                },
            },
            Some(common::comp::Body::BipedLarge(biped_large)) => match biped_large.species {
                _ => match rng.gen_range(0, 9) {
                    0 => {
                        assets::load_expect::<Lottery<String>>("common.loot_tables.loot_table_food")
                    },
                    1 => assets::load_expect::<Lottery<String>>(
                        "common.loot_tables.loot_table_armor_misc",
                    ),
                    2 => assets::load_expect::<Lottery<String>>(
                        "common.loot_tables.loot_table_armor_light",
                    ),
                    3 => assets::load_expect::<Lottery<String>>(
                        "common.loot_tables.loot_table_armor_heavy",
                    ),
                    4 => assets::load_expect::<Lottery<String>>(
                        "common.loot_tables.loot_table_armor_misc",
                    ),
                    5 => assets::load_expect::<Lottery<String>>(
                        "common.loot_tables.loot_table_weapon_common",
                    ),
                    6 => assets::load_expect::<Lottery<String>>(
                        "common.loot_tables.loot_table_weapon_uncommon",
                    ),
                    7 => assets::load_expect::<Lottery<String>>(
                        "common.loot_tables.loot_table_weapon_rare",
                    ),
                    _ => assets::load_expect::<Lottery<String>>("common.loot_tables.loot_table"),
                },
            },
            Some(common::comp::Body::Golem(golem)) => match golem.species {
                _ => match rng.gen_range(0, 9) {
                    0 => {
                        assets::load_expect::<Lottery<String>>("common.loot_tables.loot_table_food")
                    },
                    1 => assets::load_expect::<Lottery<String>>(
                        "common.loot_tables.loot_table_armor_misc",
                    ),
                    2 => assets::load_expect::<Lottery<String>>(
                        "common.loot_tables.loot_table_armor_light",
                    ),
                    3 => assets::load_expect::<Lottery<String>>(
                        "common.loot_tables.loot_table_armor_heavy",
                    ),
                    4 => assets::load_expect::<Lottery<String>>(
                        "common.loot_tables.loot_table_armor_misc",
                    ),
                    5 => assets::load_expect::<Lottery<String>>(
                        "common.loot_tables.loot_table_weapon_common",
                    ),
                    6 => assets::load_expect::<Lottery<String>>(
                        "common.loot_tables.loot_table_weapon_uncommon",
                    ),
                    7 => assets::load_expect::<Lottery<String>>(
                        "common.loot_tables.loot_table_weapon_rare",
                    ),
                    _ => assets::load_expect::<Lottery<String>>("common.loot_tables.loot_table"),
                },
            },
            Some(common::comp::Body::Critter(critter)) => match critter.species {
                _ => match rng.gen_range(0, 3) {
                    0 => {
                        assets::load_expect::<Lottery<String>>("common.loot_tables.loot_table_food")
                    },
                    1 => assets::load_expect::<Lottery<String>>(
                        "common.loot_tables.loot_table_animal_parts",
                    ),
                    _ => assets::load_expect::<Lottery<String>>("common.loot_tables.loot_table"),
                },
            },
            Some(common::comp::Body::Dragon(_)) => {
                assets::load_expect::<Lottery<String>>("common.loot_tables.loot_table_weapon_rare")
            },
            Some(common::comp::Body::QuadrupedLow(quadruped_low)) => match quadruped_low.species {
                _ => match rng.gen_range(0, 3) {
                    0 => {
                        assets::load_expect::<Lottery<String>>("common.loot_tables.loot_table_food")
                    },
                    1 => assets::load_expect::<Lottery<String>>(
                        "common.loot_tables.loot_table_animal_parts",
                    ),
                    _ => assets::load_expect::<Lottery<String>>("common.loot_tables.loot_table"),
                },
            },
            _ => assets::load_expect::<Lottery<String>>("common.loot_tables.loot_table"),
        };
        let drop = drop.choose();
        // Replace npc with lootbag containing drop
        let _ = state
            .ecs()
            .write_storage()
            .insert(entity, Body::Object(object::Body::Pouch));
        let mut item_drops = state.ecs().write_storage::<comp::ItemDrop>();
        let item = if let Some(item_drop) = item_drops.get(entity).cloned() {
            item_drops.remove(entity);
            item_drop.0
        } else {
            assets::load_expect_cloned(drop)
        };

        let _ = state.ecs().write_storage().insert(entity, item);

        state.ecs().write_storage::<comp::Stats>().remove(entity);
        state.ecs().write_storage::<comp::Agent>().remove(entity);
        state
            .ecs()
            .write_storage::<comp::LightEmitter>()
            .remove(entity);
        state
            .ecs()
            .write_storage::<comp::CharacterState>()
            .remove(entity);
        state
            .ecs()
            .write_storage::<comp::Controller>()
            .remove(entity);
    } else {
        let _ = state
            .delete_entity_recorded(entity)
            .map_err(|e| error!(?e, ?entity, "Failed to delete destroyed entity"));
    }

    // TODO: Add Delete(time_left: Duration) component
    /*
    // If not a player delete the entity
    if let Err(err) = state.delete_entity_recorded(entity) {
        error!(?e, "Failed to delete destroyed entity");
    }
    */
}

pub fn handle_land_on_ground(server: &Server, entity: EcsEntity, vel: Vec3<f32>) {
    let state = &server.state;
    if vel.z <= -30.0 {
        if let Some(stats) = state.ecs().write_storage::<comp::Stats>().get_mut(entity) {
            let falldmg = (vel.z.powi(2) / 20.0 - 40.0) * 10.0;
            let mut damage = Damage {
                healthchange: -falldmg,
                source: DamageSource::Falling,
            };
            if let Some(loadout) = state.ecs().read_storage::<comp::Loadout>().get(entity) {
                damage.modify_damage(false, loadout);
            }
            stats.health.change_by(comp::HealthChange {
                amount: damage.healthchange as i32,
                cause: comp::HealthSource::World,
            });
        }
    }
}

pub fn handle_respawn(server: &Server, entity: EcsEntity) {
    let state = &server.state;

    // Only clients can respawn
    if state
        .ecs()
        .write_storage::<Client>()
        .get_mut(entity)
        .is_some()
    {
        let respawn_point = state
            .read_component_cloned::<comp::Waypoint>(entity)
            .map(|wp| wp.get_pos())
            .unwrap_or(state.ecs().read_resource::<SpawnPoint>().0);

        state
            .ecs()
            .write_storage::<comp::Stats>()
            .get_mut(entity)
            .map(|stats| stats.revive());
        state
            .ecs()
            .write_storage::<comp::Pos>()
            .get_mut(entity)
            .map(|pos| pos.0 = respawn_point);
        state
            .ecs()
            .write_storage()
            .insert(entity, comp::ForceUpdate)
            .err()
            .map(|e| {
                error!(
                    ?e,
                    "Error inserting ForceUpdate component when respawning client"
                )
            });
    }
}

pub fn handle_explosion(
    server: &Server,
    pos: Vec3<f32>,
    power: f32,
    owner: Option<Uid>,
    friendly_damage: bool,
    reagent: Option<Reagent>,
) {
    // Go through all other entities
    let hit_range = 3.0 * power;
    let ecs = &server.state.ecs();

    // Add an outcome
    ecs.write_resource::<Vec<Outcome>>()
        .push(Outcome::Explosion {
            pos,
            power,
            reagent,
        });

    let owner_entity = owner.and_then(|uid| {
        ecs.read_resource::<UidAllocator>()
            .retrieve_entity_internal(uid.into())
    });
    let groups = ecs.read_storage::<comp::Group>();

    for (entity_b, pos_b, ori_b, character_b, stats_b, loadout_b) in (
        &ecs.entities(),
        &ecs.read_storage::<comp::Pos>(),
        &ecs.read_storage::<comp::Ori>(),
        ecs.read_storage::<comp::CharacterState>().maybe(),
        &mut ecs.write_storage::<comp::Stats>(),
        ecs.read_storage::<comp::Loadout>().maybe(),
    )
        .join()
    {
        let distance_squared = pos.distance_squared(pos_b.0);
        // Check if it is a hit
        if !stats_b.is_dead
            // RADIUS
            && distance_squared < hit_range.powi(2)
            // Skip if they are in the same group and friendly_damage is turned off for the
            // explosion
            && (friendly_damage || !owner_entity
                    .and_then(|e| groups.get(e))
                    .map_or(false, |group_a| Some(group_a) == groups.get(entity_b)))
        {
            // Weapon gives base damage
            let dmg = (1.0 - distance_squared / hit_range.powi(2)) * power * 130.0;

            let mut damage = Damage {
                healthchange: -dmg,
                source: DamageSource::Explosion,
            };

            let block = character_b.map(|c_b| c_b.is_block()).unwrap_or(false)
                && ori_b.0.angle_between(pos - pos_b.0) < BLOCK_ANGLE.to_radians() / 2.0;

            if let Some(loadout) = loadout_b {
                damage.modify_damage(block, loadout);
            }

            stats_b.health.change_by(HealthChange {
                amount: damage.healthchange as i32,
                cause: HealthSource::Projectile { owner },
            });
        }
    }

    const RAYS: usize = 500;

    // Color terrain
    let mut touched_blocks = Vec::new();
    let color_range = power * 2.7;
    for _ in 0..RAYS {
        let dir = Vec3::new(
            rand::random::<f32>() - 0.5,
            rand::random::<f32>() - 0.5,
            rand::random::<f32>() - 0.5,
        )
        .normalized();

        let _ = ecs
            .read_resource::<TerrainGrid>()
            .ray(pos, pos + dir * color_range)
            .until(|_| rand::random::<f32>() < 0.05)
            .for_each(|_: &Block, pos| touched_blocks.push(pos))
            .cast();
    }

    let terrain = ecs.read_resource::<TerrainGrid>();
    let mut block_change = ecs.write_resource::<BlockChange>();
    for block_pos in touched_blocks {
        if let Ok(block) = terrain.get(block_pos) {
            let diff2 = block_pos.map(|b| b as f32).distance_squared(pos);
            let fade = (1.0 - diff2 / color_range.powi(2)).max(0.0);
            if let Some(mut color) = block.get_color() {
                let r = color[0] as f32 + (fade * (color[0] as f32 * 0.5 - color[0] as f32));
                let g = color[1] as f32 + (fade * (color[1] as f32 * 0.3 - color[1] as f32));
                let b = color[2] as f32 + (fade * (color[2] as f32 * 0.3 - color[2] as f32));
                color[0] = r as u8;
                color[1] = g as u8;
                color[2] = b as u8;
                block_change.set(block_pos, Block::new(block.kind(), color));
            }
        }
    }

    // Destroy terrain
    for _ in 0..RAYS {
        let dir = Vec3::new(
            rand::random::<f32>() - 0.5,
            rand::random::<f32>() - 0.5,
            rand::random::<f32>() - 0.15,
        )
        .normalized();

        let terrain = ecs.read_resource::<TerrainGrid>();
        let _ = terrain
            .ray(pos, pos + dir * power)
            .until(|block| block.is_fluid() || rand::random::<f32>() < 0.05)
            .for_each(|block: &Block, pos| {
                if block.is_explodable() {
                    block_change.set(pos, Block::empty());
                }
            })
            .cast();
    }
}

pub fn handle_level_up(server: &mut Server, entity: EcsEntity, new_level: u32) {
    let uids = server.state.ecs().read_storage::<Uid>();
    let uid = uids
        .get(entity)
        .expect("Failed to fetch uid component for entity.");

    server
        .state
        .notify_registered_clients(ServerMsg::PlayerListUpdate(PlayerListUpdate::LevelChange(
            *uid, new_level,
        )));
}
