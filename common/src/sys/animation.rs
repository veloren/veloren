use crate::{
    comp::{ActionState, Animation, AnimationInfo, ForceUpdate},
    state::DeltaTime,
};
use specs::{Entities, Join, Read, ReadStorage, System, WriteStorage};

/// This system will apply the animation that fits best to the users actions
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, DeltaTime>,
        ReadStorage<'a, ActionState>,
        WriteStorage<'a, AnimationInfo>,
    );

    fn run(&mut self, (entities, dt, action_states, mut animation_infos): Self::SystemData) {
        for (entity, a) in (&entities, &action_states).join() {
            fn impossible_animation(message: &str) -> Animation {
                warn!("{}", message);
                Animation::Idle
            }

            let animation = match (a.on_ground, a.moving, a.attacking, a.gliding, a.rolling) {
                (_, _, true, true, _) => impossible_animation("Attack while gliding"),
                (_, _, true, _, true) => impossible_animation("Roll while attacking"),
                (_, _, _, true, true) => impossible_animation("Roll while gliding"),
                (_, false, _, _, true) => impossible_animation("Roll without moving"),
                (_, true, false, false, true) => Animation::Roll,
                (true, false, false, false, false) => Animation::Idle,
                (true, true, false, false, false) => Animation::Run,
                (false, _, false, false, false) => Animation::Jump,
                (_, _, false, true, false) => Animation::Gliding,
                (_, _, true, false, false) => Animation::Attack,
            };

            let new_time = animation_infos
                .get(entity)
                .filter(|i| i.animation == animation)
                .map(|i| i.time + dt.0 as f64);

            animation_infos.insert(
                entity,
                AnimationInfo {
                    animation,
                    time: new_time.unwrap_or(0.0),
                },
            );
        }
    }
}
