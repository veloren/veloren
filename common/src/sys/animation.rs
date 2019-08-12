use crate::{
    comp::{Ability, Animation, AnimationInfo, Attack, Glide, PhysicsState, Roll, Vel, Wield},
    state::DeltaTime,
};
use specs::{Entities, Join, Read, ReadStorage, System, WriteStorage};

/// This system will apply the animation that fits best to the users actions
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, DeltaTime>,
        ReadStorage<'a, PhysicsState>,
        ReadStorage<'a, Vel>,
        ReadStorage<'a, Ability<Attack>>,
        ReadStorage<'a, Ability<Glide>>,
        ReadStorage<'a, Ability<Roll>>,
        ReadStorage<'a, Ability<Wield>>,
        WriteStorage<'a, AnimationInfo>,
    );

    fn run(
        &mut self,
        (
            entities,
            dt,
            physics_states,
            velocities,
            attacks,
            glides,
            rolls,
            wields,
            mut animation_infos,
        ): Self::SystemData,
    ) {
        for (entity, physics_state, vel, attack, glide, roll, wield) in (
            &entities,
            &physics_states,
            &velocities,
            attacks.maybe(),
            glides.maybe(),
            rolls.maybe(),
            wields.maybe(),
        )
            .join()
        {
            fn impossible_animation(message: &str) -> Animation {
                warn!("{}", message);
                Animation::Idle
            }
            let animation = match (
                physics_state.on_ground,
                vel.0.magnitude_squared() > 0.4,
                attack.filter(|a| a.started()).is_some(),
                glide.filter(|a| a.started()).is_some(),
                roll.filter(|r| r.started()).is_some(),
                wield.filter(|w| w.started()).is_some(),
            ) {
                (_, _, true, true, _, _) => impossible_animation("Attack while gliding"),
                (_, _, true, _, true, _) => impossible_animation("Roll while attacking"),
                (_, _, _, true, true, _) => impossible_animation("Roll while gliding"),
                (_, false, _, _, true, _) => impossible_animation("Roll without moving"),
                (_, true, false, false, true, _) => Animation::Roll,
                (true, false, false, false, false, false) => Animation::Idle,
                (true, true, false, false, false, false) => Animation::Run,
                (false, _, false, false, false, false) => Animation::Jump,
                (true, false, false, false, false, true) => Animation::Cidle,
                (true, true, false, false, false, true) => Animation::Crun,
                (false, _, false, false, false, true) => Animation::Cjump,
                (_, _, false, true, false, _) => Animation::Gliding,
                (_, _, true, false, false, _) => Animation::Attack,
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
