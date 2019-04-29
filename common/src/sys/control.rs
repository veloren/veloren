// Library
use specs::{Entities, Join, Read, ReadStorage, System, WriteStorage};
use vek::*;

// Crate
use crate::comp::{
    phys::{Dir, Pos, Vel},
    Animation, AnimationHistory, Control,
};

// Basic ECS AI agent system
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Dir>,
        WriteStorage<'a, AnimationHistory>,
        ReadStorage<'a, Control>,
    );

    fn run(&mut self, (entities, mut vels, mut dirs, mut anims, controls): Self::SystemData) {
        for (entity, mut vel, mut dir, control) in
            (&entities, &mut vels, &mut dirs, &controls).join()
        {
            // TODO: Don't hard-code this
            // Apply physics to the player: acceleration and non-linear decceleration
            vel.0 += control.move_dir * 2.0 - vel.0.map(|e| e * e.abs() + e) * 0.03;

            let animation = if control.move_dir.magnitude() > 0.01 {
                dir.0 = vel.0.normalized() * Vec3::new(1.0, 1.0, 0.0);
                Animation::Run
            } else {
                Animation::Idle
            };

            let last_animation = anims.get_mut(entity).map(|h| h.current);

            anims.insert(
                entity,
                AnimationHistory {
                    last: last_animation,
                    current: animation,
                },
            );
        }
    }
}
