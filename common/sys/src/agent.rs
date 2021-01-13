use common::{
    comp::{
        self,
        agent::Activity,
        group,
        group::Invite,
        inventory::slot::EquipSlot,
        item::{
            tool::{ToolKind, UniqueKind},
            ItemKind,
        },
        skills::{AxeSkill, BowSkill, HammerSkill, Skill, StaffSkill, SwordSkill},
        Agent, Alignment, Body, CharacterState, ControlAction, ControlEvent, Controller, Energy,
        GroupManip, Health, Inventory, LightEmitter, MountState, Ori, PhysicsState, Pos, Scale,
        Stats, UnresolvedChatMsg, Vel,
    },
    event::{EventBus, ServerEvent},
    metrics::SysMetrics,
    path::{Chaser, TraversalConfig},
    resources::{DeltaTime, Time, TimeOfDay},
    span,
    terrain::{Block, TerrainGrid},
    time::DayPeriod,
    uid::{Uid, UidAllocator},
    util::Dir,
    vol::ReadVol,
};
use rand::{thread_rng, Rng};
use rayon::iter::ParallelIterator;
use specs::{
    saveload::{Marker, MarkerAllocator},
    Entities, Join, ParJoin, Read, ReadExpect, ReadStorage, System, Write, WriteStorage,
};
use std::f32::consts::PI;
use vek::*;

/// This system will allow NPCs to modify their controller
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        (
            Read<'a, UidAllocator>,
            Read<'a, Time>,
            Read<'a, DeltaTime>,
            Read<'a, group::GroupManager>,
        ),
        ReadExpect<'a, SysMetrics>,
        Write<'a, EventBus<ServerEvent>>,
        Entities<'a>,
        ReadStorage<'a, Energy>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Vel>,
        ReadStorage<'a, Ori>,
        ReadStorage<'a, Scale>,
        ReadStorage<'a, Health>,
        ReadStorage<'a, Inventory>,
        ReadStorage<'a, Stats>,
        ReadStorage<'a, PhysicsState>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, group::Group>,
        ReadExpect<'a, TerrainGrid>,
        ReadStorage<'a, Alignment>,
        ReadStorage<'a, Body>,
        WriteStorage<'a, Agent>,
        WriteStorage<'a, Controller>,
        ReadStorage<'a, MountState>,
        ReadStorage<'a, Invite>,
        Read<'a, TimeOfDay>,
        ReadStorage<'a, LightEmitter>,
        ReadStorage<'a, CharacterState>,
    );

    #[allow(clippy::or_fun_call)] // TODO: Pending review in #587
    fn run(
        &mut self,
        (
            (uid_allocator, time, dt, group_manager),
            sys_metrics,
            event_bus,
            entities,
            energies,
            positions,
            velocities,
            orientations,
            scales,
            healths,
            inventories,
            stats,
            physics_states,
            uids,
            groups,
            terrain,
            alignments,
            bodies,
            mut agents,
            mut controllers,
            mount_states,
            invites,
            time_of_day,
            light_emitter,
            char_states,
        ): Self::SystemData,
    ) {
        let start_time = std::time::Instant::now();
        span!(_guard, "run", "agent::Sys::run");

        (
            &entities,
            &energies,
            &positions,
            &velocities,
            &orientations,
            alignments.maybe(),
            &inventories,
            &stats,
            &physics_states,
            bodies.maybe(),
            &uids,
            &mut agents,
            &mut controllers,
            mount_states.maybe(),
            groups.maybe(),
            light_emitter.maybe(),
        )
            .par_join()
            .filter(|(_, _, _, _, _, _, _, _, _, _, _, _, _, mount_state, _, _)| {
                // Skip mounted entities
                mount_state.map(|ms| *ms == MountState::Unmounted).unwrap_or(true)
            })
            .for_each(|(
                entity,
                energy,
                pos,
                vel,
                ori,
                alignment,
                inventory,
                stats,
                physics_state,
                body,
                uid,
                agent,
                controller,
                _,
                group,
                light_emitter,
            )| {
                // Hack, replace with better system when groups are more sophisticated
                // Override alignment if in a group unless entity is owned already
                let alignment = if !matches!(alignment, Some(Alignment::Owned(_))) {
                    group
                        .and_then(|g| group_manager.group_info(*g))
                        .and_then(|info| uids.get(info.leader))
                        .copied()
                        .map(Alignment::Owned)
                        .or(alignment.copied())
                } else {
                    alignment.copied()
                };

                controller.reset();
                let mut event_emitter = event_bus.emitter();
                // Light lanterns at night
                // TODO Add a method to turn on NPC lanterns underground
                let lantern_equipped = inventory.equipped(EquipSlot::Lantern).as_ref().map_or(false, |item| {
                    matches!(item.kind(), comp::item::ItemKind::Lantern(_))
                });
                let lantern_turned_on = light_emitter.is_some();
                let day_period = DayPeriod::from(time_of_day.0);
                // Only emit event for agents that have a lantern equipped
                if lantern_equipped {
                    let mut rng = thread_rng();
                    if day_period.is_dark() && !lantern_turned_on {
                        // Agents with turned off lanterns turn them on randomly once it's nighttime and
                        // keep them on
                        // Only emit event for agents that sill need to
                        // turn on their lantern
                        if let 0 = rng.gen_range(0, 1000) {
                            controller.events.push(ControlEvent::EnableLantern)
                        }
                    } else if lantern_turned_on && day_period.is_light() {
                        // agents with turned on lanterns turn them off randomly once it's daytime and
                        // keep them off
                        if let 0 = rng.gen_range(0, 2000) {
                            controller.events.push(ControlEvent::DisableLantern)
                        }
                    }
                };

                let mut inputs = &mut controller.inputs;

                // Default to looking in orientation direction (can be overridden below)
                inputs.look_dir = ori.0;

                const AVG_FOLLOW_DIST: f32 = 6.0;
                const MAX_FOLLOW_DIST: f32 = 12.0;
                const MAX_CHASE_DIST: f32 = 18.0;
                const LISTEN_DIST: f32 = 16.0;
                const SEARCH_DIST: f32 = 48.0;
                const SIGHT_DIST: f32 = 80.0;
                const MIN_ATTACK_DIST: f32 = 2.0;
                const MAX_FLEE_DIST: f32 = 20.0;
                const SNEAK_COEFFICIENT: f32 = 0.25;

                let scale = scales.get(entity).map(|s| s.0).unwrap_or(1.0);

                // This controls how picky NPCs are about their pathfinding. Giants are larger
                // and so can afford to be less precise when trying to move around
                // the world (especially since they would otherwise get stuck on
                // obstacles that smaller entities would not).
                let node_tolerance = scale * 1.5;
                let slow_factor = body.map(|b| b.base_accel() / 250.0).unwrap_or(0.0).min(1.0);
                let traversal_config = TraversalConfig {
                    node_tolerance,
                    slow_factor,
                    on_ground: physics_state.on_ground,
                    in_liquid: physics_state.in_liquid.is_some(),
                    min_tgt_dist: 1.0,
                    can_climb: body.map(|b| b.can_climb()).unwrap_or(false),
                    can_fly: body.map(|b| b.can_fly()).unwrap_or(false),
                };

                let mut do_idle = false;
                let mut choose_target = false;

                'activity: {
                    match &mut agent.activity {
                        Activity::Idle { bearing, chaser } => {
                            if let Some(travel_to) = agent.rtsim_controller.travel_to {
                                // if it has an rtsim destination and can fly then it should
                                // if it is flying and bumps something above it then it should move down
                                inputs.fly.set_state(traversal_config.can_fly && !terrain
                                    .ray(
                                        pos.0,
                                        pos.0 + (Vec3::unit_z() * 3.0))
                                    .until(Block::is_solid)
                                    .cast()
                                    .1
                                    .map_or(true, |b| b.is_some()));
                                if let Some((bearing, speed)) =
                                    chaser.chase(&*terrain, pos.0, vel.0, travel_to, TraversalConfig {
                                        min_tgt_dist: 1.25,
                                        ..traversal_config
                                    })
                                {
                                    inputs.move_dir =
                                        bearing.xy().try_normalized().unwrap_or(Vec2::zero())
                                            * speed.min(agent.rtsim_controller.speed_factor);
                                    inputs.jump.set_state(bearing.z > 1.5 || traversal_config.can_fly && traversal_config.on_ground);
                                    inputs.climb = Some(comp::Climb::Up);
                                    //.filter(|_| bearing.z > 0.1 || physics_state.in_liquid.is_some());

                                    inputs.move_z = bearing.z + if traversal_config.can_fly {
                                        if terrain
                                            .ray(
                                                pos.0 + Vec3::unit_z(),
                                                pos.0
                                                    + bearing
                                                    .try_normalized()
                                                    .unwrap_or(Vec3::unit_y())
                                                    * 60.0
                                                    + Vec3::unit_z(),
                                            )
                                            .until(Block::is_solid)
                                            .cast()
                                            .1
                                            .map_or(true, |b| b.is_some())
                                        {
                                            1.0 //fly up when approaching obstacles
                                        } else { -0.1 } //flying things should slowly come down from the stratosphere
                                    } else {
                                        0.05 //normal land traveller offset
                                    };
                                }
                            } else {
                                *bearing += Vec2::new(
                                    thread_rng().gen::<f32>() - 0.5,
                                    thread_rng().gen::<f32>() - 0.5,
                                ) * 0.1
                                    - *bearing * 0.003
                                    - agent.patrol_origin.map_or(Vec2::zero(), |patrol_origin| {
                                        (pos.0 - patrol_origin).xy() * 0.0002
                                    });

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
                                        .until(Block::is_solid)
                                        .cast()
                                        .1
                                        .map_or(true, |b| b.is_none())
                                    {
                                        0.9
                                    } else {
                                        0.0
                                    };

                                if bearing.magnitude_squared() > 0.5f32.powi(2) {
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
                            }

                            controller.actions.push(ControlAction::Unwield);

                            // Sometimes try searching for new targets
                            if thread_rng().gen::<f32>() < 0.1 {
                                choose_target = true;
                            }
                        },
                        Activity::Follow { target, chaser } => {
                            if let (Some(tgt_pos), _tgt_health) =
                                (positions.get(*target), healths.get(*target))
                            {
                                let dist = pos.0.distance(tgt_pos.0);
                                // Follow, or return to idle
                                if dist > AVG_FOLLOW_DIST {
                                    if let Some((bearing, speed)) = chaser.chase(
                                        &*terrain,
                                        pos.0,
                                        vel.0,
                                        tgt_pos.0,
                                        TraversalConfig {
                                            min_tgt_dist: AVG_FOLLOW_DIST,
                                            ..traversal_config
                                        },
                                    ) {
                                        inputs.move_dir =
                                            bearing.xy().try_normalized().unwrap_or(Vec2::zero())
                                                * speed.min(0.2 + (dist - AVG_FOLLOW_DIST) / 8.0);
                                        inputs.jump.set_state(bearing.z > 1.5);
                                        inputs.move_z = bearing.z;
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
                            #[derive(Eq, PartialEq)]
                            enum Tactic {
                                Melee,
                                Axe,
                                Hammer,
                                Sword,
                                Bow,
                                Staff,
                                StoneGolemBoss,
                                CircleCharge { radius: u32, circle_time: u32 },
                                QuadLowRanged,
                                TailSlap,
                                QuadLowQuick,
                                QuadLowBasic,
                                QuadMedJump,
                                QuadMedBasic,
                                Lavadrake,
                                Theropod,
                            }

                            let tactic = match inventory.equipped(EquipSlot::Mainhand).as_ref().and_then(|item| {
                                if let ItemKind::Tool(tool) = &item.kind() {
                                    Some(&tool.kind)
                                } else {
                                    None
                                }
                            }) {
                                Some(ToolKind::Bow) => Tactic::Bow,
                                Some(ToolKind::Staff) => Tactic::Staff,
                                Some(ToolKind::Hammer) => Tactic::Hammer,
                                Some(ToolKind::Sword) => Tactic::Sword,
                                Some(ToolKind::Axe) => Tactic::Axe,
                                Some(ToolKind::Unique(UniqueKind::StoneGolemFist)) => {
                                    Tactic::StoneGolemBoss
                                },
                                Some(ToolKind::Unique(UniqueKind::QuadMedQuick)) => {
                                    Tactic::CircleCharge {
                                        radius: 3,
                                        circle_time: 2,
                                    }
                                },
                                Some(ToolKind::Unique(UniqueKind::QuadMedCharge)) => {
                                    Tactic::CircleCharge {
                                        radius: 15,
                                        circle_time: 1,
                                    }
                                },

                                Some(ToolKind::Unique(UniqueKind::QuadMedJump)) => Tactic::QuadMedJump,
                                Some(ToolKind::Unique(UniqueKind::QuadMedBasic)) => {
                                    Tactic::QuadMedBasic
                                },
                                Some(ToolKind::Unique(UniqueKind::QuadLowRanged)) => {
                                    Tactic::QuadLowRanged
                                },
                                Some(ToolKind::Unique(UniqueKind::QuadLowTail)) => Tactic::TailSlap,
                                Some(ToolKind::Unique(UniqueKind::QuadLowQuick)) => {
                                    Tactic::QuadLowQuick
                                },
                                Some(ToolKind::Unique(UniqueKind::QuadLowBasic)) => {
                                    Tactic::QuadLowBasic
                                },
                                Some(ToolKind::Unique(UniqueKind::QuadLowBreathe)) => Tactic::Lavadrake,
                                Some(ToolKind::Unique(UniqueKind::TheropodBasic)) => Tactic::Theropod,
                                Some(ToolKind::Unique(UniqueKind::TheropodBird)) => Tactic::Theropod,
                                _ => Tactic::Melee,
                            };

                            if let (Some(tgt_pos), Some(tgt_health), tgt_alignment) = (
                                positions.get(*target),
                                healths.get(*target),
                                alignments.get(*target).copied().unwrap_or(
                                    uids.get(*target)
                                        .copied()
                                        .map(Alignment::Owned)
                                        .unwrap_or(Alignment::Wild),
                                ),
                            ) {
                                // Wield the weapon as running towards the target
                                controller.actions.push(ControlAction::Wield);

                                let eye_offset = body.map_or(0.0, |b| b.eye_height());

                                let tgt_eye_offset = bodies.get(*target).map_or(0.0, |b| b.eye_height()) +
                                    // Special case for jumping attacks to jump at the body
                                    // of the target and not the ground around the target
                                    // For the ranged it is to shoot at the feet and not
                                    // the head to get splash damage
                                    if tactic == Tactic::QuadMedJump {
                                        1.0
                                    } else if matches!(tactic, Tactic::QuadLowRanged) {
                                        -1.0
                                    } else {
                                        0.0
                                    };

                                // Hacky distance offset for ranged weapons
                                let distance_offset = match tactic {
                                    Tactic::Bow => 0.0004 /* Yay magic numbers */ * pos.0.distance_squared(tgt_pos.0),
                                    Tactic::Staff => 0.0015 /* Yay magic numbers */ * pos.0.distance_squared(tgt_pos.0),
                                    Tactic::QuadLowRanged => 0.03 /* Yay magic numbers */ * pos.0.distance_squared(tgt_pos.0),
                                    _ => 0.0,
                                };

                                // Apply the distance and eye offsets to make the
                                // look_dir the vector from projectile launch to
                                // target point
                                if let Some(dir) = Dir::from_unnormalized(
                                    Vec3::new(
                                        tgt_pos.0.x,
                                        tgt_pos.0.y,
                                        tgt_pos.0.z + tgt_eye_offset + distance_offset,
                                    ) - Vec3::new(pos.0.x, pos.0.y, pos.0.z + eye_offset),
                                ) {
                                    inputs.look_dir = dir;
                                }

                                // Don't attack entities we are passive towards
                                // TODO: This is here, it's a bit of a hack
                                if let Some(alignment) = alignment {
                                    if alignment.passive_towards(tgt_alignment) || tgt_health.is_dead {
                                        do_idle = true;
                                        break 'activity;
                                    }
                                }

                                let dist_sqrd = pos.0.distance_squared(tgt_pos.0);

                                let damage = healths
                                    .get(entity)
                                    .map(|h| h.current() as f32 / h.maximum() as f32)
                                    .unwrap_or(0.5);

                                // Flee
                                let flees = alignment
                                    .map(|a| !matches!(a, Alignment::Enemy | Alignment::Owned(_)))
                                    .unwrap_or(true);
                                if 1.0 - agent.psyche.aggro > damage && flees {
                                    if let Some(body) = body {
                                        if body.can_strafe() {
                                            controller.actions.push(ControlAction::Unwield);
                                        }
                                    }
                                    if dist_sqrd < MAX_FLEE_DIST.powi(2) {
                                        if let Some((bearing, speed)) = chaser.chase(
                                            &*terrain,
                                            pos.0,
                                            vel.0,
                                            // Away from the target (ironically)
                                            pos.0
                                                + (pos.0 - tgt_pos.0)
                                                    .try_normalized()
                                                    .unwrap_or_else(Vec3::unit_y)
                                                    * 50.0,
                                            TraversalConfig {
                                                min_tgt_dist: 1.25,
                                                ..traversal_config
                                            },
                                        ) {
                                            inputs.move_dir =
                                                bearing.xy().try_normalized().unwrap_or(Vec2::zero())
                                                    * speed;
                                            inputs.jump.set_state(bearing.z > 1.5);
                                            inputs.move_z = bearing.z;
                                        }
                                    } else {
                                        do_idle = true;
                                    }
                                } else {
                                    // Match on tactic. Each tactic has different controls
                                    // depending on the distance from the agent to the target
                                    match tactic {
                                        Tactic::Melee => {
                                            if dist_sqrd < (MIN_ATTACK_DIST * scale).powi(2) {
                                                inputs.primary.set_state(true);
                                                inputs.move_dir = Vec2::zero();
                                            } else if dist_sqrd < MAX_CHASE_DIST.powi(2)
                                                || (dist_sqrd < SIGHT_DIST.powi(2) && !*been_close)
                                            {
                                                if dist_sqrd < MAX_CHASE_DIST.powi(2) {
                                                    *been_close = true;
                                                }
                                                if let Some((bearing, speed)) = chaser.chase(
                                                    &*terrain,
                                                    pos.0,
                                                    vel.0,
                                                    tgt_pos.0,
                                                    TraversalConfig {
                                                        min_tgt_dist: 1.25,
                                                        ..traversal_config
                                                    },
                                                ) {
                                                    inputs.move_dir = bearing
                                                        .xy()
                                                        .try_normalized()
                                                        .unwrap_or(Vec2::zero())
                                                        * speed;
                                                    inputs.jump.set_state(bearing.z > 1.5);
                                                    inputs.move_z = bearing.z;
                                                }

                                                if dist_sqrd < 16.0f32.powi(2)
                                                    && thread_rng().gen::<f32>() < 0.02
                                                {
                                                    inputs.roll.set_state(true);
                                                }
                                            } else {
                                                do_idle = true;
                                            }
                                        },
                                        Tactic::Axe => {
                                            if dist_sqrd < (MIN_ATTACK_DIST * scale).powi(2) {
                                                inputs.move_dir = Vec2::zero();
                                                if *powerup > 6.0 {
                                                    inputs.secondary.set_state(false);
                                                    *powerup = 0.0;
                                                } else if *powerup > 4.0 && energy.current() > 10 {
                                                    inputs.secondary.set_state(true);
                                                    *powerup += dt.0;
                                                } else if stats.skill_set.skills.contains_key(&Skill::Axe(AxeSkill::UnlockLeap)) && energy.current() > 800 && thread_rng().gen_bool(0.5) {
                                                    inputs.ability3.set_state(true);
                                                    *powerup += dt.0;
                                                } else {
                                                    inputs.primary.set_state(true);
                                                    *powerup += dt.0;
                                                }
                                            } else if dist_sqrd < MAX_CHASE_DIST.powi(2)
                                                || (dist_sqrd < SIGHT_DIST.powi(2) && !*been_close)
                                            {
                                                if dist_sqrd < MAX_CHASE_DIST.powi(2) {
                                                    *been_close = true;
                                                }
                                                if let Some((bearing, speed)) = chaser.chase(
                                                    &*terrain,
                                                    pos.0,
                                                    vel.0,
                                                    tgt_pos.0,
                                                    TraversalConfig {
                                                        min_tgt_dist: 1.25,
                                                        ..traversal_config
                                                    },
                                                ) {
                                                    inputs.move_dir = bearing
                                                        .xy()
                                                        .try_normalized()
                                                        .unwrap_or(Vec2::zero())
                                                        * speed;
                                                    inputs.jump.set_state(bearing.z > 1.5);
                                                    inputs.move_z = bearing.z;
                                                }
                                                if dist_sqrd < 16.0f32.powi(2)
                                                    && thread_rng().gen::<f32>() < 0.02
                                                {
                                                    inputs.roll.set_state(true);
                                                }
                                            } else {
                                                do_idle = true;
                                            }
                                        },
                                        Tactic::Hammer => {
                                            if dist_sqrd < (MIN_ATTACK_DIST * scale).powi(2) {
                                                inputs.move_dir = Vec2::zero();
                                                if *powerup > 4.0 {
                                                    inputs.secondary.set_state(false);
                                                    *powerup = 0.0;
                                                } else if *powerup > 2.0 {
                                                    inputs.secondary.set_state(true);
                                                    *powerup += dt.0;
                                                } else if stats.skill_set.skills.contains_key(&Skill::Hammer(HammerSkill::UnlockLeap)) && energy.current() > 700
                                                    && thread_rng().gen_bool(0.9) {
                                                    inputs.ability3.set_state(true);
                                                    *powerup += dt.0;
                                                } else {
                                                    inputs.primary.set_state(true);
                                                    *powerup += dt.0;
                                                }
                                            } else if dist_sqrd < MAX_CHASE_DIST.powi(2)
                                                || (dist_sqrd < SIGHT_DIST.powi(2) && !*been_close)
                                            {
                                                if dist_sqrd < MAX_CHASE_DIST.powi(2) {
                                                    *been_close = true;
                                                }
                                                if let Some((bearing, speed)) = chaser.chase(
                                                    &*terrain,
                                                    pos.0,
                                                    vel.0,
                                                    tgt_pos.0,
                                                    TraversalConfig {
                                                        min_tgt_dist: 1.25,
                                                        ..traversal_config
                                                    },
                                                ) {
                                                    if can_see_tgt(&*terrain, pos, tgt_pos, dist_sqrd) {
                                                        inputs.move_dir = bearing
                                                            .xy()
                                                            .try_normalized()
                                                            .unwrap_or(Vec2::zero())
                                                            * speed;
                                                        if stats.skill_set.skills.contains_key(&Skill::Hammer(HammerSkill::UnlockLeap)) && *powerup > 5.0 {
                                                            inputs.ability3.set_state(true);
                                                            *powerup = 0.0;
                                                        } else {
                                                            *powerup += dt.0;
                                                        }
                                                    } else {
                                                        inputs.move_dir = bearing
                                                            .xy()
                                                            .try_normalized()
                                                            .unwrap_or(Vec2::zero())
                                                            * speed;
                                                        inputs.jump.set_state(bearing.z > 1.5);
                                                        inputs.move_z = bearing.z;
                                                    }
                                                }
                                                if dist_sqrd < 16.0f32.powi(2)
                                                    && thread_rng().gen::<f32>() < 0.02
                                                {
                                                    inputs.roll.set_state(true);
                                                }
                                            } else {
                                                do_idle = true;
                                            }
                                        },
                                        Tactic::Sword => {
                                            if dist_sqrd < (MIN_ATTACK_DIST * scale).powi(2) {
                                                inputs.move_dir = Vec2::zero();
                                                if stats.skill_set.skills.contains_key(&Skill::Sword(SwordSkill::UnlockSpin)) && *powerup < 2.0 && energy.current() > 600 {
                                                    inputs.ability3.set_state(true);
                                                    *powerup += dt.0;
                                                } else if *powerup > 2.0 {
                                                    *powerup = 0.0;
                                                } else {
                                                    inputs.primary.set_state(true);
                                                    *powerup += dt.0;
                                                }
                                            } else if dist_sqrd < MAX_CHASE_DIST.powi(2)
                                                || (dist_sqrd < SIGHT_DIST.powi(2) && !*been_close)
                                            {
                                                if dist_sqrd < MAX_CHASE_DIST.powi(2) {
                                                    *been_close = true;
                                                }
                                                if let Some((bearing, speed)) = chaser.chase(
                                                    &*terrain,
                                                    pos.0,
                                                    vel.0,
                                                    tgt_pos.0,
                                                    TraversalConfig {
                                                        min_tgt_dist: 1.25,
                                                        ..traversal_config
                                                    },
                                                ) {
                                                    if can_see_tgt(&*terrain, pos, tgt_pos, dist_sqrd) {
                                                        inputs.move_dir = bearing
                                                            .xy()
                                                            .try_normalized()
                                                            .unwrap_or(Vec2::zero())
                                                            * speed;
                                                        if *powerup > 4.0 {
                                                            inputs.secondary.set_state(true);
                                                            *powerup = 0.0;
                                                        } else {
                                                            *powerup += dt.0;
                                                        }
                                                    } else {
                                                        inputs.move_dir = bearing
                                                            .xy()
                                                            .try_normalized()
                                                            .unwrap_or(Vec2::zero())
                                                            * speed;
                                                        inputs.jump.set_state(bearing.z > 1.5);
                                                        inputs.move_z = bearing.z;
                                                    }
                                                }
                                                if dist_sqrd < 16.0f32.powi(2)
                                                    && thread_rng().gen::<f32>() < 0.02
                                                {
                                                    inputs.roll.set_state(true);
                                                }
                                            } else {
                                                do_idle = true;
                                            }
                                        },
                                        Tactic::Bow => {
                                            if dist_sqrd < (2.0 * MIN_ATTACK_DIST * scale).powi(2) {
                                                inputs.roll.set_state(true);
                                            } else if dist_sqrd < MAX_CHASE_DIST.powi(2)
                                                || (dist_sqrd < SIGHT_DIST.powi(2) && !*been_close)
                                            {
                                                if dist_sqrd < MAX_CHASE_DIST.powi(2) {
                                                    *been_close = true;
                                                }
                                                if let Some((bearing, speed)) = chaser.chase(
                                                    &*terrain,
                                                    pos.0,
                                                    vel.0,
                                                    tgt_pos.0,
                                                    TraversalConfig {
                                                        min_tgt_dist: 1.25,
                                                        ..traversal_config
                                                    },
                                                ) {
                                                    if can_see_tgt(&*terrain, pos, tgt_pos, dist_sqrd) {
                                                        inputs.move_dir = bearing
                                                            .xy()
                                                            .rotated_z(
                                                                thread_rng().gen_range(0.5, 1.57),
                                                            )
                                                            .try_normalized()
                                                            .unwrap_or(Vec2::zero())
                                                            * speed;
                                                        if *powerup > 4.0 {
                                                            inputs.secondary.set_state(false);
                                                            *powerup = 0.0;
                                                        } else if *powerup > 2.0
                                                            && energy.current() > 300
                                                        {
                                                            inputs.secondary.set_state(true);
                                                            *powerup += dt.0;
                                                        } else if stats.skill_set.skills.contains_key(&Skill::Bow(BowSkill::UnlockRepeater)) && energy.current() > 400
                                                            && thread_rng().gen_bool(0.8)
                                                        {
                                                            inputs.secondary.set_state(false);
                                                            inputs.ability3.set_state(true);
                                                            *powerup += dt.0;
                                                        } else {
                                                            inputs.secondary.set_state(false);
                                                            inputs.primary.set_state(true);
                                                            *powerup += dt.0;
                                                        }
                                                    } else {
                                                        inputs.move_dir = bearing
                                                            .xy()
                                                            .try_normalized()
                                                            .unwrap_or(Vec2::zero())
                                                            * speed;
                                                        inputs.jump.set_state(bearing.z > 1.5);
                                                        inputs.move_z = bearing.z;
                                                    }
                                                }
                                                if dist_sqrd < 16.0f32.powi(2)
                                                    && thread_rng().gen::<f32>() < 0.02
                                                {
                                                    inputs.roll.set_state(true);
                                                }
                                            } else {
                                                do_idle = true;
                                            }
                                        },
                                        Tactic::Staff => {
                                            if dist_sqrd < (MIN_ATTACK_DIST * scale).powi(2) {
                                                inputs.roll.set_state(true);
                                            } else if dist_sqrd
                                                < (5.0 * MIN_ATTACK_DIST * scale).powi(2)
                                            {
                                                if *powerup < 1.5 {
                                                    inputs.move_dir = (tgt_pos.0 - pos.0)
                                                        .xy()
                                                        .rotated_z(0.47 * PI)
                                                        .try_normalized()
                                                        .unwrap_or(Vec2::unit_y());
                                                    *powerup += dt.0;
                                                } else if *powerup < 3.0 {
                                                    inputs.move_dir = (tgt_pos.0 - pos.0)
                                                        .xy()
                                                        .rotated_z(-0.47 * PI)
                                                        .try_normalized()
                                                        .unwrap_or(Vec2::unit_y());
                                                    *powerup += dt.0;
                                                } else {
                                                    *powerup = 0.0;
                                                }
                                                if stats.skill_set.skills.contains_key(&Skill::Staff(StaffSkill::UnlockShockwave)) && energy.current() > 800
                                                    && thread_rng().gen::<f32>() > 0.8
                                                {
                                                    inputs.ability3.set_state(true);
                                                } else if energy.current() > 10 {
                                                    inputs.secondary.set_state(true);
                                                } else {
                                                    inputs.primary.set_state(true);
                                                }
                                            } else if dist_sqrd < MAX_CHASE_DIST.powi(2)
                                                || (dist_sqrd < SIGHT_DIST.powi(2) && !*been_close)
                                            {
                                                if dist_sqrd < MAX_CHASE_DIST.powi(2) {
                                                    *been_close = true;
                                                }
                                                if let Some((bearing, speed)) = chaser.chase(
                                                    &*terrain,
                                                    pos.0,
                                                    vel.0,
                                                    tgt_pos.0,
                                                    TraversalConfig {
                                                        min_tgt_dist: 1.25,
                                                        ..traversal_config
                                                    },
                                                ) {
                                                    if can_see_tgt(&*terrain, pos, tgt_pos, dist_sqrd) {
                                                        inputs.move_dir = bearing
                                                            .xy()
                                                            .rotated_z(
                                                                thread_rng().gen_range(-1.57, -0.5),
                                                            )
                                                            .try_normalized()
                                                            .unwrap_or(Vec2::zero())
                                                            * speed;
                                                        inputs.primary.set_state(true);
                                                    } else {
                                                        inputs.move_dir = bearing
                                                            .xy()
                                                            .try_normalized()
                                                            .unwrap_or(Vec2::zero())
                                                            * speed;
                                                        inputs.jump.set_state(bearing.z > 1.5);
                                                        inputs.move_z = bearing.z;
                                                    }
                                                }
                                                if dist_sqrd < 16.0f32.powi(2)
                                                    && thread_rng().gen::<f32>() < 0.02
                                                {
                                                    inputs.roll.set_state(true);
                                                }
                                            } else {
                                                do_idle = true;
                                            }
                                        },
                                        Tactic::StoneGolemBoss => {
                                            if dist_sqrd < (MIN_ATTACK_DIST * scale).powi(2) {
                                                inputs.move_dir = Vec2::zero();
                                                inputs.primary.set_state(true);
                                            } else if dist_sqrd < MAX_CHASE_DIST.powi(2)
                                                || (dist_sqrd < SIGHT_DIST.powi(2) && !*been_close)
                                            {
                                                if dist_sqrd < MAX_CHASE_DIST.powi(2) {
                                                    *been_close = true;
                                                }
                                                if let Some((bearing, speed)) = chaser.chase(
                                                    &*terrain,
                                                    pos.0,
                                                    vel.0,
                                                    tgt_pos.0,
                                                    TraversalConfig {
                                                        min_tgt_dist: 1.25,
                                                        ..traversal_config
                                                    },
                                                ) {
                                                    if can_see_tgt(&*terrain, pos, tgt_pos, dist_sqrd) {
                                                        inputs.move_dir = bearing
                                                            .xy()
                                                            .try_normalized()
                                                            .unwrap_or(Vec2::zero())
                                                            * speed;
                                                        if *powerup > 5.0 {
                                                            inputs.secondary.set_state(true);
                                                            *powerup = 0.0;
                                                        } else {
                                                            *powerup += dt.0;
                                                        }
                                                    } else {
                                                        inputs.move_dir = bearing
                                                            .xy()
                                                            .try_normalized()
                                                            .unwrap_or(Vec2::zero())
                                                            * speed;
                                                        inputs.jump.set_state(bearing.z > 1.5);
                                                        inputs.move_z = bearing.z;
                                                    }
                                                }
                                            } else {
                                                do_idle = true;
                                            }
                                        },
                                        Tactic::CircleCharge {
                                            radius,
                                            circle_time,
                                        } => {
                                            if dist_sqrd < (MIN_ATTACK_DIST * scale).powi(2)
                                                && thread_rng().gen_bool(0.5)
                                            {
                                                inputs.move_dir = Vec2::zero();
                                                inputs.primary.set_state(true);
                                            } else if dist_sqrd
                                                < (radius as f32 * MIN_ATTACK_DIST * scale).powi(2)
                                            {
                                                inputs.move_dir = (pos.0 - tgt_pos.0)
                                                    .xy()
                                                    .try_normalized()
                                                    .unwrap_or(Vec2::unit_y());
                                            } else if dist_sqrd
                                                < ((radius as f32 + 1.0) * MIN_ATTACK_DIST * scale)
                                                    .powi(2)
                                                && dist_sqrd
                                                    > (radius as f32 * MIN_ATTACK_DIST * scale).powi(2)
                                            {
                                                if *powerup < circle_time as f32 {
                                                    inputs.move_dir = (tgt_pos.0 - pos.0)
                                                        .xy()
                                                        .rotated_z(0.47 * PI)
                                                        .try_normalized()
                                                        .unwrap_or(Vec2::unit_y());
                                                    *powerup += dt.0;
                                                } else if *powerup < circle_time as f32 + 0.5 {
                                                    inputs.secondary.set_state(true);
                                                    *powerup += dt.0;
                                                } else if *powerup < 2.0 * circle_time as f32 + 0.5 {
                                                    inputs.move_dir = (tgt_pos.0 - pos.0)
                                                        .xy()
                                                        .rotated_z(-0.47 * PI)
                                                        .try_normalized()
                                                        .unwrap_or(Vec2::unit_y());
                                                    *powerup += dt.0;
                                                } else if *powerup < 2.0 * circle_time as f32 + 1.0 {
                                                    inputs.secondary.set_state(true);
                                                    *powerup += dt.0;
                                                } else {
                                                    *powerup = 0.0;
                                                }
                                            } else if dist_sqrd < MAX_CHASE_DIST.powi(2)
                                                || (dist_sqrd < SIGHT_DIST.powi(2) && !*been_close)
                                            {
                                                if dist_sqrd < MAX_CHASE_DIST.powi(2) {
                                                    *been_close = true;
                                                }
                                                if let Some((bearing, speed)) = chaser.chase(
                                                    &*terrain,
                                                    pos.0,
                                                    vel.0,
                                                    tgt_pos.0,
                                                    TraversalConfig {
                                                        min_tgt_dist: 1.25,
                                                        ..traversal_config
                                                    },
                                                ) {
                                                    inputs.move_dir = bearing
                                                        .xy()
                                                        .try_normalized()
                                                        .unwrap_or(Vec2::zero())
                                                        * speed;
                                                    inputs.jump.set_state(bearing.z > 1.5);
                                                    inputs.move_z = bearing.z;
                                                }
                                            } else {
                                                do_idle = true;
                                            }
                                        },
                                        Tactic::QuadLowRanged => {
                                            if dist_sqrd < (5.0 * MIN_ATTACK_DIST * scale).powi(2) {
                                                inputs.move_dir = (tgt_pos.0 - pos.0)
                                                    .xy()
                                                    .try_normalized()
                                                    .unwrap_or(Vec2::unit_y());
                                                inputs.primary.set_state(true);
                                            } else if dist_sqrd < MAX_CHASE_DIST.powi(2)
                                                || (dist_sqrd < SIGHT_DIST.powi(2) && !*been_close)
                                            {
                                                if dist_sqrd < MAX_CHASE_DIST.powi(2) {
                                                    *been_close = true;
                                                }
                                                if let Some((bearing, speed)) = chaser.chase(
                                                    &*terrain,
                                                    pos.0,
                                                    vel.0,
                                                    tgt_pos.0,
                                                    TraversalConfig {
                                                        min_tgt_dist: 1.25,
                                                        ..traversal_config
                                                    },
                                                ) {
                                                    if can_see_tgt(&*terrain, pos, tgt_pos, dist_sqrd) {
                                                        if *powerup > 5.0 {
                                                            *powerup = 0.0;
                                                        } else if *powerup > 2.5 {
                                                            inputs.move_dir = (tgt_pos.0 - pos.0)
                                                                .xy()
                                                                .rotated_z(1.75 * PI)
                                                                .try_normalized()
                                                                .unwrap_or(Vec2::zero())
                                                                * speed;
                                                            *powerup += dt.0;
                                                        } else {
                                                            inputs.move_dir = (tgt_pos.0 - pos.0)
                                                                .xy()
                                                                .rotated_z(0.25 * PI)
                                                                .try_normalized()
                                                                .unwrap_or(Vec2::zero())
                                                                * speed;
                                                            *powerup += dt.0;
                                                        }
                                                        inputs.secondary.set_state(true);
                                                        inputs.jump.set_state(bearing.z > 1.5);
                                                        inputs.move_z = bearing.z;
                                                    } else {
                                                        inputs.move_dir = bearing
                                                            .xy()
                                                            .try_normalized()
                                                            .unwrap_or(Vec2::zero())
                                                            * speed;
                                                        inputs.jump.set_state(bearing.z > 1.5);
                                                        inputs.move_z = bearing.z;
                                                    }
                                                } else {
                                                    do_idle = true;
                                                }
                                            } else {
                                                do_idle = true;
                                            }
                                        },
                                        Tactic::TailSlap => {
                                            if dist_sqrd < (1.5 * MIN_ATTACK_DIST * scale).powi(2) {
                                                if *powerup > 4.0 {
                                                    inputs.primary.set_state(false);
                                                    *powerup = 0.0;
                                                } else if *powerup > 1.0 {
                                                    inputs.primary.set_state(true);
                                                    *powerup += dt.0;
                                                } else {
                                                    inputs.secondary.set_state(true);
                                                    *powerup += dt.0;
                                                }
                                                inputs.move_dir = (tgt_pos.0 - pos.0)
                                                    .xy()
                                                    .try_normalized()
                                                    .unwrap_or(Vec2::unit_y())
                                                    * 0.1;
                                            } else if dist_sqrd < MAX_CHASE_DIST.powi(2)
                                                || (dist_sqrd < SIGHT_DIST.powi(2) && !*been_close)
                                            {
                                                if dist_sqrd < MAX_CHASE_DIST.powi(2) {
                                                    *been_close = true;
                                                }
                                                if let Some((bearing, speed)) = chaser.chase(
                                                    &*terrain,
                                                    pos.0,
                                                    vel.0,
                                                    tgt_pos.0,
                                                    TraversalConfig {
                                                        min_tgt_dist: 1.25,
                                                        ..traversal_config
                                                    },
                                                ) {
                                                    inputs.move_dir = bearing
                                                        .xy()
                                                        .try_normalized()
                                                        .unwrap_or(Vec2::zero())
                                                        * speed;
                                                    inputs.jump.set_state(bearing.z > 1.5);
                                                    inputs.move_z = bearing.z;
                                                }
                                            } else {
                                                do_idle = true;
                                            }
                                        },
                                        Tactic::QuadLowQuick => {
                                            if dist_sqrd < (1.5 * MIN_ATTACK_DIST * scale).powi(2) {
                                                inputs.move_dir = Vec2::zero();
                                                inputs.secondary.set_state(true);
                                            } else if dist_sqrd
                                                < (3.0 * MIN_ATTACK_DIST * scale).powi(2)
                                                && dist_sqrd > (2.0 * MIN_ATTACK_DIST * scale).powi(2)
                                            {
                                                inputs.primary.set_state(true);
                                                inputs.move_dir = (tgt_pos.0 - pos.0)
                                                    .xy()
                                                    .rotated_z(-0.47 * PI)
                                                    .try_normalized()
                                                    .unwrap_or(Vec2::unit_y());
                                            } else if dist_sqrd < MAX_CHASE_DIST.powi(2)
                                                || (dist_sqrd < SIGHT_DIST.powi(2) && !*been_close)
                                            {
                                                if dist_sqrd < MAX_CHASE_DIST.powi(2) {
                                                    *been_close = true;
                                                }
                                                if let Some((bearing, speed)) = chaser.chase(
                                                    &*terrain,
                                                    pos.0,
                                                    vel.0,
                                                    tgt_pos.0,
                                                    TraversalConfig {
                                                        min_tgt_dist: 1.25,
                                                        ..traversal_config
                                                    },
                                                ) {
                                                    inputs.move_dir = bearing
                                                        .xy()
                                                        .try_normalized()
                                                        .unwrap_or(Vec2::zero())
                                                        * speed;
                                                    inputs.jump.set_state(bearing.z > 1.5);
                                                    inputs.move_z = bearing.z;
                                                }
                                            } else {
                                                do_idle = true;
                                            }
                                        },
                                        Tactic::QuadLowBasic => {
                                            if dist_sqrd < (1.5 * MIN_ATTACK_DIST * scale).powi(2) {
                                                inputs.move_dir = Vec2::zero();
                                                if *powerup > 5.0 {
                                                    *powerup = 0.0;
                                                } else if *powerup > 2.0 {
                                                    inputs.secondary.set_state(true);
                                                    *powerup += dt.0;
                                                } else {
                                                    inputs.primary.set_state(true);
                                                    *powerup += dt.0;
                                                }
                                            } else if dist_sqrd < MAX_CHASE_DIST.powi(2)
                                                || (dist_sqrd < SIGHT_DIST.powi(2) && !*been_close)
                                            {
                                                if dist_sqrd < MAX_CHASE_DIST.powi(2) {
                                                    *been_close = true;
                                                }
                                                if let Some((bearing, speed)) = chaser.chase(
                                                    &*terrain,
                                                    pos.0,
                                                    vel.0,
                                                    tgt_pos.0,
                                                    TraversalConfig {
                                                        min_tgt_dist: 1.25,
                                                        ..traversal_config
                                                    },
                                                ) {
                                                    inputs.move_dir = bearing
                                                        .xy()
                                                        .try_normalized()
                                                        .unwrap_or(Vec2::zero())
                                                        * speed;
                                                    inputs.jump.set_state(bearing.z > 1.5);
                                                    inputs.move_z = bearing.z;
                                                }
                                            } else {
                                                do_idle = true;
                                            }
                                        },
                                        Tactic::QuadMedJump => {
                                            if dist_sqrd < (1.5 * MIN_ATTACK_DIST * scale).powi(2) {
                                                inputs.move_dir = Vec2::zero();
                                                inputs.secondary.set_state(true);
                                            } else if dist_sqrd
                                                < (5.0 * MIN_ATTACK_DIST * scale).powi(2)
                                            {
                                                inputs.ability3.set_state(true);
                                            } else if dist_sqrd < MAX_CHASE_DIST.powi(2)
                                                || (dist_sqrd < SIGHT_DIST.powi(2) && !*been_close)
                                            {
                                                if dist_sqrd < MAX_CHASE_DIST.powi(2) {
                                                    *been_close = true;
                                                }
                                                if let Some((bearing, speed)) = chaser.chase(
                                                    &*terrain,
                                                    pos.0,
                                                    vel.0,
                                                    tgt_pos.0,
                                                    TraversalConfig {
                                                        min_tgt_dist: 1.25,
                                                        ..traversal_config
                                                    },
                                                ) {
                                                    if can_see_tgt(&*terrain, pos, tgt_pos, dist_sqrd) {
                                                        inputs.primary.set_state(true);
                                                        inputs.move_dir = bearing
                                                            .xy()
                                                            .try_normalized()
                                                            .unwrap_or(Vec2::zero())
                                                            * speed;
                                                    } else {
                                                        inputs.move_dir = bearing
                                                            .xy()
                                                            .try_normalized()
                                                            .unwrap_or(Vec2::zero())
                                                            * speed;
                                                        inputs.jump.set_state(bearing.z > 1.5);
                                                        inputs.move_z = bearing.z;
                                                    }
                                                }
                                            } else {
                                                do_idle = true;
                                            }
                                        },
                                        Tactic::QuadMedBasic => {
                                            if dist_sqrd < (MIN_ATTACK_DIST * scale).powi(2) {
                                                inputs.move_dir = Vec2::zero();
                                                if *powerup < 2.0 {
                                                    inputs.secondary.set_state(true);
                                                    *powerup += dt.0;
                                                } else if *powerup < 3.0 {
                                                    inputs.primary.set_state(true);
                                                    *powerup += dt.0;
                                                } else {
                                                    *powerup = 0.0;
                                                }
                                            } else if dist_sqrd < MAX_CHASE_DIST.powi(2)
                                                || (dist_sqrd < SIGHT_DIST.powi(2) && !*been_close)
                                            {
                                                if dist_sqrd < MAX_CHASE_DIST.powi(2) {
                                                    *been_close = true;
                                                }
                                                if let Some((bearing, speed)) = chaser.chase(
                                                    &*terrain,
                                                    pos.0,
                                                    vel.0,
                                                    tgt_pos.0,
                                                    TraversalConfig {
                                                        min_tgt_dist: 1.25,
                                                        ..traversal_config
                                                    },
                                                ) {
                                                    inputs.move_dir = bearing
                                                        .xy()
                                                        .try_normalized()
                                                        .unwrap_or(Vec2::zero())
                                                        * speed;
                                                    inputs.jump.set_state(bearing.z > 1.5);
                                                    inputs.move_z = bearing.z;
                                                }
                                            } else {
                                                do_idle = true;
                                            }
                                        },
                                        Tactic::Lavadrake => {
                                            if dist_sqrd < (2.5 * MIN_ATTACK_DIST * scale).powi(2) {
                                                inputs.move_dir = Vec2::zero();
                                                inputs.secondary.set_state(true);
                                            } else if dist_sqrd
                                                < (7.0 * MIN_ATTACK_DIST * scale).powi(2)
                                            {
                                                if *powerup < 2.0 {
                                                    inputs.move_dir = (tgt_pos.0 - pos.0)
                                                        .xy()
                                                        .rotated_z(0.47 * PI)
                                                        .try_normalized()
                                                        .unwrap_or(Vec2::unit_y());
                                                    inputs.primary.set_state(true);
                                                    *powerup += dt.0;
                                                } else if *powerup < 4.0 {
                                                    inputs.move_dir = (tgt_pos.0 - pos.0)
                                                        .xy()
                                                        .rotated_z(-0.47 * PI)
                                                        .try_normalized()
                                                        .unwrap_or(Vec2::unit_y());
                                                    inputs.primary.set_state(true);
                                                    *powerup += dt.0;
                                                } else if *powerup < 6.0 {
                                                    inputs.ability3.set_state(true);
                                                    *powerup += dt.0;
                                                } else {
                                                    *powerup = 0.0;
                                                }
                                            } else if dist_sqrd < MAX_CHASE_DIST.powi(2)
                                                || (dist_sqrd < SIGHT_DIST.powi(2) && !*been_close)
                                            {
                                                if dist_sqrd < MAX_CHASE_DIST.powi(2) {
                                                    *been_close = true;
                                                }
                                                if let Some((bearing, speed)) = chaser.chase(
                                                    &*terrain,
                                                    pos.0,
                                                    vel.0,
                                                    tgt_pos.0,
                                                    TraversalConfig {
                                                        min_tgt_dist: 1.25,
                                                        ..traversal_config
                                                    },
                                                ) {
                                                    inputs.move_dir = bearing
                                                        .xy()
                                                        .try_normalized()
                                                        .unwrap_or(Vec2::zero())
                                                        * speed;
                                                    inputs.jump.set_state(bearing.z > 1.5);
                                                    inputs.move_z = bearing.z;
                                                }
                                            } else {
                                                do_idle = true;
                                            }
                                        },
                                        Tactic::Theropod => {
                                            if dist_sqrd < (2.0 * MIN_ATTACK_DIST * scale).powi(2) {
                                                inputs.move_dir = Vec2::zero();
                                                inputs.primary.set_state(true);
                                            } else if dist_sqrd < MAX_CHASE_DIST.powi(2)
                                                || (dist_sqrd < SIGHT_DIST.powi(2) && !*been_close)
                                            {
                                                if dist_sqrd < MAX_CHASE_DIST.powi(2) {
                                                    *been_close = true;
                                                }
                                                if let Some((bearing, speed)) = chaser.chase(
                                                    &*terrain,
                                                    pos.0,
                                                    vel.0,
                                                    tgt_pos.0,
                                                    TraversalConfig {
                                                        min_tgt_dist: 1.25,
                                                        ..traversal_config
                                                    },
                                                ) {
                                                    inputs.move_dir = bearing
                                                        .xy()
                                                        .try_normalized()
                                                        .unwrap_or(Vec2::zero())
                                                        * speed;
                                                    inputs.jump.set_state(bearing.z > 1.5);
                                                    inputs.move_z = bearing.z;
                                                }
                                            } else {
                                                do_idle = true;
                                            }
                                        },
                                    }
                                }
                            } else {
                                do_idle = true;
                            }
                        },
                    }
                }

                if do_idle {
                    agent.activity = Activity::Idle {
                        bearing: Vec2::zero(),
                        chaser: Chaser::default(),
                    };
                }

                // Choose a new target to attack: only go out of our way to attack targets we
                // are hostile toward!
                if choose_target {
                    // Search for new targets (this looks expensive, but it's only run occasionally)
                    // TODO: Replace this with a better system that doesn't consider *all* entities
                    let closest_entity = (&entities, &positions, &healths, alignments.maybe(), char_states.maybe())
                        .join()
                        .filter(|(e, e_pos, e_health, e_alignment, char_state)| {
                            let mut search_dist = SEARCH_DIST;
                            let mut listen_dist = LISTEN_DIST;
                            if char_state.map_or(false, |c_s| c_s.is_stealthy()) {
                                // TODO: make sneak more effective based on a stat like e_stats.fitness
                                search_dist *= SNEAK_COEFFICIENT;
                                listen_dist *= SNEAK_COEFFICIENT;
                            }
                            ((e_pos.0.distance_squared(pos.0) < search_dist.powi(2) &&
                                // Within our view
                                (e_pos.0 - pos.0).try_normalized().map(|v| v.dot(*inputs.look_dir) > 0.15).unwrap_or(true))
                                    // Within listen distance
                                    || e_pos.0.distance_squared(pos.0) < listen_dist.powi(2))
                                && *e != entity
                                && !e_health.is_dead
                                && alignment
                                    .and_then(|a| e_alignment.map(|b| a.hostile_towards(*b)))
                                    .unwrap_or(false)
                        })
                        // Can we even see them?
                        .filter(|(_, e_pos, _, _, _)| terrain
                            .ray(pos.0 + Vec3::unit_z(), e_pos.0 + Vec3::unit_z())
                            .until(Block::is_opaque)
                            .cast()
                            .0 >= e_pos.0.distance(pos.0))
                        .min_by_key(|(_, e_pos, _, _, _)| (e_pos.0.distance_squared(pos.0) * 100.0) as i32)
                        .map(|(e, _, _, _, _)| e);

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
                if let Some(my_health) = healths.get(entity) {
                    // Only if the attack was recent
                    if my_health.last_change.0 < 3.0 {
                        if let comp::HealthSource::Damage { by: Some(by), .. } =
                            my_health.last_change.1.cause
                        {
                            if !agent.activity.is_attack() {
                                if let Some(attacker) = uid_allocator.retrieve_entity_internal(by.id())
                                {
                                    if healths.get(attacker).map_or(false, |a| !a.is_dead) {
                                        match agent.activity {
                                            Activity::Attack { target, .. } if target == attacker => {},
                                            _ => {
                                                if agent.can_speak {
                                                    let msg =
                                                        "npc.speech.villager_under_attack".to_string();
                                                    event_emitter.emit(ServerEvent::Chat(
                                                        UnresolvedChatMsg::npc(*uid, msg),
                                                    ));
                                                }

                                                agent.activity = Activity::Attack {
                                                    target: attacker,
                                                    chaser: Chaser::default(),
                                                    time: time.0,
                                                    been_close: false,
                                                    powerup: 0.0,
                                                };
                                            },
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Follow owner if we're too far, or if they're under attack
                if let Some(Alignment::Owned(owner)) = alignment {
                    (|| {
                        let owner = uid_allocator.retrieve_entity_internal(owner.id())?;

                        let owner_pos = positions.get(owner)?;
                        let dist_sqrd = pos.0.distance_squared(owner_pos.0);
                        if dist_sqrd > MAX_FOLLOW_DIST.powi(2) && !agent.activity.is_follow() {
                            agent.activity = Activity::Follow {
                                target: owner,
                                chaser: Chaser::default(),
                            };
                        }

                        // Attack owner's attacker
                        let owner_health = healths.get(owner)?;
                        if owner_health.last_change.0 < 5.0 && owner_health.last_change.1.amount < 0 {
                            if let comp::HealthSource::Damage { by: Some(by), .. } =
                                owner_health.last_change.1.cause
                            {
                                if !agent.activity.is_attack() {
                                    let attacker = uid_allocator.retrieve_entity_internal(by.id())?;

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

                        Some(())
                    })();
                }

                debug_assert!(inputs.move_dir.map(|e| !e.is_nan()).reduce_and());
                debug_assert!(inputs.look_dir.map(|e| !e.is_nan()).reduce_and());
            });

        // Process group invites
        for (_invite, /*alignment,*/ agent, controller) in
            (&invites, /*&alignments,*/ &mut agents, &mut controllers).join()
        {
            let accept = false; // set back to "matches!(alignment, Alignment::Npc)" when we got better NPC recruitment mechanics
            if accept {
                // Clear agent comp
                *agent = Agent::default();
                controller
                    .events
                    .push(ControlEvent::GroupManip(GroupManip::Accept));
            } else {
                controller
                    .events
                    .push(ControlEvent::GroupManip(GroupManip::Decline));
            }
        }
        sys_metrics.agent_ns.store(
            start_time.elapsed().as_nanos() as u64,
            std::sync::atomic::Ordering::Relaxed,
        );
    }
}

fn can_see_tgt(terrain: &TerrainGrid, pos: &Pos, tgt_pos: &Pos, dist_sqrd: f32) -> bool {
    terrain
        .ray(pos.0 + Vec3::unit_z(), tgt_pos.0 + Vec3::unit_z())
        .until(Block::is_opaque)
        .cast()
        .0
        .powi(2)
        >= dist_sqrd
}
