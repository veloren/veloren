use crate::{client::Client, Server, SpawnPoint, StateExt};
use common::{
    assets,
    comp::{self, object, Body, HealthChange, HealthSource, Item, Player, Stats},
    msg::ServerMsg,
    state::BlockChange,
    sync::{Uid, WorldSyncExt},
    terrain::{Block, TerrainGrid},
    vol::{ReadVol, Vox},
};
use log::error;
use specs::{Entity as EcsEntity, WorldExt};
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

pub fn handle_destroy(server: &mut Server, entity: EcsEntity, cause: HealthSource) {
    let state = server.state_mut();

    // Chat message
    if let Some(player) = state.ecs().read_storage::<Player>().get(entity) {
        let msg = if let HealthSource::Attack { by } = cause {
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

        state.notify_registered_clients(ServerMsg::kill(msg));
    }

    {
        // Give EXP to the killer if entity had stats
        let mut stats = state.ecs().write_storage::<Stats>();
        if let Some(entity_stats) = stats.get(entity).cloned() {
            if let HealthSource::Attack { by } = cause {
                state.ecs().entity_from_uid(by.into()).map(|attacker| {
                    if let Some(attacker_stats) = stats.get_mut(attacker) {
                        // TODO: Discuss whether we should give EXP by Player
                        // Killing or not.
                        attacker_stats
                            .exp
                            .change_by((entity_stats.level.level() * 10) as i64);
                    }
                });
            }
        }
    }

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
            .map(|err| error!("Failed to set zero vel on dead client: {:?}", err));
        state
            .ecs()
            .write_storage()
            .insert(entity, comp::ForceUpdate)
            .err()
            .map(|err| error!("Failed to insert ForceUpdate on dead client: {:?}", err));
        state
            .ecs()
            .write_storage::<comp::Energy>()
            .get_mut(entity)
            .map(|energy| energy.set_to(energy.maximum(), comp::EnergySource::Revive));
        let _ = state
            .ecs()
            .write_storage::<comp::CharacterState>()
            .insert(entity, comp::CharacterState::default());
    } else {
        // Replace npc with loot
        let _ = state
            .ecs()
            .write_storage()
            .insert(entity, Body::Object(object::Body::Pouch));
        let _ = state.ecs().write_storage().insert(
            entity,
            assets::load_expect_cloned::<Item>("common.items.cheese"),
        );
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

        // TODO: Add Delete(time_left: Duration) component
        /*
        // If not a player delete the entity
        if let Err(err) = state.delete_entity_recorded(entity) {
            error!("Failed to delete destroyed entity: {:?}", err);
        }
        */
    }
}

pub fn handle_land_on_ground(server: &Server, entity: EcsEntity, vel: Vec3<f32>) {
    let state = &server.state;
    if vel.z <= -37.0 {
        if let Some(stats) = state.ecs().write_storage::<comp::Stats>().get_mut(entity) {
            let falldmg = (vel.z / 2.5) as i32;
            if falldmg < 0 {
                stats.health.change_by(comp::HealthChange {
                    amount: falldmg,
                    cause: comp::HealthSource::World,
                });
            }
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
            .map(|err| {
                error!(
                    "Error inserting ForceUpdate component when respawning client: {:?}",
                    err
                )
            });
    }
}

pub fn handle_explosion(server: &Server, pos: Vec3<f32>, radius: f32) {
    const RAYS: usize = 500;

    for _ in 0..RAYS {
        let dir = Vec3::new(
            rand::random::<f32>() - 0.5,
            rand::random::<f32>() - 0.5,
            rand::random::<f32>() - 0.5,
        )
        .normalized();

        let ecs = server.state.ecs();
        let mut block_change = ecs.write_resource::<BlockChange>();

        let _ = ecs
            .read_resource::<TerrainGrid>()
            .ray(pos, pos + dir * radius)
            .until(|_| rand::random::<f32>() < 0.05)
            .for_each(|pos| block_change.set(pos, Block::empty()))
            .cast();
    }
}
