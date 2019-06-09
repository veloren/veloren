use crate::{
    comp::{phys, Animation, AnimationInfo, Attacking, Gliding, Jumping, OnGround},
    state::DeltaTime,
};
use specs::{Entities, Join, Read, ReadStorage, System, WriteStorage};

// Basic ECS AI agent system
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, DeltaTime>,
        ReadStorage<'a, phys::Vel>,
        ReadStorage<'a, OnGround>,
        ReadStorage<'a, Jumping>,
        ReadStorage<'a, Gliding>,
        ReadStorage<'a, Attacking>,
        WriteStorage<'a, AnimationInfo>,
    );

    fn run(
        &mut self,
        (entities, dt, velocities, on_grounds, jumpings, glidings, attackings, mut animation_infos): Self::SystemData,
    ) {
        for (entity, vel, on_ground, jumping, gliding, attacking, mut animation_info) in (
            &entities,
            &velocities,
            on_grounds.maybe(),
            jumpings.maybe(),
            glidings.maybe(),
            attackings.maybe(),
            &mut animation_infos,
        )
            .join()
        {
            animation_info.time += dt.0 as f64;

            let animation = if on_ground.is_some() {
                if vel.0.magnitude() > 3.0 {
                    Animation::Run
                } else if attacking.is_some() {
                    Animation::Attack
                } else {
                    Animation::Idle
                }
            } else if gliding.is_some() {
                Animation::Gliding
            } else {
                Animation::Jump
            };

            let last = animation_info.clone();
            let changed = last.animation != animation;

            *animation_info = AnimationInfo {
                animation,
                time: if changed { 0.0 } else { last.time },
                changed,
            };
        }
    }
}
