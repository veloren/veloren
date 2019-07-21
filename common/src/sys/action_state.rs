use crate::{
    comp::{ActionState, Attacking, Controller, Gliding, OnGround, Rolling, Vel, Wielding},
    sys::movement::MOVEMENT_THRESHOLD_VEL,
};
use specs::{Entities, Join, ReadStorage, System, WriteStorage};

/// This system will set the ActionState component as specified by other components
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Controller>,
        ReadStorage<'a, Vel>,
        ReadStorage<'a, OnGround>,
        ReadStorage<'a, Gliding>,
        ReadStorage<'a, Attacking>,
        ReadStorage<'a, Wielding>,
        ReadStorage<'a, Rolling>,
        WriteStorage<'a, ActionState>,
    );

    fn run(
        &mut self,
        (
            entities,
            controllers, // To make sure it only runs on the single client and the server
            velocities,
            on_grounds,
            glidings,
            attackings,
            wieldings,
            rollings,
            mut action_states,
        ): Self::SystemData,
    ) {
        for (
            _entity,
            vel,
            _controller,
            on_ground,
            gliding,
            attacking,
            wielding,
            rolling,
            action_state,
        ) in (
            &entities,
            &velocities,
            &controllers,
            on_grounds.maybe(),
            glidings.maybe(),
            attackings.maybe(),
            wieldings.maybe(),
            rollings.maybe(),
            &mut action_states,
        )
            .join()
        {
            *action_state = ActionState {
                on_ground: on_ground.is_some(),
                moving: vel.0.magnitude_squared() > MOVEMENT_THRESHOLD_VEL.powf(2.0),
                attacking: attacking.is_some(),
                wielding: wielding.is_some(),
                rolling: rolling.is_some(),
                gliding: gliding.is_some(),
            };
        }
    }
}
