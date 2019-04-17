// Library
use specs::{Join, Read, ReadStorage, System, WriteStorage, Entities};
use vek::*;

// Crate
use crate::comp::{Control, Animation, phys::{Pos, Vel, Dir}};

// Basic ECS AI agent system
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Dir>,
        WriteStorage<'a, Animation>,
        ReadStorage<'a, Control>,
    );

    fn run(&mut self, (entities, mut vels, mut dirs, mut anims, controls): Self::SystemData) {
        for (entity, mut vel, mut dir, control) in (&entities, &mut vels, &mut dirs, &controls).join() {
            // TODO: Don't hard-code this
            // Apply physics to the player: acceleration and non-linear decceleration
            vel.0 += control.move_dir * 2.0 - vel.0.map(|e| e * e.abs() + e) * 0.03;

            if control.move_dir.magnitude() > 0.01 {
                dir.0 = vel.0.normalized() * Vec3::new(1.0, 1.0, 0.0);
                anims.insert(entity, Animation::Run);
            } else {
                anims.insert(entity, Animation::Run);
            }
        }
    }
}
