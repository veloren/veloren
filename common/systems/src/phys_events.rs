use common::{
    comp::{Fluid, Mass, PhysicsState, Pos, PreviousPhysCache, Vel},
    event::EventBus,
    outcome::Outcome,
};
use common_ecs::System;
use specs::{Join, Read, ReadStorage};

#[derive(Default)]
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Read<'a, EventBus<Outcome>>,
        ReadStorage<'a, PhysicsState>,
        ReadStorage<'a, PreviousPhysCache>,
        ReadStorage<'a, Vel>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Mass>,
    );

    const NAME: &'static str = "phys_events";
    const ORIGIN: common_ecs::Origin = common_ecs::Origin::Common;
    const PHASE: common_ecs::Phase = common_ecs::Phase::Create;

    fn run(
        _job: &mut common_ecs::Job<Self>,
        (outcomes, physics_states, previous_phys_cache, velocities, positions, masses): Self::SystemData,
    ) {
        let mut outcomes = outcomes.emitter();
        for (physics_state, prev, vel, pos, mass) in (
            &physics_states,
            &previous_phys_cache,
            &velocities,
            &positions,
            &masses,
        )
            .join()
        {
            // Only splash when going from air into a liquid
            if let (Some(Fluid::Liquid { kind, .. }), Some(Fluid::Air { .. })) =
                (physics_state.in_fluid, prev.in_fluid)
            {
                outcomes.emit(Outcome::Splash {
                    pos: pos.0,
                    vel: if vel.0.magnitude_squared() > prev.velocity.magnitude_squared() {
                        vel.0
                    } else {
                        prev.velocity
                    },
                    mass: mass.0,
                    kind,
                });
            }
        }
    }
}
