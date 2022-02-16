pub mod attack;
pub mod consts;
pub mod data;
pub mod util;

use crate::{
    rtsim::RtSim,
    sys::agent::{
        consts::{
            AVG_FOLLOW_DIST, AWARENESS_INVESTIGATE_THRESHOLD, DAMAGE_MEMORY_DURATION,
            DEFAULT_ATTACK_RANGE, FLEE_DURATION, HEALING_ITEM_THRESHOLD,
            IDLE_HEALING_ITEM_THRESHOLD, MAX_FLEE_DIST, MAX_FOLLOW_DIST, PARTIAL_PATH_DIST,
            RETARGETING_THRESHOLD_SECONDS, SEPARATION_BIAS, SEPARATION_DIST,
        },
        data::{AgentData, AttackData, ReadData, Tactic, TargetData},
        util::{
            aim_projectile, can_see_tgt, get_entity_by_id, is_dead, is_dead_or_invulnerable,
            is_invulnerable, try_owner_alignment,
        },
    },
};
use common::{
    combat,
    comp::{
        self,
        agent::{
            AgentEvent, Sound, SoundKind, Target, TimerAction, DEFAULT_INTERACTION_TIME,
            TRADE_INTERACTION_TIME,
        },
        buff::BuffKind,
        compass::{Direction, Distance},
        dialogue::{MoodContext, MoodState, Subject},
        inventory::{item::ItemTag, slot::EquipSlot},
        invite::{InviteKind, InviteResponse},
        item::{
            tool::{AbilitySpec, ToolKind},
            ConsumableKind, Item, ItemDesc, ItemKind,
        },
        projectile::ProjectileConstructor,
        Agent, Alignment, BehaviorState, Body, CharacterState, ControlAction, ControlEvent,
        Controller, Health, HealthChange, InputKind, Inventory, InventoryAction, Pos, Scale, Stats,
        UnresolvedChatMsg, UtteranceKind,
    },
    effect::{BuffEffect, Effect},
    event::{Emitter, EventBus, ServerEvent},
    path::TraversalConfig,
    rtsim::{Memory, MemoryItem, RtSimEvent},
    states::basic_beam,
    terrain::{Block, TerrainGrid},
    time::DayPeriod,
    trade::{TradeAction, TradePhase, TradeResult},
    util::Dir,
    vol::ReadVol,
};
use common_base::prof_span;
use common_ecs::{Job, Origin, ParMode, Phase, System};
use rand::{thread_rng, Rng};
use rayon::iter::ParallelIterator;
use specs::{
    saveload::{Marker, MarkerAllocator},
    Entity as EcsEntity, Join, ParJoin, Write, WriteExpect, WriteStorage,
};
use vek::*;

/// This system will allow NPCs to modify their controller
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        ReadData<'a>,
        Write<'a, EventBus<ServerEvent>>,
        WriteStorage<'a, Agent>,
        WriteStorage<'a, Controller>,
        WriteExpect<'a, RtSim>,
    );

    const NAME: &'static str = "agent";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        job: &mut Job<Self>,
        (read_data, event_bus, mut agents, mut controllers, mut rtsim): Self::SystemData,
    ) {
        let rtsim = &mut *rtsim;
        job.cpu_stats.measure(ParMode::Rayon);

        (
            &read_data.entities,
            (&read_data.energies, read_data.healths.maybe()),
            (
                &read_data.positions,
                &read_data.velocities,
                &read_data.orientations,
            ),
            read_data.bodies.maybe(),
            &read_data.inventories,
            (
                &read_data.char_states,
                &read_data.skill_set,
                &read_data.active_abilities,
            ),
            &read_data.physics_states,
            &read_data.uids,
            &mut agents,
            &mut controllers,
            read_data.light_emitter.maybe(),
            read_data.groups.maybe(),
            !&read_data.is_mounts,
        )
            .par_join()
            .for_each_init(
                || {
                    prof_span!(guard, "agent rayon job");
                    guard
                },
                |_guard,
                 (
                    entity,
                    (energy, health),
                    (pos, vel, ori),
                    body,
                    inventory,
                    (char_state, skill_set, active_abilities),
                    physics_state,
                    uid,
                    agent,
                    controller,
                    light_emitter,
                    group,
                    _,
                )| {
                    let mut event_emitter = event_bus.emitter();
                    let mut rng = thread_rng();

                    // Hack, replace with better system when groups are more sophisticated
                    // Override alignment if in a group unless entity is owned already
                    let alignment = if matches!(
                        &read_data.alignments.get(entity),
                        &Some(Alignment::Owned(_))
                    ) {
                        read_data.alignments.get(entity).copied()
                    } else {
                        group
                            .and_then(|g| read_data.group_manager.group_info(*g))
                            .and_then(|info| read_data.uids.get(info.leader))
                            .copied()
                            .map_or_else(
                                || read_data.alignments.get(entity).copied(),
                                |uid| Some(Alignment::Owned(uid)),
                            )
                    };

                    if !matches!(char_state, CharacterState::LeapMelee(_)) {
                        // Default to looking in orientation direction
                        // (can be overridden below)
                        //
                        // This definetly breaks LeapMelee and
                        // probably not only that, do we really need this at all?
                        controller.reset();
                        controller.inputs.look_dir = ori.look_dir();
                    }

                    let scale = read_data.scales.get(entity).map_or(1.0, |Scale(s)| *s);

                    let glider_equipped = inventory
                        .equipped(EquipSlot::Glider)
                        .as_ref()
                        .map_or(false, |item| {
                            matches!(item.kind(), comp::item::ItemKind::Glider(_))
                        });

                    let is_gliding = matches!(
                        read_data.char_states.get(entity),
                        Some(CharacterState::GlideWield(_) | CharacterState::Glide(_))
                    ) && physics_state.on_ground.is_none();

                    if let Some(pid) = agent.position_pid_controller.as_mut() {
                        pid.add_measurement(read_data.time.0, pos.0);
                    }

                    // This controls how picky NPCs are about their pathfinding.
                    // Giants are larger and so can afford to be less precise
                    // when trying to move around the world
                    // (especially since they would otherwise get stuck on
                    // obstacles that smaller entities would not).
                    let node_tolerance = scale * 1.5;
                    let slow_factor = body.map_or(0.0, |b| b.base_accel() / 250.0).min(1.0);
                    let traversal_config = TraversalConfig {
                        node_tolerance,
                        slow_factor,
                        on_ground: physics_state.on_ground.is_some(),
                        in_liquid: physics_state.in_liquid().is_some(),
                        min_tgt_dist: 1.0,
                        can_climb: body.map_or(false, Body::can_climb),
                        can_fly: body.map_or(false, |b| b.fly_thrust().is_some()),
                    };
                    let health_fraction = health.map_or(1.0, Health::fraction);
                    let rtsim_entity = read_data
                        .rtsim_entities
                        .get(entity)
                        .and_then(|rtsim_ent| rtsim.get_entity(rtsim_ent.0));

                    if traversal_config.can_fly && matches!(body, Some(Body::Ship(_))) {
                        // hack (kinda): Never turn off flight airships
                        // since it results in stuttering and falling back to the ground.
                        //
                        // TODO: look into `controller.reset()` line above
                        // and see if it fixes it
                        controller.push_basic_input(InputKind::Fly);
                    }

                    // Package all this agent's data into a convenient struct
                    let data = AgentData {
                        entity: &entity,
                        rtsim_entity,
                        uid,
                        pos,
                        vel,
                        ori,
                        energy,
                        body,
                        inventory,
                        skill_set,
                        physics_state,
                        alignment: alignment.as_ref(),
                        traversal_config,
                        scale,
                        damage: health_fraction,
                        light_emitter,
                        glider_equipped,
                        is_gliding,
                        health: read_data.healths.get(entity),
                        char_state,
                        active_abilities,
                        cached_spatial_grid: &read_data.cached_spatial_grid,
                    };
                    ///////////////////////////////////////////////////////////
                    // Behavior tree
                    ///////////////////////////////////////////////////////////
                    // The behavior tree is meant to make decisions for agents
                    // *but should not* mutate any data (only action nodes
                    // should do that). Each path should lead to one (and only
                    // one) action node. This makes bugfinding much easier and
                    // debugging way easier. If you don't think so, try
                    // debugging the agent code before this MR
                    // (https://gitlab.com/veloren/veloren/-/merge_requests/1801).
                    // Each tick should arrive at one (1) action node which
                    // then determines what the agent does. If this makes you
                    // uncomfortable, consider dt the response time of the
                    // NPC. To make the tree easier to read, subtrees can be
                    // created as methods on `AgentData`. Action nodes are
                    // also methods on the `AgentData` struct. Action nodes
                    // are the only parts of this tree that should provide
                    // inputs.

                    // Falling damage starts from 30.0 as of time of writing
                    // But keep in mind our 25 m/s gravity
                    let is_falling_dangerous = data.vel.0.z < -20.0;

                    let is_on_fire = read_data
                        .buffs
                        .get(entity)
                        .map_or(false, |b| b.kinds.contains_key(&BuffKind::Burning));

                    // If falling velocity is critical, throw everything
                    // and save yourself!
                    //
                    // If can fly - fly.
                    // If have glider - glide.
                    // Else, rest in peace.
                    if is_falling_dangerous && data.traversal_config.can_fly {
                        data.fly_upward(controller)
                    } else if is_falling_dangerous && data.glider_equipped {
                        data.glider_fall(controller);
                    // If on fire and able, stop, drop, and roll
                    } else if is_on_fire
                        && data.body.map_or(false, |b| b.is_humanoid())
                        && data.physics_state.on_ground.is_some()
                        && rng.gen_bool((2.0 * read_data.dt.0).into())
                    {
                        controller.inputs.move_dir = ori
                            .look_vec()
                            .xy()
                            .try_normalized()
                            .unwrap_or_else(Vec2::zero);
                        controller.push_basic_input(InputKind::Roll);
                    } else {
                        // Target an entity that's attacking us if the attack
                        // was recent and we have a health component
                        match health {
                            Some(health)
                                if read_data.time.0 - health.last_change.time.0
                                    < DAMAGE_MEMORY_DURATION =>
                            {
                                if let Some(by) = health.last_change.damage_by() {
                                    if let Some(attacker) =
                                        read_data.uid_allocator.retrieve_entity_internal(by.uid().0)
                                    {
                                        // If target is dead or invulnerable (for now, this only
                                        // means safezone), untarget them and idle.
                                        if is_dead_or_invulnerable(attacker, &read_data) {
                                            agent.target = None;
                                        } else if let Some(tgt_pos) =
                                            read_data.positions.get(attacker)
                                        {
                                            if agent.target.is_none() {
                                                controller.push_event(ControlEvent::Utterance(
                                                    UtteranceKind::Angry,
                                                ));
                                            }

                                            // Determine whether the new target should be a priority
                                            // over the old one (i.e: because it's either close or
                                            // because they attacked us)
                                            let more_dangerous_than_old_target =
                                                agent.target.map_or(true, |old_tgt| {
                                                    if let Some(old_tgt_pos) =
                                                        read_data.positions.get(old_tgt.target)
                                                    {
                                                        // Fuzzy factor that makes it harder for
                                                        // players to cheese enemies by making them
                                                        // quickly flip aggro between two players.
                                                        // It
                                                        // does this by only switching aggro if the
                                                        // new target is closer to the enemy by a
                                                        // specific proportional threshold.
                                                        const FUZZY_DIST_COMPARISON: f32 = 0.8;
                                                        // Only switch to new target if it is closer
                                                        // than the old target, or if the old target
                                                        // had not triggered aggro (the new target
                                                        // has because damage always triggers it)
                                                        let old_tgt_not_threat = !old_tgt.aggro_on;
                                                        let old_tgt_further =
                                                            tgt_pos.0.distance(pos.0)
                                                                < old_tgt_pos.0.distance(pos.0)
                                                                    * FUZZY_DIST_COMPARISON;
                                                        let new_tgt_hostile = read_data
                                                            .alignments
                                                            .get(attacker)
                                                            .zip(alignment)
                                                            .map_or(false, |(attacker, us)| {
                                                                us.hostile_towards(*attacker)
                                                            });
                                                        old_tgt_not_threat
                                                            || (old_tgt_further && new_tgt_hostile)
                                                    } else {
                                                        true
                                                    }
                                                });

                                            // Select the attacker as the new target
                                            if more_dangerous_than_old_target {
                                                agent.target = Some(Target {
                                                    target: attacker,
                                                    hostile: true,
                                                    selected_at: read_data.time.0,
                                                    aggro_on: true,
                                                });
                                            }

                                            // Remember this attack if we're an RtSim entity
                                            if let Some(tgt_stats) =
                                                data.rtsim_entity.and(read_data.stats.get(attacker))
                                            {
                                                agent.add_enemy(&tgt_stats.name, read_data.time.0);
                                            }
                                        }
                                    }
                                }
                            },
                            _ => {},
                        }

                        if let Some(target_info) = agent.target {
                            data.react_to_target(
                                agent,
                                controller,
                                &read_data,
                                &mut event_emitter,
                                target_info,
                                &mut rng,
                            );
                        } else {
                            data.idle_tree(
                                agent,
                                controller,
                                &read_data,
                                &mut event_emitter,
                                &mut rng,
                            );
                        }
                    }

                    debug_assert!(controller.inputs.move_dir.map(|e| !e.is_nan()).reduce_and());
                    debug_assert!(controller.inputs.look_dir.map(|e| !e.is_nan()).reduce_and());
                },
            );
        for (agent, rtsim_entity) in (&mut agents, &read_data.rtsim_entities).join() {
            // Entity must be loaded in as it has an agent component :)
            // React to all events in the controller
            for event in core::mem::take(&mut agent.rtsim_controller.events) {
                match event {
                    RtSimEvent::AddMemory(memory) => {
                        rtsim.insert_entity_memory(rtsim_entity.0, memory.clone());
                    },
                    RtSimEvent::ForgetEnemy(name) => {
                        rtsim.forget_entity_enemy(rtsim_entity.0, &name);
                    },
                    RtSimEvent::SetMood(memory) => {
                        rtsim.set_entity_mood(rtsim_entity.0, memory.clone());
                    },
                    RtSimEvent::PrintMemories => {},
                }
            }
        }
    }
}

impl<'a> AgentData<'a> {
    ////////////////////////////////////////
    // Subtrees
    ////////////////////////////////////////
    fn react_to_target(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        read_data: &ReadData,
        event_emitter: &mut Emitter<'_, ServerEvent>,
        target_info: Target,
        rng: &mut impl Rng,
    ) {
        let Target {
            target, hostile, ..
        } = target_info;
        if let Some(tgt_health) = read_data.healths.get(target) {
            // If target is dead, forget them
            if tgt_health.is_dead {
                if let Some(tgt_stats) = self.rtsim_entity.and(read_data.stats.get(target)) {
                    agent.forget_enemy(&tgt_stats.name);
                }
                agent.target = None;
            // Else, if target is hostile, hostile tree
            } else if hostile {
                self.hostile_tree(agent, controller, read_data, event_emitter, rng);
            // Else, if owned, act as pet to them
            } else if let Some(Alignment::Owned(uid)) = self.alignment {
                if read_data.uids.get(target) == Some(uid) {
                    self.react_as_pet(agent, controller, read_data, event_emitter, target, rng);
                } else {
                    agent.target = None;
                    self.idle_tree(agent, controller, read_data, event_emitter, rng);
                };
            } else {
                self.idle_tree(agent, controller, read_data, event_emitter, rng);
            }
        } else {
            agent.target = None;
            self.idle_tree(agent, controller, read_data, event_emitter, rng);
        }
    }

    fn react_as_pet(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        read_data: &ReadData,
        event_emitter: &mut Emitter<'_, ServerEvent>,
        target: EcsEntity,
        rng: &mut impl Rng,
    ) {
        if let Some(tgt_pos) = read_data.positions.get(target) {
            let dist_sqrd = self.pos.0.distance_squared(tgt_pos.0);

            let owner_recently_attacked = if let Some(target_health) = read_data.healths.get(target)
            {
                read_data.time.0 - target_health.last_change.time.0 < 5.0
                    && target_health.last_change.amount < 0.0
            } else {
                false
            };

            // If too far away, then follow
            if dist_sqrd > (MAX_FOLLOW_DIST).powi(2) {
                self.follow(agent, controller, &read_data.terrain, tgt_pos);
            // Else, attack target's attacker (if there is one)
            // Target is the owner in this case
            } else if owner_recently_attacked {
                self.attack_target_attacker(agent, read_data, controller, rng);
            // Otherwise, just idle
            } else {
                self.idle_tree(agent, controller, read_data, event_emitter, rng);
            }
        }
    }

    fn idle_tree(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        read_data: &ReadData,
        event_emitter: &mut Emitter<'_, ServerEvent>,
        rng: &mut impl Rng,
    ) {
        agent.decrement_awareness(read_data.dt.0);
        agent.forget_old_sounds(read_data.time.0);

        let small_chance = rng.gen_bool(0.1);
        // Set owner if no target
        if agent.target.is_none() && small_chance {
            if let Some(Alignment::Owned(owner)) = self.alignment {
                if let Some(owner) = get_entity_by_id(owner.id(), read_data) {
                    agent.target = Some(Target::new(owner, false, read_data.time.0, false));
                }
            }
        }
        // Interact if incoming messages
        if !agent.inbox.is_empty() {
            if matches!(
                agent.inbox.front(),
                Some(AgentEvent::ServerSound(_)) | Some(AgentEvent::Hurt)
            ) {
                let sound = agent.inbox.pop_front();
                match sound {
                    Some(AgentEvent::ServerSound(sound)) => {
                        agent.sounds_heard.push(sound);
                        agent.awareness += sound.vol;
                    },
                    Some(AgentEvent::Hurt) => {
                        // Hurt utterances at random upon receiving damage
                        if rng.gen::<f32>() < 0.4 {
                            controller.push_utterance(UtteranceKind::Hurt);
                        }
                    },
                    //Note: this should be unreachable
                    Some(_) | None => return,
                }
            } else {
                agent.action_state.timer = 0.1;
            }
        }

        // If we receive a new interaction, start the interaction timer
        if agent.allowed_to_speak()
            && self.recv_interaction(agent, controller, read_data, event_emitter)
        {
            agent.timer.start(read_data.time.0, TimerAction::Interact);
        }

        let timeout = if agent.behavior.is(BehaviorState::TRADING) {
            TRADE_INTERACTION_TIME
        } else {
            DEFAULT_INTERACTION_TIME
        };

        match agent
            .timer
            .timeout_elapsed(read_data.time.0, TimerAction::Interact, timeout as f64)
        {
            None => {
                // Look toward the interacting entity for a while
                if let Some(Target { target, .. }) = &agent.target {
                    self.look_toward(controller, read_data, *target);
                    controller.push_action(ControlAction::Talk);
                }
            },
            Some(just_ended) => {
                if just_ended {
                    agent.target = None;
                    controller.push_action(ControlAction::Stand);
                }

                if rng.gen::<f32>() < 0.1 {
                    self.choose_target(agent, controller, read_data, event_emitter);
                } else if agent.awareness > AWARENESS_INVESTIGATE_THRESHOLD {
                    self.handle_elevated_awareness(agent, controller, read_data, rng);
                } else {
                    self.idle(agent, controller, read_data, rng);
                }
            },
        }
    }

    fn hostile_tree(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        read_data: &ReadData,
        event_emitter: &mut Emitter<'_, ServerEvent>,
        rng: &mut impl Rng,
    ) {
        if self.damage < HEALING_ITEM_THRESHOLD && self.heal_self(agent, controller, false) {
            agent.action_state.timer = 0.01;
            return;
        }

        if let Some(AgentEvent::Hurt) = agent.inbox.pop_front() {
            // Hurt utterances at random upon receiving damage
            if rng.gen::<f32>() < 0.4 {
                controller.push_utterance(UtteranceKind::Hurt);
            }
        }

        if let Some(Target {
            target,
            selected_at,
            aggro_on,
            ..
        }) = &mut agent.target
        {
            let target = *target;
            let selected_at = *selected_at;

            if let Some(tgt_pos) = read_data.positions.get(target) {
                let dist_sqrd = self.pos.0.distance_squared(tgt_pos.0);
                let in_aggro_range = agent
                    .psyche
                    .aggro_dist
                    .map_or(true, |ad| dist_sqrd < ad.powi(2));

                if in_aggro_range {
                    *aggro_on = true;
                }
                let aggro_on = *aggro_on;

                let should_flee = self.damage.min(1.0) < agent.psyche.flee_health;
                if should_flee {
                    let has_opportunity_to_flee = agent.action_state.timer < FLEE_DURATION;
                    let within_flee_distance = dist_sqrd < MAX_FLEE_DIST.powi(2);

                    // FIXME: Using the action state timer to see if an agent is allowed to speak is
                    // a hack.
                    if agent.action_state.timer == 0.0 {
                        self.cry_out(agent, read_data.time.0, event_emitter);
                        agent.action_state.timer = 0.01;
                    } else if within_flee_distance && has_opportunity_to_flee {
                        self.flee(agent, controller, &read_data.terrain, tgt_pos);
                        agent.action_state.timer += read_data.dt.0;
                    } else {
                        agent.action_state.timer = 0.0;
                        agent.target = None;
                        self.idle(agent, controller, read_data, rng);
                    }
                } else if is_dead(target, read_data) {
                    self.exclaim_relief_about_enemy_dead(agent, event_emitter);
                    agent.target = None;
                    self.idle(agent, controller, read_data, rng);
                } else if is_invulnerable(target, read_data) {
                    agent.target = None;
                    self.idle(agent, controller, read_data, rng);
                } else {
                    let is_time_to_retarget =
                        read_data.time.0 - selected_at > RETARGETING_THRESHOLD_SECONDS;

                    if !in_aggro_range && is_time_to_retarget {
                        self.choose_target(agent, controller, read_data, event_emitter);
                    }

                    if aggro_on {
                        let target_data = TargetData::new(
                            tgt_pos,
                            read_data.bodies.get(target),
                            read_data.scales.get(target),
                        );
                        self.attack(agent, controller, &target_data, read_data, rng);
                    } else {
                        self.menacing(agent, target, controller, read_data, event_emitter, rng);
                    }
                }
            }
        }
    }

    ////////////////////////////////////////
    // Action Nodes
    ////////////////////////////////////////

    fn glider_fall(&self, controller: &mut Controller) {
        controller.push_action(ControlAction::GlideWield);

        let flight_direction =
            Vec3::from(self.vel.0.xy().try_normalized().unwrap_or_else(Vec2::zero));
        let flight_ori = Quaternion::from_scalar_and_vec3((1.0, flight_direction));

        let ori = self.ori.look_vec();
        let look_dir = if ori.z > 0.0 {
            flight_ori.rotated_x(-0.1)
        } else {
            flight_ori.rotated_x(0.1)
        };

        let (_, look_dir) = look_dir.into_scalar_and_vec3();
        controller.inputs.look_dir = Dir::from_unnormalized(look_dir).unwrap_or_else(Dir::forward);
    }

    fn fly_upward(&self, controller: &mut Controller) {
        controller.push_basic_input(InputKind::Fly);
        controller.inputs.move_z = 1.0;
    }

    fn idle(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        read_data: &ReadData,
        rng: &mut impl Rng,
    ) {
        // Light lanterns at night
        // TODO Add a method to turn on NPC lanterns underground
        let lantern_equipped = self
            .inventory
            .equipped(EquipSlot::Lantern)
            .as_ref()
            .map_or(false, |item| {
                matches!(item.kind(), comp::item::ItemKind::Lantern(_))
            });
        let lantern_turned_on = self.light_emitter.is_some();
        let day_period = DayPeriod::from(read_data.time_of_day.0);
        // Only emit event for agents that have a lantern equipped
        if lantern_equipped && rng.gen_bool(0.001) {
            if day_period.is_dark() && !lantern_turned_on {
                // Agents with turned off lanterns turn them on randomly once it's
                // nighttime and keep them on
                // Only emit event for agents that sill need to
                // turn on their lantern
                controller.push_event(ControlEvent::EnableLantern)
            } else if lantern_turned_on && day_period.is_light() {
                // agents with turned on lanterns turn them off randomly once it's
                // daytime and keep them off
                controller.push_event(ControlEvent::DisableLantern)
            }
        };

        if self.damage < IDLE_HEALING_ITEM_THRESHOLD && self.heal_self(agent, controller, true) {
            agent.action_state.timer = 0.01;
            return;
        }

        agent.action_state.timer = 0.0;
        if let Some((travel_to, _destination)) = &agent.rtsim_controller.travel_to {
            // if it has an rtsim destination and can fly then it should
            // if it is flying and bumps something above it then it should move down
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

            if let Some((bearing, speed)) = agent.chaser.chase(
                &*read_data.terrain,
                self.pos.0,
                self.vel.0,
                *travel_to,
                TraversalConfig {
                    min_tgt_dist: 1.25,
                    ..self.traversal_config
                },
            ) {
                controller.inputs.move_dir =
                    bearing.xy().try_normalized().unwrap_or_else(Vec2::zero)
                        * speed.min(agent.rtsim_controller.speed_factor);
                self.jump_if(controller, bearing.z > 1.5 || self.traversal_config.can_fly);
                controller.inputs.climb = Some(comp::Climb::Up);
                //.filter(|_| bearing.z > 0.1 || self.physics_state.in_liquid().is_some());

                let height_offset = bearing.z
                    + if self.traversal_config.can_fly {
                        // NOTE: costs 4 us (imbris)
                        let obstacle_ahead = read_data
                            .terrain
                            .ray(
                                self.pos.0 + Vec3::unit_z(),
                                self.pos.0
                                    + bearing.try_normalized().unwrap_or_else(Vec3::unit_y) * 80.0
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
                                        .get_alt_approx(self.pos.0.xy().map(|x: f32| x as i32))
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
                            // Lerp between a line straight ahead and straight down to detect a
                            // wedge of obstacles we might fly into (inclusive so that both vectors
                            // are sampled)
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
                                    .map_or(false, |b| b.is_some())
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
                if let Some(pid) = agent.position_pid_controller.as_mut() {
                    pid.sp = self.pos.0.z + height_offset * Vec3::unit_z();
                    controller.inputs.move_z = pid.calc_err();
                } else {
                    controller.inputs.move_z = height_offset;
                }
                // Put away weapon
                if rng.gen_bool(0.1)
                    && matches!(
                        read_data.char_states.get(*self.entity),
                        Some(CharacterState::Wielding(_))
                    )
                {
                    controller.push_action(ControlAction::Unwield);
                }
            }
        } else {
            agent.bearing += Vec2::new(rng.gen::<f32>() - 0.5, rng.gen::<f32>() - 0.5) * 0.1
                - agent.bearing * 0.003
                - agent.patrol_origin.map_or(Vec2::zero(), |patrol_origin| {
                    (self.pos.0 - patrol_origin).xy() * 0.0002
                });

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
                controller.inputs.move_dir = agent.bearing * 0.65;
            }

            // Put away weapon
            if rng.gen_bool(0.1)
                && matches!(
                    read_data.char_states.get(*self.entity),
                    Some(CharacterState::Wielding(_))
                )
            {
                controller.push_action(ControlAction::Unwield);
            }

            if rng.gen::<f32>() < 0.0015 {
                controller.push_utterance(UtteranceKind::Calm);
            }

            // Sit
            if rng.gen::<f32>() < 0.0035 {
                controller.push_action(ControlAction::Sit);
            }
        }
    }

    pub fn follow(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        terrain: &TerrainGrid,
        tgt_pos: &Pos,
    ) {
        if let Some((bearing, speed)) = agent.chaser.chase(
            &*terrain,
            self.pos.0,
            self.vel.0,
            tgt_pos.0,
            TraversalConfig {
                min_tgt_dist: AVG_FOLLOW_DIST,
                ..self.traversal_config
            },
        ) {
            let dist_sqrd = self.pos.0.distance_squared(tgt_pos.0);
            controller.inputs.move_dir = bearing.xy().try_normalized().unwrap_or_else(Vec2::zero)
                * speed.min(0.2 + (dist_sqrd - AVG_FOLLOW_DIST.powi(2)) / 8.0);
            self.jump_if(controller, bearing.z > 1.5);
            controller.inputs.move_z = bearing.z;
        }
    }

    fn recv_interaction(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        read_data: &ReadData,
        event_emitter: &mut Emitter<'_, ServerEvent>,
    ) -> bool {
        // TODO: Process group invites
        // TODO: Add Group AgentEvent
        // let accept = false;  // set back to "matches!(alignment, Alignment::Npc)"
        // when we got better NPC recruitment mechanics if accept {
        //     // Clear agent comp
        //     //*agent = Agent::default();
        //     controller
        //         .push_event(ControlEvent::InviteResponse(InviteResponse::Accept));
        // } else {
        //     controller
        //         .push_event(ControlEvent::InviteResponse(InviteResponse::Decline));
        // }
        agent.action_state.timer += read_data.dt.0;

        let msg = agent.inbox.pop_front();
        match msg {
            Some(AgentEvent::Talk(by, subject)) => {
                if agent.allowed_to_speak() {
                    if let Some(target) = get_entity_by_id(by.id(), read_data) {
                        agent.target = Some(Target::new(target, false, read_data.time.0, false));

                        if self.look_toward(controller, read_data, target) {
                            controller.push_action(ControlAction::Stand);
                            controller.push_action(ControlAction::Talk);
                            controller.push_utterance(UtteranceKind::Greeting);

                            match subject {
                                Subject::Regular => {
                                    if let (
                                        Some((_travel_to, destination_name)),
                                        Some(rtsim_entity),
                                    ) = (&agent.rtsim_controller.travel_to, &self.rtsim_entity)
                                    {
                                        let msg =
                                            if let Some(tgt_stats) = read_data.stats.get(target) {
                                                agent.rtsim_controller.events.push(
                                                    RtSimEvent::AddMemory(Memory {
                                                        item: MemoryItem::CharacterInteraction {
                                                            name: tgt_stats.name.clone(),
                                                        },
                                                        time_to_forget: read_data.time.0 + 600.0,
                                                    }),
                                                );
                                                if rtsim_entity
                                                    .brain
                                                    .remembers_character(&tgt_stats.name)
                                                {
                                                    format!(
                                                        "Greetings fair {}! It has been far too \
                                                         long since last I saw you. I'm going to \
                                                         {} right now.",
                                                        &tgt_stats.name, destination_name
                                                    )
                                                } else {
                                                    format!(
                                                        "I'm heading to {}! Want to come along?",
                                                        destination_name
                                                    )
                                                }
                                            } else {
                                                format!(
                                                    "I'm heading to {}! Want to come along?",
                                                    destination_name
                                                )
                                            };
                                        self.chat_npc(msg, event_emitter);
                                    } else if agent.behavior.can_trade() {
                                        if !agent.behavior.is(BehaviorState::TRADING) {
                                            controller.push_initiate_invite(by, InviteKind::Trade);
                                            self.chat_npc(
                                                "npc.speech.merchant_advertisement",
                                                event_emitter,
                                            );
                                        } else {
                                            self.chat_npc(
                                                "npc.speech.merchant_busy",
                                                event_emitter,
                                            );
                                        }
                                    } else {
                                        self.chat_npc("npc.speech.villager", event_emitter);
                                    }
                                },
                                Subject::Trade => {
                                    if agent.behavior.can_trade() {
                                        if !agent.behavior.is(BehaviorState::TRADING) {
                                            controller.push_initiate_invite(by, InviteKind::Trade);
                                            self.chat_npc(
                                                "npc.speech.merchant_advertisement",
                                                event_emitter,
                                            );
                                        } else {
                                            self.chat_npc(
                                                "npc.speech.merchant_busy",
                                                event_emitter,
                                            );
                                        }
                                    } else {
                                        // TODO: maybe make some travellers willing to trade with
                                        // simpler goods like potions
                                        self.chat_npc(
                                            "npc.speech.villager_decline_trade",
                                            event_emitter,
                                        );
                                    }
                                },
                                Subject::Mood => {
                                    if let Some(rtsim_entity) = self.rtsim_entity {
                                        if !rtsim_entity.brain.remembers_mood() {
                                            // TODO: the following code will need a rework to
                                            // implement more mood contexts
                                            // This require that town NPCs becomes rtsim_entities to
                                            // work fully.
                                            match rand::random::<u32>() % 3 {
                                                0 => agent.rtsim_controller.events.push(
                                                    RtSimEvent::SetMood(Memory {
                                                        item: MemoryItem::Mood {
                                                            state: MoodState::Good(
                                                                MoodContext::GoodWeather,
                                                            ),
                                                        },
                                                        time_to_forget: read_data.time.0 + 21200.0,
                                                    }),
                                                ),
                                                1 => agent.rtsim_controller.events.push(
                                                    RtSimEvent::SetMood(Memory {
                                                        item: MemoryItem::Mood {
                                                            state: MoodState::Neutral(
                                                                MoodContext::EverydayLife,
                                                            ),
                                                        },
                                                        time_to_forget: read_data.time.0 + 21200.0,
                                                    }),
                                                ),
                                                2 => agent.rtsim_controller.events.push(
                                                    RtSimEvent::SetMood(Memory {
                                                        item: MemoryItem::Mood {
                                                            state: MoodState::Bad(
                                                                MoodContext::GoodWeather,
                                                            ),
                                                        },
                                                        time_to_forget: read_data.time.0 + 86400.0,
                                                    }),
                                                ),
                                                _ => {}, // will never happen
                                            }
                                        }
                                        if let Some(memory) = rtsim_entity.brain.get_mood() {
                                            let msg = match &memory.item {
                                                MemoryItem::Mood { state } => state.describe(),
                                                _ => "".to_string(),
                                            };
                                            self.chat_npc(msg, event_emitter);
                                        }
                                    }
                                },
                                Subject::Location(location) => {
                                    if let Some(tgt_pos) = read_data.positions.get(target) {
                                        let raw_dir = location.origin.as_::<f32>() - tgt_pos.0.xy();
                                        let dist = Distance::from_dir(raw_dir).name();
                                        let dir = Direction::from_dir(raw_dir).name();

                                        let msg = format!(
                                            "{} ? I think it's {} {} from here!",
                                            location.name, dist, dir
                                        );
                                        self.chat_npc(msg, event_emitter);
                                    }
                                },
                                Subject::Person(person) => {
                                    if let Some(src_pos) = read_data.positions.get(target) {
                                        let msg = if let Some(person_pos) = person.origin {
                                            let distance = Distance::from_dir(
                                                person_pos.xy() - src_pos.0.xy(),
                                            );
                                            match distance {
                                                Distance::NextTo | Distance::Near => {
                                                    format!(
                                                        "{} ? I think he's {} {} from here!",
                                                        person.name(),
                                                        distance.name(),
                                                        Direction::from_dir(
                                                            person_pos.xy() - src_pos.0.xy(),
                                                        )
                                                        .name()
                                                    )
                                                },
                                                _ => {
                                                    format!(
                                                        "{} ? I think he's gone visiting another \
                                                         town. Come back later!",
                                                        person.name()
                                                    )
                                                },
                                            }
                                        } else {
                                            format!(
                                                "{} ? Sorry, I don't know where you can find him.",
                                                person.name()
                                            )
                                        };
                                        self.chat_npc(msg, event_emitter);
                                    }
                                },
                                Subject::Work => {},
                            }
                        }
                    }
                }
            },
            Some(AgentEvent::TradeInvite(with)) => {
                if agent.behavior.can_trade() {
                    if !agent.behavior.is(BehaviorState::TRADING) {
                        // stand still and looking towards the trading player
                        controller.push_action(ControlAction::Stand);
                        controller.push_action(ControlAction::Talk);
                        if let Some(target) = get_entity_by_id(with.id(), read_data) {
                            agent.target =
                                Some(Target::new(target, false, read_data.time.0, false));
                        }
                        controller.push_invite_response(InviteResponse::Accept);
                        agent.behavior.unset(BehaviorState::TRADING_ISSUER);
                        agent.behavior.set(BehaviorState::TRADING);
                    } else {
                        controller.push_invite_response(InviteResponse::Decline);
                        self.chat_npc_if_allowed_to_speak(
                            "npc.speech.merchant_busy",
                            agent,
                            event_emitter,
                        );
                    }
                } else {
                    // TODO: Provide a hint where to find the closest merchant?
                    controller.push_invite_response(InviteResponse::Decline);
                    self.chat_npc_if_allowed_to_speak(
                        "npc.speech.villager_decline_trade",
                        agent,
                        event_emitter,
                    );
                }
            },
            Some(AgentEvent::TradeAccepted(with)) => {
                if !agent.behavior.is(BehaviorState::TRADING) {
                    if let Some(target) = get_entity_by_id(with.id(), read_data) {
                        agent.target = Some(Target::new(target, false, read_data.time.0, false));
                    }
                    agent.behavior.set(BehaviorState::TRADING);
                    agent.behavior.set(BehaviorState::TRADING_ISSUER);
                }
            },
            Some(AgentEvent::FinishedTrade(result)) => {
                if agent.behavior.is(BehaviorState::TRADING) {
                    match result {
                        TradeResult::Completed => {
                            self.chat_npc("npc.speech.merchant_trade_successful", event_emitter);
                        },
                        _ => {
                            self.chat_npc("npc.speech.merchant_trade_declined", event_emitter);
                        },
                    }
                    agent.behavior.unset(BehaviorState::TRADING);
                }
            },
            Some(AgentEvent::UpdatePendingTrade(boxval)) => {
                let (tradeid, pending, prices, inventories) = *boxval;
                if agent.behavior.is(BehaviorState::TRADING) {
                    let who: usize = if agent.behavior.is(BehaviorState::TRADING_ISSUER) {
                        0
                    } else {
                        1
                    };
                    let balance0: f32 =
                        prices.balance(&pending.offers, &inventories, 1 - who, true);
                    let balance1: f32 = prices.balance(&pending.offers, &inventories, who, false);
                    if balance0 >= balance1 {
                        // If the trade is favourable to us, only send an accept message if we're
                        // not already accepting (since otherwise, spamclicking the accept button
                        // results in lagging and moving to the review phase of an unfavorable trade
                        // (although since the phase is included in the message, this shouldn't
                        // result in fully accepting an unfavourable trade))
                        if !pending.accept_flags[who] && !pending.is_empty_trade() {
                            event_emitter.emit(ServerEvent::ProcessTradeAction(
                                *self.entity,
                                tradeid,
                                TradeAction::Accept(pending.phase),
                            ));
                            tracing::trace!(?tradeid, ?balance0, ?balance1, "Accept Pending Trade");
                        }
                    } else {
                        if balance1 > 0.0 {
                            let msg = format!(
                                "That only covers {:.0}% of my costs!",
                                (balance0 / balance1 * 100.0).floor()
                            );
                            if let Some(tgt_data) = &agent.target {
                                // If talking with someone in particular, "tell" it only to them
                                if let Some(with) = read_data.uids.get(tgt_data.target) {
                                    event_emitter.emit(ServerEvent::Chat(
                                        UnresolvedChatMsg::npc_tell(*self.uid, *with, msg),
                                    ));
                                } else {
                                    event_emitter.emit(ServerEvent::Chat(
                                        UnresolvedChatMsg::npc_say(*self.uid, msg),
                                    ));
                                }
                            } else {
                                event_emitter.emit(ServerEvent::Chat(UnresolvedChatMsg::npc_say(
                                    *self.uid, msg,
                                )));
                            }
                        }
                        if pending.phase != TradePhase::Mutate {
                            // we got into the review phase but without balanced goods, decline
                            agent.behavior.unset(BehaviorState::TRADING);
                            event_emitter.emit(ServerEvent::ProcessTradeAction(
                                *self.entity,
                                tradeid,
                                TradeAction::Decline,
                            ));
                        }
                    }
                }
            },
            Some(AgentEvent::ServerSound(_)) => {},
            Some(AgentEvent::Hurt) => {},
            None => return false,
        }
        true
    }

    fn look_toward(
        &self,
        controller: &mut Controller,
        read_data: &ReadData,
        target: EcsEntity,
    ) -> bool {
        if let Some(tgt_pos) = read_data.positions.get(target) {
            let eye_offset = self.body.map_or(0.0, |b| b.eye_height());
            let tgt_eye_offset = read_data.bodies.get(target).map_or(0.0, |b| b.eye_height());
            if let Some(dir) = Dir::from_unnormalized(
                Vec3::new(tgt_pos.0.x, tgt_pos.0.y, tgt_pos.0.z + tgt_eye_offset)
                    - Vec3::new(self.pos.0.x, self.pos.0.y, self.pos.0.z + eye_offset),
            ) {
                controller.inputs.look_dir = dir;
            }
            true
        } else {
            false
        }
    }

    fn menacing(
        &self,
        agent: &Agent,
        target: EcsEntity,
        controller: &mut Controller,
        read_data: &ReadData,
        event_emitter: &mut Emitter<ServerEvent>,
        rng: &mut impl Rng,
    ) {
        let max_move = 0.5;
        let move_dir = controller.inputs.move_dir;
        let move_dir_mag = move_dir.magnitude();
        let small_chance = rng.gen::<f32>() < read_data.dt.0 * 0.25;

        self.look_toward(controller, read_data, target);
        controller.push_action(ControlAction::Wield);

        if move_dir_mag > max_move {
            controller.inputs.move_dir = max_move * move_dir / move_dir_mag;
        }

        if small_chance {
            self.chat_npc_if_allowed_to_speak("npc.speech.menacing", agent, event_emitter);
            controller.push_utterance(UtteranceKind::Angry);
        }
    }

    fn flee(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        terrain: &TerrainGrid,
        tgt_pos: &Pos,
    ) {
        if let Some(body) = self.body {
            if body.can_strafe() && !self.is_gliding {
                controller.push_action(ControlAction::Unwield);
            }
        }
        if let Some((bearing, speed)) = agent.chaser.chase(
            &*terrain,
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
        ) {
            controller.inputs.move_dir =
                bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
            self.jump_if(controller, bearing.z > 1.5);
            controller.inputs.move_z = bearing.z;
        }
    }

    /// Attempt to consume a healing item, and return whether any healing items
    /// were queued. Callers should use this to implement a delay so that
    /// the healing isn't interrupted. If `relaxed` is `true`, we allow eating
    /// food and prioritise healing.
    fn heal_self(&self, _agent: &mut Agent, controller: &mut Controller, relaxed: bool) -> bool {
        let healing_value = |item: &Item| {
            let mut value = 0.0;

            if let ItemKind::Consumable { kind, effects, .. } = &item.kind {
                if matches!(kind, ConsumableKind::Drink)
                    || (relaxed && matches!(kind, ConsumableKind::Food))
                {
                    for effect in effects.iter() {
                        use BuffKind::*;
                        match effect {
                            Effect::Health(HealthChange { amount, .. }) => {
                                value += *amount;
                            },
                            Effect::Buff(BuffEffect { kind, data, .. })
                                if matches!(kind, Regeneration | Saturation | Potion) =>
                            {
                                value += data.strength
                                    * data.duration.map_or(0.0, |d| d.as_secs() as f32);
                            },
                            _ => {},
                        }
                    }
                }
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

    fn choose_target(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        read_data: &ReadData,
        event_emitter: &mut Emitter<'_, ServerEvent>,
    ) {
        agent.action_state.timer = 0.0;
        let mut aggro_on = false;

        let worth_choosing = |entity| {
            read_data
                .positions
                .get(entity)
                .and_then(|pos| read_data.healths.get(entity).map(|h| (pos, h)))
                .and_then(|(pos, health)| {
                    read_data
                        .stats
                        .get(entity)
                        .map(|stats| (pos, health, stats))
                })
                .and_then(|(pos, health, stats)| {
                    read_data
                        .inventories
                        .get(entity)
                        .map(|inventory| (pos, health, stats, inventory))
                })
                .map(|(pos, health, stats, inventory)| {
                    (
                        entity,
                        pos,
                        health,
                        stats,
                        inventory,
                        read_data.alignments.get(entity),
                        read_data.char_states.get(entity),
                        read_data.bodies.get(entity),
                    )
                })
        };

        let max_search_dist = agent.psyche.search_dist();
        let max_sight_dist = agent.psyche.sight_dist;
        let max_listen_dist = agent.psyche.listen_dist;
        let in_sight_dist =
            |e_pos: &Pos, e_char_state: Option<&CharacterState>, inventory: &Inventory| {
                let search_dist = max_sight_dist
                    / if e_char_state.map_or(false, CharacterState::is_stealthy) {
                        combat::compute_stealth_coefficient(Some(inventory))
                    } else {
                        1.0
                    };
                e_pos.0.distance_squared(self.pos.0) < search_dist.powi(2)
            };

        let within_fov = |e_pos: &Pos| {
            (e_pos.0 - self.pos.0)
                .try_normalized()
                .map_or(true, |v| v.dot(*controller.inputs.look_dir) > 0.15)
        };

        let in_listen_dist =
            |e_pos: &Pos, e_char_state: Option<&CharacterState>, inventory: &Inventory| {
                let listen_dist = max_listen_dist
                    / if e_char_state.map_or(false, CharacterState::is_stealthy) {
                        combat::compute_stealth_coefficient(Some(inventory))
                    } else {
                        1.0
                    };
                // TODO implement proper sound system for agents
                e_pos.0.distance_squared(self.pos.0) < listen_dist.powi(2)
            };

        let within_reach =
            |e_pos: &Pos, e_char_state: Option<&CharacterState>, e_inventory: &Inventory| {
                (in_sight_dist(e_pos, e_char_state, e_inventory) && within_fov(e_pos))
                    || in_listen_dist(e_pos, e_char_state, e_inventory)
            };

        let is_owner_hostile = |e_alignment: Option<&Alignment>| {
            try_owner_alignment(self.alignment, read_data).map_or(false, |owner_alignment| {
                try_owner_alignment(e_alignment, read_data).map_or(false, |e_owner_alignment| {
                    owner_alignment.hostile_towards(*e_owner_alignment)
                })
            })
        };

        let guard_other =
            |e_health: &Health, e_body: Option<&Body>, e_alignment: Option<&Alignment>| {
                let i_am_a_guard = read_data
                    .stats
                    .get(*self.entity)
                    .map_or(false, |stats| stats.name == "Guard");
                let other_is_a_villager = matches!(e_alignment, Some(Alignment::Npc));
                let we_are_friendly: bool = self.alignment.map_or(false, |ma| {
                    e_alignment.map_or(false, |ea| !ea.hostile_towards(*ma))
                });
                let we_share_species: bool = self.body.map_or(false, |mb| {
                    e_body.map_or(false, |eb| {
                        eb.is_same_species_as(mb) || (eb.is_humanoid() && mb.is_humanoid())
                    })
                });
                let i_own_other =
                    matches!(e_alignment, Some(Alignment::Owned(ouid)) if self.uid == ouid);
                let other_has_taken_damage = read_data.time.0 - e_health.last_change.time.0 < 5.0;
                let attacker_of = |health: &Health| health.last_change.damage_by();

                let i_should_defend = other_has_taken_damage
                    && ((we_are_friendly && we_share_species)
                        || (i_am_a_guard && other_is_a_villager)
                        || i_own_other);

                i_should_defend
                    .then(|| {
                        attacker_of(e_health)
                            .and_then(|damage_contributor| {
                                get_entity_by_id(damage_contributor.uid().0, read_data)
                            })
                            .and_then(|attacker| {
                                read_data.alignments.get(attacker).and_then(|aa| {
                                    self.alignment.and_then({
                                        |ma| {
                                            if !ma.passive_towards(*aa) {
                                                read_data
                                                    .positions
                                                    .get(attacker)
                                                    .map(|a_pos| (attacker, *a_pos))
                                            } else {
                                                None
                                            }
                                        }
                                    })
                                })
                            })
                    })
                    .flatten()
            };

        let rtsim_remember =
            |target_stats: &Stats,
             agent: &mut Agent,
             event_emitter: &mut Emitter<'_, ServerEvent>| {
                self.rtsim_entity.map_or(false, |rtsim_entity| {
                    if rtsim_entity
                        .brain
                        .remembers_fight_with_character(&target_stats.name)
                    {
                        agent.add_enemy(&target_stats.name, read_data.time.0);
                        self.chat_npc_if_allowed_to_speak(
                            format!(
                                "{}! How dare you cross me again!",
                                target_stats.name.clone()
                            ),
                            agent,
                            event_emitter,
                        );
                        true
                    } else {
                        false
                    }
                })
            };

        let npc_sees_cultist =
            |target_stats: &Stats,
             target_inventory: &Inventory,
             agent: &mut Agent,
             event_emitter: &mut Emitter<'_, ServerEvent>| {
                self.alignment.map_or(false, |alignment| {
                    if matches!(alignment, Alignment::Npc)
                        && target_inventory
                            .equipped_items()
                            .filter(|item| item.tags().contains(&ItemTag::Cultist))
                            .count()
                            > 2
                    {
                        if self.rtsim_entity.is_some() {
                            agent.add_enemy(&target_stats.name, read_data.time.0);
                        }

                        self.chat_npc_if_allowed_to_speak(
                            "npc.speech.villager_cultist_alarm",
                            agent,
                            event_emitter,
                        );

                        true
                    } else {
                        false
                    }
                })
            };

        let possible_target = |(
            entity,
            e_pos,
            e_health,
            e_stats,
            e_inventory,
            e_alignment,
            e_char_state,
            e_body,
        ): (
            EcsEntity,
            &Pos,
            &Health,
            &Stats,
            &Inventory,
            Option<&Alignment>,
            Option<&CharacterState>,
            Option<&Body>,
        )| {
            let can_target = within_reach(e_pos, e_char_state, e_inventory)
                && entity != *self.entity
                && !e_health.is_dead
                && !is_invulnerable(entity, read_data);

            if !can_target {
                None
            } else if is_owner_hostile(e_alignment) {
                Some((entity, *e_pos))
            } else if let Some(villain_info) = guard_other(e_health, e_body, e_alignment) {
                aggro_on = true;
                Some(villain_info)
            } else if rtsim_remember(e_stats, agent, event_emitter)
                || npc_sees_cultist(e_stats, e_inventory, agent, event_emitter)
            {
                Some((entity, *e_pos))
            } else {
                None
            }
        };

        // Search area
        // TODO choose target by more than just distance
        let common::CachedSpatialGrid(grid) = self.cached_spatial_grid;
        let target = grid
            .in_circle_aabr(self.pos.0.xy(), max_search_dist)
            .filter_map(worth_choosing)
            .filter_map(possible_target)
            // TODO: This seems expensive. Cache this to avoid recomputing each tick
            .filter(|(_, e_pos)| can_see_tgt(&read_data.terrain, self.pos, e_pos, e_pos.0.distance_squared(self.pos.0)))
            .min_by_key(|(_, e_pos)| (e_pos.0.distance_squared(self.pos.0) * 100.0) as i32)
            .map(|(e, _)| e);

        if agent.target.is_none() && target.is_some() {
            if aggro_on {
                controller.push_utterance(UtteranceKind::Angry);
            } else {
                controller.push_utterance(UtteranceKind::Surprised);
            }
        }

        agent.target = target.map(|target| Target {
            target,
            hostile: true,
            selected_at: read_data.time.0,
            aggro_on,
        });
    }

    fn attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        tgt_data: &TargetData,
        read_data: &ReadData,
        rng: &mut impl Rng,
    ) {
        let tool_tactic = |tool_kind| match tool_kind {
            ToolKind::Bow => Tactic::Bow,
            ToolKind::Staff => Tactic::Staff,
            ToolKind::Sceptre => Tactic::Sceptre,
            ToolKind::Hammer => Tactic::Hammer,
            ToolKind::Sword | ToolKind::Spear | ToolKind::Blowgun => Tactic::Sword,
            ToolKind::Axe => Tactic::Axe,
            _ => Tactic::SimpleMelee,
        };

        let tactic = self
            .inventory
            .equipped(EquipSlot::ActiveMainhand)
            .as_ref()
            .map(|item| {
                if let Some(ability_spec) = item.ability_spec() {
                    match ability_spec {
                        AbilitySpec::Custom(spec) => match spec.as_str() {
                            "Oni" | "Sword Simple" => Tactic::Sword,
                            "Staff Simple" => Tactic::Staff,
                            "Bow Simple" => Tactic::Bow,
                            "Stone Golem" => Tactic::StoneGolem,
                            "Quad Med Quick" => Tactic::CircleCharge {
                                radius: 3,
                                circle_time: 2,
                            },
                            "Quad Med Jump" => Tactic::QuadMedJump,
                            "Quad Med Charge" => Tactic::CircleCharge {
                                radius: 6,
                                circle_time: 1,
                            },
                            "Quad Med Basic" => Tactic::QuadMedBasic,
                            "Asp" | "Maneater" => Tactic::QuadLowRanged,
                            "Quad Low Breathe" | "Quad Low Beam" | "Basilisk" => {
                                Tactic::QuadLowBeam
                            },
                            "Quad Low Tail" | "Husk Brute" => Tactic::TailSlap,
                            "Quad Low Quick" => Tactic::QuadLowQuick,
                            "Quad Low Basic" => Tactic::QuadLowBasic,
                            "Theropod Basic" | "Theropod Bird" => Tactic::Theropod,
                            "Arthropod Basic" => Tactic::ArthropodBasic,
                            "Arthropod Charge" => Tactic::ArthropodCharge,
                            "Arthropod Ranged" => Tactic::ArthropodRanged,
                            "Arthropod Leap" => Tactic::ArthropodLeap,
                            "Theropod Charge" => Tactic::CircleCharge {
                                radius: 6,
                                circle_time: 1,
                            },
                            "Turret" => Tactic::Turret,
                            "Haniwa Sentry" => Tactic::RotatingTurret,
                            "Bird Large Breathe" => Tactic::BirdLargeBreathe,
                            "Bird Large Fire" => Tactic::BirdLargeFire,
                            "Bird Large Basic" => Tactic::BirdLargeBasic,
                            "Mindflayer" => Tactic::Mindflayer,
                            "Minotaur" => Tactic::Minotaur,
                            "Clay Golem" => Tactic::ClayGolem,
                            "Tidal Warrior" => Tactic::TidalWarrior,
                            "Tidal Totem"
                            | "Tornado"
                            | "Gnarling Totem Red"
                            | "Gnarling Totem Green"
                            | "Gnarling Totem White" => Tactic::RadialTurret,
                            "Yeti" => Tactic::Yeti,
                            "Harvester" => Tactic::Harvester,
                            "Gnarling Dagger" => Tactic::SimpleBackstab,
                            "Gnarling Blowgun" => Tactic::ElevatedRanged,
                            "Deadwood" => Tactic::Deadwood,
                            "Mandragora" => Tactic::Mandragora,
                            "Wood Golem" => Tactic::WoodGolem,
                            "Gnarling Chieftain" => Tactic::GnarlingChieftain,
                            _ => Tactic::SimpleMelee,
                        },
                        AbilitySpec::Tool(tool_kind) => tool_tactic(*tool_kind),
                    }
                } else if let ItemKind::Tool(tool) = &item.kind() {
                    tool_tactic(tool.kind)
                } else {
                    Tactic::SimpleMelee
                }
            })
            .unwrap_or(Tactic::SimpleMelee);

        // Wield the weapon as running towards the target
        controller.push_action(ControlAction::Wield);

        let min_attack_dist = (self.body.map_or(0.5, |b| b.max_radius()) + DEFAULT_ATTACK_RANGE)
            * self.scale
            + tgt_data.body.map_or(0.5, |b| b.max_radius()) * tgt_data.scale.map_or(1.0, |s| s.0);
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

        let eye_offset = self.body.map_or(0.0, |b| b.eye_height());

        let tgt_eye_height = tgt_data.body.map_or(0.0, |b| b.eye_height());
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
                            body.projectile_offsets(self.ori.look_vec())
                        }),
                    Vec3::new(
                        tgt_data.pos.0.x,
                        tgt_data.pos.0.y,
                        tgt_data.pos.0.z + tgt_eye_offset,
                    ),
                )
            },
            CharacterState::BasicRanged(c) => {
                let offset_z = match c.static_data.projectile {
                    // Aim fireballs at feet instead of eyes for splash damage
                    ProjectileConstructor::Fireball {
                        damage: _,
                        radius: _,
                        energy_regen: _,
                        min_falloff: _,
                    } => 0.0,
                    _ => tgt_eye_offset,
                };
                let projectile_speed = c.static_data.projectile_speed;
                aim_projectile(
                    projectile_speed,
                    self.pos.0
                        + self.body.map_or(Vec3::zero(), |body| {
                            body.projectile_offsets(self.ori.look_vec())
                        }),
                    Vec3::new(
                        tgt_data.pos.0.x,
                        tgt_data.pos.0.y,
                        tgt_data.pos.0.z + offset_z,
                    ),
                )
            },
            CharacterState::RepeaterRanged(c) => {
                let projectile_speed = c.static_data.projectile_speed;
                aim_projectile(
                    projectile_speed,
                    self.pos.0
                        + self.body.map_or(Vec3::zero(), |body| {
                            body.projectile_offsets(self.ori.look_vec())
                        }),
                    Vec3::new(
                        tgt_data.pos.0.x,
                        tgt_data.pos.0.y,
                        tgt_data.pos.0.z + tgt_eye_offset,
                    ),
                )
            },
            CharacterState::LeapMelee(_) if matches!(tactic, Tactic::Hammer | Tactic::Axe) => {
                let direction_weight = match tactic {
                    Tactic::Hammer => 0.1,
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
            min_attack_dist,
            dist_sqrd,
            angle,
            angle_xy,
        };

        // Match on tactic. Each tactic has different controls
        // depending on the distance from the agent to the target
        match tactic {
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
            Tactic::QuadLowBeam => self.handle_quadlow_beam_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::Theropod => {
                self.handle_theropod_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::ArthropodBasic => self.handle_arthropod_basic_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::ArthropodCharge => self.handle_arthropod_charge_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::ArthropodLeap => self.handle_arthropod_leap_attack(
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
            Tactic::RotatingTurret => self.handle_rotating_turret_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::Mindflayer => self.handle_mindflayer_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
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
            Tactic::Minotaur => {
                self.handle_minotaur_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::ClayGolem => {
                self.handle_clay_golem_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::TidalWarrior => self.handle_tidal_warrior_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::RadialTurret => self.handle_radial_turret_attack(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
            ),
            Tactic::Yeti => {
                self.handle_yeti_attack(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::Harvester => {
                self.handle_harvester_attack(agent, controller, &attack_data, tgt_data, read_data)
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
                self.handle_wood_golem(agent, controller, &attack_data, tgt_data, read_data)
            },
            Tactic::GnarlingChieftain => self.handle_gnarling_chieftain(
                agent,
                controller,
                &attack_data,
                tgt_data,
                read_data,
                rng,
            ),
        }
    }

    fn handle_elevated_awareness(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        read_data: &ReadData,
        rng: &mut impl Rng,
    ) {
        if is_invulnerable(*self.entity, read_data) {
            self.idle(agent, controller, read_data, rng);
            return;
        }

        if let Some(sound) = agent.sounds_heard.last() {
            if let Some(agent_stats) = read_data.stats.get(*self.entity) {
                let sound_pos = Pos(sound.pos);
                let dist_sqrd = self.pos.0.distance_squared(sound_pos.0);

                // FIXME: We need to be able to change the name of a guard without breaking this
                // logic The `Mark` enum from common::agent could be used to
                // match with `agent::Mark::Guard`
                let is_village_guard = agent_stats.name == *"Guard".to_string();
                let is_enemy = matches!(self.alignment, Some(Alignment::Enemy));

                if is_enemy {
                    let far_enough = dist_sqrd > 10.0_f32.powi(2);

                    if far_enough {
                        self.follow(agent, controller, &read_data.terrain, &sound_pos);
                    } else {
                        // TODO: Change this to a search action instead of idle
                        self.idle(agent, controller, read_data, rng);
                    }
                } else if is_village_guard {
                    self.follow(agent, controller, &read_data.terrain, &sound_pos);
                } else if !is_village_guard {
                    let flee_health = agent.psyche.flee_health;
                    let close_enough = dist_sqrd < 35.0_f32.powi(2);
                    let sound_was_loud = sound.vol >= 10.0;

                    if close_enough
                        && (flee_health <= 0.7 || (flee_health <= 0.5 && sound_was_loud))
                    {
                        self.flee(agent, controller, &read_data.terrain, &sound_pos);
                    } else {
                        self.idle(agent, controller, read_data, rng);
                    }
                } else {
                    // TODO: Change this to a search action instead of idle
                    self.idle(agent, controller, read_data, rng);
                }
            }
        }
    }

    fn attack_target_attacker(
        &self,
        agent: &mut Agent,
        read_data: &ReadData,
        controller: &mut Controller,
        rng: &mut impl Rng,
    ) {
        if let Some(Target { target, .. }) = agent.target {
            if let Some(tgt_health) = read_data.healths.get(target) {
                if let Some(by) = tgt_health.last_change.damage_by() {
                    if let Some(attacker) = get_entity_by_id(by.uid().0, read_data) {
                        if agent.target.is_none() {
                            controller.push_utterance(UtteranceKind::Angry);
                        }

                        agent.target = Some(Target::new(attacker, true, read_data.time.0, true));

                        if let Some(tgt_pos) = read_data.positions.get(attacker) {
                            if is_dead_or_invulnerable(attacker, read_data) {
                                agent.target =
                                    Some(Target::new(target, false, read_data.time.0, false));

                                self.idle(agent, controller, read_data, rng);
                            } else {
                                let target_data = TargetData::new(
                                    tgt_pos,
                                    read_data.bodies.get(target),
                                    read_data.scales.get(target),
                                );

                                self.attack(agent, controller, &target_data, read_data, rng);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Directs the entity to path and move toward the target
    /// If full_path is false, the entity will path to a location 50 units along
    /// the vector between the entity and the target. The speed multiplier
    /// multiplies the movement speed by a value less than 1.0.
    /// A `None` value implies a multiplier of 1.0.
    /// Returns `false` if the pathfinding algorithm fails to return a path
    fn path_toward_target(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        tgt_data: &TargetData,
        read_data: &ReadData,
        full_path: bool,
        separate: bool,
        speed_multiplier: Option<f32>,
    ) -> bool {
        let pathing_pos = if separate {
            let mut sep_vec: Vec3<f32> = Vec3::<f32>::zero();

            for entity in read_data
                .cached_spatial_grid
                .0
                .in_circle_aabr(self.pos.0.xy(), SEPARATION_DIST)
            {
                if let (Some(alignment), Some(other_alignment)) =
                    (self.alignment, read_data.alignments.get(entity))
                {
                    if Alignment::passive_towards(*alignment, *other_alignment) {
                        if let (Some(pos), Some(body), Some(other_body)) = (
                            read_data.positions.get(entity),
                            self.body,
                            read_data.bodies.get(entity),
                        ) {
                            if self.pos.0.xy().distance(pos.0.xy())
                                < body.spacing_radius() + other_body.spacing_radius()
                            {
                                sep_vec += (self.pos.0.xy() - pos.0.xy())
                                    .try_normalized()
                                    .unwrap_or_else(Vec2::zero)
                                    * (((body.spacing_radius() + other_body.spacing_radius())
                                        - self.pos.0.xy().distance(pos.0.xy()))
                                        / (body.spacing_radius() + other_body.spacing_radius()));
                            }
                        }
                    }
                }
            }
            self.pos.0
                + PARTIAL_PATH_DIST
                    * (sep_vec * SEPARATION_BIAS
                        + (tgt_data.pos.0 - self.pos.0) * (1.0 - SEPARATION_BIAS))
                        .try_normalized()
                        .unwrap_or_else(Vec3::zero)
        } else if full_path {
            tgt_data.pos.0
        } else {
            self.pos.0
                + PARTIAL_PATH_DIST
                    * (tgt_data.pos.0 - self.pos.0)
                        .try_normalized()
                        .unwrap_or_else(Vec3::zero)
        };
        let speed_multiplier = speed_multiplier.unwrap_or(1.0).min(1.0);
        if let Some((bearing, speed)) = agent.chaser.chase(
            &*read_data.terrain,
            self.pos.0,
            self.vel.0,
            pathing_pos,
            TraversalConfig {
                min_tgt_dist: 1.25,
                ..self.traversal_config
            },
        ) {
            controller.inputs.move_dir =
                bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed * speed_multiplier;
            self.jump_if(controller, bearing.z > 1.5);
            controller.inputs.move_z = bearing.z;
            true
        } else {
            false
        }
    }

    fn chat_npc_if_allowed_to_speak(
        &self,
        msg: impl ToString,
        agent: &Agent,
        event_emitter: &mut Emitter<'_, ServerEvent>,
    ) -> bool {
        if agent.allowed_to_speak() {
            self.chat_npc(msg, event_emitter);
            true
        } else {
            false
        }
    }

    fn jump_if(&self, controller: &mut Controller, condition: bool) {
        if condition {
            controller.push_basic_input(InputKind::Jump);
        } else {
            controller.push_cancel_input(InputKind::Jump)
        }
    }

    fn chat_npc(&self, msg: impl ToString, event_emitter: &mut Emitter<'_, ServerEvent>) {
        event_emitter.emit(ServerEvent::Chat(UnresolvedChatMsg::npc(
            *self.uid,
            msg.to_string(),
        )));
    }

    fn emit_scream(&self, time: f64, event_emitter: &mut Emitter<'_, ServerEvent>) {
        if let Some(body) = self.body {
            event_emitter.emit(ServerEvent::Sound {
                sound: Sound::new(
                    SoundKind::Utterance(UtteranceKind::Scream, *body),
                    self.pos.0,
                    100.0,
                    time,
                ),
            });
        }
    }

    fn cry_out(&self, agent: &Agent, time: f64, event_emitter: &mut Emitter<'_, ServerEvent>) {
        let is_enemy = matches!(self.alignment, Some(Alignment::Enemy));
        // FIXME: This is not necessarily a "villager"
        let is_villager = matches!(self.alignment, Some(Alignment::Npc));

        if is_enemy {
            self.chat_npc_if_allowed_to_speak(
                "npc.speech.cultist_low_health_fleeing",
                agent,
                event_emitter,
            );
        } else if is_villager {
            self.chat_npc_if_allowed_to_speak(
                "npc.speech.villager_under_attack",
                agent,
                event_emitter,
            );
            self.emit_scream(time, event_emitter);
        }
    }

    fn exclaim_relief_about_enemy_dead(
        &self,
        agent: &Agent,
        event_emitter: &mut Emitter<'_, ServerEvent>,
    ) {
        let is_villager = matches!(self.alignment, Some(Alignment::Npc));

        if is_villager {
            self.chat_npc_if_allowed_to_speak(
                "npc.speech.villager_enemy_killed",
                agent,
                event_emitter,
            );
        }
    }
}
