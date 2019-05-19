// Library
use specs::{Join, Read, ReadStorage, System, WriteStorage};
use vek::*;

// Crate
use crate::comp::{phys::Pos, Agent, Control};

// Basic ECS AI agent system
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        WriteStorage<'a, Agent>,
        ReadStorage<'a, Pos>,
        WriteStorage<'a, Control>,
    );

    fn run(&mut self, (mut agents, positions, mut controls): Self::SystemData) {
        for (mut agent, pos, mut control) in (&mut agents, &positions, &mut controls).join() {
            match agent {
                Agent::Wanderer(bearing) => {
                    *bearing += Vec2::new(rand::random::<f32>() - 0.5, rand::random::<f32>() - 0.5)
                        * 0.1
                        - *bearing * 0.01
                        - pos.0 * 0.0002;

                    if bearing.magnitude_squared() != 0.0 {
                        control.move_dir = bearing.normalized();
                    }
                }
                Agent::Pet { target, offset } => {
                    // Run towards target
                    match positions.get(*target) {
                        Some(tgt_pos) => {
                            let tgt_pos = tgt_pos.0 + *offset;

                            // Jump with target.
                            control.jumping = tgt_pos.z > pos.0.z + 1.0;

                            // Move towards the target.
                            let dist = tgt_pos.distance(pos.0);
                            control.move_dir = if dist > 5.0 {
                                Vec2::from(tgt_pos - pos.0).normalized()
                            } else if dist < 1.5 && dist > 0.0 {
                                Vec2::from(pos.0 - tgt_pos).normalized()
                            } else {
                                Vec2::zero()
                            };
                        }
                        _ => control.move_dir = Vec2::zero(),
                    }

                    // Change offset occasionally.
                    if rand::random::<f32>() < 0.003 {
                        *offset =
                            Vec2::new(rand::random::<f32>() - 0.5, rand::random::<f32>() - 0.5)
                                * 10.0;
                    }
                }
            }
        }
    }
}
