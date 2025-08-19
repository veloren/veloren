use crate::{
    consts::{
        AVG_FOLLOW_DIST, DEFAULT_ATTACK_RANGE, IDLE_HEALING_ITEM_THRESHOLD, MAX_PATROL_DIST,
        SEPARATION_BIAS, SEPARATION_DIST, STD_AWARENESS_DECAY_RATE,
    },
    data::{AgentData, AgentEmitters, AttackData, Path, ReadData, Tactic, TargetData},
    util::{
        aim_projectile, are_our_owners_hostile, entities_have_line_of_sight, get_attacker,
        get_entity_by_id, is_dead_or_invulnerable, is_dressed_as_cultist, is_dressed_as_pirate,
        is_dressed_as_witch, is_invulnerable, is_steering, is_village_guard, is_villager,
    },
};
use common::{
    combat::perception_dist_multiplier_from_stealth,
    comp::{
        self, Agent, Alignment, Body, CharacterState, Content, ControlAction, ControlEvent,
        Controller, HealthChange, InputKind, InventoryAction, Pos, PresenceKind, Scale,
        UnresolvedChatMsg, UtteranceKind,
        ability::BASE_ABILITY_LIMIT,
        agent::{FlightMode, PidControllers, Sound, SoundKind, Target},
        biped_large, body,
        inventory::slot::EquipSlot,
        item::{
            ConsumableKind, Effects, Item, ItemDesc, ItemKind,
            tool::{AbilitySpec, ToolKind},
        },
        projectile::ProjectileConstructorKind,
    },
    consts::MAX_MOUNT_RANGE,
    effect::{BuffEffect, Effect},
    event::{ChatEvent, EmitExt, SoundEvent},
    interaction::InteractionKind,
    match_some,
    mounting::VolumePos,
    path::TraversalConfig,
    rtsim::NpcActivity,
    states::basic_beam,
    terrain::Block,
    time::DayPeriod,
    util::Dir,
    vol::ReadVol,
};
use itertools::Itertools;
use rand::{Rng, rng};
use specs::Entity as EcsEntity;
use vek::*;

#[cfg(feature = "use-dyn-lib")]
use {crate::LIB, std::ffi::CStr};

impl AgentData<'_> {
    ////////////////////////////////////////
    // Action Nodes
    ////////////////////////////////////////
    pub fn glider_equip(&self, controller: &mut Controller, read_data: &ReadData) {
        self.dismount(controller, read_data);
        controller.push_action(ControlAction::GlideWield);
    }

    // TODO: add the ability to follow the target?
    pub fn glider_flight(&self, controller: &mut Controller, _read_data: &ReadData) {
        let Some(fluid) = self.physics_state.in_fluid else {
            return;
        };

        let vel = self.vel;

        let comp::Vel(rel_flow) = fluid.relative_flow(vel);

        let is_wind_downwards = rel_flow.z.is_sign_negative();

        let look_dir = if is_wind_downwards {
            Vec3::from(-rel_flow.xy())
        } else {
            -rel_flow
        };

        controller.inputs.look_dir = Dir::from_unnormalized(look_dir).unwrap_or_else(Dir::forward);
    }

    pub fn fly_upward(&self, controller: &mut Controller, read_data: &ReadData) {
        self.dismount(controller, read_data);

        controller.push_basic_input(InputKind::Fly);
        controller.inputs.move_z = 1.0;
    }

    /// Directs the entity to path and move toward the target
    /// If path is not Full, the entity will path to a location 50 units along
    /// the vector between the entity and the target. The speed multiplier
    /// multiplies the movement speed by a value less than 1.0.
    /// A `None` value implies a multiplier of 1.0.
    /// Returns `false` if the pathfinding algorithm fails to return a path
    pub fn path_toward_target(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        tgt_pos: Vec3<f32>,
        read_data: &ReadData,
        path: Path,
        speed_multiplier: Option<f32>,
    ) -> Option<Vec3<f32>> {
        self.dismount_uncontrollable(controller, read_data);

        let pos_difference = tgt_pos - self.pos.0;
        let pathing_pos = match path {
            Path::Separate => {
                let mut sep_vec: Vec3<f32> = Vec3::zero();

                for entity in read_data
                    .cached_spatial_grid
                    .0
                    .in_circle_aabr(self.pos.0.xy(), SEPARATION_DIST)
                {
                    if let (Some(alignment), Some(other_alignment)) =
                        (self.alignment, read_data.alignments.get(entity))
                        && Alignment::passive_towards(*alignment, *other_alignment)
                        && let (Some(pos), Some(body), Some(other_body)) = (
                            read_data.positions.get(entity),
                            self.body,
                            read_data.bodies.get(entity),
                        )
                    {
                        let dist_xy = self.pos.0.xy().distance(pos.0.xy());
                        let spacing = body.spacing_radius() + other_body.spacing_radius();
                        if dist_xy < spacing {
                            let pos_diff = self.pos.0.xy() - pos.0.xy();
                            sep_vec += pos_diff.try_normalized().unwrap_or_else(Vec2::zero)
                                * ((spacing - dist_xy) / spacing);
                        }
                    }
                }

                tgt_pos + sep_vec * SEPARATION_BIAS + pos_difference * (1.0 - SEPARATION_BIAS)
            },
            Path::AtTarget => tgt_pos,
        };
        let speed_multiplier = speed_multiplier.unwrap_or(1.0).min(1.0);

        let in_loaded_chunk = |pos: Vec3<f32>| {
            read_data
                .terrain
                .contains_key(read_data.terrain.pos_key(pos.map(|e| e.floor() as i32)))
        };

        // If current position lies inside a loaded chunk, we need to plan routes using
        // voxel info. If target happens to be in an unloaded chunk,
        // we need to make our way to the current chunk border, and
        // then reroute if needed.
        let is_target_loaded = in_loaded_chunk(pathing_pos);

        if let Some((bearing, speed, stuck)) = agent.chaser.chase(
            &*read_data.terrain,
            self.pos.0,
            self.vel.0,
            pathing_pos,
            TraversalConfig {
                min_tgt_dist: 0.25,
                is_target_loaded,
                ..self.traversal_config
            },
            &read_data.time,
        ) {
            self.unstuck_if(stuck, controller);
            self.traverse(controller, bearing, speed * speed_multiplier);
            Some(bearing)
        } else {
            None
        }
    }

    fn traverse(&self, controller: &mut Controller, bearing: Vec3<f32>, speed: f32) {
        controller.inputs.move_dir =
            bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;

        // Only jump if we are grounded and can't blockhop or if we can fly
        self.jump_if(
            (self.physics_state.on_ground.is_some() && bearing.z > 1.5)
                || self.traversal_config.can_fly,
            controller,
        );
        controller.inputs.move_z = bearing.z;
    }

    pub fn unstuck_if(&self, condition: bool, controller: &mut Controller) {
        if condition && rng().random_bool(0.05) {
            if matches!(self.char_state, CharacterState::Climb(_)) || rng().random_bool(0.5) {
                controller.push_basic_input(InputKind::Jump);
            } else {
                controller.push_basic_input(InputKind::Roll);
            }
        } else {
            if controller.queued_inputs.contains_key(&InputKind::Jump) {
                controller.push_cancel_input(InputKind::Jump);
            }
            if controller.queued_inputs.contains_key(&InputKind::Roll) {
                controller.push_cancel_input(InputKind::Roll);
            }
        }
    }

    pub fn jump_if(&self, condition: bool, controller: &mut Controller) {
        if condition {
            controller.push_basic_input(InputKind::Jump);
        } else if controller.queued_inputs.contains_key(&InputKind::Jump) {
            controller.push_cancel_input(InputKind::Jump)
        }
    }

    pub fn idle(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        read_data: &ReadData,
        _emitters: &mut AgentEmitters,
        rng: &mut impl Rng,
    ) {
        enum ActionTimers {
            TimerIdle = 0,
        }

        agent
            .awareness
            .change_by(STD_AWARENESS_DECAY_RATE * read_data.dt.0);

        // Light lanterns at night
        // TODO Add a method to turn on NPC lanterns underground
        let lantern_equipped = self
            .inventory
            .equipped(EquipSlot::Lantern)
            .as_ref()
            .is_some_and(|item| matches!(&*item.kind(), comp::item::ItemKind::Lantern(_)));
        let lantern_turned_on = self.light_emitter.is_some();
        let day_period = DayPeriod::from(read_data.time_of_day.0);
        // Only emit event for agents that have a lantern equipped
        if lantern_equipped && rng.random_bool(0.001) {
            if day_period.is_dark() && !lantern_turned_on {
                // Agents with turned off lanterns turn them on randomly once it's
                // nighttime and keep them on.
                // Only emit event for agents that sill need to
                // turn on their lantern.
                controller.push_event(ControlEvent::EnableLantern)
            } else if lantern_turned_on && day_period.is_light() {
                // agents with turned on lanterns turn them off randomly once it's
                // daytime and keep them off.
                controller.push_event(ControlEvent::DisableLantern)
            }
        };

        if let Some(body) = self.body {
            let attempt_heal = if matches!(body, Body::Humanoid(_)) {
                self.damage < IDLE_HEALING_ITEM_THRESHOLD
            } else {
                true
            };
            if attempt_heal && self.heal_self(agent, controller, true) {
                agent.behavior_state.timers[ActionTimers::TimerIdle as usize] = 0.01;
                return;
            }
        } else {
            agent.behavior_state.timers[ActionTimers::TimerIdle as usize] = 0.01;
            return;
        }

        agent.behavior_state.timers[ActionTimers::TimerIdle as usize] = 0.0;

        'activity: {
            match agent.rtsim_controller.activity {
                Some(NpcActivity::Goto(travel_to, speed_factor)) => {
                    self.dismount_uncontrollable(controller, read_data);

                    agent.bearing = Vec2::zero();

                    // If it has an rtsim destination and can fly, then it should.
                    // If it is flying and bumps something above it, then it should move down.
                    if self.traversal_config.can_fly
                        && !read_data
                            .terrain
                            .ray(self.pos.0, self.pos.0 + (Vec3::unit_z() * 3.0))
                            .until(Block::is_solid)
                            .cast()
                            .1
                            .map_or(true, |b| b.is_some())
                    {
                        controller.push_basic_input(InputKind::Fly);
                    } else {
                        controller.push_cancel_input(InputKind::Fly)
                    }

                    if let Some(bearing) = self.path_toward_target(
                        agent,
                        controller,
                        travel_to,
                        read_data,
                        Path::AtTarget,
                        Some(speed_factor),
                    ) {
                        let height_offset = bearing.z
                            + if self.traversal_config.can_fly {
                                // NOTE: costs 4 us (imbris)
                                let obstacle_ahead = read_data
                                    .terrain
                                    .ray(
                                        self.pos.0 + Vec3::unit_z(),
                                        self.pos.0
                                            + bearing.try_normalized().unwrap_or_else(Vec3::unit_y)
                                                * 80.0
                                            + Vec3::unit_z(),
                                    )
                                    .until(Block::is_solid)
                                    .cast()
                                    .1
                                    .map_or(true, |b| b.is_some());

                                let mut ground_too_close = self
                                    .body
                                    .map(|body| {
                                        #[cfg(feature = "worldgen")]
                                        let height_approx = self.pos.0.z
                                            - read_data
                                                .world
                                                .sim()
                                                .get_alt_approx(
                                                    self.pos.0.xy().map(|x: f32| x as i32),
                                                )
                                                .unwrap_or(0.0);
                                        #[cfg(not(feature = "worldgen"))]
                                        let height_approx = self.pos.0.z;

                                        height_approx < body.flying_height()
                                    })
                                    .unwrap_or(false);

                                const NUM_RAYS: usize = 5;

                                // NOTE: costs 15-20 us (imbris)
                                for i in 0..=NUM_RAYS {
                                    let magnitude = self.body.map_or(20.0, |b| b.flying_height());
                                    // Lerp between a line straight ahead and straight down to
                                    // detect a
                                    // wedge of obstacles we might fly into (inclusive so that both
                                    // vectors are sampled)
                                    if let Some(dir) = Lerp::lerp(
                                        -Vec3::unit_z(),
                                        Vec3::new(bearing.x, bearing.y, 0.0),
                                        i as f32 / NUM_RAYS as f32,
                                    )
                                    .try_normalized()
                                    {
                                        ground_too_close |= read_data
                                            .terrain
                                            .ray(self.pos.0, self.pos.0 + magnitude * dir)
                                            .until(|b: &Block| b.is_solid() || b.is_liquid())
                                            .cast()
                                            .1
                                            .is_ok_and(|b| b.is_some())
                                    }
                                }

                                if obstacle_ahead || ground_too_close {
                                    5.0 //fly up when approaching obstacles
                                } else {
                                    -2.0
                                } //flying things should slowly come down from the stratosphere
                            } else {
                                0.05 //normal land traveller offset
                            };

                        if let Some(mpid) = agent.multi_pid_controllers.as_mut() {
                            if let Some(z_controller) = mpid.z_controller.as_mut() {
                                z_controller.sp = self.pos.0.z + height_offset;
                                controller.inputs.move_z = z_controller.calc_err();
                                // when changing setpoints, limit PID windup
                                z_controller.limit_integral_windup(|z| *z = z.clamp(-10.0, 10.0));
                            } else {
                                controller.inputs.move_z = 0.0;
                            }
                        } else {
                            controller.inputs.move_z = height_offset;
                        }
                    }

                    // Put away weapon
                    if rng.random_bool(0.1)
                        && matches!(
                            read_data.char_states.get(*self.entity),
                            Some(CharacterState::Wielding(_))
                        )
                    {
                        controller.push_action(ControlAction::Unwield);
                    }
                    break 'activity; // Don't fall through to idle wandering
                },

                Some(NpcActivity::GotoFlying(
                    travel_to,
                    speed_factor,
                    height_offset,
                    direction_override,
                    flight_mode,
                )) => {
                    self.dismount_uncontrollable(controller, read_data);

                    if self.traversal_config.vectored_propulsion {
                        // This is the action for Airships.

                        // Note - when the Agent code is run, the entity will be the captain that is
                        // mounted on the ship and the movement calculations
                        // must be done relative to the captain's position
                        // which is offset from the ship's position and apparently scaled.
                        // When the State system runs to apply the movement accel and velocity, the
                        // ship entity will be the subject entity.

                        // entities that have vectored propulsion should always be flying
                        // and do not depend on forward movement or displacement to move.
                        // E.g., Airships.
                        controller.push_basic_input(InputKind::Fly);

                        // These entities can either:
                        // - Move in any direction, following the terrain
                        // - Move essentially vertically, as in
                        //   - Hover in place (station-keeping), like at a dock
                        //   - Move straight up or down, as when taking off or landing

                        // If there is lateral movement, then the entity's direction should be
                        // aligned with that movement direction. If there is
                        // no or minimal lateral movement, then the entity
                        // is either hovering or moving vertically, and the entity's direction
                        // should not change. This is indicated by the direction_override parameter.

                        // If a direction override is provided, attempt to orient the entity in that
                        // direction.
                        if let Some(direction) = direction_override {
                            controller.inputs.look_dir = direction;
                        } else {
                            // else orient the entity in the direction of travel, but keep it level
                            controller.inputs.look_dir =
                                Dir::from_unnormalized((travel_to - self.pos.0).xy().with_z(0.0))
                                    .unwrap_or_default();
                        }

                        // the look_dir will be used as the orientation override. Orientation
                        // override is always enabled for airships, so this
                        // code must set controller.inputs.look_dir for
                        // all cases (vertical or lateral movement).

                        // When pid_mode is PureZ, only the z component of movement is is adjusted
                        // by the PID controller.

                        // If the PID controller is not set or the mode or gain has changed, create
                        // a new one. PidControllers is a wrapper around one
                        // or more PID controllers. Each controller acts on
                        // one axis of movement. There are three controllers for FixedDirection mode
                        // and one for PureZ mode.
                        if agent
                            .multi_pid_controllers
                            .as_ref()
                            .is_some_and(|mpid| mpid.mode != flight_mode)
                        {
                            agent.multi_pid_controllers = None;
                        }
                        let mpid = agent.multi_pid_controllers.get_or_insert_with(|| {
                            PidControllers::<16>::new_multi_pid_controllers(flight_mode, travel_to)
                        });
                        let sample_time = read_data.time.0;

                        #[allow(unused_variables)]
                        let terrain_alt_with_lookahead = |dist: f32| -> f32 {
                            // look ahead some blocks to sample the terrain altitude
                            #[cfg(feature = "worldgen")]
                            let terrain_alt = read_data
                                .world
                                .sim()
                                .get_alt_approx(
                                    (self.pos.0.xy()
                                        + controller.inputs.look_dir.to_vec().xy() * dist)
                                        .map(|x: f32| x as i32),
                                )
                                .unwrap_or(0.0);
                            #[cfg(not(feature = "worldgen"))]
                            let terrain_alt = 0.0;
                            terrain_alt
                        };

                        if flight_mode == FlightMode::FlyThrough {
                            let travel_vec = travel_to - self.pos.0;
                            let bearing =
                                travel_vec.xy().try_normalized().unwrap_or_else(Vec2::zero);
                            controller.inputs.move_dir = bearing * speed_factor;
                            let terrain_alt = terrain_alt_with_lookahead(32.0);
                            let height = height_offset.unwrap_or(100.0);
                            if let Some(z_controller) = mpid.z_controller.as_mut() {
                                z_controller.sp = terrain_alt + height;
                            }
                            mpid.add_measurement(sample_time, self.pos.0);
                            // check if getting close to terrain
                            if terrain_alt >= self.pos.0.z - 32.0 {
                                // It's likely the airship will hit an upslope. Maximize the climb
                                // rate.
                                controller.inputs.move_z = 1.0 * speed_factor;
                                // try to stop forward movement
                                controller.inputs.move_dir =
                                    self.vel.0.xy().try_normalized().unwrap_or_else(Vec2::zero)
                                        * -1.0
                                        * speed_factor;
                            } else {
                                controller.inputs.move_z =
                                    mpid.calc_err_z().unwrap_or(0.0).min(1.0) * speed_factor;
                            }
                            // PID controllers that change the setpoint suffer from "windup", where
                            // the integral term accumulates error.
                            // There are several ways to compensate for this. One way is to limit
                            // the integral term to a range.
                            mpid.limit_windup_z(|z| *z = z.clamp(-20.0, 20.0));
                        } else {
                            // When doing step-wise movement, the target waypoint changes. Make sure
                            // the PID controller setpoints keep up with
                            // the changes.
                            if let Some(x_controller) = mpid.x_controller.as_mut() {
                                x_controller.sp = travel_to.x;
                            }
                            if let Some(y_controller) = mpid.y_controller.as_mut() {
                                y_controller.sp = travel_to.y;
                            }

                            // If terrain following, get the terrain altitude at the current
                            // position. Set the z setpoint to the max
                            // of terrain alt + height offset or the
                            // target z.
                            let z_setpoint = if let Some(height) = height_offset {
                                let clearance_alt = terrain_alt_with_lookahead(16.0) + height;
                                clearance_alt.max(travel_to.z)
                            } else {
                                travel_to.z
                            };
                            if let Some(z_controller) = mpid.z_controller.as_mut() {
                                z_controller.sp = z_setpoint;
                            }

                            mpid.add_measurement(sample_time, self.pos.0);
                            controller.inputs.move_dir.x =
                                mpid.calc_err_x().unwrap_or(0.0).min(1.0) * speed_factor;
                            controller.inputs.move_dir.y =
                                mpid.calc_err_y().unwrap_or(0.0).min(1.0) * speed_factor;
                            controller.inputs.move_z =
                                mpid.calc_err_z().unwrap_or(0.0).min(1.0) * speed_factor;

                            // Limit the integral term to a range to prevent windup.
                            mpid.limit_windup_x(|x| *x = x.clamp(-1.0, 1.0));
                            mpid.limit_windup_y(|y| *y = y.clamp(-1.0, 1.0));
                            mpid.limit_windup_z(|z| *z = z.clamp(-1.0, 1.0));
                        }
                    }
                    break 'activity; // Don't fall through to idle wandering
                },
                Some(NpcActivity::Gather(_resources)) => {
                    // TODO: Implement
                    controller.push_action(ControlAction::Dance);
                    break 'activity; // Don't fall through to idle wandering
                },
                Some(NpcActivity::Dance(dir)) => {
                    // Look at targets specified by rtsim
                    if let Some(look_dir) = dir {
                        controller.inputs.look_dir = look_dir;
                        if self.ori.look_dir().dot(look_dir.to_vec()) < 0.95 {
                            controller.inputs.move_dir = look_dir.to_vec().xy() * 0.01;
                            break 'activity;
                        } else {
                            controller.inputs.move_dir = Vec2::zero();
                        }
                    }
                    controller.push_action(ControlAction::Dance);
                    break 'activity; // Don't fall through to idle wandering
                },
                Some(NpcActivity::Cheer(dir)) => {
                    if let Some(look_dir) = dir {
                        controller.inputs.look_dir = look_dir;
                        if self.ori.look_dir().dot(look_dir.to_vec()) < 0.95 {
                            controller.inputs.move_dir = look_dir.to_vec().xy() * 0.01;
                            break 'activity;
                        } else {
                            controller.inputs.move_dir = Vec2::zero();
                        }
                    }
                    controller.push_action(ControlAction::Talk(None));
                    break 'activity; // Don't fall through to idle wandering
                },
                Some(NpcActivity::Sit(dir, pos)) => {
                    if let Some(pos) =
                        pos.filter(|p| read_data.terrain.get(*p).is_ok_and(|b| b.is_mountable()))
                    {
                        if !read_data.is_volume_riders.contains(*self.entity) {
                            controller
                                .push_event(ControlEvent::MountVolume(VolumePos::terrain(pos)));
                        }
                    } else {
                        if let Some(look_dir) = dir {
                            controller.inputs.look_dir = look_dir;
                            if self.ori.look_dir().dot(look_dir.to_vec()) < 0.95 {
                                controller.inputs.move_dir = look_dir.to_vec().xy() * 0.01;
                                break 'activity;
                            } else {
                                controller.inputs.move_dir = Vec2::zero();
                            }
                        }
                        controller.push_action(ControlAction::Sit);
                    }
                    break 'activity; // Don't fall through to idle wandering
                },
                Some(NpcActivity::HuntAnimals) => {
                    if rng.random::<f32>() < 0.1 {
                        self.choose_target(
                            agent,
                            controller,
                            read_data,
                            AgentData::is_hunting_animal,
                        );
                    }
                },
                Some(NpcActivity::Talk(target)) => {
                    if agent.target.is_none()
                        && let Some(target) = read_data.id_maps.actor_entity(target)
                        && let Some(target_uid) = read_data.uids.get(target)
                    {
                        // We're always aware of someone we're talking to
                        controller.push_action(ControlAction::Stand);
                        self.look_toward(controller, read_data, target);
                        controller.push_action(ControlAction::Talk(Some(*target_uid)));
                        break 'activity;
                    }
                },
                None => {},
            }

            let owner_uid = self
                .alignment
                .and_then(|alignment| match_some!(alignment, Alignment::Owned(uid) => uid));

            let owner = owner_uid.and_then(|owner_uid| get_entity_by_id(*owner_uid, read_data));

            let is_being_pet = read_data
                .interactors
                .get(*self.entity)
                .and_then(|interactors| interactors.get(*owner_uid?))
                .is_some_and(|interaction| matches!(interaction.kind, InteractionKind::Pet));

            let is_in_range = owner
                .and_then(|owner| read_data.positions.get(owner))
                .is_some_and(|pos| pos.0.distance_squared(self.pos.0) < MAX_MOUNT_RANGE.powi(2));

            // Idle NPCs should try to jump on the shoulders of their owner, sometimes.
            if read_data.is_riders.contains(*self.entity) {
                if rng.random_bool(0.0001) {
                    self.dismount_uncontrollable(controller, read_data);
                } else {
                    break 'activity;
                }
            } else if let Some(owner_uid) = owner_uid
                && is_in_range
                && !is_being_pet
                && rng.random_bool(0.01)
            {
                controller.push_event(ControlEvent::Mount(*owner_uid));
                break 'activity;
            }

            // Bats should fly
            // Use a proportional controller as the bouncing effect mimics bat flight
            if self.traversal_config.can_fly
                && self
                    .inventory
                    .equipped(EquipSlot::ActiveMainhand)
                    .as_ref()
                    .is_some_and(|item| {
                        item.ability_spec().is_some_and(|a_s| match &*a_s {
                            AbilitySpec::Custom(spec) => {
                                matches!(
                                    spec.as_str(),
                                    "Simple Flying Melee"
                                        | "Bloodmoon Bat"
                                        | "Vampire Bat"
                                        | "Flame Wyvern"
                                        | "Frost Wyvern"
                                        | "Cloud Wyvern"
                                        | "Sea Wyvern"
                                        | "Weald Wyvern"
                                )
                            },
                            _ => false,
                        })
                    })
            {
                // Bats don't like the ground, so make sure they are always flying
                controller.push_basic_input(InputKind::Fly);
                // Use a proportional controller with a coefficient of 1.0 to
                // maintain altitude
                let alt = read_data
                    .terrain
                    .ray(self.pos.0, self.pos.0 - (Vec3::unit_z() * 7.0))
                    .until(Block::is_solid)
                    .cast()
                    .0;
                let set_point = 5.0;
                let error = set_point - alt;
                controller.inputs.move_z = error;
                // If on the ground, jump
                if self.physics_state.on_ground.is_some() {
                    controller.push_basic_input(InputKind::Jump);
                }
            }

            let diff = Vec2::new(rng.random::<f32>() - 0.5, rng.random::<f32>() - 0.5);
            agent.bearing += (diff * 0.1 - agent.bearing * 0.01)
                * agent.psyche.idle_wander_factor.max(0.0).sqrt()
                * agent.psyche.aggro_range_multiplier.max(0.0).sqrt();
            if let Some(patrol_origin) = agent.patrol_origin
                // Use owner as patrol origin otherwise
                .or_else(|| if let Some(Alignment::Owned(owner_uid)) = self.alignment
                    && let Some(owner) = get_entity_by_id(*owner_uid, read_data)
                    && let Some(pos) = read_data.positions.get(owner)
                {
                    Some(pos.0)
                } else {
                    None
                })
            {
                agent.bearing += ((patrol_origin.xy() - self.pos.0.xy())
                    / (0.01 + MAX_PATROL_DIST * agent.psyche.idle_wander_factor))
                    * 0.015
                    * agent.psyche.idle_wander_factor;
            }

            // Stop if we're too close to a wall
            // or about to walk off a cliff
            // NOTE: costs 1 us (imbris) <- before cliff raycast added
            agent.bearing *= 0.1
                + if read_data
                    .terrain
                    .ray(
                        self.pos.0 + Vec3::unit_z(),
                        self.pos.0
                            + Vec3::from(agent.bearing)
                                .try_normalized()
                                .unwrap_or_else(Vec3::unit_y)
                                * 5.0
                            + Vec3::unit_z(),
                    )
                    .until(Block::is_solid)
                    .cast()
                    .1
                    .map_or(true, |b| b.is_none())
                    && read_data
                        .terrain
                        .ray(
                            self.pos.0
                                + Vec3::from(agent.bearing)
                                    .try_normalized()
                                    .unwrap_or_else(Vec3::unit_y),
                            self.pos.0
                                + Vec3::from(agent.bearing)
                                    .try_normalized()
                                    .unwrap_or_else(Vec3::unit_y)
                                - Vec3::unit_z() * 4.0,
                        )
                        .until(Block::is_solid)
                        .cast()
                        .0
                        < 3.0
                {
                    0.9
                } else {
                    0.0
                };

            if agent.bearing.magnitude_squared() > 0.5f32.powi(2) {
                controller.inputs.move_dir = agent.bearing;
            }

            // Put away weapon
            if rng.random_bool(0.1)
                && matches!(
                    read_data.char_states.get(*self.entity),
                    Some(CharacterState::Wielding(_))
                )
            {
                controller.push_action(ControlAction::Unwield);
            }

            if rng.random::<f32>() < 0.0015 {
                controller.push_utterance(UtteranceKind::Calm);
            }

            // Sit
            if rng.random::<f32>() < 0.0035 {
                controller.push_action(ControlAction::Sit);
            }
        }
    }

    pub fn follow(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        read_data: &ReadData,
        tgt_pos: &Pos,
    ) {
        self.dismount_uncontrollable(controller, read_data);

        if let Some((bearing, speed, stuck)) = agent.chaser.chase(
            &*read_data.terrain,
            self.pos.0,
            self.vel.0,
            tgt_pos.0,
            TraversalConfig {
                min_tgt_dist: AVG_FOLLOW_DIST,
                ..self.traversal_config
            },
            &read_data.time,
        ) {
            self.unstuck_if(stuck, controller);
            let dist_sqrd = self.pos.0.distance_squared(tgt_pos.0);
            self.traverse(
                controller,
                bearing,
                speed.min(0.2 + (dist_sqrd - AVG_FOLLOW_DIST.powi(2)) / 8.0),
            );
        }
    }

    pub fn look_toward(
        &self,
        controller: &mut Controller,
        read_data: &ReadData,
        target: EcsEntity,
    ) -> bool {
        if let Some(tgt_pos) = read_data.positions.get(target)
            && !is_steering(*self.entity, read_data)
            && let Some(dir) = Dir::look_toward(
                self.pos,
                self.body,
                Some(&comp::Scale(self.scale)),
                tgt_pos,
                read_data.bodies.get(target),
                read_data.scales.get(target),
            )
        {
            controller.inputs.look_dir = dir;
            true
        } else {
            false
        }
    }

    pub fn flee(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        read_data: &ReadData,
        tgt_pos: &Pos,
    ) {
        // Proportion of full speed
        const MAX_FLEE_SPEED: f32 = 0.65;

        self.dismount_uncontrollable(controller, read_data);

        if let Some(body) = self.body
            && body.can_strafe()
            && !self.is_gliding
        {
            controller.push_action(ControlAction::Unwield);
        }

        if let Some((bearing, speed, stuck)) = agent.chaser.chase(
            &*read_data.terrain,
            self.pos.0,
            self.vel.0,
            // Away from the target (ironically)
            self.pos.0
                + (self.pos.0 - tgt_pos.0)
                    .try_normalized()
                    .unwrap_or_else(Vec3::unit_y)
                    * 50.0,
            TraversalConfig {
                min_tgt_dist: 1.25,
                ..self.traversal_config
            },
            &read_data.time,
        ) {
            self.unstuck_if(stuck, controller);
            self.traverse(controller, bearing, speed.min(MAX_FLEE_SPEED));
        }
    }

    /// Attempt to consume a healing item, and return whether any healing items
    /// were queued. Callers should use this to implement a delay so that
    /// the healing isn't interrupted. If `relaxed` is `true`, we allow eating
    /// food and prioritise healing.
    pub fn heal_self(
        &self,
        _agent: &mut Agent,
        controller: &mut Controller,
        relaxed: bool,
    ) -> bool {
        // If we already have a healing buff active, don't start another one.
        if self.buffs.is_some_and(|buffs| {
            buffs.iter_active().flatten().any(|buff| {
                buff.kind.effects(&buff.data, None).iter().any(|effect| {
                    if let comp::BuffEffect::HealthChangeOverTime { rate, .. } = effect
                        && *rate > 0.0
                    {
                        true
                    } else {
                        false
                    }
                })
            })
        }) {
            return false;
        }

        // Wait for potion sickness to wear off if potions are less than 50% effective.
        let heal_multiplier = self.stats.map_or(1.0, |s| s.item_effect_reduction);
        if heal_multiplier < 0.5 {
            return false;
        }
        // (healing_value, heal_reduction)
        let effect_healing_value = |effect: &Effect| -> (f32, f32) {
            let mut value = 0.0;
            let mut heal_reduction = 0.0;
            match effect {
                Effect::Health(HealthChange { amount, .. }) => {
                    value += *amount;
                },
                Effect::Buff(BuffEffect { kind, data, .. }) => {
                    if let Some(duration) = data.duration {
                        // We don't care about seeing the optional combat requirements that can be
                        // tacked onto buff effects, so we'll just pass in None to this
                        for effect in kind.effects(data, None) {
                            match effect {
                                comp::BuffEffect::HealthChangeOverTime { rate, kind, .. } => {
                                    let amount = match kind {
                                        comp::ModifierKind::Additive => rate * duration.0 as f32,
                                        comp::ModifierKind::Multiplicative => {
                                            (1.0 + rate).powf(duration.0 as f32)
                                        },
                                    };

                                    value += amount;
                                },
                                comp::BuffEffect::ItemEffectReduction(amount) => {
                                    heal_reduction =
                                        heal_reduction + amount - heal_reduction * amount;
                                },
                                _ => {},
                            }
                        }
                        value += data.strength * data.duration.map_or(0.0, |d| d.0 as f32);
                    }
                },

                _ => {},
            }

            (value, heal_reduction)
        };
        let healing_value = |item: &Item| {
            let mut value = 0.0;
            let mut heal_multiplier_value = 1.0;

            if let ItemKind::Consumable { kind, effects, .. } = &*item.kind()
                && (matches!(kind, ConsumableKind::Drink)
                    || (relaxed && matches!(kind, ConsumableKind::Food)))
            {
                match effects {
                    Effects::Any(effects) => {
                        // Add the average of all effects.
                        for effect in effects.iter() {
                            let (add, red) = effect_healing_value(effect);
                            value += add / effects.len() as f32;
                            heal_multiplier_value *= 1.0 - red / effects.len() as f32;
                        }
                    },
                    Effects::All(_) | Effects::One(_) => {
                        for effect in effects.effects() {
                            let (add, red) = effect_healing_value(effect);
                            value += add;
                            heal_multiplier_value *= 1.0 - red;
                        }
                    },
                }
            }
            // Prefer non-potion sources of healing when under at least one stack of potion
            // sickness, or when incurring potion sickness is unnecessary
            if heal_multiplier_value < 1.0 && (heal_multiplier < 1.0 || relaxed) {
                value *= 0.1;
            }
            value as i32
        };

        let item = self
            .inventory
            .slots_with_id()
            .filter_map(|(id, slot)| match slot {
                Some(item) if healing_value(item) > 0 => Some((id, item)),
                _ => None,
            })
            .max_by_key(|(_, item)| {
                if relaxed {
                    -healing_value(item)
                } else {
                    healing_value(item)
                }
            });

        if let Some((id, _)) = item {
            use comp::inventory::slot::Slot;
            controller.push_action(ControlAction::InventoryAction(InventoryAction::Use(
                Slot::Inventory(id),
            )));
            true
        } else {
            false
        }
    }

    pub fn choose_target(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        read_data: &ReadData,
        is_enemy: fn(&Self, EcsEntity, &ReadData) -> bool,
    ) {
        enum ActionStateTimers {
            TimerChooseTarget = 0,
        }
        agent.behavior_state.timers[ActionStateTimers::TimerChooseTarget as usize] = 0.0;
        let mut aggro_on = false;

        // Search the area.
        // TODO: choose target by more than just distance
        let common::CachedSpatialGrid(grid) = self.cached_spatial_grid;

        let entities_nearby = grid
            .in_circle_aabr(self.pos.0.xy(), agent.psyche.search_dist())
            .collect_vec();

        let get_pos = |entity| read_data.positions.get(entity);
        let get_enemy = |(entity, attack_target): (EcsEntity, bool)| {
            if attack_target {
                if is_enemy(self, entity, read_data) {
                    Some((entity, true))
                } else if self.should_defend(entity, read_data) {
                    if let Some(attacker) = get_attacker(entity, read_data) {
                        if !self.passive_towards(attacker, read_data) {
                            // aggro_on: attack immediately, do not warn/menace.
                            aggro_on = true;
                            Some((attacker, true))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                Some((entity, false))
            }
        };
        let is_valid_target = |entity: EcsEntity| match read_data.bodies.get(entity) {
            Some(Body::Item(item)) => {
                if !matches!(item, body::item::Body::Thrown(_)) {
                    let is_humanoid = matches!(self.body, Some(Body::Humanoid(_)));
                    let avoids_item_drops = matches!(
                        self.body,
                        Some(Body::BipedLarge(biped_large::Body {
                            species: biped_large::Species::Gigasfrost
                                | biped_large::Species::Gigasfire,
                            ..
                        }))
                    );
                    // If the agent is humanoid, it will pick up all kinds of item drops. If the
                    // agent isn't humanoid, it will pick up only consumable item drops.
                    let wants_pickup = !avoids_item_drops
                        && (is_humanoid || matches!(item, body::item::Body::Consumable));

                    // The agent will attempt to pickup the item if it wants to pick it up and
                    // is allowed to
                    let attempt_pickup = wants_pickup
                    && read_data
                        .loot_owners
                        .get(entity).is_none_or(|loot_owner| {
                            !(is_humanoid
                                && loot_owner.can_pickup(
                                    *self.uid,
                                    read_data.groups.get(entity),
                                    self.alignment,
                                    self.body,
                                    None,
                                )
                                && (
                                    !loot_owner.is_soft() ||
                                    // If we are hostile towards the owner, ignore their wish to not pick up the loot
                                    loot_owner
                                        .uid()
                                        .and_then(|uid| read_data.id_maps.uid_entity(uid)).is_none_or(|entity| !is_enemy(self, entity, read_data)))
                                )
                        });

                    if attempt_pickup {
                        Some((entity, false))
                    } else {
                        None
                    }
                } else {
                    None
                }
            },
            _ => {
                if read_data
                    .healths
                    .get(entity)
                    .is_some_and(|health| !health.is_dead && !is_invulnerable(entity, read_data))
                {
                    let needs_saving = comp::is_downed(
                        read_data.healths.get(entity),
                        read_data.char_states.get(entity),
                    );

                    let wants_to_save = match (self.alignment, read_data.alignments.get(entity)) {
                        // Npcs generally do want to save players. Could have extra checks for
                        // sentiment in the future.
                        (Some(Alignment::Npc), _) if read_data.presences.get(entity).is_some_and(|presence| matches!(presence.kind, PresenceKind::Character(_))) => true,
                        (Some(Alignment::Npc), Some(Alignment::Npc)) => true,
                        (Some(Alignment::Enemy), Some(Alignment::Enemy)) => true,
                        _ => false,
                    } && agent.allowed_to_speak()
                        // Check that anyone else isn't already saving them.
                        && read_data
                            .interactors
                            .get(entity).is_none_or(|interactors| {
                                !interactors.has_interaction(InteractionKind::HelpDowned)
                            }) && self.char_state.can_interact();

                    // TODO: Make targets that need saving have less priority as a target.
                    Some((entity, !(needs_saving && wants_to_save)))
                } else {
                    None
                }
            },
        };

        let is_detected = |entity: &EcsEntity, e_pos: &Pos, e_scale: Option<&Scale>| {
            self.detects_other(agent, controller, entity, e_pos, e_scale, read_data)
        };

        let target = entities_nearby
            .iter()
            .filter_map(|e| is_valid_target(*e))
            .filter_map(get_enemy)
            .filter_map(|(entity, attack_target)| {
                get_pos(entity).map(|pos| (entity, pos, attack_target))
            })
            .filter(|(entity, e_pos, _)| is_detected(entity, e_pos, read_data.scales.get(*entity)))
            .min_by_key(|(_, e_pos, attack_target)| {
                (
                    *attack_target,
                    (e_pos.0.distance_squared(self.pos.0) * 100.0) as i32,
                )
            })
            .map(|(entity, _, attack_target)| (entity, attack_target));

        if agent.target.is_none() && target.is_some() {
            if aggro_on {
                controller.push_utterance(UtteranceKind::Angry);
            } else {
                controller.push_utterance(UtteranceKind::Surprised);
            }
        }
        if agent.psyche.should_stop_pursuing || target.is_some() {
            agent.target = target.map(|(entity, attack_target)| Target {
                target: entity,
                hostile: attack_target,
                selected_at: read_data.time.0,
                aggro_on,
                last_known_pos: get_pos(entity).map(|pos| pos.0),
            })
        }
    }

    pub fn attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        tgt_data: &TargetData,
        read_data: &ReadData,
        rng: &mut impl Rng,
    ) {
        #[cfg(any(feature = "be-dyn-lib", feature = "use-dyn-lib"))]
        let _rng = rng;

        #[cfg(not(feature = "use-dyn-lib"))]
        {
            #[cfg(not(feature = "be-dyn-lib"))]
            self.attack_inner(agent, controller, tgt_data, read_data, rng);
            #[cfg(feature = "be-dyn-lib")]
            self.attack_inner(agent, controller, tgt_data, read_data);
        }
        #[cfg(feature = "use-dyn-lib")]
        {
            let lock = LIB.lock().unwrap();
            let lib = &lock.as_ref().unwrap().lib;
            const ATTACK_FN: &[u8] = b"attack_inner\0";

            let attack_fn: common_dynlib::Symbol<
                fn(&Self, &mut Agent, &mut Controller, &TargetData, &ReadData),
            > = unsafe { lib.get(ATTACK_FN) }.unwrap_or_else(|e| {
                panic!(
                    "Trying to use: {} but had error: {:?}",
                    CStr::from_bytes_with_nul(ATTACK_FN)
                        .map(CStr::to_str)
                        .unwrap()
                        .unwrap(),
                    e
                )
            });
            attack_fn(self, agent, controller, tgt_data, read_data);
        }
    }

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "attack_inner"))]
    pub fn attack_inner(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        tgt_data: &TargetData,
        read_data: &ReadData,
        #[cfg(not(feature = "be-dyn-lib"))] rng: &mut impl Rng,
    ) {
        #[cfg(feature = "be-dyn-lib")]
        let rng = &mut rng();

        self.dismount_uncontrollable(controller, read_data);

        let tool_tactic = |tool_kind| match tool_kind {
            ToolKind::Bow => Tactic::Bow,
            ToolKind::Staff => Tactic::Staff,
            ToolKind::Sceptre => Tactic::Sceptre,
            ToolKind::Hammer => Tactic::Hammer,
            ToolKind::Sword | ToolKind::Blowgun => Tactic::Sword,
            ToolKind::Axe => Tactic::Axe,
            _ => Tactic::SimpleMelee,
        };

        let tactic = self
            .inventory
            .equipped(EquipSlot::ActiveMainhand)
            .as_ref()
            .map(|item| {
                if let Some(ability_spec) = item.ability_spec() {
                    match &*ability_spec {
                        AbilitySpec::Custom(spec) => match spec.as_str() {
                            "Oni" | "Sword Simple" | "BipedLargeCultistSword" => {
                                Tactic::SwordSimple
                            },
                            "Staff Simple" | "BipedLargeCultistStaff" | "Ogre Staff" => {
                                Tactic::Staff
                            },
                            "BipedLargeCultistHammer" => Tactic::Hammer,
                            "Simple Flying Melee" => Tactic::SimpleFlyingMelee,
                            "Bow Simple" | "BipedLargeCultistBow" => Tactic::Bow,
                            "Stone Golem" | "Coral Golem" => Tactic::StoneGolem,
                            "Iron Golem" => Tactic::IronGolem,
                            "Quad Med Quick" => Tactic::CircleCharge {
                                radius: 5,
                                circle_time: 2,
                            },
                            "Quad Med Jump" | "Darkhound" => Tactic::QuadMedJump,
                            "Quad Med Charge" => Tactic::CircleCharge {
                                radius: 6,
                                circle_time: 1,
                            },
                            "Quad Med Basic" => Tactic::QuadMedBasic,
                            "Quad Med Hoof" => Tactic::QuadMedHoof,
                            "ClaySteed" => Tactic::ClaySteed,
                            "Elephant" => Tactic::Elephant,
                            "Rocksnapper" => Tactic::Rocksnapper,
                            "Roshwalr" => Tactic::Roshwalr,
                            "Asp" | "Maneater" => Tactic::QuadLowRanged,
                            "Quad Low Breathe" | "Quad Low Beam" | "Basilisk" => {
                                Tactic::QuadLowBeam
                            },
                            "Organ" => Tactic::OrganAura,
                            "Quad Low Tail" | "Husk Brute" => Tactic::TailSlap,
                            "Quad Low Quick" => Tactic::QuadLowQuick,
                            "Quad Low Basic" => Tactic::QuadLowBasic,
                            "Theropod Basic" | "Theropod Bird" | "Theropod Small" => {
                                Tactic::Theropod
                            },
                            // Arthropods
                            "Antlion" => Tactic::ArthropodMelee,
                            "Tarantula" | "Horn Beetle" => Tactic::ArthropodAmbush,
                            "Weevil" | "Black Widow" | "Crawler" => Tactic::ArthropodRanged,
                            "Theropod Charge" => Tactic::CircleCharge {
                                radius: 6,
                                circle_time: 1,
                            },
                            "Turret" => Tactic::RadialTurret,
                            "Flamethrower" => Tactic::RadialTurret,
                            "Haniwa Sentry" => Tactic::RotatingTurret,
                            "Bird Large Breathe" => Tactic::BirdLargeBreathe,
                            "Bird Large Fire" => Tactic::BirdLargeFire,
                            "Bird Large Basic" => Tactic::BirdLargeBasic,
                            "Flame Wyvern" | "Frost Wyvern" | "Cloud Wyvern" | "Sea Wyvern"
                            | "Weald Wyvern" => Tactic::Wyvern,
                            "Bird Medium Basic" => Tactic::BirdMediumBasic,
                            "Bushly" | "Cactid" | "Irrwurz" | "Driggle" | "Mossy Snail"
                            | "Strigoi Claws" | "Harlequin" => Tactic::SimpleDouble,
                            "Clay Golem" => Tactic::ClayGolem,
                            "Ancient Effigy" => Tactic::AncientEffigy,
                            "TerracottaStatue" | "Mogwai" => Tactic::TerracottaStatue,
                            "TerracottaBesieger" => Tactic::Bow,
                            "TerracottaDemolisher" => Tactic::SimpleDouble,
                            "TerracottaPunisher" => Tactic::SimpleMelee,
                            "TerracottaPursuer" => Tactic::SwordSimple,
                            "Cursekeeper" => Tactic::Cursekeeper,
                            "CursekeeperFake" => Tactic::CursekeeperFake,
                            "ShamanicSpirit" => Tactic::ShamanicSpirit,
                            "Jiangshi" => Tactic::Jiangshi,
                            "Mindflayer" => Tactic::Mindflayer,
                            "Flamekeeper" => Tactic::Flamekeeper,
                            "Forgemaster" => Tactic::Forgemaster,
                            "Minotaur" => Tactic::Minotaur,
                            "Cyclops" => Tactic::Cyclops,
                            "Dullahan" => Tactic::Dullahan,
                            "Grave Warden" => Tactic::GraveWarden,
                            "Tidal Warrior" => Tactic::TidalWarrior,
                            "Karkatha" => Tactic::Karkatha,
                            "Tidal Totem"
                            | "Tornado"
                            | "Gnarling Totem Red"
                            | "Gnarling Totem Green"
                            | "Gnarling Totem White" => Tactic::RadialTurret,
                            "FieryTornado" => Tactic::FieryTornado,
                            "Yeti" => Tactic::Yeti,
                            "Harvester" => Tactic::Harvester,
                            "Cardinal" => Tactic::Cardinal,
                            "Sea Bishop" => Tactic::SeaBishop,
                            "Dagon" => Tactic::Dagon,
                            "Snaretongue" => Tactic::Snaretongue,
                            "Dagonite" => Tactic::ArthropodAmbush,
                            "Gnarling Dagger" => Tactic::SimpleBackstab,
                            "Gnarling Blowgun" => Tactic::ElevatedRanged,
                            "Deadwood" => Tactic::Deadwood,
                            "Mandragora" => Tactic::Mandragora,
                            "Wood Golem" => Tactic::WoodGolem,
                            "Gnarling Chieftain" => Tactic::GnarlingChieftain,
                            "Frost Gigas" => Tactic::FrostGigas,
                            "Boreal Hammer" => Tactic::BorealHammer,
                            "Boreal Bow" => Tactic::BorealBow,
                            "Fire Gigas" => Tactic::FireGigas,
                            "Ashen Axe" => Tactic::AshenAxe,
                            "Ashen Staff" => Tactic::AshenStaff,
                            "Adlet Hunter" => Tactic::AdletHunter,
                            "Adlet Icepicker" => Tactic::AdletIcepicker,
                            "Adlet Tracker" => Tactic::AdletTracker,
                            "Hydra" => Tactic::Hydra,
                            "Ice Drake" => Tactic::IceDrake,
                            "Frostfang" => Tactic::RandomAbilities {
                                primary: 1,
                                secondary: 3,
                                abilities: [0; BASE_ABILITY_LIMIT],
                            },
                            "Tursus Claws" => Tactic::RandomAbilities {
                                primary: 2,
                                secondary: 1,
                                abilities: [4, 0, 0, 0, 0],
                            },
                            "Adlet Elder" => Tactic::AdletElder,
                            "Haniwa Soldier" => Tactic::HaniwaSoldier,
                            "Haniwa Guard" => Tactic::HaniwaGuard,
                            "Haniwa Archer" => Tactic::HaniwaArcher,
                            "Bloodmoon Bat" => Tactic::BloodmoonBat,
                            "Vampire Bat" => Tactic::VampireBat,
                            "Bloodmoon Heiress" => Tactic::BloodmoonHeiress,

                            _ => Tactic::SimpleMelee,
                        },
                        AbilitySpec::Tool(tool_kind) => tool_tactic(*tool_kind),
                    }
                } else if let ItemKind::Tool(tool) = &*item.kind() {
                    tool_tactic(tool.kind)
                } else {
                    Tactic::SimpleMelee
                }
            })
            .unwrap_or(Tactic::SimpleMelee);

        // Wield the weapon as running towards the target
        controller.push_action(ControlAction::Wield);

        // Information for attack checks
        // 'min_attack_dist' uses DEFAULT_ATTACK_RANGE, while 'body_dist' does not
        let self_radius = self.body.map_or(0.5, |b| b.max_radius()) * self.scale;
        let self_attack_range =
            (self.body.map_or(0.5, |b| b.front_radius()) + DEFAULT_ATTACK_RANGE) * self.scale;
        let tgt_radius =
            tgt_data.body.map_or(0.5, |b| b.max_radius()) * tgt_data.scale.map_or(1.0, |s| s.0);
        let min_attack_dist = self_attack_range + tgt_radius;
        let body_dist = self_radius + tgt_radius;
        let dist_sqrd = self.pos.0.distance_squared(tgt_data.pos.0);
        let angle = self
            .ori
            .look_vec()
            .angle_between(tgt_data.pos.0 - self.pos.0)
            .to_degrees();
        let angle_xy = self
            .ori
            .look_vec()
            .xy()
            .angle_between((tgt_data.pos.0 - self.pos.0).xy())
            .to_degrees();

        let eye_offset = self.body.map_or(0.0, |b| b.eye_height(self.scale));

        let tgt_eye_height = tgt_data
            .body
            .map_or(0.0, |b| b.eye_height(tgt_data.scale.map_or(1.0, |s| s.0)));
        let tgt_eye_offset = tgt_eye_height +
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

        // FIXME:
        // 1) Retrieve actual projectile speed!
        // We have to assume projectiles are faster than base speed because there are
        // skills that increase it, and in most cases this will cause agents to
        // overshoot
        //
        // 2) We use eye_offset-s which isn't actually ideal.
        // Some attacks (beam for example) may use different offsets,
        // we should probably use offsets from corresponding states.
        //
        // 3) Should we even have this big switch?
        // Not all attacks may want their direction overwritten.
        // And this is quite hard to debug when you don't see it in actual
        // attack handler.
        if let Some(dir) = match self.char_state {
            CharacterState::ChargedRanged(c) if dist_sqrd > 0.0 => {
                let charge_factor =
                    c.timer.as_secs_f32() / c.static_data.charge_duration.as_secs_f32();
                let projectile_speed = c.static_data.initial_projectile_speed
                    + charge_factor * c.static_data.scaled_projectile_speed;
                aim_projectile(
                    projectile_speed,
                    self.pos.0
                        + self.body.map_or(Vec3::zero(), |body| {
                            body.projectile_offsets(self.ori.look_vec(), self.scale)
                        }),
                    Vec3::new(
                        tgt_data.pos.0.x,
                        tgt_data.pos.0.y,
                        tgt_data.pos.0.z + tgt_eye_offset,
                    ),
                )
            },
            CharacterState::BasicRanged(c) => {
                let offset_z = match c.static_data.projectile.kind {
                    // Aim explosives and hazards at feet instead of eyes for splash damage
                    ProjectileConstructorKind::Explosive { .. }
                    | ProjectileConstructorKind::ExplosiveHazard { .. }
                    | ProjectileConstructorKind::Hazard { .. } => 0.0,
                    _ => tgt_eye_offset,
                };
                let projectile_speed = c.static_data.projectile_speed;
                aim_projectile(
                    projectile_speed,
                    self.pos.0
                        + self.body.map_or(Vec3::zero(), |body| {
                            body.projectile_offsets(self.ori.look_vec(), self.scale)
                        }),
                    Vec3::new(
                        tgt_data.pos.0.x,
                        tgt_data.pos.0.y,
                        tgt_data.pos.0.z + offset_z,
                    ),
                )
            },
            CharacterState::RapidRanged(c) => {
                let projectile_speed = c.static_data.projectile_speed;
                aim_projectile(
                    projectile_speed,
                    self.pos.0
                        + self.body.map_or(Vec3::zero(), |body| {
                            body.projectile_offsets(self.ori.look_vec(), self.scale)
                        }),
                    Vec3::new(
                        tgt_data.pos.0.x,
                        tgt_data.pos.0.y,
                        tgt_data.pos.0.z + tgt_eye_offset,
                    ),
                )
            },
            CharacterState::LeapMelee(_)
                if matches!(tactic, Tactic::Hammer | Tactic::BorealHammer | Tactic::Axe) =>
            {
                let direction_weight = match tactic {
                    Tactic::Hammer | Tactic::BorealHammer => 0.1,
                    Tactic::Axe => 0.3,
                    _ => unreachable!("Direction weight called on incorrect tactic."),
                };

                let tgt_pos = tgt_data.pos.0;
                let self_pos = self.pos.0;

                let delta_x = (tgt_pos.x - self_pos.x) * direction_weight;
                let delta_y = (tgt_pos.y - self_pos.y) * direction_weight;

                Dir::from_unnormalized(Vec3::new(delta_x, delta_y, -1.0))
            },
            CharacterState::BasicBeam(_) => {
                let aim_from = self.body.map_or(self.pos.0, |body| {
                    self.pos.0
                        + basic_beam::beam_offsets(
                            body,
                            controller.inputs.look_dir,
                            self.ori.look_vec(),
                            // Try to match animation by getting some context
                            self.vel.0 - self.physics_state.ground_vel,
                            self.physics_state.on_ground,
                        )
                });
                let aim_to = Vec3::new(
                    tgt_data.pos.0.x,
                    tgt_data.pos.0.y,
                    tgt_data.pos.0.z + tgt_eye_offset,
                );
                Dir::from_unnormalized(aim_to - aim_from)
            },
            _ => {
                let aim_from = Vec3::new(self.pos.0.x, self.pos.0.y, self.pos.0.z + eye_offset);
                let aim_to = Vec3::new(
                    tgt_data.pos.0.x,
                    tgt_data.pos.0.y,
                    tgt_data.pos.0.z + tgt_eye_offset,
                );
                Dir::from_unnormalized(aim_to - aim_from)
            },
        } {
            controller.inputs.look_dir = dir;
        }

        let attack_data = AttackData {
            body_dist,
            min_attack_dist,
            dist_sqrd,
            angle,
            angle_xy,
        };

        // Match on tactic. Each tactic has different controls depending on the distance
        // from the agent to the target.
        match tactic {
            Tactic::SimpleFlyingMelee => self.handle_simple_flying_melee(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::SimpleMelee => {
                self.handle_simple_melee(agent, controller, &attack_data, tgt_data, read_data, rng)
            },
            Tactic::Axe => {
                self.handle_axe_attack(agent, controller, &attack_data, tgt_data, read_data, rng)
            },
            Tactic::Hammer => {
                self.handle_hammer_attack(agent, controller, &attack_data, tgt_data, read_data, rng)
            },
            Tactic::Sword => {
                self.handle_sword_attack(agent, controller, &attack_data, tgt_data, read_data, rng)
            },
            Tactic::Bow => {
                self.handle_bow_attack(agent, controller, &attack_data, tgt_data, read_data, rng)
            },
            Tactic::Staff => {
                self.handle_staff_attack(agent, controller, &attack_data, tgt_data, read_data, rng)
            },
            Tactic::Sceptre => self.handle_sceptre_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::StoneGolem => {
                self.handle_stone_golem_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::IronGolem => {
                self.handle_iron_golem_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::CircleCharge {
                radius,
                circle_time,
            } => self.handle_circle_charge_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                radius,
                circle_time,
                rng,
            ),
            Tactic::QuadLowRanged => self.handle_quadlow_ranged_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::TailSlap => {
                self.handle_tail_slap_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::QuadLowQuick => self.handle_quadlow_quick_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::QuadLowBasic => self.handle_quadlow_basic_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::QuadMedJump => self.handle_quadmed_jump_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::QuadMedBasic => self.handle_quadmed_basic_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::QuadMedHoof => self.handle_quadmed_hoof_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::QuadLowBeam => self.handle_quadlow_beam_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::Elephant => self.handle_elephant_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::Rocksnapper => {
                self.handle_rocksnapper_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::Roshwalr => {
                self.handle_roshwalr_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::OrganAura => {
                self.handle_organ_aura_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::Theropod => {
                self.handle_theropod_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::ArthropodMelee => self.handle_arthropod_melee_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::ArthropodAmbush => self.handle_arthropod_ambush_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::ArthropodRanged => self.handle_arthropod_ranged_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::Turret => {
                self.handle_turret_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::FixedTurret => self.handle_fixed_turret_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::RotatingTurret => {
                self.handle_rotating_turret_attack(agent, controller, tgt_data, read_data)
            },
            Tactic::Mindflayer => self.handle_mindflayer_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::Flamekeeper => {
                self.handle_flamekeeper_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::Forgemaster => {
                self.handle_forgemaster_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::BirdLargeFire => self.handle_birdlarge_fire_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            // Mostly identical to BirdLargeFire but tweaked for flamethrower instead of shockwave
            Tactic::BirdLargeBreathe => self.handle_birdlarge_breathe_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::BirdLargeBasic => self.handle_birdlarge_basic_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::Wyvern => {
                self.handle_wyvern_attack(agent, controller, &attack_data, tgt_data, read_data, rng)
            },
            Tactic::BirdMediumBasic => {
                self.handle_simple_melee(agent, controller, &attack_data, tgt_data, read_data, rng)
            },
            Tactic::SimpleDouble => self.handle_simple_double_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::Jiangshi => {
                self.handle_jiangshi_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::ClayGolem => {
                self.handle_clay_golem_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::ClaySteed => {
                self.handle_clay_steed_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::AncientEffigy => self.handle_ancient_effigy_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::TerracottaStatue => {
                self.handle_terracotta_statue_attack(agent, controller, &attack_data, read_data)
            },
            Tactic::Minotaur => {
                self.handle_minotaur_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::Cyclops => {
                self.handle_cyclops_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::Dullahan => {
                self.handle_dullahan_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::GraveWarden => self.handle_grave_warden_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::TidalWarrior => self.handle_tidal_warrior_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::Karkatha => self.handle_karkatha_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::RadialTurret => self.handle_radial_turret_attack(controller),
            Tactic::FieryTornado => self.handle_fiery_tornado_attack(agent, controller),
            Tactic::Yeti => {
                self.handle_yeti_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::Harvester => self.handle_harvester_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::Cardinal => self.handle_cardinal_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::SeaBishop => self.handle_sea_bishop_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::Cursekeeper => self.handle_cursekeeper_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::CursekeeperFake => {
                self.handle_cursekeeper_fake_attack(controller, &attack_data)
            },
            Tactic::ShamanicSpirit => self.handle_shamanic_spirit_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::Dagon => {
                self.handle_dagon_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::Snaretongue => {
                self.handle_snaretongue_attack(agent, controller, &attack_data, read_data)
            },
            Tactic::SimpleBackstab => {
                self.handle_simple_backstab(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::ElevatedRanged => {
                self.handle_elevated_ranged(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::Deadwood => {
                self.handle_deadwood(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::Mandragora => {
                self.handle_mandragora(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::WoodGolem => {
                self.handle_wood_golem(agent, controller, &attack_data, tgt_data, read_data, rng)
            },
            Tactic::GnarlingChieftain => self.handle_gnarling_chieftain(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::FrostGigas => self.handle_frostgigas_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::BorealHammer => self.handle_boreal_hammer_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::BorealBow => self.handle_boreal_bow_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::FireGigas => self.handle_firegigas_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::AshenAxe => self.handle_ashen_axe_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::AshenStaff => self.handle_ashen_staff_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::SwordSimple => self.handle_sword_simple_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::AdletHunter => {
                self.handle_adlet_hunter(agent, controller, &attack_data, tgt_data, read_data, rng)
            },
            Tactic::AdletIcepicker => {
                self.handle_adlet_icepicker(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::AdletTracker => {
                self.handle_adlet_tracker(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::IceDrake => {
                self.handle_icedrake(agent, controller, &attack_data, tgt_data, read_data, rng)
            },
            Tactic::Hydra => {
                self.handle_hydra(agent, controller, &attack_data, tgt_data, read_data, rng)
            },
            Tactic::BloodmoonBat => self.handle_bloodmoon_bat_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::VampireBat => self.handle_vampire_bat_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::BloodmoonHeiress => self.handle_bloodmoon_heiress_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
            Tactic::RandomAbilities {
                primary,
                secondary,
                abilities,
            } => self.handle_random_abilities(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
                primary,
                secondary,
                abilities,
            ),
            Tactic::AdletElder => {
                self.handle_adlet_elder(agent, controller, &attack_data, tgt_data, read_data, rng)
            },
            Tactic::HaniwaSoldier => {
                self.handle_haniwa_soldier(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::HaniwaGuard => {
                self.handle_haniwa_guard(agent, controller, &attack_data, tgt_data, read_data, rng)
            },
            Tactic::HaniwaArcher => {
                self.handle_haniwa_archer(agent, controller, &attack_data, tgt_data, read_data)
            },
        }
    }

    pub fn handle_sounds_heard(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        read_data: &ReadData,
        emitters: &mut AgentEmitters,
        rng: &mut impl Rng,
    ) {
        agent.forget_old_sounds(read_data.time.0);

        if is_invulnerable(*self.entity, read_data) || is_steering(*self.entity, read_data) {
            self.idle(agent, controller, read_data, emitters, rng);
            return;
        }

        if let Some(sound) = agent.sounds_heard.last() {
            let sound_pos = Pos(sound.pos);
            let dist_sqrd = self.pos.0.distance_squared(sound_pos.0);
            // NOTE: There is an implicit distance requirement given that sound volume
            // dissipates as it travels, but we will not want to flee if a sound is super
            // loud but heard from a great distance, regardless of how loud it was.
            // `is_close` is this limiter.
            let is_close = dist_sqrd < 35.0_f32.powi(2);

            let sound_was_loud = sound.vol >= 10.0;
            let sound_was_threatening = sound_was_loud
                || matches!(sound.kind, SoundKind::Utterance(UtteranceKind::Scream, _));

            let has_enemy_alignment = matches!(self.alignment, Some(Alignment::Enemy));
            let follows_threatening_sounds =
                has_enemy_alignment || is_village_guard(*self.entity, read_data);

            if sound_was_threatening && is_close {
                if !self.below_flee_health(agent) && follows_threatening_sounds {
                    self.follow(agent, controller, read_data, &sound_pos);
                } else if self.below_flee_health(agent) || !follows_threatening_sounds {
                    self.flee(agent, controller, read_data, &sound_pos);
                } else {
                    self.idle(agent, controller, read_data, emitters, rng);
                }
            } else {
                self.idle(agent, controller, read_data, emitters, rng);
            }
        } else {
            self.idle(agent, controller, read_data, emitters, rng);
        }
    }

    pub fn attack_target_attacker(
        &self,
        agent: &mut Agent,
        read_data: &ReadData,
        controller: &mut Controller,
        emitters: &mut AgentEmitters,
        rng: &mut impl Rng,
    ) {
        if let Some(Target { target, .. }) = agent.target
            && let Some(tgt_health) = read_data.healths.get(target)
            && let Some(by) = tgt_health.last_change.damage_by()
            && let Some(attacker) = get_entity_by_id(by.uid(), read_data)
        {
            if agent.target.is_none() {
                controller.push_utterance(UtteranceKind::Angry);
            }

            let attacker_pos = read_data.positions.get(attacker).map(|pos| pos.0);
            agent.target = Some(Target::new(
                attacker,
                true,
                read_data.time.0,
                true,
                attacker_pos,
            ));

            if let Some(tgt_pos) = read_data.positions.get(attacker) {
                if is_dead_or_invulnerable(attacker, read_data) {
                    agent.target = Some(Target::new(
                        target,
                        false,
                        read_data.time.0,
                        false,
                        Some(tgt_pos.0),
                    ));

                    self.idle(agent, controller, read_data, emitters, rng);
                } else {
                    let target_data = TargetData::new(tgt_pos, target, read_data);
                    // TODO: Reimplement this in rtsim
                    // if let Some(tgt_name) =
                    //     read_data.stats.get(target).map(|stats| stats.name.clone())
                    // {
                    //     agent.add_fight_to_memory(&tgt_name, read_data.time.0)
                    // }
                    self.attack(agent, controller, &target_data, read_data, rng);
                }
            }
        }
    }

    // TODO: Pass a localisation key instead of `Content` to avoid allocating if
    // we're not permitted to speak.
    pub fn chat_npc_if_allowed_to_speak(
        &self,
        msg: Content,
        agent: &Agent,
        emitters: &mut AgentEmitters,
    ) -> bool {
        if agent.allowed_to_speak() {
            self.chat_npc(msg, emitters);
            true
        } else {
            false
        }
    }

    pub fn chat_npc(&self, content: Content, emitters: &mut AgentEmitters) {
        emitters.emit(ChatEvent {
            msg: UnresolvedChatMsg::npc(*self.uid, content),
            from_client: false,
        });
    }

    fn emit_scream(&self, time: f64, emitters: &mut AgentEmitters) {
        if let Some(body) = self.body {
            emitters.emit(SoundEvent {
                sound: Sound::new(
                    SoundKind::Utterance(UtteranceKind::Scream, *body),
                    self.pos.0,
                    13.0,
                    time,
                ),
            });
        }
    }

    pub fn cry_out(&self, agent: &Agent, emitters: &mut AgentEmitters, read_data: &ReadData) {
        let has_enemy_alignment = matches!(self.alignment, Some(Alignment::Enemy));
        let is_below_flee_health = self.below_flee_health(agent);

        if has_enemy_alignment && is_below_flee_health {
            self.chat_npc_if_allowed_to_speak(
                Content::localized("npc-speech-cultist_low_health_fleeing"),
                agent,
                emitters,
            );
        } else if is_villager(self.alignment) {
            self.chat_npc_if_allowed_to_speak(
                Content::localized("npc-speech-villager_under_attack"),
                agent,
                emitters,
            );
            self.emit_scream(read_data.time.0, emitters);
        }
    }

    pub fn exclaim_relief_about_enemy_dead(&self, agent: &Agent, emitters: &mut AgentEmitters) {
        if is_villager(self.alignment) {
            self.chat_npc_if_allowed_to_speak(
                Content::localized("npc-speech-villager_enemy_killed"),
                agent,
                emitters,
            );
        }
    }

    pub fn below_flee_health(&self, agent: &Agent) -> bool {
        self.damage.min(1.0) < agent.psyche.flee_health
    }

    pub fn is_more_dangerous_than_target(
        &self,
        entity: EcsEntity,
        target: Target,
        read_data: &ReadData,
    ) -> bool {
        let entity_pos = read_data.positions.get(entity);
        let target_pos = read_data.positions.get(target.target);

        entity_pos.is_some_and(|entity_pos| {
            target_pos.is_none_or(|target_pos| {
                // Fuzzy factor that makes it harder for players to cheese enemies by making
                // them quickly flip aggro between two players.
                // It does this by only switching aggro if the entity is closer to the enemy by
                // a specific proportional threshold.
                const FUZZY_DIST_COMPARISON: f32 = 0.8;

                let is_target_further = target_pos.0.distance(entity_pos.0)
                    < target_pos.0.distance(entity_pos.0) * FUZZY_DIST_COMPARISON;
                let is_entity_hostile = read_data
                    .alignments
                    .get(entity)
                    .zip(self.alignment)
                    .is_some_and(|(entity, me)| me.hostile_towards(*entity));

                // Consider entity more dangerous than target if entity is closer or if target
                // had not triggered aggro.
                !target.aggro_on || (is_target_further && is_entity_hostile)
            })
        })
    }

    pub fn is_enemy(&self, entity: EcsEntity, read_data: &ReadData) -> bool {
        let other_alignment = read_data.alignments.get(entity);

        (entity != *self.entity)
            && !self.passive_towards(entity, read_data)
            && (are_our_owners_hostile(self.alignment, other_alignment, read_data)
                || (is_villager(self.alignment) && is_dressed_as_cultist(entity, read_data)
                    || (is_villager(self.alignment) && is_dressed_as_witch(entity, read_data))
                    || (is_villager(self.alignment) && is_dressed_as_pirate(entity, read_data))))
    }

    pub fn is_hunting_animal(&self, entity: EcsEntity, read_data: &ReadData) -> bool {
        (entity != *self.entity)
            && !self.friendly_towards(entity, read_data)
            && matches!(read_data.bodies.get(entity), Some(Body::QuadrupedSmall(_)))
    }

    fn should_defend(&self, entity: EcsEntity, read_data: &ReadData) -> bool {
        let entity_alignment = read_data.alignments.get(entity);

        let we_are_friendly = entity_alignment.is_some_and(|entity_alignment| {
            self.alignment
                .is_some_and(|alignment| !alignment.hostile_towards(*entity_alignment))
        });
        let we_share_species = read_data.bodies.get(entity).is_some_and(|entity_body| {
            self.body.is_some_and(|body| {
                entity_body.is_same_species_as(body)
                    || (entity_body.is_humanoid() && body.is_humanoid())
            })
        });
        let self_owns_entity =
            matches!(entity_alignment, Some(Alignment::Owned(ouid)) if *self.uid == *ouid);

        (we_are_friendly && we_share_species)
            || (is_village_guard(*self.entity, read_data) && is_villager(entity_alignment))
            || self_owns_entity
    }

    fn passive_towards(&self, entity: EcsEntity, read_data: &ReadData) -> bool {
        if let (Some(self_alignment), Some(other_alignment)) =
            (self.alignment, read_data.alignments.get(entity))
        {
            self_alignment.passive_towards(*other_alignment)
        } else {
            false
        }
    }

    fn friendly_towards(&self, entity: EcsEntity, read_data: &ReadData) -> bool {
        if let (Some(self_alignment), Some(other_alignment)) =
            (self.alignment, read_data.alignments.get(entity))
        {
            self_alignment.friendly_towards(*other_alignment)
        } else {
            false
        }
    }

    pub fn can_see_entity(
        &self,
        agent: &Agent,
        controller: &Controller,
        other: EcsEntity,
        other_pos: &Pos,
        other_scale: Option<&Scale>,
        read_data: &ReadData,
    ) -> bool {
        let other_stealth_multiplier = {
            let other_inventory = read_data.inventories.get(other);
            let other_char_state = read_data.char_states.get(other);

            perception_dist_multiplier_from_stealth(other_inventory, other_char_state, self.msm)
        };

        let within_sight_dist = {
            let sight_dist = agent.psyche.sight_dist * other_stealth_multiplier;
            let dist_sqrd = other_pos.0.distance_squared(self.pos.0);

            dist_sqrd < sight_dist.powi(2)
        };

        let within_fov = (other_pos.0 - self.pos.0)
            .try_normalized()
            .is_some_and(|v| v.dot(*controller.inputs.look_dir) > 0.15);

        let other_body = read_data.bodies.get(other);

        (within_sight_dist)
            && within_fov
            && entities_have_line_of_sight(
                self.pos,
                self.body,
                self.scale,
                other_pos,
                other_body,
                other_scale,
                read_data,
            )
    }

    pub fn detects_other(
        &self,
        agent: &Agent,
        controller: &Controller,
        other: &EcsEntity,
        other_pos: &Pos,
        other_scale: Option<&Scale>,
        read_data: &ReadData,
    ) -> bool {
        self.can_sense_directly_near(other_pos)
            || self.can_see_entity(agent, controller, *other, other_pos, other_scale, read_data)
    }

    pub fn can_sense_directly_near(&self, e_pos: &Pos) -> bool {
        let chance = rng().random_bool(0.3);
        e_pos.0.distance_squared(self.pos.0) < 5_f32.powi(2) && chance
    }

    pub fn menacing(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        target: EcsEntity,
        tgt_data: &TargetData,
        read_data: &ReadData,
        emitters: &mut AgentEmitters,
        remembers_fight_with_target: bool,
    ) {
        let max_move = 0.5;
        let move_dir = controller.inputs.move_dir;
        let move_dir_mag = move_dir.magnitude();
        let mut chat = |agent: &mut Agent, content: Content| {
            self.chat_npc_if_allowed_to_speak(content, agent, emitters);
        };
        let mut chat_villager_remembers_fighting = |agent: &mut Agent| {
            let tgt_name = read_data.stats.get(target).map(|stats| stats.name.clone());

            // TODO: Localise
            // Is this thing even used??
            if let Some(tgt_name) = tgt_name.as_ref().and_then(|name| name.as_plain()) {
                chat(
                    agent,
                    Content::localized_with_args("npc-speech-remembers-fight", [(
                        "name", tgt_name,
                    )]),
                )
            } else {
                chat(
                    agent,
                    Content::localized("npc-speech-remembers-fight-no-name"),
                );
            }
        };

        self.look_toward(controller, read_data, target);
        controller.push_action(ControlAction::Wield);

        if move_dir_mag > max_move {
            controller.inputs.move_dir = max_move * move_dir / move_dir_mag;
        }

        match agent
            .timer
            .timeout_elapsed(read_data.time.0, comp::agent::TimerAction::Warn, 5.0)
        {
            Some(true) | None => {
                self.path_toward_target(
                    agent,
                    controller,
                    tgt_data.pos.0,
                    read_data,
                    Path::AtTarget,
                    Some(0.4),
                );
            },
            Some(false) => {
                agent
                    .timer
                    .start(read_data.time.0, comp::agent::TimerAction::Warn);
                controller.push_utterance(UtteranceKind::Angry);
                if is_villager(self.alignment) {
                    if remembers_fight_with_target {
                        chat_villager_remembers_fighting(agent);
                    } else if is_dressed_as_cultist(target, read_data) {
                        chat(
                            agent,
                            Content::localized("npc-speech-villager_cultist_alarm"),
                        );
                    } else if is_dressed_as_witch(target, read_data) {
                        chat(agent, Content::localized("npc-speech-villager_witch_alarm"));
                    } else if is_dressed_as_pirate(target, read_data) {
                        chat(
                            agent,
                            Content::localized("npc-speech-villager_pirate_alarm"),
                        );
                    } else {
                        chat(agent, Content::localized("npc-speech-menacing"));
                    }
                } else {
                    chat(agent, Content::localized("npc-speech-menacing"));
                }
            },
        }
    }

    /// Dismount if riding something the agent can't control.
    pub fn dismount_uncontrollable(&self, controller: &mut Controller, read_data: &ReadData) {
        if read_data.is_riders.get(*self.entity).is_some_and(|mount| {
            read_data
                .id_maps
                .uid_entity(mount.mount)
                .and_then(|e| read_data.bodies.get(e))
                .is_none_or(|b| b.has_free_will())
        }) || read_data
            .is_volume_riders
            .get(*self.entity)
            .is_some_and(|r| !r.is_steering_entity())
        {
            controller.push_event(ControlEvent::Unmount);
        }
    }

    /// Dismount if riding something.
    ///
    /// Currently there's an exception for if the agent is steering a volume
    /// entity.
    pub fn dismount(&self, controller: &mut Controller, read_data: &ReadData) {
        if read_data.is_riders.contains(*self.entity)
            || read_data
                .is_volume_riders
                .get(*self.entity)
                .is_some_and(|r| !r.is_steering_entity())
        {
            controller.push_event(ControlEvent::Unmount);
        }
    }
}
