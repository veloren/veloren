use crate::{
    comp::{projectile, HealthSource, Ori, PhysicsState, Projectile, Vel},
    event::{EventBus, ServerEvent},
};
use specs::{Entities, Join, Read, ReadStorage, System, WriteStorage};

/// This system is responsible for handling projectile effect triggers
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, EventBus<ServerEvent>>,
        ReadStorage<'a, Vel>,
        ReadStorage<'a, PhysicsState>,
        WriteStorage<'a, Ori>,
        WriteStorage<'a, Projectile>,
    );

    fn run(
        &mut self,
        (
            entities,
            server_bus,
            velocities,
            physics_states,
            mut orientations,
            mut projectiles,
        ): Self::SystemData,
    ) {
        let mut server_emitter = server_bus.emitter();

        // Attacks
        for (entity, vel, physics, ori, projectile) in (
            &entities,
            &velocities,
            &physics_states,
            &mut orientations,
            &mut projectiles,
        )
            .join()
        {
            ori.0 = vel.0.normalized();

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
            // Hit entity
            if let Some(other) = physics.touch_entity {
                for effect in projectile.hit_entity.drain(..) {
                    match effect {
                        projectile::Effect::Damage(power) => {
                            server_emitter.emit(ServerEvent::Damage {
                                uid: other,
                                dmg: power,
                                cause: HealthSource::Projectile,
                            })
                        }
                        projectile::Effect::Vanish => server_emitter.emit(ServerEvent::Destroy {
                            entity,
                            cause: HealthSource::World,
                        }),
                    }
                }
            }
        }
    }
}
