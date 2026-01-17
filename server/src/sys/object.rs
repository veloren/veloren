use common::{
    CachedSpatialGrid, Damage, DamageKind, GroupTarget,
    combat::{Attack, AttackDamage},
    comp::{Body, Object, Pos, Teleporting, Vel, beam, object},
    consts::TELEPORTER_RADIUS,
    event::{ChangeBodyEvent, DeleteEvent, EmitExt, EventBus},
    event_emitters,
    outcome::Outcome,
    resources::{DeltaTime, Secs, Time},
    states::basic_summon::BeamPillarIndicatorSpecifier,
};
use common_ecs::{Job, Origin, Phase, System};
use hashbrown::HashMap;
use specs::{Entities, Join, LazyUpdate, LendJoin, Read, ReadStorage, WriteStorage};
use vek::{QuadraticBezier3, Vec3};

event_emitters! {
    struct Events[Emitters] {
        delete: DeleteEvent,
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
        WriteStorage<'a, Object>,
        ReadStorage<'a, Body>,
        ReadStorage<'a, Teleporting>,
        ReadStorage<'a, beam::Beam>,
        Read<'a, LazyUpdate>,
    );

    const NAME: &'static str = "object";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (
            entities,
            events,
            dt,
            time,
            outcome_bus,
            spatial_grid,
            positions,
            velocities,
            mut objects,
            bodies,
            teleporting,
            beams,
            updater,
        ): Self::SystemData,
    ) {
        let mut emitters = events.get_emitters();

        // Objects
        for (entity, pos, vel, object, body) in (
            &entities,
            &positions,
            velocities.maybe(),
            &mut objects,
            bodies.maybe(),
        )
            .join()
        {
            match object {
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
                                .is_some_and(|(teleporter_pos, _)| {
                                    pos.0.distance_squared(teleporter_pos.0)
                                        <= TELEPORTER_RADIUS.powi(2)
                                })
                        });

                    if body.is_some_and(|body| {
                        (*body == Body::Object(object::Body::PortalActive)) != is_active
                    }) {
                        emitters.emit(ChangeBodyEvent {
                            entity,
                            new_body: Body::Object(if is_active {
                                outcome_bus.emit_now(Outcome::PortalActivated { pos: pos.0 });
                                object::Body::PortalActive
                            } else {
                                object::Body::Portal
                            }),
                            permanent_change: None,
                        });
                    }
                },
                Object::BeamPillar {
                    spawned_at,
                    buildup_duration,
                    attack_duration,
                    beam_duration,
                    radius,
                    height,
                    damage,
                    damage_effect,
                    dodgeable,
                    tick_rate,
                    specifier,
                    indicator_specifier,
                } => {
                    match indicator_specifier {
                        BeamPillarIndicatorSpecifier::FirePillar => {
                            outcome_bus.emit_now(Outcome::FirePillarIndicator {
                                pos: pos.0,
                                radius: *radius,
                            })
                        },
                    }

                    let age = (time.0 - spawned_at.0).max(0.0);
                    let buildup = buildup_duration.as_secs_f64();
                    let attack = attack_duration.as_secs_f64();

                    if age > buildup + attack {
                        emitters.emit(DeleteEvent(entity));
                    } else if age > buildup && !beams.contains(entity) {
                        let mut attack_damage = AttackDamage::new(
                            Damage {
                                kind: DamageKind::Energy,
                                value: *damage,
                            },
                            Some(GroupTarget::OutOfGroup),
                            rand::random(),
                        );
                        if let Some(combat_effect) = damage_effect {
                            attack_damage = attack_damage.with_effect(combat_effect.clone());
                        }

                        updater.insert(entity, beam::Beam {
                            attack: Attack::new(None).with_damage(attack_damage),
                            dodgeable: *dodgeable,
                            start_radius: *radius,
                            end_radius: *radius,
                            range: *height,
                            duration: Secs(beam_duration.as_secs_f64()),
                            tick_dur: Secs(1.0 / *tick_rate as f64),
                            hit_entities: Vec::new(),
                            hit_durations: HashMap::new(),
                            specifier: *specifier,
                            bezier: QuadraticBezier3 {
                                start: pos.0,
                                ctrl: pos.0,
                                end: pos.0,
                            },
                        });
                    }
                },
                Object::Crux { pid_controller, .. } => {
                    if let Some(vel) = vel
                        && let Some(pid_controller) = pid_controller
                        && let Some(accel) = body.and_then(|body| {
                            body.fly_thrust()
                                .map(|fly_thrust| fly_thrust / body.mass().0)
                        })
                    {
                        pid_controller.add_measurement(time.0, pos.0.z);
                        let dir = pid_controller.calc_err();
                        pid_controller.limit_integral_windup(|z| *z = z.clamp(-1.0, 1.0));

                        updater
                            .insert(entity, Vel((vel.0.z + dir * accel * dt.0) * Vec3::unit_z()));
                    }
                },
            }
        }
    }
}
