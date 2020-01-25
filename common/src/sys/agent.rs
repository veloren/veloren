use crate::terrain::TerrainGrid;
use crate::{
    comp::{self, Agent, Alignment, CharacterState, Controller, MountState, Pos, Stats},
    state::Time,
    sync::UidAllocator,
};
use rand::{seq::SliceRandom, thread_rng, Rng};
use specs::{
    saveload::{Marker, MarkerAllocator},
    Entities, Join, Read, ReadExpect, ReadStorage, System, WriteStorage,
};
use vek::*;

/// This system will allow NPCs to modify their controller
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Read<'a, UidAllocator>,
        Read<'a, Time>,
        Entities<'a>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Stats>,
        ReadStorage<'a, CharacterState>,
        ReadExpect<'a, TerrainGrid>,
        ReadStorage<'a, Alignment>,
        WriteStorage<'a, Agent>,
        WriteStorage<'a, Controller>,
        ReadStorage<'a, MountState>,
    );

    fn run(
        &mut self,
        (
            uid_allocator,
            time,
            entities,
            positions,
            stats,
            character_states,
            terrain,
            alignments,
            mut agents,
            mut controllers,
            mount_states,
        ): Self::SystemData,
    ) {
        for (entity, pos, alignment, agent, controller, mount_state) in (
            &entities,
            &positions,
            alignments.maybe(),
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

            const PET_DIST: f32 = 12.0;
            const PATROL_DIST: f32 = 32.0;
            const SIGHT_DIST: f32 = 24.0;
            const MIN_ATTACK_DIST: f32 = 3.25;
            const CHASE_TIME_MIN: f64 = 4.0;

            let mut chase_tgt = None;
            let mut choose_target = false;
            let mut new_target = None;

            if let Some((target, aggro_time)) = agent.target {
                // Chase / attack target
                if let (Some(tgt_pos), stats) = (positions.get(target), stats.get(target)) {
                    if stats.map(|s| s.is_dead).unwrap_or(false) {
                        // Don't target dead entities
                        choose_target = true;
                    } else if pos.0.distance(tgt_pos.0) < SIGHT_DIST
                        || (time.0 - aggro_time) < CHASE_TIME_MIN
                    {
                        chase_tgt = Some((tgt_pos.0, 1.5, true))
                    } else {
                        // Lose sight of enemies
                        choose_target = true;
                    }
                } else {
                    choose_target = true;
                }
            }

            // Return to owner
            if let Some(owner) = agent.owner {
                if let Some(tgt_pos) = positions.get(owner) {
                    if pos.0.distance(tgt_pos.0) > PET_DIST {
                        // Follow owner
                        chase_tgt = Some((tgt_pos.0, 6.0, false));
                    } else if agent.target.is_none() {
                        choose_target = thread_rng().gen::<f32>() < 0.02;
                    }
                } else {
                    agent.owner = None;
                }
            } else if let Some(patrol_origin) = agent.patrol_origin {
                if pos.0.distance(patrol_origin) > PATROL_DIST {
                    // Return to patrol origin
                    chase_tgt = Some((patrol_origin, 64.0, false));
                }
            } else {
                choose_target = thread_rng().gen::<f32>() < 0.05;
            }

            // Attack a target that's attacking us
            if let Some(stats) = stats.get(entity) {
                match stats.health.last_change.1.cause {
                    comp::HealthSource::Attack { by } => {
                        if agent.target.is_none() {
                            new_target = uid_allocator.retrieve_entity_internal(by.id());
                        } else if thread_rng().gen::<f32>() < 0.005 {
                            new_target = uid_allocator.retrieve_entity_internal(by.id());
                        }
                    }
                    _ => {}
                }
            }

            // Choose a new target
            if choose_target {
                // Search for new targets
                let entities = (&entities, &positions, &stats, alignments.maybe())
                    .join()
                    .filter(|(e, e_pos, e_stats, e_alignment)| {
                        (e_pos.0 - pos.0).magnitude() < SIGHT_DIST
                            && *e != entity
                            && !e_stats.is_dead
                            && alignment
                                .and_then(|a| e_alignment.map(|b| a.hostile_towards(*b)))
                                .unwrap_or(false)
                    })
                    .map(|(e, _, _, _)| e)
                    .collect::<Vec<_>>();

                new_target = (&entities).choose(&mut thread_rng()).cloned();
            }

            // Update target when attack begins
            if let Some(tgt) = new_target {
                agent.target = Some((tgt, time.0));
            }

            // Chase target
            if let Some((tgt_pos, min_dist, aggressive)) = chase_tgt {
                if let Some(bearing) = agent.chaser.chase(&*terrain, pos.0, tgt_pos, min_dist) {
                    inputs.move_dir = Vec2::from(bearing).try_normalized().unwrap_or(Vec2::zero());
                    inputs.jump.set_state(bearing.z > 1.0);
                }

                if aggressive && pos.0.distance(tgt_pos) < MIN_ATTACK_DIST {
                    inputs.look_dir = tgt_pos - pos.0;
                    inputs.move_dir = Vec2::from(tgt_pos - pos.0)
                        .try_normalized()
                        .unwrap_or(Vec2::zero())
                        * 0.01;
                    inputs.primary.set_state(true);
                }

                // We're not wandering
                agent.wander_pos = None;
            } else {
                if let Some(wander_pos) = agent.wander_pos {
                    if pos.0.distance(wander_pos) < 4.0 {
                        agent.wander_pos = None;
                    } else {
                        if let Some(bearing) = agent.chaser.chase(&*terrain, pos.0, wander_pos, 3.0)
                        {
                            inputs.move_dir =
                                Vec2::from(bearing).try_normalized().unwrap_or(Vec2::zero()) * 0.5;
                            inputs.jump.set_state(bearing.z > 1.0);
                        }
                    }
                }

                // Choose new wander position
                if agent.wander_pos.is_none() || thread_rng().gen::<f32>() < 0.005 {
                    agent.wander_pos = if thread_rng().gen::<f32>() < 0.5 {
                        let max_dist = if agent.owner.is_some() {
                            PET_DIST
                        } else {
                            PATROL_DIST
                        };
                        Some(
                            agent
                                .patrol_origin
                                .unwrap_or(pos.0)
                                .map(|e| e + (thread_rng().gen::<f32>() - 0.5) * max_dist),
                        )
                    } else {
                        None
                    };
                }
            }

            debug_assert!(inputs.move_dir.map(|e| !e.is_nan()).reduce_and());
            debug_assert!(inputs.look_dir.map(|e| !e.is_nan()).reduce_and());
        }
    }
}
