use crate::{
    client::Client,
    comp::{biped_large, quadruped_medium, quadruped_small, PhysicsState},
    rtsim::RtSim,
    Server, SpawnPoint, StateExt,
};
use common::{
    assets::AssetExt,
    comp::{
        self, aura, buff,
        chat::{KillSource, KillType},
        object, Alignment, Body, Energy, EnergyChange, Group, Health, HealthChange, HealthSource,
        Item, Player, Pos, Stats,
    },
    effect::Effect,
    lottery::Lottery,
    outcome::Outcome,
    rtsim::RtSimEntity,
    terrain::{Block, TerrainGrid},
    uid::{Uid, UidAllocator},
    vol::ReadVol,
    Damage, DamageSource, Explosion, GroupTarget, RadiusEffect,
};
use common_net::{
    msg::{PlayerListUpdate, ServerGeneral},
    sync::WorldSyncExt,
};
use common_sys::state::BlockChange;
use comp::item::Reagent;
use rand::prelude::*;
use specs::{join::Join, saveload::MarkerAllocator, Entity as EcsEntity, WorldExt};
use tracing::error;
use vek::Vec3;

pub fn handle_damage(server: &Server, entity: EcsEntity, change: HealthChange) {
    let ecs = &server.state.ecs();
    if let Some(health) = ecs.write_storage::<Health>().get_mut(entity) {
        health.change_by(change);
    }
}

pub fn handle_knockback(server: &Server, entity: EcsEntity, impulse: Vec3<f32>) {
    let ecs = &server.state.ecs();
    let clients = ecs.read_storage::<Client>();

    if let Some(physics) = ecs.read_storage::<PhysicsState>().get(entity) {
        //Check if the entity is on a surface. If it is not, reduce knockback.
        let mut impulse = impulse
            * if physics.on_surface().is_some() {
                1.0
            } else {
                0.4
            };
        if let Some(mass) = ecs.read_storage::<comp::Mass>().get(entity) {
            impulse /= mass.0;
        }
        let mut velocities = ecs.write_storage::<comp::Vel>();
        if let Some(vel) = velocities.get_mut(entity) {
            vel.0 = impulse;
        }
        if let Some(client) = clients.get(entity) {
            client.send_fallible(ServerGeneral::Knockback(impulse));
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

    // TODO: Investigate duplicate `Destroy` events (but don't remove this).
    // If the entity was already deleted, it can't be destroyed again.
    if !state.ecs().is_alive(entity) {
        return;
    }

    // Chat message
    // If it was a player that died
    if let Some(_player) = state.ecs().read_storage::<Player>().get(entity) {
        if let Some(uid) = state.ecs().read_storage::<Uid>().get(entity) {
            let kill_source = match cause {
                HealthSource::Damage {
                    kind: DamageSource::Melee,
                    by: Some(by),
                } => {
                    // Get attacker entity
                    if let Some(char_entity) = state.ecs().entity_from_uid(by.into()) {
                        // Check if attacker is another player or entity with stats (npc)
                        if state
                            .ecs()
                            .read_storage::<Player>()
                            .get(char_entity)
                            .is_some()
                        {
                            KillSource::Player(by, KillType::Melee)
                        } else if let Some(stats) =
                            state.ecs().read_storage::<Stats>().get(char_entity)
                        {
                            KillSource::NonPlayer(stats.name.clone(), KillType::Melee)
                        } else {
                            KillSource::NonPlayer("<?>".to_string(), KillType::Melee)
                        }
                    } else {
                        KillSource::NonPlayer("<?>".to_string(), KillType::Melee)
                    }
                },
                HealthSource::Damage {
                    kind: DamageSource::Projectile,
                    by: Some(by),
                } => {
                    // Get projectile owner entity TODO: add names to projectiles and send in
                    // message
                    if let Some(char_entity) = state.ecs().entity_from_uid(by.into()) {
                        // Check if attacker is another player or entity with stats (npc)
                        if state
                            .ecs()
                            .read_storage::<Player>()
                            .get(char_entity)
                            .is_some()
                        {
                            KillSource::Player(by, KillType::Projectile)
                        } else if let Some(stats) =
                            state.ecs().read_storage::<Stats>().get(char_entity)
                        {
                            KillSource::NonPlayer(stats.name.clone(), KillType::Projectile)
                        } else {
                            KillSource::NonPlayer("<?>".to_string(), KillType::Projectile)
                        }
                    } else {
                        KillSource::NonPlayer("<?>".to_string(), KillType::Projectile)
                    }
                },
                HealthSource::Damage {
                    kind: DamageSource::Explosion,
                    by: Some(by),
                } => {
                    // Get explosion owner entity
                    if let Some(char_entity) = state.ecs().entity_from_uid(by.into()) {
                        // Check if attacker is another player or entity with stats (npc)
                        if state
                            .ecs()
                            .read_storage::<Player>()
                            .get(char_entity)
                            .is_some()
                        {
                            KillSource::Player(by, KillType::Explosion)
                        } else if let Some(stats) =
                            state.ecs().read_storage::<Stats>().get(char_entity)
                        {
                            KillSource::NonPlayer(stats.name.clone(), KillType::Explosion)
                        } else {
                            KillSource::NonPlayer("<?>".to_string(), KillType::Explosion)
                        }
                    } else {
                        KillSource::NonPlayer("<?>".to_string(), KillType::Explosion)
                    }
                },
                HealthSource::Damage {
                    kind: DamageSource::Energy,
                    by: Some(by),
                } => {
                    // Get energy owner entity
                    if let Some(char_entity) = state.ecs().entity_from_uid(by.into()) {
                        // Check if attacker is another player or entity with stats (npc)
                        if state
                            .ecs()
                            .read_storage::<Player>()
                            .get(char_entity)
                            .is_some()
                        {
                            KillSource::Player(by, KillType::Energy)
                        } else if let Some(stats) =
                            state.ecs().read_storage::<Stats>().get(char_entity)
                        {
                            KillSource::NonPlayer(stats.name.clone(), KillType::Energy)
                        } else {
                            KillSource::NonPlayer("<?>".to_string(), KillType::Energy)
                        }
                    } else {
                        KillSource::NonPlayer("<?>".to_string(), KillType::Energy)
                    }
                },
                HealthSource::Damage {
                    kind: DamageSource::Other,
                    by: Some(by),
                } => {
                    // Get energy owner entity
                    if let Some(char_entity) = state.ecs().entity_from_uid(by.into()) {
                        // Check if attacker is another player or entity with stats (npc)
                        if state
                            .ecs()
                            .read_storage::<Player>()
                            .get(char_entity)
                            .is_some()
                        {
                            KillSource::Player(by, KillType::Other)
                        } else if let Some(stats) =
                            state.ecs().read_storage::<Stats>().get(char_entity)
                        {
                            KillSource::NonPlayer(stats.name.clone(), KillType::Other)
                        } else {
                            KillSource::NonPlayer("<?>".to_string(), KillType::Other)
                        }
                    } else {
                        KillSource::NonPlayer("<?>".to_string(), KillType::Other)
                    }
                },
                HealthSource::World => KillSource::FallDamage,
                HealthSource::Suicide => KillSource::Suicide,
                HealthSource::Damage { .. }
                | HealthSource::Revive
                | HealthSource::Command
                | HealthSource::LevelUp
                | HealthSource::Item
                | HealthSource::Heal { by: _ }
                | HealthSource::Unknown => KillSource::Other,
            };
            state.notify_players(ServerGeneral::server_msg(
                comp::ChatType::Kill(kill_source, *uid),
                "".to_string(),
            ));
        }
    }

    // Give EXP to the killer if entity had stats
    (|| {
        let mut stats = state.ecs().write_storage::<Stats>();
        let by = if let HealthSource::Damage { by: Some(by), .. } = cause {
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
        let mut exp_reward = entity_stats.body_type.base_exp() as f32
            * (1.0 + entity_stats.level.level() as f32 * 0.1);

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

    let should_delete = if state
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

        false
    } else if state.ecs().read_storage::<comp::Agent>().contains(entity) {
        use specs::Builder;

        // Decide for a loot drop before turning into a lootbag
        let old_body = state.ecs().write_storage::<Body>().remove(entity);
        let mut rng = rand::thread_rng();
        let mut lottery = || {
            Lottery::<String>::load_expect(match old_body {
                Some(common::comp::Body::Humanoid(_)) => match rng.gen_range(0, 4) {
                    0 => "common.loot_tables.loot_table_humanoids",
                    1 => "common.loot_tables.loot_table_armor_light",
                    2 => "common.loot_tables.loot_table_armor_cloth",
                    3 => "common.loot_tables.loot_table_weapon_common",
                    4 => "common.loots_tables.loot_table_armor_misc",
                    _ => "common.loot_tables.loot_table_humanoids",
                },
                Some(common::comp::Body::QuadrupedSmall(quadruped_small)) => {
                    match quadruped_small.species {
                        quadruped_small::Species::Dodarock => match rng.gen_range(0, 6) {
                            1 => "common.loot_tables.loot_table_rocks",
                            _ => "common.loot_tables.loot_table_rocks",
                        },
                        _ => match rng.gen_range(0, 4) {
                            0 => "common.loot_tables.loot_table_food",
                            2 => "common.loot_tables.loot_table_animal_parts",
                            _ => "common.loot_tables.loot_table_animal_parts",
                        },
                    }
                },
                Some(common::comp::Body::QuadrupedMedium(quadruped_medium)) => {
                    match quadruped_medium.species {
                        quadruped_medium::Species::Frostfang
                        | quadruped_medium::Species::Roshwalr => {
                            "common.loot_tables.loot_table_animal_ice"
                        },
                        _ => match rng.gen_range(0, 4) {
                            0 => "common.loot_tables.loot_table_food",
                            2 => "common.loot_tables.loot_table_animal_parts",
                            _ => "common.loot_tables.loot_table_animal_parts",
                        },
                    }
                },
                Some(common::comp::Body::BirdMedium(_)) => match rng.gen_range(0, 3) {
                    0 => "common.loot_tables.loot_table_food",
                    _ => "common.loot_tables.loot_table",
                },
                Some(common::comp::Body::BipedLarge(biped_large)) => match biped_large.species {
                    biped_large::Species::Wendigo => match rng.gen_range(0, 7) {
                        0 => "common.loot_tables.loot_table_food",
                        1 => "common.loot_tables.loot_table_wendigo",
                        3 => "common.loot_tables.loot_table_armor_heavy",
                        5 => "common.loot_tables.loot_table_weapon_uncommon",
                        6 => "common.loot_tables.loot_table_weapon_rare",
                        _ => "common.loot_tables.loot_table_cave_large",
                    },
                    _ => match rng.gen_range(0, 8) {
                        0 => "common.loot_tables.loot_table_food",
                        1 => "common.loot_tables.loot_table_armor_nature",
                        3 => "common.loot_tables.loot_table_armor_heavy",
                        5 => "common.loot_tables.loot_table_weapon_uncommon",
                        6 => "common.loot_tables.loot_table_weapon_rare",
                        _ => "common.loot_tables.loot_table_cave_large",
                    },
                },
                Some(common::comp::Body::Golem(_)) => match rng.gen_range(0, 9) {
                    0 => "common.loot_tables.loot_table_food",
                    2 => "common.loot_tables.loot_table_armor_light",
                    3 => "common.loot_tables.loot_table_armor_heavy",
                    5 => "common.loot_tables.loot_table_weapon_common",
                    6 => "common.loot_tables.loot_table_weapon_uncommon",
                    7 => "common.loot_tables.loot_table_weapon_rare",
                    _ => "common.loot_tables.loot_table",
                },
                Some(common::comp::Body::Theropod(_)) => {
                    "common.loot_tables.loot_table_animal_parts"
                },
                Some(common::comp::Body::Dragon(_)) => "common.loot_tables.loot_table_weapon_rare",
                Some(common::comp::Body::QuadrupedLow(_)) => match rng.gen_range(0, 3) {
                    0 => "common.loot_tables.loot_table_food",
                    1 => "common.loot_tables.loot_table_animal_parts",
                    _ => "common.loot_tables.loot_table",
                },
                _ => "common.loot_tables.loot_table",
            })
        };

        let item = {
            let mut item_drops = state.ecs().write_storage::<comp::ItemDrop>();
            item_drops.remove(entity).map_or_else(
                || Item::new_from_asset_expect(lottery().read().choose()),
                |item_drop| item_drop.0,
            )
        };

        let pos = state.ecs().read_storage::<comp::Pos>().get(entity).cloned();
        if let Some(pos) = pos {
            let _ = state
                .create_object(comp::Pos(pos.0 + Vec3::unit_z() * 0.25), match old_body {
                    Some(common::comp::Body::Humanoid(_)) => object::Body::Pouch,
                    Some(common::comp::Body::Golem(_)) => object::Body::Chest,
                    Some(common::comp::Body::BipedLarge(_))
                    | Some(common::comp::Body::QuadrupedLow(_)) => object::Body::MeatDrop,
                    _ => object::Body::Steak,
                })
                .with(item)
                .build();
        } else {
            error!(
                ?entity,
                "Entity doesn't have a position, no bag is being dropped"
            )
        }

        true
    } else {
        true
    };

    if should_delete {
        if let Some(rtsim_entity) = state
            .ecs()
            .read_storage::<RtSimEntity>()
            .get(entity)
            .copied()
        {
            state
                .ecs()
                .write_resource::<RtSim>()
                .destroy_entity(rtsim_entity.0);
        }

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

/// Delete an entity without any special actions (this is generally used for
/// temporarily unloading an entity when it leaves the view distance). As much
/// as possible, this function should simply make an entity cease to exist.
pub fn handle_delete(server: &mut Server, entity: EcsEntity) {
    let _ = server
        .state_mut()
        .delete_entity_recorded(entity)
        .map_err(|e| error!(?e, ?entity, "Failed to delete destroyed entity"));
}

pub fn handle_land_on_ground(server: &Server, entity: EcsEntity, vel: Vec3<f32>) {
    let state = &server.state;
    if vel.z <= -30.0 {
        if let Some(health) = state.ecs().write_storage::<comp::Health>().get_mut(entity) {
            let falldmg = (vel.z.powi(2) / 20.0 - 40.0) * 10.0;
            let damage = Damage {
                source: DamageSource::Falling,
                value: falldmg,
            };
            let loadouts = state.ecs().read_storage::<comp::Loadout>();
            let change = damage.modify_damage(loadouts.get(entity), None);
            health.change_by(change);
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
            .read_component_copied::<comp::Waypoint>(entity)
            .map(|wp| wp.get_pos())
            .unwrap_or(state.ecs().read_resource::<SpawnPoint>().0);

        state
            .ecs()
            .write_storage::<comp::Health>()
            .get_mut(entity)
            .map(|health| health.revive());
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

#[allow(clippy::blocks_in_if_conditions)]
pub fn handle_explosion(
    server: &Server,
    pos: Vec3<f32>,
    explosion: Explosion,
    owner: Option<Uid>,
    reagent: Option<Reagent>,
) {
    // Go through all other entities
    let ecs = &server.state.ecs();

    // Add an outcome
    // Uses radius as outcome power, makes negative if explosion has healing effect
    let outcome_power = explosion.radius
        * if explosion.effects.iter().any(|e| {
            matches!(
                e,
                RadiusEffect::Entity(
                    _,
                    Effect::Damage(Damage {
                        source: DamageSource::Healing,
                        ..
                    })
                )
            )
        }) {
            -1.0
        } else {
            1.0
        };
    ecs.write_resource::<Vec<Outcome>>()
        .push(Outcome::Explosion {
            pos,
            power: outcome_power,
            radius: explosion.radius,
            is_attack: explosion
                .effects
                .iter()
                .any(|e| matches!(e, RadiusEffect::Entity(_, Effect::Damage(_)))),
            reagent,
        });
    let owner_entity = owner.and_then(|uid| {
        ecs.read_resource::<UidAllocator>()
            .retrieve_entity_internal(uid.into())
    });
    let groups = ecs.read_storage::<comp::Group>();

    for effect in explosion.effects {
        match effect {
            RadiusEffect::TerrainDestruction(power) => {
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
                        // TODO: Faster RNG
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
                            let r = color[0] as f32
                                + (fade * (color[0] as f32 * 0.5 - color[0] as f32));
                            let g = color[1] as f32
                                + (fade * (color[1] as f32 * 0.3 - color[1] as f32));
                            let b = color[2] as f32
                                + (fade * (color[2] as f32 * 0.3 - color[2] as f32));
                            // Darken blocks, but not too much
                            color[0] = (r as u8).max(30);
                            color[1] = (g as u8).max(30);
                            color[2] = (b as u8).max(30);
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

                    let mut ray_energy = power;

                    let terrain = ecs.read_resource::<TerrainGrid>();
                    let _ = terrain
                        .ray(pos, pos + dir * power)
                        // TODO: Faster RNG
                        .until(|block: &Block| {
                            let stop = block.is_liquid() || block.explode_power().is_none() || ray_energy <= 0.0;
                            ray_energy -= block.explode_power().unwrap_or(0.0) + rand::random::<f32>() * 0.1;
                            stop
                        })
                        .for_each(|block: &Block, pos| {
                            if block.explode_power().is_some() {
                                block_change.set(pos, block.into_vacant());
                            }
                        })
                        .cast();
                }
            },
            RadiusEffect::Entity(target, mut effect) => {
                for (entity_b, pos_b) in (&ecs.entities(), &ecs.read_storage::<comp::Pos>()).join()
                {
                    // See if entities are in the same group
                    let mut same_group = owner_entity
                        .and_then(|e| groups.get(e))
                        .map_or(false, |group_a| Some(group_a) == groups.get(entity_b));
                    if let Some(entity) = owner_entity {
                        if entity == entity_b {
                            same_group = true;
                        }
                    }
                    let target_group = if same_group {
                        GroupTarget::InGroup
                    } else {
                        GroupTarget::OutOfGroup
                    };

                    if let Some(target) = target {
                        if target != target_group {
                            continue;
                        }
                    }

                    let distance_squared = pos.distance_squared(pos_b.0);
                    let strength = 1.0 - distance_squared / explosion.radius.powi(2);

                    if strength > 0.0 {
                        let is_alive = ecs
                            .read_storage::<comp::Health>()
                            .get(entity_b)
                            .map_or(false, |h| !h.is_dead);

                        if is_alive {
                            effect.modify_strength(strength);
                            server.state().apply_effect(entity_b, effect.clone(), owner);
                            // Apply energy change
                            if let Some(owner) = owner_entity {
                                if let Some(energy) =
                                    ecs.write_storage::<comp::Energy>().get_mut(owner)
                                {
                                    energy.change_by(EnergyChange {
                                        amount: explosion.energy_regen as i32,
                                        source: comp::EnergySource::HitEnemy,
                                    });
                                }
                            }
                        }
                    }
                }
            },
        }
    }
}

pub fn handle_level_up(server: &mut Server, entity: EcsEntity, new_level: u32) {
    let ecs = &server.state.ecs();
    if let Some((uid, pos)) = ecs
        .read_storage::<Uid>()
        .get(entity)
        .copied()
        .zip(ecs.read_storage::<Pos>().get(entity).map(|p| p.0))
    {
        ecs.write_resource::<Vec<Outcome>>()
            .push(Outcome::LevelUp { pos });
        server.state.notify_players(ServerGeneral::PlayerListUpdate(
            PlayerListUpdate::LevelChange(uid, new_level),
        ));
    }
}

pub fn handle_aura(server: &mut Server, entity: EcsEntity, aura_change: aura::AuraChange) {
    let ecs = &server.state.ecs();
    let mut auras_all = ecs.write_storage::<comp::Auras>();
    if let Some(auras) = auras_all.get_mut(entity) {
        use aura::AuraChange;
        match aura_change {
            AuraChange::Add(new_aura) => {
                auras.insert(new_aura);
            },
            AuraChange::RemoveByKey(keys) => {
                for key in keys {
                    auras.remove(key);
                }
            },
        }
    }
}

pub fn handle_buff(server: &mut Server, entity: EcsEntity, buff_change: buff::BuffChange) {
    let ecs = &server.state.ecs();
    let mut buffs_all = ecs.write_storage::<comp::Buffs>();
    if let Some(buffs) = buffs_all.get_mut(entity) {
        use buff::BuffChange;
        match buff_change {
            BuffChange::Add(new_buff) => {
                buffs.insert(new_buff);
            },
            BuffChange::RemoveById(ids) => {
                for id in ids {
                    buffs.remove(id);
                }
            },
            BuffChange::RemoveByKind(kind) => {
                buffs.remove_kind(kind);
            },
            BuffChange::RemoveFromController(kind) => {
                if kind.is_buff() {
                    buffs.remove_kind(kind);
                }
            },
            BuffChange::RemoveByCategory {
                all_required,
                any_required,
                none_required,
            } => {
                let mut ids_to_remove = Vec::new();
                for (id, buff) in buffs.buffs.iter() {
                    let mut required_met = true;
                    for required in &all_required {
                        if !buff.cat_ids.iter().any(|cat| cat == required) {
                            required_met = false;
                            break;
                        }
                    }
                    let mut any_met = any_required.is_empty();
                    for any in &any_required {
                        if buff.cat_ids.iter().any(|cat| cat == any) {
                            any_met = true;
                            break;
                        }
                    }
                    let mut none_met = true;
                    for none in &none_required {
                        if buff.cat_ids.iter().any(|cat| cat == none) {
                            none_met = false;
                            break;
                        }
                    }
                    if required_met && any_met && none_met {
                        ids_to_remove.push(*id);
                    }
                }
                for id in ids_to_remove {
                    buffs.remove(id);
                }
            },
        }
    }
}

pub fn handle_energy_change(server: &Server, entity: EcsEntity, change: EnergyChange) {
    let ecs = &server.state.ecs();
    if let Some(energy) = ecs.write_storage::<Energy>().get_mut(entity) {
        energy.change_by(change);
    }
}
