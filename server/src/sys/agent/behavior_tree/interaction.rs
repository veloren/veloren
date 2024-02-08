use common::{
    comp::{
        agent::{AgentEvent, Target, TimerAction},
        compass::{Direction, Distance},
        dialogue::Subject,
        inventory::item::{ItemTag, MaterialStatManifest},
        invite::{InviteKind, InviteResponse},
        tool::AbilityMap,
        BehaviorState, Content, ControlAction, Item, TradingBehavior, UnresolvedChatMsg,
        UtteranceKind,
    },
    event::{ChatEvent, EmitExt, ProcessTradeActionEvent},
    rtsim::{Actor, NpcInput, PersonalityTrait},
    trade::{TradeAction, TradePhase, TradeResult},
};
use rand::{thread_rng, Rng};

use crate::sys::agent::util::get_entity_by_id;

use super::{BehaviorData, BehaviorTree};

enum ActionStateInteractionTimers {
    TimerInteraction = 0,
}

/// Interact if incoming messages
pub fn process_inbox_sound_and_hurt(bdata: &mut BehaviorData) -> bool {
    if !bdata.agent.inbox.is_empty() {
        if matches!(
            bdata.agent.inbox.front(),
            Some(AgentEvent::ServerSound(_)) | Some(AgentEvent::Hurt)
        ) {
            let sound = bdata.agent.inbox.pop_front();
            match sound {
                Some(AgentEvent::ServerSound(sound)) => {
                    bdata.agent.sounds_heard.push(sound);
                },
                Some(AgentEvent::Hurt) => {
                    // Hurt utterances at random upon receiving damage
                    if bdata.rng.gen::<f32>() < 0.4 {
                        bdata.controller.push_utterance(UtteranceKind::Hurt);
                    }
                },
                //Note: this should be unreachable
                Some(_) | None => {},
            }
        } else {
            bdata.agent.behavior_state.timers
                [ActionStateInteractionTimers::TimerInteraction as usize] = 0.1;
        }
    }
    false
}

/// If we receive a new interaction, start the interaction timer
pub fn process_inbox_interaction(bdata: &mut BehaviorData) -> bool {
    if BehaviorTree::interaction(bdata.agent).run(bdata) {
        bdata
            .agent
            .timer
            .start(bdata.read_data.time.0, TimerAction::Interact);
    }
    false
}

/// Increment agent's behavior_state timer
pub fn increment_timer_deltatime(bdata: &mut BehaviorData) -> bool {
    bdata.agent.behavior_state.timers[ActionStateInteractionTimers::TimerInteraction as usize] +=
        bdata.read_data.dt.0;
    false
}

/// Handles Talk event if the front of the agent's inbox contains one
pub fn handle_inbox_talk(bdata: &mut BehaviorData) -> bool {
    let BehaviorData {
        agent,
        agent_data,
        read_data,
        emitters,
        controller,
        ..
    } = bdata;

    if !matches!(agent.inbox.front(), Some(AgentEvent::Talk(_, _))) {
        return false;
    }

    if let Some(AgentEvent::Talk(by, subject)) = agent.inbox.pop_front() {
        let by_entity = get_entity_by_id(by, read_data);

        if let Some(rtsim_outbox) = &mut agent.rtsim_outbox {
            if let Subject::Regular | Subject::Mood | Subject::Work = subject
                && let Some(by_entity) = by_entity
                && let Some(actor) = read_data
                    .presences
                    .get(by_entity)
                    .and_then(|p| p.kind.character_id().map(Actor::Character))
                    .or_else(|| Some(Actor::Npc(read_data.rtsim_entities.get(by_entity)?.0)))
            {
                rtsim_outbox.push_back(NpcInput::Interaction(actor, subject));
                return false;
            }
        }

        if agent.allowed_to_speak() {
            if let Some(target) = by_entity {
                let target_pos = read_data.positions.get(target).map(|pos| pos.0);

                agent.target = Some(Target::new(
                    target,
                    false,
                    read_data.time.0,
                    false,
                    target_pos,
                ));
                // We're always aware of someone we're talking to
                agent.awareness.set_maximally_aware();

                controller.push_action(ControlAction::Stand);
                controller.push_action(ControlAction::Talk);
                controller.push_utterance(UtteranceKind::Greeting);

                match subject {
                    Subject::Regular => {
                        if let Some(tgt_stats) = read_data.stats.get(target) {
                            if let Some(destination_name) = &agent.rtsim_controller.heading_to {
                                let personality = &agent.rtsim_controller.personality;
                                let standard_response_msg = || -> String {
                                    if personality.will_ambush() {
                                        format!(
                                            "I'm heading to {}! Want to come along? We'll make \
                                             great travel buddies, hehe.",
                                            destination_name
                                        )
                                    } else if personality.is(PersonalityTrait::Extroverted) {
                                        format!(
                                            "I'm heading to {}! Want to come along?",
                                            destination_name
                                        )
                                    } else if personality.is(PersonalityTrait::Disagreeable) {
                                        "Hrm.".to_string()
                                    } else {
                                        "Hello!".to_string()
                                    }
                                };
                                let msg = if false
                                /* TODO: Remembers character */
                                {
                                    if personality.will_ambush() {
                                        "Just follow me a bit more, hehe.".to_string()
                                    } else if personality.is(PersonalityTrait::Extroverted) {
                                        if personality.is(PersonalityTrait::Extroverted) {
                                            format!(
                                                "Greetings fair {}! It has been far too long \
                                                 since last I saw you. I'm going to {} right now.",
                                                &tgt_stats.name, destination_name
                                            )
                                        } else if personality.is(PersonalityTrait::Disagreeable) {
                                            "Oh. It's you again.".to_string()
                                        } else {
                                            format!(
                                                "Hi again {}! Unfortunately I'm in a hurry right \
                                                 now. See you!",
                                                &tgt_stats.name
                                            )
                                        }
                                    } else {
                                        standard_response_msg()
                                    }
                                } else {
                                    standard_response_msg()
                                };
                                // TODO: Localise
                                agent_data.chat_npc(Content::Plain(msg), emitters);
                            } else {
                                let mut rng = thread_rng();
                                agent_data.chat_npc(
                                    agent
                                        .rtsim_controller
                                        .personality
                                        .get_generic_comment(&mut rng),
                                    emitters,
                                );
                            }
                        }
                    },
                    Subject::Trade => {
                        if agent.behavior.can_trade(agent_data.alignment.copied(), by) {
                            if !agent.behavior.is(BehaviorState::TRADING) {
                                controller.push_initiate_invite(by, InviteKind::Trade);
                                agent_data.chat_npc_if_allowed_to_speak(
                                    Content::localized("npc-speech-merchant_advertisement"),
                                    agent,
                                    emitters,
                                );
                            } else {
                                agent_data.chat_npc_if_allowed_to_speak(
                                    Content::localized("npc-speech-merchant_busy"),
                                    agent,
                                    emitters,
                                );
                            }
                        } else {
                            // TODO: maybe make some travellers willing to trade with
                            // simpler goods like potions
                            agent_data.chat_npc_if_allowed_to_speak(
                                Content::localized("npc-speech-villager_decline_trade"),
                                agent,
                                emitters,
                            );
                        }
                    },
                    Subject::Mood => {
                        // TODO: Reimplement in rtsim2
                    },
                    Subject::Location(location) => {
                        if let Some(tgt_pos) = read_data.positions.get(target) {
                            let raw_dir = location.origin.as_::<f32>() - tgt_pos.0.xy();
                            let dist = Distance::from_dir(raw_dir).name();
                            let dir = Direction::from_dir(raw_dir).name();

                            // TODO: Localise
                            let msg = format!(
                                "{} ? I think it's {} {} from here!",
                                location.name, dist, dir
                            );
                            agent_data.chat_npc(Content::Plain(msg), emitters);
                        }
                    },
                    Subject::Person(person) => {
                        if let Some(src_pos) = read_data.positions.get(target) {
                            // TODO: Localise
                            let msg = if let Some(person_pos) = person.origin {
                                let distance =
                                    Distance::from_dir(person_pos.xy().as_() - src_pos.0.xy());
                                match distance {
                                    Distance::NextTo | Distance::Near => {
                                        format!(
                                            "{} ? I think he's {} {} from here!",
                                            person.name(),
                                            distance.name(),
                                            Direction::from_dir(
                                                person_pos.xy().as_() - src_pos.0.xy()
                                            )
                                            .name()
                                        )
                                    },
                                    _ => {
                                        format!(
                                            "{} ? I think he's gone visiting another town. Come \
                                             back later!",
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
                            agent_data.chat_npc(Content::Plain(msg), emitters);
                        }
                    },
                    Subject::Work => {},
                }
            }
        }
    }
    true
}

/// Handles TradeInvite event if the front of the agent's inbox contains one
pub fn handle_inbox_trade_invite(bdata: &mut BehaviorData) -> bool {
    let BehaviorData {
        agent,
        agent_data,
        read_data,
        emitters,
        controller,
        ..
    } = bdata;

    if !matches!(agent.inbox.front(), Some(AgentEvent::TradeInvite(_))) {
        return false;
    }

    if let Some(AgentEvent::TradeInvite(with)) = agent.inbox.pop_front() {
        if agent
            .behavior
            .can_trade(agent_data.alignment.copied(), with)
        {
            if !agent.behavior.is(BehaviorState::TRADING) {
                // stand still and looking towards the trading player
                controller.push_action(ControlAction::Stand);
                controller.push_action(ControlAction::Talk);
                if let Some(target) = get_entity_by_id(with, read_data) {
                    let target_pos = read_data.positions.get(target).map(|pos| pos.0);

                    agent.target = Some(Target::new(
                        target,
                        false,
                        read_data.time.0,
                        false,
                        target_pos,
                    ));
                }
                controller.push_invite_response(InviteResponse::Accept);
                agent.behavior.unset(BehaviorState::TRADING_ISSUER);
                agent.behavior.set(BehaviorState::TRADING);
            } else {
                controller.push_invite_response(InviteResponse::Decline);
                agent_data.chat_npc_if_allowed_to_speak(
                    Content::localized("npc-speech-merchant_busy"),
                    agent,
                    emitters,
                );
            }
        } else {
            // TODO: Provide a hint where to find the closest merchant?
            controller.push_invite_response(InviteResponse::Decline);
            agent_data.chat_npc_if_allowed_to_speak(
                Content::localized("npc-speech-villager_decline_trade"),
                agent,
                emitters,
            );
        }
    }
    true
}

/// Handles TradeAccepted event if the front of the agent's inbox contains one
pub fn handle_inbox_trade_accepted(bdata: &mut BehaviorData) -> bool {
    let BehaviorData {
        agent, read_data, ..
    } = bdata;

    if !matches!(agent.inbox.front(), Some(AgentEvent::TradeAccepted(_))) {
        return false;
    }

    if let Some(AgentEvent::TradeAccepted(with)) = agent.inbox.pop_front() {
        if !agent.behavior.is(BehaviorState::TRADING) {
            if let Some(target) = get_entity_by_id(with, read_data) {
                let target_pos = read_data.positions.get(target).map(|pos| pos.0);

                agent.target = Some(Target::new(
                    target,
                    false,
                    read_data.time.0,
                    false,
                    target_pos,
                ));
            }
            agent.behavior.set(BehaviorState::TRADING);
            agent.behavior.set(BehaviorState::TRADING_ISSUER);
        }
    }
    true
}

/// Handles TradeFinished event if the front of the agent's inbox contains one
pub fn handle_inbox_finished_trade(bdata: &mut BehaviorData) -> bool {
    let BehaviorData {
        agent,
        agent_data,
        emitters,
        ..
    } = bdata;

    if !matches!(agent.inbox.front(), Some(AgentEvent::FinishedTrade(_))) {
        return false;
    }

    if let Some(AgentEvent::FinishedTrade(result)) = agent.inbox.pop_front() {
        if agent.behavior.is(BehaviorState::TRADING) {
            match result {
                TradeResult::Completed => {
                    agent_data.chat_npc_if_allowed_to_speak(
                        Content::localized("npc-speech-merchant_trade_successful"),
                        agent,
                        emitters,
                    );
                },
                _ => {
                    agent_data.chat_npc_if_allowed_to_speak(
                        Content::localized("npc-speech-merchant_trade_declined"),
                        agent,
                        emitters,
                    );
                },
            }
            agent.behavior.unset(BehaviorState::TRADING);
            agent.target = None;
        }
    }
    true
}

/// Handles UpdatePendingTrade event if the front of the agent's inbox contains
/// one
pub fn handle_inbox_update_pending_trade(bdata: &mut BehaviorData) -> bool {
    let BehaviorData {
        agent,
        agent_data,
        read_data,
        emitters,
        ..
    } = bdata;

    if !matches!(agent.inbox.front(), Some(AgentEvent::UpdatePendingTrade(_))) {
        return false;
    }

    if let Some(AgentEvent::UpdatePendingTrade(boxval)) = agent.inbox.pop_front() {
        let (tradeid, pending, prices, inventories) = *boxval;
        if agent.behavior.is(BehaviorState::TRADING) {
            let who = usize::from(!agent.behavior.is(BehaviorState::TRADING_ISSUER));
            let mut message = |content: Content| {
                if let Some(with) = agent
                    .target
                    .as_ref()
                    .and_then(|tgt_data| read_data.uids.get(tgt_data.target))
                {
                    emitters.emit(ChatEvent(UnresolvedChatMsg::npc_tell(
                        *agent_data.uid,
                        *with,
                        content,
                    )));
                } else {
                    emitters.emit(ChatEvent(UnresolvedChatMsg::npc_say(
                        *agent_data.uid,
                        content,
                    )));
                }
            };
            match agent.behavior.trading_behavior {
                TradingBehavior::RequireBalanced { .. } => {
                    let balance0 = prices.balance(&pending.offers, &inventories, 1 - who, true);
                    let balance1 = prices.balance(&pending.offers, &inventories, who, false);
                    match (balance0, balance1) {
                        // TODO: Localise
                        (_, None) => message(Content::Plain(
                            "I'm not willing to sell that item".to_string(),
                        )),
                        // TODO: Localise
                        (None, _) => message(Content::Plain(
                            "I'm not willing to buy that item".to_string(),
                        )),
                        (Some(balance0), Some(balance1)) => {
                            if balance0 >= balance1 {
                                // If the trade is favourable to us, only send an accept message if
                                // we're not already accepting
                                // (since otherwise, spam-clicking the accept button
                                // results in lagging and moving to the review phase of an
                                // unfavorable trade (although since
                                // the phase is included in the message, this shouldn't
                                // result in fully accepting an unfavourable trade))
                                if !pending.accept_flags[who] && !pending.is_empty_trade() {
                                    emitters.emit(ProcessTradeActionEvent(
                                        *agent_data.entity,
                                        tradeid,
                                        TradeAction::Accept(pending.phase),
                                    ));
                                    tracing::trace!(
                                        ?tradeid,
                                        ?balance0,
                                        ?balance1,
                                        "Accept Pending Trade"
                                    );
                                }
                            } else {
                                if balance1 > 0.0 {
                                    // TODO: Localise
                                    message(Content::Plain(format!(
                                        "That only covers {:.0}% of my costs!",
                                        (balance0 / balance1 * 100.0).floor()
                                    )));
                                }
                                if pending.phase != TradePhase::Mutate {
                                    // we got into the review phase but without balanced goods,
                                    // decline
                                    agent.behavior.unset(BehaviorState::TRADING);
                                    agent.target = None;
                                    emitters.emit(ProcessTradeActionEvent(
                                        *agent_data.entity,
                                        tradeid,
                                        TradeAction::Decline,
                                    ));
                                }
                            }
                        },
                    }
                },
                TradingBehavior::AcceptFood => {
                    let mut only_food = true;
                    let ability_map = AbilityMap::load().read();
                    let msm = MaterialStatManifest::load().read();
                    if let Some(ri) = &inventories[1 - who] {
                        for (slot, _) in pending.offers[1 - who].iter() {
                            if let Some(item) = ri.inventory.get(slot) {
                                if let Ok(item) = Item::new_from_item_definition_id(
                                    item.name.as_ref(),
                                    &ability_map,
                                    &msm,
                                ) {
                                    if !item.tags().contains(&ItemTag::Food) {
                                        only_food = false;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    if !pending.accept_flags[who]
                        && pending.offers[who].is_empty()
                        && !pending.offers[1 - who].is_empty()
                        && only_food
                    {
                        emitters.emit(ProcessTradeActionEvent(
                            *agent_data.entity,
                            tradeid,
                            TradeAction::Accept(pending.phase),
                        ));
                    }
                },
                TradingBehavior::None => {
                    agent.behavior.unset(BehaviorState::TRADING);
                    agent.target = None;
                    emitters.emit(ProcessTradeActionEvent(
                        *agent_data.entity,
                        tradeid,
                        TradeAction::Decline,
                    ));
                },
            }
        }
    }
    true
}

/// Deny any received interaction:
/// - `AgentEvent::Talk` and `AgentEvent::TradeAccepted` are cut short by an
///   "I'm busy" message
/// - `AgentEvent::TradeInvite` are denied
/// - `AgentEvent::FinishedTrade` are still handled
/// - `AgentEvent::UpdatePendingTrade` will immediately close the trade
pub fn handle_inbox_cancel_interactions(bdata: &mut BehaviorData) -> bool {
    let BehaviorData {
        agent,
        agent_data,
        emitters,
        controller,
        ..
    } = bdata;

    if let Some(msg) = agent.inbox.front() {
        match msg {
            AgentEvent::Talk(by, _) | AgentEvent::TradeAccepted(by) => {
                if agent
                    .target
                    .zip(get_entity_by_id(*by, bdata.read_data))
                    // in combat, speak to players that aren't the current target
                    .map_or(false, |(target, speaker)| !target.hostile || target.target != speaker)
                {
                    agent_data.chat_npc_if_allowed_to_speak(
                        Content::localized("npc-speech-villager_busy"),
                        agent,
                        emitters,
                    );
                }
            },
            AgentEvent::TradeInvite(by) => {
                controller.push_invite_response(InviteResponse::Decline);
                if let (Some(target), Some(speaker)) =
                    (agent.target, get_entity_by_id(*by, bdata.read_data))
                {
                    // in combat, speak to players that aren't the current target
                    if !target.hostile || target.target != speaker {
                        if agent.behavior.can_trade(agent_data.alignment.copied(), *by) {
                            agent_data.chat_npc_if_allowed_to_speak(
                                Content::localized("npc-speech-merchant_busy"),
                                agent,
                                emitters,
                            );
                        } else {
                            agent_data.chat_npc_if_allowed_to_speak(
                                Content::localized("npc-speech-villager_busy"),
                                agent,
                                emitters,
                            );
                        }
                    }
                }
            },
            AgentEvent::FinishedTrade(result) => {
                // copy pasted from recv_interaction
                // because the trade is not cancellable in this state
                if agent.behavior.is(BehaviorState::TRADING) {
                    match result {
                        TradeResult::Completed => {
                            agent_data.chat_npc_if_allowed_to_speak(
                                Content::localized("npc-speech-merchant_trade_successful"),
                                agent,
                                emitters,
                            );
                        },
                        _ => {
                            agent_data.chat_npc_if_allowed_to_speak(
                                Content::localized("npc-speech-merchant_trade_declined"),
                                agent,
                                emitters,
                            );
                        },
                    }
                    agent.behavior.unset(BehaviorState::TRADING);
                    agent.target = None;
                }
            },
            AgentEvent::UpdatePendingTrade(boxval) => {
                // immediately cancel the trade
                let (tradeid, _pending, _prices, _inventories) = &**boxval;
                agent.behavior.unset(BehaviorState::TRADING);
                agent.target = None;
                emitters.emit(ProcessTradeActionEvent(
                    *agent_data.entity,
                    *tradeid,
                    TradeAction::Decline,
                ));
                agent_data.chat_npc_if_allowed_to_speak(
                    Content::localized("npc-speech-merchant_trade_cancelled_hostile"),
                    agent,
                    emitters,
                );
            },
            AgentEvent::ServerSound(_) | AgentEvent::Hurt => return false,
        };

        agent.inbox.pop_front();
    }
    false
}
