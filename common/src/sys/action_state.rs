use crate::{
    comp::{
        ActionState, Animation, AnimationInfo, Attacking, Controller, ForceUpdate, Gliding,
        Jumping, OnGround, Ori, Pos, Rolling, Vel,
    },
    state::DeltaTime,
};
use specs::{Entities, Join, Read, ReadStorage, System, WriteStorage};

/// This system will set the ActionState component as specified by other components
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, DeltaTime>,
        ReadStorage<'a, Controller>,
        ReadStorage<'a, Vel>,
        ReadStorage<'a, OnGround>,
        ReadStorage<'a, Jumping>,
        ReadStorage<'a, Gliding>,
        ReadStorage<'a, Attacking>,
        ReadStorage<'a, Rolling>,
        WriteStorage<'a, ActionState>,
    );

    fn run(
        &mut self,
        (
            entities,
            dt,
            controllers, // To make sure it only runs on the single client and the server
            velocities,
            on_grounds,
            jumpings,
            glidings,
            attackings,
            rollings,
            mut action_states,
        ): Self::SystemData,
    ) {
        for (
            entity,
            vel,
            _controller,
            on_ground,
            jumping,
            gliding,
            attacking,
            rolling,
            mut action_state,
        ) in (
            &entities,
            &velocities,
            &controllers,
            on_grounds.maybe(),
            jumpings.maybe(),
            glidings.maybe(),
            attackings.maybe(),
            rollings.maybe(),
            &mut action_states,
        )
            .join()
        {
            *action_state = ActionState {
                on_ground: on_ground.is_some(),
                moving: vel.0.magnitude_squared() > 10.0,
                attacking: attacking.is_some(),
                rolling: rolling.is_some(),
                gliding: gliding.is_some(),
            };
        }
    }
}
