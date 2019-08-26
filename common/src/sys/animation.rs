use crate::{
    comp::{
        ActionState::*, Animation, AnimationInfo, CharacterState, MovementState::*, PhysicsState,
        Stats,
    },
    state::DeltaTime,
};
use specs::{Entities, Join, Read, ReadStorage, System, WriteStorage};
use std::fmt::Debug;

/// This system will apply the animation that fits best to the users actions
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, DeltaTime>,
        ReadStorage<'a, Stats>,
        ReadStorage<'a, CharacterState>,
        ReadStorage<'a, PhysicsState>,
        WriteStorage<'a, AnimationInfo>,
    );

    fn run(
        &mut self,
        (entities, dt, stats, character_states, physics_states, mut animation_infos): Self::SystemData,
    ) {
        for (entity, stats, character, physics) in
            (&entities, &stats, &character_states, &physics_states).join()
        {
            if stats.is_dead {
                continue;
            }

            let animation = match (physics.on_ground, &character.movement, &character.action) {
                (_, Roll { .. }, Idle) => Animation::Roll,
                (true, Stand, _) => Animation::Stand, //if standing still, legs still
                (true, Stand, Idle) => Animation::Idle, //if standing still and not acting, idle the body
                (true, Run, _) => Animation::Run, //if running, legs run
                (true, Run, Idle) => Animation::Lean, //if running and not acting, lean the body
                (false, Jump, Idle) => Animation::Jump,
                (true, Stand, Wield { .. }) => Animation::Cidle,
                (true, Run, Wield { .. }) => Animation::Crun,
                (false, Jump, Wield { .. }) => Animation::Cjump,
                (_, Glide, Idle) => Animation::Gliding,
                (_, _, Attack { .. }) => Animation::Attack,
                (_, _, Block { .. }) => Animation::Block,
                // Impossible animation (Caused by missing animations or syncing delays)
                _ => Animation::Gliding,
            };

            let new_time = animation_infos
                .get(entity)
                .filter(|i| i.animation == animation)
                .map(|i| i.time + f64::from(dt.0));

            let _ = animation_infos.insert(
                entity,
                AnimationInfo {
                    animation,
                    time: new_time.unwrap_or(0.0),
                },
            );
        }
    }
}
