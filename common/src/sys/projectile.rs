use crate::{
    comp::{projectile, HealthSource, Ori, PhysicsState, Pos, Projectile, Vel},
    event::{EventBus, LocalEvent, ServerEvent},
    state::DeltaTime,
    sync::UidAllocator,
};
use specs::{saveload::MarkerAllocator, Entities, Join, Read, ReadStorage, System, WriteStorage};
use std::time::Duration;
use vek::*;

/// This system is responsible for handling projectile effect triggers
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, DeltaTime>,
        Read<'a, UidAllocator>,
        Read<'a, EventBus<LocalEvent>>,
        Read<'a, EventBus<ServerEvent>>,
        ReadStorage<'a, Pos>,
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
            uid_allocator,
            local_bus,
            server_bus,
            positions,
            physics_states,
            velocities,
            mut orientations,
            mut projectiles,
        ): Self::SystemData,
    ) {
        let mut local_emitter = local_bus.emitter();
        let mut server_emitter = server_bus.emitter();

        // Attacks
        for (entity, pos, physics, ori, projectile) in (
            &entities,
            &positions,
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
                        projectile::Effect::Explode { power } => {
                            server_emitter.emit(ServerEvent::Explosion {
                                pos: pos.0,
                                power,
                                owner: projectile.owner,
                            })
                        },
                        projectile::Effect::Vanish => server_emitter.emit(ServerEvent::Destroy {
                            entity,
                            cause: HealthSource::World,
                        }),
                        _ => {},
                    }
                }
            }
            // Hit wall
            else if physics.on_wall.is_some() {
                for effect in projectile.hit_wall.drain(..) {
                    match effect {
                        projectile::Effect::Explode { power } => {
                            server_emitter.emit(ServerEvent::Explosion {
                                pos: pos.0,
                                power,
                                owner: projectile.owner,
                            })
                        },
                        projectile::Effect::Vanish => server_emitter.emit(ServerEvent::Destroy {
                            entity,
                            cause: HealthSource::World,
                        }),
                        _ => {},
                    }
                }
            }
            // Hit entity
            else if let Some(other) = physics.touch_entity {
                for effect in projectile.hit_entity.drain(..) {
                    match effect {
                        projectile::Effect::Damage(change) => {
                            server_emitter.emit(ServerEvent::Damage { uid: other, change })
                        },
                        projectile::Effect::Knockback(knockback) => {
                            if let Some(entity) =
                                uid_allocator.retrieve_entity_internal(other.into())
                            {
                                local_emitter.emit(LocalEvent::ApplyForce {
                                    entity,
                                    dir: Vec3::slerp(ori.0, Vec3::new(0.0, 0.0, 1.0), 0.5),
                                    force: knockback,
                                });
                            }
                        },
                        projectile::Effect::Explode { power } => {
                            server_emitter.emit(ServerEvent::Explosion {
                                pos: pos.0,
                                power,
                                owner: projectile.owner,
                            })
                        },
                        projectile::Effect::Vanish => server_emitter.emit(ServerEvent::Destroy {
                            entity,
                            cause: HealthSource::World,
                        }),
                        projectile::Effect::Possess => {
                            if let Some(owner) = projectile.owner {
                                server_emitter.emit(ServerEvent::Possess(owner.into(), other));
                            }
                        },
                        _ => {},
                    }
                }
            } else {
                if let Some(dir) = velocities
                    .get(entity)
                    .and_then(|vel| vel.0.try_normalized())
                {
                    ori.0 = dir;
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
