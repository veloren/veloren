use common::{
    comp::{HealthSource, Object, PhysicsState, PoiseChange, PoiseSource, Pos, Vel},
    effect::Effect,
    event::{EventBus, ServerEvent},
    resources::DeltaTime,
    span, Damage, DamageSource, Explosion, RadiusEffect,
};
use specs::{Entities, Join, Read, ReadStorage, System, WriteStorage};

/// This system is responsible for handling misc object behaviours
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Entities<'a>,
        Read<'a, DeltaTime>,
        Read<'a, EventBus<ServerEvent>>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Vel>,
        ReadStorage<'a, PhysicsState>,
        WriteStorage<'a, Object>,
    );

    fn run(
        &mut self,
        (entities, _dt, server_bus, positions, velocities, physics_states, mut objects): Self::SystemData,
    ) {
        span!(_guard, "run", "object::Sys::run");
        let mut server_emitter = server_bus.emitter();

        // Objects
        for (entity, pos, vel, physics, object) in (
            &entities,
            &positions,
            &velocities,
            &physics_states,
            &mut objects,
        )
            .join()
        {
            match object {
                Object::Bomb { owner } => {
                    if physics.on_surface().is_some() {
                        server_emitter.emit(ServerEvent::Destroy {
                            entity,
                            cause: HealthSource::Suicide,
                        });
                        server_emitter.emit(ServerEvent::Explosion {
                            pos: pos.0,
                            explosion: Explosion {
                                effects: vec![
                                    RadiusEffect::Entity(Effect::Damage(Damage {
                                        source: DamageSource::Explosion,
                                        value: 500.0,
                                    })),
                                    RadiusEffect::Entity(Effect::PoiseChange(PoiseChange {
                                        source: PoiseSource::Explosion,
                                        amount: -100,
                                    })),
                                    RadiusEffect::TerrainDestruction(4.0),
                                ],
                                radius: 12.0,
                                reagent: None,
                            },
                            owner: *owner,
                        });
                    }
                },
                Object::Firework { owner, reagent } => {
                    if vel.0.z < 0.0 {
                        const ENABLE_RECURSIVE_FIREWORKS: bool = true;
                        if ENABLE_RECURSIVE_FIREWORKS {
                            use common::{
                                comp::{object, Body, Gravity, LightEmitter, Projectile},
                                util::Dir,
                            };
                            use rand::Rng;
                            use std::{f32::consts::PI, time::Duration};
                            use vek::{Rgb, Vec3};
                            let mut rng = rand::thread_rng();
                            // Note that if the expected fireworks per firework is > 1, this will
                            // eventually cause enough server lag that more players can't log in.
                            let thresholds: &[(f32, usize)] = &[(0.25, 2), (0.7, 1)];
                            let expected = {
                                let mut total = 0.0;
                                let mut cumulative_probability = 0.0;
                                for (p, n) in thresholds {
                                    total += (p - cumulative_probability) * *n as f32;
                                    cumulative_probability += p;
                                }
                                total
                            };
                            assert!(expected < 1.0);
                            let num_fireworks = (|| {
                                let x = rng.gen_range(0.0..1.0);
                                for (p, n) in thresholds {
                                    if x < *p {
                                        return *n;
                                    }
                                }
                                0
                            })();
                            for _ in 0..num_fireworks {
                                let speed: f32 = rng.gen_range(40.0..80.0);
                                let theta: f32 = rng.gen_range(0.0..2.0 * PI);
                                let phi: f32 = rng.gen_range(0.25 * PI..0.5 * PI);
                                let dir = Dir::from_unnormalized(Vec3::new(
                                    theta.cos(),
                                    theta.sin(),
                                    phi.sin(),
                                ))
                                .expect("nonzero vector should normalize");
                                server_emitter.emit(ServerEvent::Shoot {
                                    entity,
                                    dir,
                                    body: Body::Object(object::Body::for_firework(*reagent)),
                                    light: Some(LightEmitter {
                                        animated: true,
                                        flicker: 2.0,
                                        strength: 2.0,
                                        col: Rgb::new(1.0, 1.0, 0.0),
                                    }),
                                    projectile: Projectile {
                                        hit_solid: Vec::new(),
                                        hit_entity: Vec::new(),
                                        time_left: Duration::from_secs(60),
                                        owner: *owner,
                                        ignore_group: true,
                                    },
                                    gravity: Some(Gravity(1.0)),
                                    speed,
                                    object: Some(Object::Firework {
                                        owner: *owner,
                                        reagent: *reagent,
                                    }),
                                });
                            }
                        }
                        server_emitter.emit(ServerEvent::Destroy {
                            entity,
                            cause: HealthSource::Suicide,
                        });
                        server_emitter.emit(ServerEvent::Explosion {
                            pos: pos.0,
                            explosion: Explosion {
                                effects: vec![
                                    RadiusEffect::Entity(Effect::Damage(Damage {
                                        source: DamageSource::Explosion,
                                        value: 50.0,
                                    })),
                                    RadiusEffect::Entity(Effect::PoiseChange(PoiseChange {
                                        source: PoiseSource::Explosion,
                                        amount: -40,
                                    })),
                                    RadiusEffect::TerrainDestruction(4.0),
                                ],
                                radius: 12.0,
                                reagent: Some(*reagent),
                            },
                            owner: *owner,
                        });
                    }
                },
            }
        }
    }
}
