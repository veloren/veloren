use crate::{
    comp::{
        self,
        agent::Activity,
        item::{tool::ToolKind, ItemKind},
        Agent, Alignment, CharacterState, ChatMsg, ControlAction, Controller, Loadout, MountState,
        Ori, Pos, Scale, Stats, Vel,
    },
    event::{EventBus, ServerEvent},
    path::Chaser,
    state::{DeltaTime, Time},
    sync::{Uid, UidAllocator},
    terrain::TerrainGrid,
    util::Dir,
    vol::ReadVol,
};
use rand::{thread_rng, Rng};
use specs::{
    saveload::{Marker, MarkerAllocator},
    Entities, Join, Read, ReadExpect, ReadStorage, System, Write, WriteStorage,
};
use vek::*;

/// This system will allow NPCs to modify their controller
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Read<'a, UidAllocator>,
        Read<'a, Time>,
        Read<'a, DeltaTime>,
        Write<'a, EventBus<ServerEvent>>,
        Entities<'a>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Vel>,
        ReadStorage<'a, Ori>,
        ReadStorage<'a, Scale>,
        ReadStorage<'a, Stats>,
        ReadStorage<'a, Loadout>,
        ReadStorage<'a, CharacterState>,
        ReadStorage<'a, Uid>,
        ReadExpect<'a, TerrainGrid>,
        ReadStorage<'a, Alignment>,
        WriteStorage<'a, Agent>,
        WriteStorage<'a, Controller>,
        ReadStorage<'a, MountState>,
    );

    #[allow(clippy::or_fun_call)] // TODO: Pending review in #587
    fn run(
        &mut self,
        (
            uid_allocator,
            time,
            dt,
            event_bus,
            entities,
            positions,
            velocities,
            orientations,
            scales,
            stats,
            loadouts,
            character_states,
            uids,
            terrain,
            alignments,
            mut agents,
            mut controllers,
            mount_states,
        ): Self::SystemData,
    ) {
        for (
            entity,
            pos,
            vel,
            ori,
            alignment,
            loadout,
            character_state,
            uid,
            agent,
            controller,
            mount_state,
        ) in (
            &entities,
            &positions,
            &velocities,
            &orientations,
            alignments.maybe(),
            &loadouts,
            &character_states,
            &uids,
            &mut agents,
            &mut controllers,
            mount_states.maybe(),
        )
            .join()
        {
            // Skip mounted entities
            if mount_state
                .map(|ms| *ms != MountState::Unmounted)
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
            const LISTEN_DIST: f32 = 16.0;
            const SEARCH_DIST: f32 = 48.0;
            const SIGHT_DIST: f32 = 128.0;
            const MIN_ATTACK_DIST: f32 = 3.25;

            let scale = scales.get(entity).map(|s| s.0).unwrap_or(1.0);

            // This controls how picky NPCs are about their pathfinding. Giants are larger
            // and so can afford to be less precise when trying to move around
            // the world (especially since they would otherwise get stuck on
            // obstacles that smaller entities would not).
            let traversal_tolerance = scale + vel.0.magnitude() * 0.3;

            let mut do_idle = false;
            let mut choose_target = false;

            'activity: {
                match &mut agent.activity {
                    Activity::Idle(bearing) => {
                        *bearing += Vec2::new(
                            thread_rng().gen::<f32>() - 0.5,
                            thread_rng().gen::<f32>() - 0.5,
                        ) * 0.1
                            - *bearing * 0.003
                            - if let Some(patrol_origin) = agent.patrol_origin {
                                Vec2::<f32>::from(pos.0 - patrol_origin) * 0.0002
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
                                            .unwrap_or(Vec3::unit_y())
                                            * 5.0
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

                        if bearing.magnitude_squared() > 0.5f32.powf(2.0) {
                            inputs.move_dir = *bearing * 0.65;
                        }

                        // Put away weapon
                        if thread_rng().gen::<f32>() < 0.005 {
                            controller.actions.push(ControlAction::Unwield);
                        }

                        // Sit
                        if thread_rng().gen::<f32>() < 0.0035 {
                            controller.actions.push(ControlAction::Sit);
                        }

                        // Sometimes try searching for new targets
                        if thread_rng().gen::<f32>() < 0.1 {
                            choose_target = true;
                        }
                    },
                    Activity::Follow { target, chaser } => {
                        if let (Some(tgt_pos), _tgt_stats) =
                            (positions.get(*target), stats.get(*target))
                        {
                            let dist_sqrd = pos.0.distance_squared(tgt_pos.0);
                            // Follow, or return to idle
                            if dist_sqrd > AVG_FOLLOW_DIST.powf(2.0) {
                                if let Some((bearing, speed)) = chaser.chase(
                                    &*terrain,
                                    pos.0,
                                    vel.0,
                                    tgt_pos.0,
                                    AVG_FOLLOW_DIST,
                                    traversal_tolerance,
                                ) {
                                    inputs.move_dir =
                                        bearing.xy().try_normalized().unwrap_or(Vec2::zero())
                                            * speed;
                                    inputs.jump.set_state(bearing.z > 1.5);
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
                        powerup,
                        ..
                    } => {
                        enum Tactic {
                            Melee,
                            RangedPowerup,
                            Staff,
                        }

                        let tactic = match loadout.active_item.as_ref().and_then(|ic| {
                            if let ItemKind::Tool(tool) = &ic.item.kind {
                                Some(&tool.kind)
                            } else {
                                None
                            }
                        }) {
                            Some(ToolKind::Bow(_)) => Tactic::RangedPowerup,
                            Some(ToolKind::Staff(_)) => Tactic::Staff,
                            _ => Tactic::Melee,
                        };

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
                            if dist_sqrd < (MIN_ATTACK_DIST * scale).powf(2.0) {
                                // Close-range attack
                                /*inputs.move_dir = Vec2::from(tgt_pos.0 - pos.0)
                                .try_normalized()
                                .unwrap_or(Vec2::unit_y())
                                * 0.7;*/

                                match tactic {
                                    Tactic::Melee | Tactic::Staff => inputs.primary.set_state(true),
                                    Tactic::RangedPowerup => inputs.roll.set_state(true),
                                }
                            } else if dist_sqrd < MAX_CHASE_DIST.powf(2.0)
                                || (dist_sqrd < SIGHT_DIST.powf(2.0)
                                    && (!*been_close || !matches!(tactic, Tactic::Melee)))
                            {
                                let can_see_tgt = terrain
                                    .ray(pos.0 + Vec3::unit_z(), tgt_pos.0 + Vec3::unit_z())
                                    .until(|block| !block.is_air())
                                    .cast()
                                    .0
                                    .powf(2.0)
                                    >= dist_sqrd;

                                if can_see_tgt {
                                    if let Tactic::RangedPowerup = tactic {
                                        if *powerup > 2.0 {
                                            inputs.primary.set_state(false);
                                            *powerup = 0.0;
                                        } else {
                                            inputs.primary.set_state(true);
                                            *powerup += dt.0;
                                        }
                                    } else if let Tactic::Staff = tactic {
                                        if !character_state.is_wield() {
                                            inputs.primary.set_state(true);
                                        }

                                        inputs.secondary.set_state(true);
                                    }
                                }

                                if dist_sqrd < MAX_CHASE_DIST.powf(2.0) {
                                    *been_close = true;
                                }

                                // Long-range chase
                                if let Some((bearing, speed)) = chaser.chase(
                                    &*terrain,
                                    pos.0,
                                    vel.0,
                                    tgt_pos.0,
                                    1.25,
                                    traversal_tolerance,
                                ) {
                                    inputs.move_dir = Vec2::from(bearing)
                                        .try_normalized()
                                        .unwrap_or(Vec2::zero())
                                        * speed;
                                    inputs.jump.set_state(bearing.z > 1.5);
                                }

                                if dist_sqrd < 16.0f32.powf(2.0)
                                    && matches!(tactic, Tactic::Melee)
                                    && thread_rng().gen::<f32>() < 0.02
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
                        ((e_pos.0.distance_squared(pos.0) < SEARCH_DIST.powf(2.0) &&
                            // Within our view
                            (e_pos.0 - pos.0).try_normalized().map(|v| v.dot(*inputs.look_dir) > 0.15).unwrap_or(true))
                                // Within listen distance
                                || e_pos.0.distance_squared(pos.0) < LISTEN_DIST.powf(2.0))
                            && *e != entity
                            && !e_stats.is_dead
                            && alignment
                                .and_then(|a| e_alignment.map(|b| a.hostile_towards(*b)))
                                .unwrap_or(false)
                    })
                    // Can we even see them?
                    .filter(|(_, e_pos, _, _)| terrain
                        .ray(pos.0 + Vec3::unit_z(), e_pos.0 + Vec3::unit_z())
                        .until(|block| !block.is_air())
                        .cast()
                        .0 >= e_pos.0.distance(pos.0))
                    .min_by_key(|(_, e_pos, _, _)| (e_pos.0.distance_squared(pos.0) * 100.0) as i32)
                    .map(|(e, _, _, _)| e);

                if let Some(target) = closest_entity {
                    agent.activity = Activity::Attack {
                        target,
                        chaser: Chaser::default(),
                        time: time.0,
                        been_close: false,
                        powerup: 0.0,
                    };
                }
            }

            // --- Activity overrides (in reverse order of priority: most important goes
            // last!) ---

            // Attack a target that's attacking us
            if let Some(my_stats) = stats.get(entity) {
                // Only if the attack was recent
                if my_stats.health.last_change.0 < 5.0 {
                    if let comp::HealthSource::Attack { by }
                    | comp::HealthSource::Projectile { owner: Some(by) } =
                        my_stats.health.last_change.1.cause
                    {
                        if !agent.activity.is_attack() {
                            if let Some(attacker) = uid_allocator.retrieve_entity_internal(by.id())
                            {
                                if stats.get(attacker).map_or(false, |a| !a.is_dead) {
                                    if agent.can_speak {
                                        let msg = "npc.speech.villager_under_attack".to_string();
                                        event_bus
                                            .emit_now(ServerEvent::Chat(ChatMsg::npc(*uid, msg)));
                                    }

                                    agent.activity = Activity::Attack {
                                        target: attacker,
                                        chaser: Chaser::default(),
                                        time: time.0,
                                        been_close: false,
                                        powerup: 0.0,
                                    };
                                }
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
                        agent.activity = Activity::Follow {
                            target: owner,
                            chaser: Chaser::default(),
                        };
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
                                            powerup: 0.0,
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
