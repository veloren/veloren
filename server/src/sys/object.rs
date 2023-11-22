use common::{
    comp::{object, Body, Object, PhysicsState, Pos, Teleporting, Vel},
    consts::TELEPORTER_RADIUS,
    effect::Effect,
    event::{ChangeBodyEvent, DeleteEvent, EmitExt, EventBus, ExplosionEvent, ShootEvent},
    event_emitters,
    outcome::Outcome,
    resources::{DeltaTime, Time},
    CachedSpatialGrid, Damage, DamageKind, DamageSource, Explosion, RadiusEffect,
};
use common_ecs::{Job, Origin, Phase, System};
use specs::{Entities, Join, LendJoin, Read, ReadStorage};
use vek::Rgb;

event_emitters! {
    struct Events[Emitters] {
        delete: DeleteEvent,
        explosion: ExplosionEvent,
        shoot: ShootEvent,
        change_body: ChangeBodyEvent,
    }
}

/// This system is responsible for handling misc object behaviours
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Events<'a>,
        Read<'a, DeltaTime>,
        Read<'a, Time>,
        Read<'a, EventBus<Outcome>>,
        Read<'a, CachedSpatialGrid>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Vel>,
        ReadStorage<'a, PhysicsState>,
        ReadStorage<'a, Object>,
        ReadStorage<'a, Body>,
        ReadStorage<'a, Teleporting>,
    );

    const NAME: &'static str = "object";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (
            entities,
            events,
            _dt,
            time,
            outcome_bus,
            spatial_grid,
            positions,
            velocities,
            physics_states,
            objects,
            bodies,
            teleporting,
        ): Self::SystemData,
    ) {
        let mut emitters = events.get_emitters();

        // Objects
        for (entity, pos, vel, physics, object, body) in (
            &entities,
            &positions,
            &velocities,
            &physics_states,
            &objects,
            &bodies,
        )
            .join()
        {
            match object {
                Object::Bomb { owner } => {
                    if physics.on_surface().is_some() {
                        emitters.emit(DeleteEvent(entity));
                        emitters.emit(ExplosionEvent {
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
                                comp::{LightEmitter, Projectile},
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
                                emitters.emit(ShootEvent {
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
                        emitters.emit(DeleteEvent(entity));
                        emitters.emit(ExplosionEvent {
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
                Object::DeleteAfter {
                    spawned_at,
                    timeout,
                } => {
                    if (time.0 - spawned_at.0).max(0.0) > timeout.as_secs_f64() {
                        emitters.emit(DeleteEvent(entity));
                    }
                },
                Object::Portal { .. } => {
                    let is_active = spatial_grid
                        .0
                        .in_circle_aabr(pos.0.xy(), TELEPORTER_RADIUS)
                        .any(|entity| {
                            (&positions, &teleporting)
                                .lend_join()
                                .get(entity, &entities)
                                .map_or(false, |(teleporter_pos, _)| {
                                    pos.0.distance_squared(teleporter_pos.0)
                                        <= TELEPORTER_RADIUS.powi(2)
                                })
                        });

                    if (*body == Body::Object(object::Body::PortalActive)) != is_active {
                        emitters.emit(ChangeBodyEvent {
                            entity,
                            new_body: Body::Object(if is_active {
                                outcome_bus.emit_now(Outcome::PortalActivated { pos: pos.0 });
                                object::Body::PortalActive
                            } else {
                                object::Body::Portal
                            }),
                        });
                    }
                },
            }
        }
    }
}
