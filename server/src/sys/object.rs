use common::{
    CachedSpatialGrid,
    comp::{Body, Object, Pos, Teleporting, object},
    consts::TELEPORTER_RADIUS,
    event::{ChangeBodyEvent, DeleteEvent, EmitExt, EventBus, ExplosionEvent, ShootEvent},
    event_emitters,
    outcome::Outcome,
    resources::{DeltaTime, Time},
};
use common_ecs::{Job, Origin, Phase, System};
use specs::{Entities, Join, LendJoin, Read, ReadStorage};

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
            objects,
            bodies,
            teleporting,
        ): Self::SystemData,
    ) {
        let mut emitters = events.get_emitters();

        // Objects
        for (entity, pos, object, body) in (&entities, &positions, &objects, bodies.maybe()).join()
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
                            permanent: false,
                        });
                    }
                },
            }
        }
    }
}
