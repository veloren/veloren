use crate::comp::{
    Agent, CharacterState, Controller, MountState, MovementState::Glide, Pos, Stats,
};
use crate::hierarchical::ChunkPath;
use crate::pathfinding::WorldPath;
use crate::terrain::TerrainGrid;
use rand::{seq::SliceRandom, thread_rng};
use specs::{Entities, Join, ReadExpect, ReadStorage, System, WriteStorage};
use vek::*;

/// This system will allow NPCs to modify their controller
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Stats>,
        ReadStorage<'a, CharacterState>,
        ReadExpect<'a, TerrainGrid>,
        WriteStorage<'a, Agent>,
        WriteStorage<'a, Controller>,
        ReadStorage<'a, MountState>,
    );

    fn run(
        &mut self,
        (
            entities,
            positions,
            stats,
            character_states,
            terrain,
            mut agents,
            mut controllers,
            mount_states,
        ): Self::SystemData,
    ) {
        for (entity, pos, agent, controller, mount_state) in (
            &entities,
            &positions,
            &mut agents,
            &mut controllers,
            mount_states.maybe(),
        )
            .join()
        {
            // Skip mounted entities
            if mount_state
                .map(|ms| {
                    if let MountState::Unmounted = ms {
                        false
                    } else {
                        true
                    }
                })
                .unwrap_or(false)
            {
                continue;
            }

            controller.reset();

            let mut inputs = &mut controller.inputs;

            match agent {
                Agent::Traveler { path } => {
                    let mut new_path: Option<WorldPath> = None;
                    let is_destination = |cur_pos: Vec3<i32>, dest: Vec3<i32>| {
                        Vec2::<i32>::from(cur_pos) == Vec2::<i32>::from(dest)
                    };

                    let found_destination = || {
                        const MAX_TRAVEL_DIST: f32 = 200.0;
                        let new_dest = Vec3::new(rand::random::<f32>(), rand::random::<f32>(), 0.0)
                            * MAX_TRAVEL_DIST;
                        new_path = Some(
                            ChunkPath::new(&*terrain, pos.0, pos.0 + new_dest)
                                .get_worldpath(&*terrain),
                        );
                    };

                    path.move_along_path(
                        &*terrain,
                        pos,
                        &mut inputs,
                        is_destination,
                        found_destination,
                    );

                    if let Some(new_path) = new_path {
                        *path = new_path;
                    }
                }
                Agent::Wanderer(bearing) => {
                    *bearing += Vec2::new(rand::random::<f32>() - 0.5, rand::random::<f32>() - 0.5)
                        * 0.1
                        - *bearing * 0.01
                        - pos.0 * 0.0002;

                    if bearing.magnitude_squared() > 0.001 {
                        inputs.move_dir = bearing.normalized();
                    }
                }
                Agent::Pet { target, offset } => {
                    // Run towards target.
                    match positions.get(*target) {
                        Some(tgt_pos) => {
                            let tgt_pos = tgt_pos.0 + *offset;

                            if tgt_pos.z > pos.0.z + 1.0 {
                                inputs.jump.set_state(true);
                            }

                            // Move towards the target.
                            let dist: f32 = Vec2::from(tgt_pos - pos.0).magnitude();
                            inputs.move_dir = if dist > 5.0 {
                                Vec2::from(tgt_pos - pos.0).normalized()
                            } else if dist < 1.5 && dist > 0.001 {
                                Vec2::from(pos.0 - tgt_pos).normalized()
                            } else {
                                Vec2::zero()
                            };
                        }
                        _ => inputs.move_dir = Vec2::zero(),
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
                    const MIN_ATTACK_DIST: f32 = 3.25;
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
                        inputs.look_dir = target_pos.0 - pos.0;

                        let dist = Vec2::<f32>::from(target_pos.0 - pos.0).magnitude();
                        if target_stats.is_dead {
                            choose_new = true;
                        } else if dist < 0.001 {
                            // Probably can only happen when entities are at a different z-level
                            // since at the same level repulsion would keep them apart.
                            // Distinct from the first if block since we may want to change the
                            // behavior for this case.
                            choose_new = true;
                        } else if dist < MIN_ATTACK_DIST {
                            // Fight (and slowly move closer)
                            inputs.move_dir =
                                Vec2::<f32>::from(target_pos.0 - pos.0).normalized() * 0.01;
                            inputs.primary.set_state(true);
                        } else if dist < SIGHT_DIST {
                            inputs.move_dir =
                                Vec2::<f32>::from(target_pos.0 - pos.0).normalized() * 0.96;

                            if rand::random::<f32>() < 0.02 {
                                inputs.roll.set_state(true);
                            }

                            if target_character.movement == Glide && target_pos.0.z > pos.0.z + 5.0
                            {
                                inputs.glide.set_state(true);
                                inputs.jump.set_state(true);
                            }
                        } else {
                            choose_new = true;
                        }
                    } else {
                        *bearing +=
                            Vec2::new(rand::random::<f32>() - 0.5, rand::random::<f32>() - 0.5)
                                * 0.1
                                - *bearing * 0.005;

                        inputs.move_dir = if bearing.magnitude_squared() > 0.001 {
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

            debug_assert!(inputs.move_dir.map(|e| !e.is_nan()).reduce_and());
            debug_assert!(inputs.look_dir.map(|e| !e.is_nan()).reduce_and());
        }
    }
}
