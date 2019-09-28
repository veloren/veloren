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
        ReadStorage<'a, PhysicsState>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Ori>,
        WriteStorage<'a, Projectile>,
    );

    fn run(
        &mut self,
        (
            entities,
            server_bus,
            physics_states,
            mut velocities,
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
            if let Some(vel) = velocities.get(entity) {
                ori.0 = vel.0.normalized();
            }

            // Hit ground
            if physics.on_ground {
                for effect in projectile.hit_ground.drain(..) {
                    match effect {
                        projectile::Effect::Stick => {
                            velocities.remove(entity);
                        }
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
                        _ => {}
                    }
                }
            }
        }
    }
}
