use crate::{
    comp::{self, agent::Activity, Agent, Alignment, Controller, MountState, Ori, Pos, Stats},
    path::Chaser,
    state::Time,
    sync::UidAllocator,
    terrain::TerrainGrid,
    util::Dir,
    vol::ReadVol,
};
use rand::{thread_rng, Rng};
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
        ReadStorage<'a, Ori>,
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
            orientations,
            stats,
            terrain,
            alignments,
            mut agents,
            mut controllers,
            mount_states,
        ): Self::SystemData,
    ) {
        for (entity, pos, ori, alignment, agent, controller, mount_state) in (
            &entities,
            &positions,
            &orientations,
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

            // Default to looking in orientation direction
            inputs.look_dir = ori.0;

            const AVG_FOLLOW_DIST: f32 = 6.0;
            const MAX_FOLLOW_DIST: f32 = 12.0;
            const MAX_CHASE_DIST: f32 = 24.0;
            const SEARCH_DIST: f32 = 30.0;
            const SIGHT_DIST: f32 = 64.0;
            const MIN_ATTACK_DIST: f32 = 3.25;

            let mut do_idle = false;
            let mut choose_target = false;

            'activity: {
                match &mut agent.activity {
                    Activity::Idle(bearing) => {
                        *bearing += Vec2::new(
                            thread_rng().gen::<f32>() - 0.5,
                            thread_rng().gen::<f32>() - 0.5,
                        ) * 0.1
                            - *bearing * 0.01
                            - if let Some(patrol_origin) = agent.patrol_origin {
                                Vec2::<f32>::from(pos.0 - patrol_origin) * 0.0002
                                    + Vec3::one() / Vec2::<f32>::from(pos.0 - patrol_origin)
                            } else {
                                Vec2::zero()
                            };
                        // Stop if we're too close to a wall
                        *bearing *= 0.1
                            + if terrain
                                .ray(
                                    pos.0 + Vec3::unit_z(),
                                    pos.0
                                        + Vec3::from(*bearing)
                                            .try_normalized()
                                            .unwrap_or(Vec3::zero())
                                            * 1.5
                                        + Vec3::unit_z(),
                                )
                                .until(|block| block.is_solid())
                                .cast()
                                .1
                                .map(|b| b.is_none())
                                .unwrap_or(true)
                            {
                                0.9
                            } else {
                                0.0
                            };

                        if bearing.magnitude_squared() > 0.25f32.powf(2.0) {
                            inputs.move_dir =
                                bearing.try_normalized().unwrap_or(Vec2::zero()) * 0.65;
                        }

                        // Sometimes try searching for new targets
                        if thread_rng().gen::<f32>() < 0.1 {
                            choose_target = true;
                        }
                    },
                    Activity::Follow(target, chaser) => {
                        if let (Some(tgt_pos), _tgt_stats) =
                            (positions.get(*target), stats.get(*target))
                        {
                            let dist_sqrd = pos.0.distance_squared(tgt_pos.0);
                            // Follow, or return to idle
                            if dist_sqrd > AVG_FOLLOW_DIST.powf(2.0) {
                                if let Some(bearing) =
                                    chaser.chase(&*terrain, pos.0, tgt_pos.0, AVG_FOLLOW_DIST)
                                {
                                    inputs.move_dir = Vec2::from(bearing)
                                        .try_normalized()
                                        .unwrap_or(Vec2::zero());
                                    inputs.jump.set_state(bearing.z > 1.0);
                                }
                            } else {
                                do_idle = true;
                            }
                        } else {
                            do_idle = true;
                        }
                    },
                    Activity::Attack {
                        target,
                        chaser,
                        been_close,
                        ..
                    } => {
                        if let (Some(tgt_pos), Some(tgt_stats), tgt_alignment) = (
                            positions.get(*target),
                            stats.get(*target),
                            alignments
                                .get(*target)
                                .copied()
                                .unwrap_or(Alignment::Owned(*target)),
                        ) {
                            if let Some(dir) = Dir::from_unnormalized(tgt_pos.0 - pos.0) {
                                inputs.look_dir = dir;
                            }

                            // Don't attack entities we are passive towards
                            // TODO: This is here, it's a bit of a hack
                            if let Some(alignment) = alignment {
                                if (*alignment).passive_towards(tgt_alignment) || tgt_stats.is_dead
                                {
                                    do_idle = true;
                                    break 'activity;
                                }
                            }

                            let dist_sqrd = pos.0.distance_squared(tgt_pos.0);
                            if dist_sqrd < MIN_ATTACK_DIST.powf(2.0) {
                                // Close-range attack
                                inputs.move_dir = Vec2::from(tgt_pos.0 - pos.0)
                                    .try_normalized()
                                    .unwrap_or(Vec2::unit_y())
                                    * 0.7;
                                inputs.primary.set_state(true);
                            } else if dist_sqrd < MAX_CHASE_DIST.powf(2.0)
                                || (dist_sqrd < SIGHT_DIST.powf(2.0) && !*been_close)
                            {
                                if dist_sqrd < MAX_CHASE_DIST.powf(2.0) {
                                    *been_close = true;
                                }

                                // Long-range chase
                                if let Some(bearing) =
                                    chaser.chase(&*terrain, pos.0, tgt_pos.0, 1.25)
                                {
                                    inputs.move_dir = Vec2::from(bearing)
                                        .try_normalized()
                                        .unwrap_or(Vec2::zero());
                                    inputs.jump.set_state(bearing.z > 1.0);
                                }

                                if dist_sqrd < (MAX_CHASE_DIST * 0.65).powf(2.0)
                                    && thread_rng().gen::<f32>() < 0.01
                                {
                                    inputs.roll.set_state(true);
                                }
                            } else {
                                do_idle = true;
                            }
                        } else {
                            do_idle = true;
                        }
                    },
                }
            }

            if do_idle {
                agent.activity = Activity::Idle(Vec2::zero());
            }

            // Choose a new target to attack: only go out of our way to attack targets we
            // are hostile toward!
            if choose_target {
                // Search for new targets (this looks expensive, but it's only run occasionally)
                // TODO: Replace this with a better system that doesn't consider *all* entities
                let closest_entity = (&entities, &positions, &stats, alignments.maybe())
                    .join()
                    .filter(|(e, e_pos, e_stats, e_alignment)| {
                        e_pos.0.distance_squared(pos.0) < SEARCH_DIST.powf(2.0)
                            && *e != entity
                            && !e_stats.is_dead
                            && alignment
                                .and_then(|a| e_alignment.map(|b| a.hostile_towards(*b)))
                                .unwrap_or(false)
                    })
                    .min_by_key(|(_, e_pos, _, _)| (e_pos.0.distance_squared(pos.0) * 100.0) as i32)
                    .map(|(e, _, _, _)| e);

                if let Some(target) = closest_entity {
                    agent.activity = Activity::Attack {
                        target,
                        chaser: Chaser::default(),
                        time: time.0,
                        been_close: false,
                    };
                }
            }

            // --- Activity overrides (in reverse order of priority: most important goes
            // last!) ---

            // Attack a target that's attacking us
            if let Some(stats) = stats.get(entity) {
                // Only if the attack was recent
                if stats.health.last_change.0 < 5.0 {
                    if let comp::HealthSource::Attack { by }
                    | comp::HealthSource::Projectile { owner: Some(by) } =
                        stats.health.last_change.1.cause
                    {
                        if !agent.activity.is_attack() {
                            if let Some(attacker) = uid_allocator.retrieve_entity_internal(by.id())
                            {
                                agent.activity = Activity::Attack {
                                    target: attacker,
                                    chaser: Chaser::default(),
                                    time: time.0,
                                    been_close: false,
                                };
                            }
                        }
                    }
                }
            }

            // Follow owner if we're too far, or if they're under attack
            if let Some(Alignment::Owned(owner)) = alignment.copied() {
                if let Some(owner_pos) = positions.get(owner) {
                    let dist_sqrd = pos.0.distance_squared(owner_pos.0);
                    if dist_sqrd > MAX_FOLLOW_DIST.powf(2.0) && !agent.activity.is_follow() {
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
                                        agent.activity = Activity::Attack {
                                            target: attacker,
                                            chaser: Chaser::default(),
                                            time: time.0,
                                            been_close: false,
                                        };
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
