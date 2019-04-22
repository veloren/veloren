// Library
use specs::{Join, Read, ReadStorage, System, WriteStorage};
use vek::*;

// Crate
use crate::comp::{Agent, Control, phys::Pos};

// Basic ECS AI agent system
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        WriteStorage<'a, Agent>,
        ReadStorage<'a, Pos>,
        WriteStorage<'a, Control>,
    );

    fn run(&mut self, (mut agents, pos, mut controls): Self::SystemData) {
        for (mut agent, pos, mut control) in (&mut agents, &pos, &mut controls).join() {
            match agent {
                Agent::Wanderer(bearing) => {
                    *bearing += Vec2::new(
                        rand::random::<f32>().fract() - 0.5,
                        rand::random::<f32>().fract() - 0.5,
                    ) * 0.1 - *bearing * 0.01 - pos.0 * 0.0002;

                    if bearing.magnitude_squared() != 0.0 {
                        control.move_dir = bearing.normalized();
                    }
                },
            }
        }
    }
}
