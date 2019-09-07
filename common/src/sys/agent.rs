use crate::comp::{Agent, CharacterState, Controller, MovementState::Glide, Pos, Stats};
use rand::{seq::SliceRandom, thread_rng};
use specs::{Entities, Join, ReadStorage, System, WriteStorage};
use vek::*;

/// This system will allow NPCs to modify their controller
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Stats>,
        ReadStorage<'a, CharacterState>,
        WriteStorage<'a, Agent>,
        WriteStorage<'a, Controller>,
    );

    fn run(
        &mut self,
        (entities, positions, stats, character_states, mut agents, mut controllers): Self::SystemData,
    ) {
        for (entity, pos, agent, controller) in
            (&entities, &positions, &mut agents, &mut controllers).join()
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
                    const SIGHT_DIST: f32 = 30.0;
                    const MIN_ATTACK_DIST: f32 = 3.5;
                    let mut choose_new = false;

                    if let Some((Some(target_pos), Some(target_stats), Some(target_character))) =
                        target.map(|target| {
                            (
                                positions.get(target),
                                stats.get(target),
                                character_states.get(target),
                            )
                        })
                    {
                        controller.look_dir = target_pos.0 - pos.0;

                        let dist = Vec2::<f32>::from(target_pos.0 - pos.0).magnitude();
                        if target_stats.is_dead {
                            choose_new = true;
                        } else if dist < MIN_ATTACK_DIST {
                            // Fight (and slowly move closer)
                            controller.move_dir =
                                Vec2::<f32>::from(target_pos.0 - pos.0).normalized() * 0.01;
                            controller.primary = true;
                        } else if dist < SIGHT_DIST {
                            controller.move_dir =
                                Vec2::<f32>::from(target_pos.0 - pos.0).normalized() * 0.96;

                            if rand::random::<f32>() < 0.02 {
                                controller.roll = true;
                            }

                            if target_character.movement == Glide && target_pos.0.z > pos.0.z + 5.0
                            {
                                controller.glide = true;
                                controller.jump = true;
                            }
                        } else {
                            choose_new = true;
                        }
                    } else {
                        *bearing +=
                            Vec2::new(rand::random::<f32>() - 0.5, rand::random::<f32>() - 0.5)
                                * 0.1
                                - *bearing * 0.005;

                        controller.move_dir = if bearing.magnitude_squared() > 0.1 {
                            bearing.normalized()
                        } else {
                            Vec2::zero()
                        };

                        choose_new = true;
                    }

                    if choose_new && rand::random::<f32>() < 0.1 {
                        let entities = (&entities, &positions, &stats)
                            .join()
                            .filter(|(e, e_pos, e_stats)| {
                                (e_pos.0 - pos.0).magnitude() < SIGHT_DIST
                                    && *e != entity
                                    && !e_stats.is_dead
                            })
                            .map(|(e, _, _)| e)
                            .collect::<Vec<_>>();

                        let mut rng = thread_rng();
                        *target = (&entities).choose(&mut rng).cloned();
                    }
                }
            }
        }
    }
}
