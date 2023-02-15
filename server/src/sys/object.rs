use common::{
    comp::{Object, PhysicsState, Pos, Vel},
    effect::Effect,
    event::{EventBus, ServerEvent},
    resources::DeltaTime,
    Damage, DamageKind, DamageSource, Explosion, RadiusEffect,
};
use common_ecs::{Job, Origin, Phase, System};
use specs::{Entities, Join, Read, ReadStorage, WriteStorage};
use vek::Rgb;

/// This system is responsible for handling misc object behaviours
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, DeltaTime>,
        Read<'a, EventBus<ServerEvent>>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Vel>,
        ReadStorage<'a, PhysicsState>,
        WriteStorage<'a, Object>,
    );

    const NAME: &'static str = "object";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (entities, _dt, server_bus, positions, velocities, physics_states, mut objects): Self::SystemData,
    ) {
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
                        server_emitter.emit(ServerEvent::Delete(entity));
                        server_emitter.emit(ServerEvent::Explosion {
                            pos: pos.0,
                            explosion: Explosion {
                                effects: vec![
                                    RadiusEffect::Entity(Effect::Damage(Damage {
                                        source: DamageSource::Explosion,
                                        kind: DamageKind::Energy,
                                        value: 40.0,
                                    })),
                                    RadiusEffect::Entity(Effect::Poise(-100.0)),
                                    RadiusEffect::TerrainDestruction(4.0, Rgb::black()),
                                ],
                                radius: 12.0,
                                reagent: None,
                                min_falloff: 0.75,
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
                                comp::{object, Body, LightEmitter, Projectile},
                                util::Dir,
                            };
                            use rand::Rng;
                            use std::{f32::consts::PI, time::Duration};
                            use vek::Vec3;
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
                                    pos: *pos,
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
                                        is_sticky: true,
                                        is_point: true,
                                    },
                                    speed,
                                    object: Some(Object::Firework {
                                        owner: *owner,
                                        reagent: *reagent,
                                    }),
                                });
                            }
                        }
                        server_emitter.emit(ServerEvent::Delete(entity));
                        server_emitter.emit(ServerEvent::Explosion {
                            pos: pos.0,
                            explosion: Explosion {
                                effects: vec![
                                    RadiusEffect::Entity(Effect::Damage(Damage {
                                        source: DamageSource::Explosion,
                                        kind: DamageKind::Energy,
                                        value: 5.0,
                                    })),
                                    RadiusEffect::Entity(Effect::Poise(-40.0)),
                                    RadiusEffect::TerrainDestruction(4.0, Rgb::black()),
                                ],
                                radius: 12.0,
                                reagent: Some(*reagent),
                                min_falloff: 0.0,
                            },
                            owner: *owner,
                        });
                    }
                },
            }
        }
    }
}
