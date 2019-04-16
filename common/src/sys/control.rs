// Library
use specs::{Join, Read, ReadStorage, System, WriteStorage};
use vek::*;

// Crate
use crate::comp::{Control, phys::{Pos, Vel, Dir}};

// Basic ECS AI agent system
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Dir>,
        ReadStorage<'a, Control>,
    );

    fn run(&mut self, (mut vels, mut dirs, controls): Self::SystemData) {
        for (mut vel, mut dir, control) in (&mut vels, &mut dirs, &controls).join() {
            // TODO: Don't hard-code this
            vel.0 = vel.0 + control.move_dir * 2.0 - vel.0.map(|e| e * e.abs() + e) * 0.03;

            if control.move_dir.magnitude() > 0.01 {
                dir.0 = vel.0.normalized() * Vec3::new(1.0, 1.0, 0.0)    ;
            }
        }
    }
}
