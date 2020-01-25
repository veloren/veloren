use crate::terrain::TerrainGrid;
use crate::{
    comp::{self, agent::Activity, Agent, Alignment, Controller, MountState, Pos, Stats},
    path::Chaser,
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

            const AVG_FOLLOW_DIST: f32 = 6.0;
            const MAX_FOLLOW_DIST: f32 = 12.0;
            const MAX_CHASE_DIST: f32 = 24.0;
            const SIGHT_DIST: f32 = 20.0;
            const MIN_ATTACK_DIST: f32 = 3.25;
            const PATROL_DIST: f32 = 32.0;

            let mut do_idle = false;

            match &mut agent.activity {
                Activity::Idle(wander_pos, chaser) => {
                    if let Some(patrol_origin) = agent.patrol_origin {
                        if thread_rng().gen::<f32>() < 0.002 {
                            *wander_pos =
                                if thread_rng().gen::<f32>() < 0.5 {
                                    Some(patrol_origin.map(|e| {
                                        e + thread_rng().gen_range(-1.0, 1.0) * PATROL_DIST
                                    }))
                                } else {
                                    None
                                };
                        }

                        if let Some(wp) = wander_pos {
                            if let Some(bearing) = chaser.chase(&*terrain, pos.0, *wp, 2.0) {
                                inputs.move_dir =
                                    Vec2::from(bearing).try_normalized().unwrap_or(Vec2::zero());
                                inputs.jump.set_state(bearing.z > 1.0);
                            } else {
                                *wander_pos = None;
                            }
                        }
                    }

                    // Sometimes try searching for new targets
                    if thread_rng().gen::<f32>() < 0.025 {
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

                        if let Some(target) = (&entities).choose(&mut thread_rng()).cloned() {
                            agent.activity = Activity::Attack(target, Chaser::default(), time.0);
                        }
                    }
                }
                Activity::Follow(target, chaser) => {
                    if let (Some(tgt_pos), _tgt_stats) =
                        (positions.get(*target), stats.get(*target))
                    {
                        let dist = pos.0.distance(tgt_pos.0);
                        // Follow, or return to idle
                        if dist > AVG_FOLLOW_DIST {
                            if let Some(bearing) =
                                chaser.chase(&*terrain, pos.0, tgt_pos.0, AVG_FOLLOW_DIST)
                            {
                                inputs.move_dir =
                                    Vec2::from(bearing).try_normalized().unwrap_or(Vec2::zero());
                                inputs.jump.set_state(bearing.z > 1.0);
                            }
                        } else {
                            do_idle = true;
                        }
                    } else {
                        do_idle = true;
                    }
                }
                Activity::Attack(target, chaser, _) => {
                    if let (Some(tgt_pos), _tgt_stats) =
                        (positions.get(*target), stats.get(*target))
                    {
                        let dist = pos.0.distance(tgt_pos.0);
                        if dist < MIN_ATTACK_DIST {
                            // Close-range attack
                            inputs.look_dir = tgt_pos.0 - pos.0;
                            inputs.move_dir = Vec2::from(tgt_pos.0 - pos.0)
                                .try_normalized()
                                .unwrap_or(Vec2::zero())
                                * 0.01;
                            inputs.primary.set_state(true);
                        } else if dist < MAX_CHASE_DIST {
                            // Long-range chase
                            if let Some(bearing) = chaser.chase(&*terrain, pos.0, tgt_pos.0, 1.25) {
                                inputs.move_dir =
                                    Vec2::from(bearing).try_normalized().unwrap_or(Vec2::zero());
                                inputs.jump.set_state(bearing.z > 1.0);
                            }
                        } else {
                            do_idle = true;
                        }
                    } else {
                        do_idle = true;
                    }
                }
            }

            if do_idle {
                agent.activity = Activity::Idle(None, Chaser::default());
            }

            // --- Activity overrides (in reverse order of priority: most important goes last!) ---

            // Attack a target that's attacking us
            if let Some(stats) = stats.get(entity) {
                // Only if the attack was recent
                if stats.health.last_change.0 < 5.0 {
                    if let comp::HealthSource::Attack { by } = stats.health.last_change.1.cause {
                        if !agent.activity.is_attack() {
                            if let Some(attacker) = uid_allocator.retrieve_entity_internal(by.id())
                            {
                                agent.activity =
                                    Activity::Attack(attacker, Chaser::default(), time.0);
                            }
                        }
                    }
                }
            }

            // Follow owner if we're too far, or if they're under attack
            if let Some(owner) = agent.owner {
                if let Some(owner_pos) = positions.get(owner) {
                    let dist = pos.0.distance(owner_pos.0);
                    if dist > MAX_FOLLOW_DIST && !agent.activity.is_follow() {
                        agent.activity = Activity::Follow(owner, Chaser::default());
                    }

                    // Attack owner's attacker
                    if let Some(owner_stats) = stats.get(owner) {
                        if owner_stats.health.last_change.0 < 5.0 {
                            if let comp::HealthSource::Attack { by } =
                                owner_stats.health.last_change.1.cause
                            {
                                if !agent.activity.is_attack() {
                                    if let Some(attacker) =
                                        uid_allocator.retrieve_entity_internal(by.id())
                                    {
                                        agent.activity =
                                            Activity::Attack(attacker, Chaser::default(), time.0);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            debug_assert!(inputs.move_dir.map(|e| !e.is_nan()).reduce_and());
            debug_assert!(inputs.look_dir.map(|e| !e.is_nan()).reduce_and());
        }
    }
}
