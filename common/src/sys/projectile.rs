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
        ReadStorage<'a, Vel>,
        WriteStorage<'a, Ori>,
        WriteStorage<'a, Projectile>,
    );

    fn run(
        &mut self,
        (
            entities,
            server_bus,
            physics_states,
            velocities,
            mut orientations,
            mut projectiles,
        ): Self::SystemData,
    ) {
        let mut server_emitter = server_bus.emitter();

        let mut todo = Vec::new();

        // Attacks
        for (entity, physics, ori, projectile) in (
            &entities,
            &physics_states,
            &mut orientations,
            &mut projectiles,
        )
            .join()
        {
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
                todo.push(entity);
            }
            // Hit ground
            else if physics.on_ground {
                for effect in projectile.hit_ground.drain(..) {
                    match effect {
                        _ => {}
                    }
                }
                todo.push(entity);
            }
            // Hit wall
            else if physics.on_wall.is_some() {
                for effect in projectile.hit_wall.drain(..) {
                    match effect {
                        _ => {}
                    }
                }
                todo.push(entity);
            } else {
                if let Some(vel) = velocities.get(entity) {
                    ori.0 = vel.0.normalized();
                }
            }
        }

        for entity in todo {
            projectiles.remove(entity);
        }
    }
}
