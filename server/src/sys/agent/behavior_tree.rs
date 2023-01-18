use crate::rtsim::Entity as RtSimEntity;
use common::{
    comp::{
        agent::{
            AgentEvent, AwarenessState, Target, TimerAction, DEFAULT_INTERACTION_TIME,
            TRADE_INTERACTION_TIME,
        },
        Agent, Alignment, BehaviorCapability, BehaviorState, Body, BuffKind, ControlAction,
        ControlEvent, Controller, InputKind, InventoryEvent, Pos, UtteranceKind,
    },
    event::{Emitter, ServerEvent},
    path::TraversalConfig,
};
use rand::{prelude::ThreadRng, thread_rng, Rng};
use specs::{
    saveload::{Marker, MarkerAllocator},
    Entity as EcsEntity,
};
use vek::{Vec2, Vec3};

use self::interaction::{
    handle_inbox_cancel_interactions, handle_inbox_finished_trade, handle_inbox_talk,
    handle_inbox_trade_accepted, handle_inbox_trade_invite, handle_inbox_update_pending_trade,
    increment_timer_deltatime, process_inbox_interaction, process_inbox_sound_and_hurt,
};

use super::{
    consts::{
        DAMAGE_MEMORY_DURATION, FLEE_DURATION, HEALING_ITEM_THRESHOLD, MAX_FOLLOW_DIST,
        NORMAL_FLEE_DIR_DIST, NPC_PICKUP_RANGE, RETARGETING_THRESHOLD_SECONDS,
        STD_AWARENESS_DECAY_RATE,
    },
    data::{AgentData, ReadData, TargetData},
    util::{get_entity_by_id, is_dead, is_dead_or_invulnerable, is_invulnerable, stop_pursuing},
};

mod interaction;

/// Struct containing essential data for running a behavior tree
pub struct BehaviorData<'a, 'b, 'c> {
    pub agent: &'a mut Agent,
    pub agent_data: AgentData<'a>,
    // TODO: Move rtsim back into AgentData after rtsim2 when it has a separate crate
    pub rtsim_entity: Option<&'a RtSimEntity>,
    pub read_data: &'a ReadData<'a>,
    pub event_emitter: &'a mut Emitter<'c, ServerEvent>,
    pub controller: &'a mut Controller,
    pub rng: &'b mut ThreadRng,
}

/// Behavior function
/// Determines if the current situation can be handled and act accordingly
/// Returns true if an action has been taken, stopping the tree execution
type BehaviorFn = fn(&mut BehaviorData) -> bool;

/// ~~list~~ ""tree"" of behavior functions
/// This struct will allow you to run through multiple behavior function until
/// one finally handles an event
pub struct BehaviorTree {
    tree: Vec<BehaviorFn>,
}

/// Enumeration of the timers used by the behavior tree.
// FIXME: We shouldnt have a global timer enumeration for the whole behavior
// tree. It isnt entirely clear where a lot of the agents in some of the bdata
// objects in behavior tree functions come from, so it's hard to granularly
// define these timers per action node. As such, the behavior tree currently has
// one global enumeration for mapping timers in all functions, regardless as to
// use case or action node currently executed -- even if the agent might be
// different between calls. This doesn't break anything as each agent has its
// own instance of timers, but it is much less clear than I would like.
//
// This may require some refactoring to fix, and I don't feel confident doing
// so.
enum ActionStateBehaviorTreeTimers {
    TimerBehaviorTree = 0,
}

impl BehaviorTree {
    /// Base BehaviorTree
    ///
    /// React to immediate dangers (fire, fall & attacks) then call subtrees
    pub fn root() -> Self {
        Self {
            tree: vec![
                react_on_dangerous_fall,
                react_if_on_fire,
                target_if_attacked,
                process_inbox_sound_and_hurt,
                process_inbox_interaction,
                do_target_tree_if_target_else_do_idle_tree,
            ],
        }
    }

    /// Target BehaviorTree
    ///
    /// React to the agent's target.
    /// Either redirect to hostile or pet tree
    pub fn target() -> Self {
        Self {
            tree: vec![
                update_last_known_pos,
                untarget_if_dead,
                update_target_awareness,
                search_last_known_pos_if_not_alert,
                do_hostile_tree_if_hostile_and_aware,
                do_pet_tree_if_owned,
                do_pickup_loot,
                do_idle_tree,
            ],
        }
    }

    /// Pet BehaviorTree
    ///
    /// Follow the owner and attack enemies
    pub fn pet() -> Self {
        Self {
            tree: vec![follow_if_far_away, attack_if_owner_hurt, do_idle_tree],
        }
    }

    /// Interaction BehaviorTree
    ///
    /// Either process the inbox for talk and trade events if the agent can
    /// talk. If not, or if we are in combat, deny all talk and trade
    /// events.
    pub fn interaction(agent: &Agent) -> Self {
        let is_in_combat = agent.target.map_or(false, |t| t.hostile);
        if !is_in_combat
            && (agent.behavior.can(BehaviorCapability::SPEAK)
                || agent.behavior.can(BehaviorCapability::TRADE))
        {
            let mut tree: Vec<BehaviorFn> = vec![increment_timer_deltatime];
            if agent.behavior.can(BehaviorCapability::SPEAK) {
                tree.push(handle_inbox_talk);
            }
            tree.extend_from_slice(&[
                handle_inbox_trade_invite,
                handle_inbox_trade_accepted,
                handle_inbox_finished_trade,
                handle_inbox_update_pending_trade,
            ]);
            Self { tree }
        } else {
            Self {
                tree: vec![handle_inbox_cancel_interactions],
            }
        }
    }

    /// Hostile BehaviorTree
    ///
    /// Attack the target, and heal self if applicable
    pub fn hostile() -> Self {
        Self {
            tree: vec![heal_self_if_hurt, hurt_utterance, do_combat],
        }
    }

    /// Idle BehaviorTree
    pub fn idle() -> Self {
        Self {
            tree: vec![set_owner_if_no_target, handle_timed_events],
        }
    }

    /// Run the behavior tree until an event has been handled
    pub fn run(&self, behavior_data: &mut BehaviorData) -> bool {
        for behavior_fn in self.tree.iter() {
            if behavior_fn(behavior_data) {
                return true;
            }
        }
        false
    }
}

/// If falling velocity is critical, throw everything
/// and save yourself!
///
/// If can fly - fly.
/// If have glider - glide.
/// Else, rest in peace.
fn react_on_dangerous_fall(bdata: &mut BehaviorData) -> bool {
    // Falling damage starts from 30.0 as of time of writing
    // But keep in mind our 25 m/s gravity
    let is_falling_dangerous = bdata.agent_data.vel.0.z < -20.0;

    if is_falling_dangerous && bdata.agent_data.traversal_config.can_fly {
        bdata.agent_data.fly_upward(bdata.controller);
        return true;
    } else if is_falling_dangerous && bdata.agent_data.glider_equipped {
        bdata.agent_data.glider_fall(bdata.controller);
        return true;
    }
    false
}

/// If on fire and able, stop, drop, and roll
fn react_if_on_fire(bdata: &mut BehaviorData) -> bool {
    let is_on_fire = bdata
        .read_data
        .buffs
        .get(*bdata.agent_data.entity)
        .map_or(false, |b| b.kinds.contains_key(&BuffKind::Burning));

    if is_on_fire
        && bdata.agent_data.body.map_or(false, |b| b.is_humanoid())
        && bdata.agent_data.physics_state.on_ground.is_some()
        && bdata
            .rng
            .gen_bool((2.0 * bdata.read_data.dt.0).clamp(0.0, 1.0) as f64)
    {
        bdata.controller.inputs.move_dir = bdata
            .agent_data
            .ori
            .look_vec()
            .xy()
            .try_normalized()
            .unwrap_or_else(Vec2::zero);
        bdata.controller.push_basic_input(InputKind::Roll);
        return true;
    }
    false
}

/// Target an entity that's attacking us if the attack was recent and we have
/// a health component
fn target_if_attacked(bdata: &mut BehaviorData) -> bool {
    match bdata.agent_data.health {
        Some(health)
            if bdata.read_data.time.0 - health.last_change.time.0 < DAMAGE_MEMORY_DURATION
                && health.last_change.amount < 0.0 =>
        {
            if let Some(by) = health.last_change.damage_by() {
                if let Some(attacker) = bdata
                    .read_data
                    .uid_allocator
                    .retrieve_entity_internal(by.uid().0)
                {
                    // If target is dead or invulnerable (for now, this only
                    // means safezone), untarget them and idle.
                    if is_dead_or_invulnerable(attacker, bdata.read_data) {
                        bdata.agent.target = None;
                    } else {
                        if bdata.agent.target.is_none() {
                            bdata
                                .controller
                                .push_event(ControlEvent::Utterance(UtteranceKind::Angry));
                        }

                        bdata.agent.awareness.change_by(1.0);

                        // Determine whether the new target should be a priority
                        // over the old one (i.e: because it's either close or
                        // because they attacked us).
                        if bdata.agent.target.map_or(true, |target| {
                            bdata.agent_data.is_more_dangerous_than_target(
                                attacker,
                                target,
                                bdata.read_data,
                            )
                        }) {
                            bdata.agent.target = Some(Target {
                                target: attacker,
                                hostile: true,
                                selected_at: bdata.read_data.time.0,
                                aggro_on: true,
                                last_known_pos: bdata
                                    .read_data
                                    .positions
                                    .get(attacker)
                                    .map(|pos| pos.0),
                            });
                        }

                        // Remember this attack if we're an RtSim entity
                        if let Some(attacker_stats) =
                            bdata.rtsim_entity.and(bdata.read_data.stats.get(attacker))
                        {
                            bdata
                                .agent
                                .add_fight_to_memory(&attacker_stats.name, bdata.read_data.time.0);
                        }
                    }
                }
            }
        },
        _ => {},
    }
    false
}

/// If the agent has a target, do the target tree, else do the idle tree
///
/// This function will never stop the BehaviorTree
fn do_target_tree_if_target_else_do_idle_tree(bdata: &mut BehaviorData) -> bool {
    if bdata.agent.target.is_some() {
        BehaviorTree::target().run(bdata);
    } else {
        BehaviorTree::idle().run(bdata);
    }
    false
}

/// Run the Idle BehaviorTree
///
/// This function can stop the BehaviorTree
fn do_idle_tree(bdata: &mut BehaviorData) -> bool { BehaviorTree::idle().run(bdata) }

/// If target is dead, forget them
fn untarget_if_dead(bdata: &mut BehaviorData) -> bool {
    if let Some(Target { target, .. }) = bdata.agent.target {
        if let Some(tgt_health) = bdata.read_data.healths.get(target) {
            // If target is dead, forget them
            if tgt_health.is_dead {
                if let Some(tgt_stats) = bdata.rtsim_entity.and(bdata.read_data.stats.get(target)) {
                    bdata.agent.forget_enemy(&tgt_stats.name);
                }
                bdata.agent.target = None;
                return true;
            }
        }
    }
    false
}

/// If target is hostile and agent is aware of target, do the hostile tree and
/// stop the current BehaviorTree
fn do_hostile_tree_if_hostile_and_aware(bdata: &mut BehaviorData) -> bool {
    let alert = bdata.agent.awareness.reached();

    if let Some(Target { hostile, .. }) = bdata.agent.target {
        if alert && hostile {
            BehaviorTree::hostile().run(bdata);
            return true;
        }
    }
    false
}

/// if owned, do the pet tree and stop the current BehaviorTree
fn do_pet_tree_if_owned(bdata: &mut BehaviorData) -> bool {
    if let (Some(Target { target, .. }), Some(Alignment::Owned(uid))) =
        (bdata.agent.target, bdata.agent_data.alignment)
    {
        if bdata.read_data.uids.get(target) == Some(uid) {
            BehaviorTree::pet().run(bdata);
        } else {
            bdata.agent.target = None;
            BehaviorTree::idle().run(bdata);
        }
        return true;
    }
    false
}

/// If the target is an ItemDrop, go pick it up
fn do_pickup_loot(bdata: &mut BehaviorData) -> bool {
    if let Some(Target { target, .. }) = bdata.agent.target {
        if matches!(bdata.read_data.bodies.get(target), Some(Body::ItemDrop(_))) {
            if let Some(tgt_pos) = bdata.read_data.positions.get(target) {
                let dist_sqrd = bdata.agent_data.pos.0.distance_squared(tgt_pos.0);
                if dist_sqrd < NPC_PICKUP_RANGE.powi(2) {
                    if let Some(uid) = bdata.read_data.uids.get(target) {
                        bdata
                            .controller
                            .push_event(ControlEvent::InventoryEvent(InventoryEvent::Pickup(*uid)));
                    }
                    bdata.agent.target = None;
                } else if let Some((bearing, speed)) = bdata.agent.chaser.chase(
                    &*bdata.read_data.terrain,
                    bdata.agent_data.pos.0,
                    bdata.agent_data.vel.0,
                    tgt_pos.0,
                    TraversalConfig {
                        min_tgt_dist: NPC_PICKUP_RANGE - 1.0,
                        ..bdata.agent_data.traversal_config
                    },
                ) {
                    bdata.controller.inputs.move_dir =
                        bearing.xy().try_normalized().unwrap_or_else(Vec2::zero)
                            * speed.min(0.2 + (dist_sqrd - (NPC_PICKUP_RANGE - 1.5).powi(2)) / 8.0);
                    bdata.agent_data.jump_if(bearing.z > 1.5, bdata.controller);
                    bdata.controller.inputs.move_z = bearing.z;
                }
            }
            return true;
        }
    }
    false
}

// If too far away, then follow the target
fn follow_if_far_away(bdata: &mut BehaviorData) -> bool {
    if let Some(Target { target, .. }) = bdata.agent.target {
        if let Some(tgt_pos) = bdata.read_data.positions.get(target) {
            let dist_sqrd = bdata.agent_data.pos.0.distance_squared(tgt_pos.0);

            if dist_sqrd > (MAX_FOLLOW_DIST).powi(2) {
                bdata.agent_data.follow(
                    bdata.agent,
                    bdata.controller,
                    &bdata.read_data.terrain,
                    tgt_pos,
                );
                return true;
            }
        }
    }
    false
}

/// Attack target's attacker (if there is one)
/// Target is the owner in this case
fn attack_if_owner_hurt(bdata: &mut BehaviorData) -> bool {
    if let Some(Target { target, .. }) = bdata.agent.target {
        if bdata.read_data.positions.get(target).is_some() {
            let owner_recently_attacked =
                if let Some(target_health) = bdata.read_data.healths.get(target) {
                    bdata.read_data.time.0 - target_health.last_change.time.0 < 5.0
                        && target_health.last_change.amount < 0.0
                } else {
                    false
                };

            if owner_recently_attacked {
                bdata.agent_data.attack_target_attacker(
                    bdata.agent,
                    bdata.read_data,
                    bdata.controller,
                    bdata.rng,
                );
                return true;
            }
        }
    }
    false
}

/// Set owner if no target
fn set_owner_if_no_target(bdata: &mut BehaviorData) -> bool {
    let small_chance = bdata.rng.gen_bool(0.1);

    if bdata.agent.target.is_none() && small_chance {
        if let Some(Alignment::Owned(owner)) = bdata.agent_data.alignment {
            if let Some(owner) = get_entity_by_id(owner.id(), bdata.read_data) {
                let owner_pos = bdata.read_data.positions.get(owner).map(|pos| pos.0);

                bdata.agent.target = Some(Target::new(
                    owner,
                    false,
                    bdata.read_data.time.0,
                    false,
                    owner_pos,
                ));
            }
        }
    }
    false
}

/// Handle timed events, like looking at the player we are talking to
fn handle_timed_events(bdata: &mut BehaviorData) -> bool {
    let timeout = if bdata.agent.behavior.is(BehaviorState::TRADING) {
        TRADE_INTERACTION_TIME
    } else {
        DEFAULT_INTERACTION_TIME
    };

    match bdata.agent.timer.timeout_elapsed(
        bdata.read_data.time.0,
        TimerAction::Interact,
        timeout as f64,
    ) {
        None => {
            // Look toward the interacting entity for a while
            if let Some(Target { target, .. }) = &bdata.agent.target {
                bdata
                    .agent_data
                    .look_toward(bdata.controller, bdata.read_data, *target);
                bdata.controller.push_action(ControlAction::Talk);
            }
        },
        Some(just_ended) => {
            if just_ended {
                bdata.agent.target = None;
                bdata.controller.push_action(ControlAction::Stand);
            }

            if bdata.rng.gen::<f32>() < 0.1 {
                bdata.agent_data.choose_target(
                    bdata.agent,
                    bdata.controller,
                    bdata.read_data,
                    bdata.event_emitter,
                    will_ambush(bdata.rtsim_entity, &bdata.agent_data),
                );
            } else {
                bdata.agent_data.handle_sounds_heard(
                    bdata.agent,
                    bdata.controller,
                    bdata.read_data,
                    bdata.rng,
                );
            }
        },
    }
    false
}

fn update_last_known_pos(bdata: &mut BehaviorData) -> bool {
    let BehaviorData {
        agent,
        agent_data,
        read_data,
        controller,
        ..
    } = bdata;

    if let Some(target_info) = agent.target {
        let target = target_info.target;

        if let Some(target_pos) = read_data.positions.get(target) {
            if agent_data.detects_other(agent, controller, &target, target_pos, read_data) {
                let updated_pos = Some(target_pos.0);

                let Target {
                    hostile,
                    selected_at,
                    aggro_on,
                    ..
                } = target_info;

                agent.target = Some(Target::new(
                    target,
                    hostile,
                    selected_at,
                    aggro_on,
                    updated_pos,
                ));
            }
        }
    }

    false
}

/// Try to heal self if our damage went below a certain threshold
fn heal_self_if_hurt(bdata: &mut BehaviorData) -> bool {
    if bdata.agent_data.damage < HEALING_ITEM_THRESHOLD
        && bdata
            .agent_data
            .heal_self(bdata.agent, bdata.controller, false)
    {
        bdata.agent.action_state.timers
            [ActionStateBehaviorTreeTimers::TimerBehaviorTree as usize] = 0.01;
        return true;
    }
    false
}

/// Hurt utterances at random upon receiving damage
fn hurt_utterance(bdata: &mut BehaviorData) -> bool {
    if matches!(bdata.agent.inbox.front(), Some(AgentEvent::Hurt)) {
        if bdata.rng.gen::<f32>() < 0.4 {
            bdata.controller.push_utterance(UtteranceKind::Hurt);
        }
        bdata.agent.inbox.pop_front();
    }
    false
}

fn update_target_awareness(bdata: &mut BehaviorData) -> bool {
    let BehaviorData {
        agent,
        agent_data,
        read_data,
        controller,
        ..
    } = bdata;

    let target = agent.target.map(|t| t.target);
    let tgt_pos = target.and_then(|t| read_data.positions.get(t));

    if let (Some(target), Some(tgt_pos)) = (target, tgt_pos) {
        if agent_data.can_see_entity(agent, controller, target, tgt_pos, read_data) {
            agent.awareness.change_by(1.75 * read_data.dt.0);
        } else if agent_data.can_sense_directly_near(tgt_pos) {
            agent.awareness.change_by(0.25);
        } else {
            agent
                .awareness
                .change_by(STD_AWARENESS_DECAY_RATE * read_data.dt.0);
        }
    } else {
        agent
            .awareness
            .change_by(STD_AWARENESS_DECAY_RATE * read_data.dt.0);
    }

    if bdata.agent.awareness.state() == AwarenessState::Unaware
        && !bdata.agent.behavior.is(BehaviorState::TRADING)
    {
        bdata.agent.target = None;
    }

    false
}

fn search_last_known_pos_if_not_alert(bdata: &mut BehaviorData) -> bool {
    let awareness = &bdata.agent.awareness;
    if awareness.reached() || awareness.state() < AwarenessState::Low {
        return false;
    }

    let BehaviorData {
        agent,
        agent_data,
        controller,
        read_data,
        ..
    } = bdata;

    if let Some(target) = agent.target {
        if let Some(last_known_pos) = target.last_known_pos {
            agent_data.follow(agent, controller, &read_data.terrain, &Pos(last_known_pos));

            return true;
        }
    }

    false
}

fn do_combat(bdata: &mut BehaviorData) -> bool {
    let BehaviorData {
        agent,
        agent_data,
        rtsim_entity,
        read_data,
        event_emitter,
        controller,
        rng,
    } = bdata;

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
            let dist_sqrd = agent_data.pos.0.distance_squared(tgt_pos.0);
            let origin_dist_sqrd = match agent.patrol_origin {
                Some(pos) => pos.distance_squared(agent_data.pos.0),
                None => 1.0,
            };

            let own_health_fraction = match agent_data.health {
                Some(val) => val.fraction(),
                None => 1.0,
            };
            let target_health_fraction = match read_data.healths.get(target) {
                Some(val) => val.fraction(),
                None => 1.0,
            };

            let in_aggro_range = agent
                .psyche
                .aggro_dist
                .map_or(true, |ad| dist_sqrd < ad.powi(2));

            if in_aggro_range {
                *aggro_on = true;
            }
            let aggro_on = *aggro_on;

            if agent_data.below_flee_health(agent) {
                let flee_timer_done = agent.action_state.timers
                    [ActionStateBehaviorTreeTimers::TimerBehaviorTree as usize]
                    > FLEE_DURATION;
                let within_normal_flee_dir_dist = dist_sqrd < NORMAL_FLEE_DIR_DIST.powi(2);

                // FIXME: Using action state timer to see if allowed to speak is a hack.
                if agent.action_state.timers
                    [ActionStateBehaviorTreeTimers::TimerBehaviorTree as usize]
                    == 0.0
                {
                    agent_data.cry_out(agent, event_emitter, read_data);
                    agent.action_state.timers
                        [ActionStateBehaviorTreeTimers::TimerBehaviorTree as usize] = 0.01;
                    agent.flee_from_pos = {
                        let random = || thread_rng().gen_range(-1.0..1.0);
                        Some(Pos(
                            agent_data.pos.0 + Vec3::new(random(), random(), random())
                        ))
                    };
                } else if !flee_timer_done {
                    if within_normal_flee_dir_dist {
                        agent_data.flee(agent, controller, tgt_pos, &read_data.terrain);
                    } else if let Some(random_pos) = agent.flee_from_pos {
                        agent_data.flee(agent, controller, &random_pos, &read_data.terrain);
                    } else {
                        agent_data.flee(agent, controller, tgt_pos, &read_data.terrain);
                    }

                    agent.action_state.timers
                        [ActionStateBehaviorTreeTimers::TimerBehaviorTree as usize] +=
                        read_data.dt.0;
                } else {
                    agent.action_state.timers
                        [ActionStateBehaviorTreeTimers::TimerBehaviorTree as usize] = 0.0;
                    agent.target = None;
                    agent.flee_from_pos = None;
                    agent_data.idle(agent, controller, read_data, rng);
                }
            } else if is_dead(target, read_data) {
                agent_data.exclaim_relief_about_enemy_dead(agent, event_emitter);
                agent.target = None;
                agent_data.idle(agent, controller, read_data, rng);
            } else if is_invulnerable(target, read_data)
                || stop_pursuing(
                    dist_sqrd,
                    origin_dist_sqrd,
                    own_health_fraction,
                    target_health_fraction,
                    read_data.time.0 - selected_at,
                    &agent.psyche,
                )
            {
                agent.target = None;
                agent_data.idle(agent, controller, read_data, rng);
            } else {
                let is_time_to_retarget =
                    read_data.time.0 - selected_at > RETARGETING_THRESHOLD_SECONDS;

                if !in_aggro_range && is_time_to_retarget {
                    agent_data.choose_target(
                        agent,
                        controller,
                        read_data,
                        event_emitter,
                        will_ambush(*rtsim_entity, agent_data),
                    );
                }

                if aggro_on {
                    let target_data = TargetData::new(tgt_pos, target, read_data);
                    let tgt_name = read_data.stats.get(target).map(|stats| stats.name.clone());

                    tgt_name.map(|tgt_name| agent.add_fight_to_memory(&tgt_name, read_data.time.0));
                    agent_data.attack(agent, controller, &target_data, read_data, rng);
                } else {
                    agent_data.menacing(
                        agent,
                        controller,
                        target,
                        read_data,
                        event_emitter,
                        rng,
                        remembers_fight_with(*rtsim_entity, read_data, target),
                    );
                    remember_fight(*rtsim_entity, read_data, agent, target);
                }
            }
        }
    }
    false
}

fn will_ambush(rtsim_entity: Option<&RtSimEntity>, agent_data: &AgentData) -> bool {
    agent_data
        .health
        .map_or(false, |h| h.current() / h.maximum() > 0.7)
        && rtsim_entity.map_or(false, |re| re.brain.personality.will_ambush)
}

fn remembers_fight_with(
    rtsim_entity: Option<&RtSimEntity>,
    read_data: &ReadData,
    other: EcsEntity,
) -> bool {
    let name = || read_data.stats.get(other).map(|stats| stats.name.clone());

    rtsim_entity.map_or(false, |rtsim_entity| {
        name().map_or(false, |name| {
            rtsim_entity.brain.remembers_fight_with_character(&name)
        })
    })
}

/// Remember target.
fn remember_fight(
    rtsim_entity: Option<&RtSimEntity>,
    read_data: &ReadData,
    agent: &mut Agent,
    target: EcsEntity,
) {
    rtsim_entity.is_some().then(|| {
        read_data
            .stats
            .get(target)
            .map(|stats| agent.add_fight_to_memory(&stats.name, read_data.time.0))
    });
}
