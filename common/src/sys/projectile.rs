use crate::{
    comp::{projectile, HealthSource, Ori, PhysicsState, Projectile, Vel},
    event::{EventBus, ServerEvent},
    state::DeltaTime,
};
use specs::{Entities, Join, Read, ReadStorage, System, WriteStorage};
use std::time::Duration;

/// This system is responsible for handling projectile effect triggers
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, DeltaTime>,
        Read<'a, EventBus<ServerEvent>>,
        ReadStorage<'a, PhysicsState>,
        ReadStorage<'a, Vel>,
        WriteStorage<'a, Ori>,
        WriteStorage<'a, Projectile>,
    );

    fn run(
        &mut self,
        (
            entities,
            dt,
            server_bus,
            physics_states,
            velocities,
            mut orientations,
            mut projectiles,
        ): Self::SystemData,
    ) {
        let mut server_emitter = server_bus.emitter();

        // Attacks
        for (entity, physics, ori, projectile) in (
            &entities,
            &physics_states,
            &mut orientations,
            &mut projectiles,
        )
            .join()
        {
            // Hit ground
            if physics.on_ground {
                for effect in projectile.hit_ground.drain(..) {
                    match effect {
                        projectile::Effect::Vanish => server_emitter.emit(ServerEvent::Destroy {
                            entity,
                            cause: HealthSource::World,
                        }),
                        _ => {}
                    }
                }
            }
            // Hit wall
            else if physics.on_wall.is_some() {
                for effect in projectile.hit_wall.drain(..) {
                    match effect {
                        projectile::Effect::Vanish => server_emitter.emit(ServerEvent::Destroy {
                            entity,
                            cause: HealthSource::World,
                        }),
                        _ => {}
                    }
                }
            }
            // Hit entity
            else if let Some(other) = physics.touch_entity {
                for effect in projectile.hit_entity.drain(..) {
                    match effect {
                        projectile::Effect::Damage(power) => {
                            server_emitter.emit(ServerEvent::Damage {
                                uid: other,
                                dmg: power,
                                cause: match projectile.owner {
                                    Some(uid) => HealthSource::Attack { by: uid },
                                    None => HealthSource::Unknown,
                                },
                            })
                        }
                        projectile::Effect::Vanish => server_emitter.emit(ServerEvent::Destroy {
                            entity,
                            cause: HealthSource::World,
                        }),
                        projectile::Effect::Possess => {
                            if let Some(uid) = projectile.owner {
                                server_emitter.emit(ServerEvent::Possess(uid.into(), other))
                            }
                        }
                        _ => {}
                    }
                }
            } else {
                if let Some(vel) = velocities.get(entity) {
                    ori.0 = vel.0.normalized();
                }
            }

            if projectile.time_left == Duration::default() {
                server_emitter.emit(ServerEvent::Destroy {
                    entity,
                    cause: HealthSource::World,
                });
            }
            projectile.time_left = projectile
                .time_left
                .checked_sub(Duration::from_secs_f32(dt.0))
                .unwrap_or_default();
        }
    }
}
