use crate::{
    comp::{
        ActionState::*, Animation, AnimationInfo, CharacterState, MovementState::*, PhysicsState,
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
        ReadStorage<'a, CharacterState>,
        ReadStorage<'a, PhysicsState>,
        WriteStorage<'a, AnimationInfo>,
    );

    fn run(
        &mut self,
        (entities, dt, character_states, physics_states, mut animation_infos): Self::SystemData,
    ) {
        for (entity, character, physics) in (&entities, &character_states, &physics_states).join() {
            fn impossible_animation(physics: PhysicsState, character: CharacterState) -> Animation {
                warn!("Impossible animation: {:?} {:?}", physics, character);
                Animation::Roll
            }

            let animation = match (physics.on_ground, &character.movement, &character.action) {
                (_, Roll { .. }, Idle) => Animation::Roll,
                (true, Stand, Idle) => Animation::Idle,
                (true, Run, Idle) => Animation::Run,
                (false, Jump, Idle) => Animation::Jump,
                (true, Stand, Wield { .. }) => Animation::Cidle,
                (true, Run, Wield { .. }) => Animation::Crun,
                (false, Jump, Wield { .. }) => Animation::Cjump,
                (_, Glide, Idle) => Animation::Gliding,
                (_, _, Attack { .. }) => Animation::Attack,
                _ => impossible_animation(physics.clone(), character.clone()),
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
