use crate::comp::{Agent, Controller, Pos};
use rand::{seq::SliceRandom, thread_rng};
use specs::{Entities, Join, ReadStorage, System, WriteStorage};
use vek::*;

/// This system will allow NPCs to modify their controller
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Agent>,
        ReadStorage<'a, Pos>,
        WriteStorage<'a, Controller>,
    );

    fn run(&mut self, (entities, mut agents, positions, mut controllers): Self::SystemData) {
        for (entity, agent, pos, controller) in
            (&entities, &mut agents, &positions, &mut controllers).join()
        {
            match agent {
                Agent::Wanderer(bearing) => {
                    *bearing += Vec2::new(rand::random::<f32>() - 0.5, rand::random::<f32>() - 0.5)
                        * 0.1
                        - *bearing * 0.01
                        - pos.0 * 0.0002;

                    if bearing.magnitude_squared() != 0.0 {
                        controller.move_dir = bearing.normalized();
                    }
                }
                Agent::Pet { target, offset } => {
                    // Run towards target.
                    match positions.get(*target) {
                        Some(tgt_pos) => {
                            let tgt_pos = tgt_pos.0 + *offset;

                            if tgt_pos.z > pos.0.z + 1.0 {
                                controller.jump = true;
                            }

                            // Move towards the target.
                            let dist: f32 = Vec2::from(tgt_pos - pos.0).magnitude();
                            controller.move_dir = if dist > 5.0 {
                                Vec2::from(tgt_pos - pos.0).normalized()
                            } else if dist < 1.5 && dist > 0.0 {
                                Vec2::from(pos.0 - tgt_pos).normalized()
                            } else {
                                Vec2::zero()
                            };
                        }
                        _ => controller.move_dir = Vec2::zero(),
                    }

                    // Change offset occasionally.
                    if rand::random::<f32>() < 0.003 {
                        *offset =
                            Vec2::new(rand::random::<f32>() - 0.5, rand::random::<f32>() - 0.5)
                                * 10.0;
                    }
                }
                Agent::Enemy { bearing, target } => {
                    let choose_new = match target.map(|tgt| positions.get(tgt)).flatten() {
                        Some(tgt_pos) => {
                            let dist = Vec2::<f32>::from(tgt_pos.0 - pos.0).magnitude();
                            if dist < 2.0 {
                                controller.move_dir = Vec2::zero();

                                if rand::random::<f32>() < 0.2 {
                                    controller.attack = true;
                                }

                                false
                            } else if dist < 60.0 {
                                controller.move_dir =
                                    Vec2::<f32>::from(tgt_pos.0 - pos.0).normalized() * 0.96;

                                false
                            } else {
                                true
                            }
                        }
                        None => {
                            *bearing +=
                                Vec2::new(rand::random::<f32>() - 0.5, rand::random::<f32>() - 0.5)
                                    * 0.1
                                    - *bearing * 0.01;

                            controller.move_dir = if bearing.magnitude_squared() > 0.5 {
                                bearing.normalized()
                            } else {
                                Vec2::zero()
                            };
                            true
                        }
                    };

                    if choose_new {
                        let entities = (&entities, &positions)
                            .join()
                            .filter(|(e, e_pos)| {
                                Vec2::<f32>::from(e_pos.0 - pos.0).magnitude() < 30.0
                                    && *e != entity
                            })
                            .map(|(e, _)| e)
                            .collect::<Vec<_>>();

                        let mut rng = thread_rng();
                        *target = (&entities).choose(&mut rng).cloned();
                    }
                }
            }
        }
    }
}
