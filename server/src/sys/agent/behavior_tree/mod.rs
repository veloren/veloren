use common::{
    comp::{
        Agent, Alignment, BehaviorCapability, BehaviorState, Body, BuffKind, CharacterState,
        ControlAction, ControlEvent, Controller, InputKind, InventoryEvent, Pos, PresenceKind,
        UtteranceKind,
        agent::{
            AgentEvent, AwarenessState, DEFAULT_INTERACTION_TIME, TRADE_INTERACTION_TIME, Target,
            TimerAction,
        },
        body, is_downed,
    },
    consts::MAX_INTERACT_RANGE,
    interaction::InteractionKind,
    path::TraversalConfig,
    rtsim::{NpcAction, RtSimEntity},
};
use rand::{Rng, prelude::ThreadRng};
use server_agent::{data::AgentEmitters, util::is_steering};
use specs::Entity as EcsEntity;
use tracing::warn;
use vek::{Vec2, Vec3};

use self::interaction::{
    handle_inbox_cancel_interactions, handle_inbox_dialogue, handle_inbox_finished_trade,
    handle_inbox_talk, handle_inbox_trade_accepted, handle_inbox_trade_invite,
    handle_inbox_update_pending_trade, increment_timer_deltatime, process_inbox_interaction,
    process_inbox_sound_and_hurt,
};

use super::{
    consts::{
        DAMAGE_MEMORY_DURATION, FLEE_DURATION, HEALING_ITEM_THRESHOLD, MAX_PATROL_DIST,
        MAX_STAY_DISTANCE, NORMAL_FLEE_DIR_DIST, NPC_PICKUP_RANGE, RETARGETING_THRESHOLD_SECONDS,
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
    pub read_data: &'a ReadData<'a>,
    pub emitters: &'a mut AgentEmitters<'c>,
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
                maintain_if_gliding,
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
                do_save_allies,
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
        let is_in_combat = agent.target.is_some_and(|t| t.hostile);
        if !is_in_combat
            && (agent.behavior.can(BehaviorCapability::SPEAK)
                || agent.behavior.can(BehaviorCapability::TRADE))
        {
            let mut tree: Vec<BehaviorFn> = vec![increment_timer_deltatime];
            if agent.behavior.can(BehaviorCapability::SPEAK) {
                tree.extend([handle_inbox_dialogue, handle_inbox_talk]);
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
            tree: vec![
                set_owner_if_no_target,
                handle_rtsim_actions,
                handle_timed_events,
            ],
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

/// If in gliding, properly maintain it
/// If on ground, unwield glider
fn maintain_if_gliding(bdata: &mut BehaviorData) -> bool {
    let Some(char_state) = bdata.read_data.char_states.get(*bdata.agent_data.entity) else {
        return false;
    };

    match char_state {
        CharacterState::Glide(_) => {
            bdata
                .agent_data
                .glider_flight(bdata.controller, bdata.read_data);
            true
        },
        CharacterState::GlideWield(_) => {
            if bdata.agent_data.physics_state.on_ground.is_some() {
                bdata.controller.push_action(ControlAction::Unwield);
            }
            // Always stop execution if during GlideWield.
            // - If on ground, the line above will unwield the glider on next
            // tick
            // - If in air, we probably wouldn't want to do anything anyway, as
            // character state code will shift itself to glide on next tick
            true
        },
        _ => false,
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

    if is_falling_dangerous {
        bdata.agent_data.dismount(bdata.controller, bdata.read_data);
        if bdata.agent_data.traversal_config.can_fly {
            bdata
                .agent_data
                .fly_upward(bdata.controller, bdata.read_data);
            return true;
        } else if bdata.agent_data.glider_equipped {
            bdata
                .agent_data
                .glider_equip(bdata.controller, bdata.read_data);
            return true;
        }
    }
    false
}

/// If on fire and able, stop, drop, and roll
fn react_if_on_fire(bdata: &mut BehaviorData) -> bool {
    let is_on_fire = bdata
        .read_data
        .buffs
        .get(*bdata.agent_data.entity)
        .is_some_and(|b| b.kinds[BuffKind::Burning].is_some());

    if is_on_fire
        && bdata.agent_data.body.is_some_and(|b| b.is_humanoid())
        && bdata.agent_data.physics_state.on_ground.is_some()
        && bdata
            .rng
            .random_bool((2.0 * bdata.read_data.dt.0).clamp(0.0, 1.0) as f64)
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
            if let Some(by) = health.last_change.damage_by()
                && let Some(attacker) = bdata.read_data.id_maps.uid_entity(by.uid())
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
                    if bdata.agent.target.is_none_or(|target| {
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
                    /*
                    if let Some(attacker_stats) =
                        bdata.rtsim_entity.and(bdata.read_data.stats.get(attacker))
                    {
                        bdata
                            .agent
                            .add_fight_to_memory(&attacker_stats.name, bdata.read_data.time.0);
                    }
                    */
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
    if bdata.agent.target.is_some() && !is_steering(*bdata.agent_data.entity, bdata.read_data) {
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
        // If target is dead or no longer exists, forget them. If the target is an item
        // we don't expect it to have a health.
        if bdata
            .read_data
            .bodies
            .get(target)
            .is_none_or(|b| !matches!(b, Body::Item(_)))
            && bdata
                .read_data
                .healths
                .get(target)
                .is_none_or(|tgt_health| tgt_health.is_dead)
        {
            /*
            if let Some(tgt_stats) = bdata.rtsim_entity.and(bdata.read_data.stats.get(target)) {
                bdata.agent.forget_enemy(&tgt_stats.name);
            }
            */
            bdata.agent.target = None;
            return true;
        }
    }
    false
}

/// If target is hostile and agent is aware of target, do the hostile tree and
/// stop the current BehaviorTree
fn do_hostile_tree_if_hostile_and_aware(bdata: &mut BehaviorData) -> bool {
    let alert = bdata.agent.awareness.reached();

    if let Some(Target { hostile, .. }) = bdata.agent.target
        && alert
        && hostile
    {
        BehaviorTree::hostile().run(bdata);
        return true;
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
    if let Some(Target { target, .. }) = bdata.agent.target
        && let Some(Body::Item(body)) = bdata.read_data.bodies.get(target)
        && !matches!(body, body::item::Body::Thrown(_))
    {
        if let Some(tgt_pos) = bdata.read_data.positions.get(target) {
            let dist_sqrd = bdata.agent_data.pos.0.distance_squared(tgt_pos.0);
            if dist_sqrd < NPC_PICKUP_RANGE.powi(2) {
                if let Some(uid) = bdata.read_data.uids.get(target) {
                    bdata
                        .controller
                        .push_event(ControlEvent::InventoryEvent(InventoryEvent::Pickup(*uid)));
                }
                bdata.agent.target = None;
            } else if let Some((bearing, speed, stuck)) = bdata.agent.chaser.chase(
                &*bdata.read_data.terrain,
                bdata.agent_data.pos.0,
                bdata.agent_data.vel.0,
                tgt_pos.0,
                TraversalConfig {
                    min_tgt_dist: NPC_PICKUP_RANGE - 1.0,
                    ..bdata.agent_data.traversal_config
                },
                &bdata.read_data.time,
            ) {
                bdata.agent_data.unstuck_if(stuck, bdata.controller);
                bdata.controller.inputs.move_dir =
                    bearing.xy().try_normalized().unwrap_or_else(Vec2::zero)
                        * speed.min(0.2 + (dist_sqrd - (NPC_PICKUP_RANGE - 1.5).powi(2)) / 8.0);
                bdata.agent_data.jump_if(bearing.z > 1.5, bdata.controller);
                bdata.controller.inputs.move_z = bearing.z;
            }
        }
        return true;
    }
    false
}

/// If there are nearby downed allies, save them.
fn do_save_allies(bdata: &mut BehaviorData) -> bool {
    if let Some(Target {
        target,
        hostile: false,
        aggro_on: false,
        ..
    }) = bdata.agent.target
        && let Some(target_uid) = bdata.read_data.uids.get(target)
    {
        let needs_saving = is_downed(
            bdata.read_data.healths.get(target),
            bdata.read_data.char_states.get(target),
        );

        let wants_to_save = match (bdata.agent_data.alignment, bdata.read_data.alignments.get(target)) {
                        // Npcs generally do want to save players. Could have extra checks for
                        // sentiment in the future.
                        (Some(Alignment::Npc), _) if bdata.read_data.presences.get(target).is_some_and(|presence| matches!(presence.kind, PresenceKind::Character(_))) => true,
                        (Some(Alignment::Npc), Some(Alignment::Npc)) => true,
                        (Some(Alignment::Enemy), Some(Alignment::Enemy)) => true,
                        _ => false,
                    } && bdata.agent.allowed_to_speak()
                        // Check that anyone else isn't already saving them.
                        && bdata.read_data
                            .interactors
                            .get(target).is_none_or(|interactors| {
                                !interactors.has_interaction(InteractionKind::HelpDowned)
                            }) && bdata.agent_data.char_state.can_interact();

        if needs_saving
            && wants_to_save
            && let Some(target_pos) = bdata.read_data.positions.get(target)
        {
            let dist_sqr = bdata.agent_data.pos.0.distance_squared(target_pos.0);
            if dist_sqr < (MAX_INTERACT_RANGE * 0.5).powi(2) {
                bdata.controller.push_event(ControlEvent::InteractWith {
                    target: *target_uid,
                    kind: common::interaction::InteractionKind::HelpDowned,
                });
                bdata.agent.target = None;
            } else if let Some((bearing, speed, stuck)) = bdata.agent.chaser.chase(
                &*bdata.read_data.terrain,
                bdata.agent_data.pos.0,
                bdata.agent_data.vel.0,
                target_pos.0,
                TraversalConfig {
                    min_tgt_dist: MAX_INTERACT_RANGE * 0.5,
                    ..bdata.agent_data.traversal_config
                },
                &bdata.read_data.time,
            ) {
                bdata.agent_data.unstuck_if(stuck, bdata.controller);
                bdata.controller.inputs.move_dir =
                    bearing.xy().try_normalized().unwrap_or_else(Vec2::zero)
                        * speed
                            .min(0.2 + (dist_sqr - (MAX_INTERACT_RANGE * 0.5 - 0.5).powi(2)) / 8.0);
                bdata.agent_data.jump_if(bearing.z > 1.5, bdata.controller);
                bdata.controller.inputs.move_z = bearing.z;
            }
            return true;
        }
    }
    false
}

/// If too far away, then follow the target
fn follow_if_far_away(bdata: &mut BehaviorData) -> bool {
    if let Some(Target { target, .. }) = bdata.agent.target
        && let Some(tgt_pos) = bdata.read_data.positions.get(target)
    {
        if let Some(stay_pos) = bdata.agent.stay_pos {
            let distance_from_stay = stay_pos.0.distance_squared(bdata.agent_data.pos.0);
            bdata.controller.push_action(ControlAction::Sit);
            if distance_from_stay > (MAX_STAY_DISTANCE).powi(2) {
                bdata
                    .agent_data
                    .follow(bdata.agent, bdata.controller, bdata.read_data, &stay_pos);
                return true;
            }
        } else {
            bdata.controller.push_action(ControlAction::Stand);
            let dist_sqrd = bdata.agent_data.pos.0.distance_squared(tgt_pos.0);
            if dist_sqrd > (MAX_PATROL_DIST * bdata.agent.psyche.idle_wander_factor).powi(2) {
                bdata
                    .agent_data
                    .follow(bdata.agent, bdata.controller, bdata.read_data, tgt_pos);
                return true;
            }
        }
    }
    false
}

/// Attack target's attacker (if there is one)
/// Target is the owner in this case
fn attack_if_owner_hurt(bdata: &mut BehaviorData) -> bool {
    if let Some(Target { target, .. }) = bdata.agent.target
        && bdata.read_data.positions.get(target).is_some()
    {
        let owner_recently_attacked =
            if let Some(target_health) = bdata.read_data.healths.get(target) {
                bdata.read_data.time.0 - target_health.last_change.time.0 < 5.0
                    && target_health.last_change.amount < 0.0
            } else {
                false
            };
        let stay = bdata.agent.stay_pos.is_some();
        if owner_recently_attacked && !stay {
            bdata.agent_data.attack_target_attacker(
                bdata.agent,
                bdata.read_data,
                bdata.controller,
                bdata.emitters,
                bdata.rng,
            );
            return true;
        }
    }
    false
}

/// Set owner if no target
fn set_owner_if_no_target(bdata: &mut BehaviorData) -> bool {
    let small_chance = bdata.rng.random_bool(0.1);

    if bdata.agent.target.is_none()
        && small_chance
        && let Some(Alignment::Owned(owner)) = bdata.agent_data.alignment
        && let Some(owner) = get_entity_by_id(*owner, bdata.read_data)
    {
        let owner_pos = bdata.read_data.positions.get(owner).map(|pos| pos.0);

        bdata.agent.target = Some(Target::new(
            owner,
            false,
            bdata.read_data.time.0,
            false,
            owner_pos,
        ));
        // Always become aware of our owner no matter what
        bdata.agent.awareness.set_maximally_aware();
    }
    false
}

/// Handle action requests from rtsim, such as talking to NPCs or attacking
fn handle_rtsim_actions(bdata: &mut BehaviorData) -> bool {
    if let Some(action) = bdata.agent.rtsim_controller.actions.pop_front() {
        match action {
            NpcAction::Say(target, msg) => {
                if bdata.agent.allowed_to_speak() {
                    // Aim the speech toward a target
                    if let Some(target) =
                        target.and_then(|tgt| bdata.read_data.id_maps.actor_entity(tgt))
                    {
                        bdata.agent.target = Some(Target::new(
                            target,
                            false,
                            bdata.read_data.time.0,
                            false,
                            bdata.read_data.positions.get(target).map(|p| p.0),
                        ));
                        // We're always aware of someone we're talking to
                        bdata.agent.awareness.set_maximally_aware();
                        // Start a timer so that we eventually stop interacting
                        bdata
                            .agent
                            .timer
                            .start(bdata.read_data.time.0, TimerAction::Interact);
                        bdata.controller.push_action(ControlAction::Stand);

                        if let Some(target_uid) = bdata.read_data.uids.get(target) {
                            bdata
                                .controller
                                .push_event(ControlEvent::Interact(*target_uid));
                        }
                    }
                    bdata.controller.push_utterance(UtteranceKind::Greeting);
                    bdata.agent_data.chat_npc(msg, bdata.emitters);
                }
            },
            NpcAction::Attack(target) => {
                if let Some(target) = bdata.read_data.id_maps.actor_entity(target) {
                    bdata.agent.target = Some(Target::new(
                        target,
                        true,
                        bdata.read_data.time.0,
                        false,
                        bdata.read_data.positions.get(target).map(|p| p.0),
                    ));
                    bdata.agent.awareness.set_maximally_aware();
                }
            },
            NpcAction::Dialogue(target, dialogue) => {
                if let Some(target) = bdata.read_data.id_maps.actor_entity(target)
                    && let Some(target_uid) = bdata.read_data.uids.get(target)
                {
                    bdata
                        .controller
                        .push_event(ControlEvent::Dialogue(*target_uid, dialogue));
                    bdata.controller.push_utterance(UtteranceKind::Greeting);
                } else {
                    warn!("NPC dialogue sent to non-existent target entity");
                }
            },
            NpcAction::RequestPirateHire { .. } => {},
        }
        true
    } else {
        false
    }
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
            if let Some(Target { target, .. }) = &bdata.agent.target
                && let Some(target_uid) = bdata.read_data.uids.get(*target)
            {
                bdata
                    .agent_data
                    .look_toward(bdata.controller, bdata.read_data, *target);
                bdata
                    .controller
                    .push_action(ControlAction::Talk(Some(*target_uid)));
            }
        },
        Some(just_ended) => {
            if just_ended {
                bdata.agent.target = None;
                bdata.controller.push_action(ControlAction::Stand);
            }

            if bdata.rng.random::<f32>() < 0.1 {
                bdata.agent_data.choose_target(
                    bdata.agent,
                    bdata.controller,
                    bdata.read_data,
                    AgentData::is_enemy,
                );
            } else {
                bdata.agent_data.handle_sounds_heard(
                    bdata.agent,
                    bdata.controller,
                    bdata.read_data,
                    bdata.emitters,
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

        if let Some(target_pos) = read_data.positions.get(target)
            && agent_data.detects_other(
                agent,
                controller,
                &target,
                target_pos,
                read_data.scales.get(target),
                read_data,
            )
        {
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

    false
}

/// Try to heal self if our damage went below a certain threshold
fn heal_self_if_hurt(bdata: &mut BehaviorData) -> bool {
    if bdata.agent_data.char_state.can_interact()
        && bdata.agent_data.damage < HEALING_ITEM_THRESHOLD
        && bdata
            .agent_data
            .heal_self(bdata.agent, bdata.controller, false)
    {
        bdata.agent.behavior_state.timers
            [ActionStateBehaviorTreeTimers::TimerBehaviorTree as usize] = 0.01;
        return true;
    }
    false
}

/// Hurt utterances at random upon receiving damage
fn hurt_utterance(bdata: &mut BehaviorData) -> bool {
    if matches!(bdata.agent.inbox.front(), Some(AgentEvent::Hurt)) {
        if bdata.rng.random::<f32>() < 0.4 {
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
    let tgt_scale = target.and_then(|t| read_data.scales.get(t));

    if let (Some(target), Some(tgt_pos)) = (target, tgt_pos) {
        if agent_data.can_see_entity(agent, controller, target, tgt_pos, tgt_scale, read_data) {
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

    if let Some(target) = agent.target
        && let Some(last_known_pos) = target.last_known_pos
    {
        agent_data.follow(agent, controller, read_data, &Pos(last_known_pos));

        return true;
    }

    false
}

fn do_combat(bdata: &mut BehaviorData) -> bool {
    let BehaviorData {
        agent,
        agent_data,
        read_data,
        emitters,
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
                .is_none_or(|ad| dist_sqrd < (ad * agent.psyche.aggro_range_multiplier).powi(2));

            if in_aggro_range {
                *aggro_on = true;
            }
            let aggro_on = *aggro_on;

            let (flee, flee_dur_mul) = match agent_data.char_state {
                CharacterState::Crawl => {
                    controller.push_action(ControlAction::Stand);

                    // Stay still if we're being helped up.
                    if let Some(interactors) = read_data.interactors.get(*agent_data.entity)
                        && interactors.has_interaction(InteractionKind::HelpDowned)
                    {
                        return true;
                    }

                    (true, 5.0)
                },
                _ => (
                    agent_data.below_flee_health(agent) || agent.stay_pos.is_some(),
                    1.0,
                ),
            };

            if flee {
                let flee_timer_done = agent.behavior_state.timers
                    [ActionStateBehaviorTreeTimers::TimerBehaviorTree as usize]
                    > FLEE_DURATION * flee_dur_mul;
                let within_normal_flee_dir_dist = dist_sqrd < NORMAL_FLEE_DIR_DIST.powi(2);

                // FIXME: Using action state timer to see if allowed to speak is a hack.
                if agent.behavior_state.timers
                    [ActionStateBehaviorTreeTimers::TimerBehaviorTree as usize]
                    == 0.0
                {
                    agent_data.cry_out(agent, emitters, read_data);
                    agent.behavior_state.timers
                        [ActionStateBehaviorTreeTimers::TimerBehaviorTree as usize] = 0.01;
                    agent.flee_from_pos = {
                        let random = || rand::rng().random_range(-1.0..1.0);
                        Some(Pos(
                            agent_data.pos.0 + Vec3::new(random(), random(), random())
                        ))
                    };
                } else if !flee_timer_done {
                    if within_normal_flee_dir_dist {
                        agent_data.flee(agent, controller, read_data, tgt_pos);
                    } else if let Some(random_pos) = agent.flee_from_pos {
                        agent_data.flee(agent, controller, read_data, &random_pos);
                    } else {
                        agent_data.flee(agent, controller, read_data, tgt_pos);
                    }

                    agent.behavior_state.timers
                        [ActionStateBehaviorTreeTimers::TimerBehaviorTree as usize] +=
                        read_data.dt.0;
                } else {
                    agent.behavior_state.timers
                        [ActionStateBehaviorTreeTimers::TimerBehaviorTree as usize] = 0.0;
                    agent.target = None;
                    agent.flee_from_pos = None;
                    agent_data.idle(agent, controller, read_data, emitters, rng);
                }
            } else if is_dead(target, read_data) {
                agent_data.exclaim_relief_about_enemy_dead(agent, emitters);
                agent.target = None;
                agent_data.idle(agent, controller, read_data, emitters, rng);
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
                agent_data.idle(agent, controller, read_data, emitters, rng);
            } else {
                let is_time_to_retarget =
                    read_data.time.0 - selected_at > RETARGETING_THRESHOLD_SECONDS;

                if (!agent.psyche.should_stop_pursuing || !in_aggro_range) && is_time_to_retarget {
                    agent_data.choose_target(agent, controller, read_data, AgentData::is_enemy);
                }

                let target_data = TargetData::new(tgt_pos, target, read_data);

                if aggro_on {
                    // let tgt_name = read_data.stats.get(target).map(|stats| stats.name.clone());

                    // TODO: Reimplement in rtsim2
                    // tgt_name.map(|tgt_name| agent.add_fight_to_memory(&tgt_name,
                    // read_data.time.0));
                    agent_data.attack(agent, controller, &target_data, read_data, rng);
                } else {
                    agent_data.menacing(
                        agent,
                        controller,
                        target,
                        &target_data,
                        read_data,
                        emitters,
                        remembers_fight_with(agent_data.rtsim_entity, read_data, target),
                    );
                    // TODO: Reimplement in rtsim2
                    // remember_fight(agent_data.rtsim_entity, read_data, agent,
                    // target);
                }
            }
        }
        // make sure world bosses and roaming entities stay aware, to continue pursuit
        if !agent.psyche.should_stop_pursuing {
            bdata.agent.awareness.set_maximally_aware();
        }
    }
    false
}

fn remembers_fight_with(
    _rtsim_entity: Option<&RtSimEntity>,
    _read_data: &ReadData,
    _other: EcsEntity,
) -> bool {
    // TODO: implement for rtsim2
    // let name = || read_data.stats.get(other).map(|stats| stats.name.clone());

    // rtsim_entity.map_or(false, |rtsim_entity| {
    //     name().map_or(false, |name| {
    //         rtsim_entity.brain.remembers_fight_with_character(&name)
    //     })
    // })
    false
}

// /// Remember target.
// fn remember_fight(
//     rtsim_entity: Option<&RtSimEntity>,
//     read_data: &ReadData,
//     agent: &mut Agent,
//     target: EcsEntity,
// ) { rtsim_entity.is_some().then(|| { read_data .stats .get(target)
//   .map(|stats| agent.add_fight_to_memory(&stats.name,
// read_data.time.0))     });
// }
