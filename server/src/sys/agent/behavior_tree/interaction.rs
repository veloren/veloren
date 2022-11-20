use common::{
    comp::{
        agent::{AgentEvent, Target, TimerAction},
        compass::{Direction, Distance},
        dialogue::{MoodContext, MoodState, Subject},
        inventory::item::{ItemTag, MaterialStatManifest},
        invite::{InviteKind, InviteResponse},
        tool::AbilityMap,
        BehaviorState, ControlAction, Item, TradingBehavior, UnresolvedChatMsg, UtteranceKind,
    },
    event::ServerEvent,
    rtsim::{Memory, MemoryItem, RtSimEvent},
    trade::{TradeAction, TradePhase, TradeResult},
};
use rand::{thread_rng, Rng};
use specs::saveload::Marker;

use crate::{
    rtsim::entity::{PersonalityTrait, RtSimEntityKind},
    sys::agent::util::get_entity_by_id,
};

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
            bdata.agent.action_state.timers
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

/// Increment agent's action_state timer
pub fn increment_timer_deltatime(bdata: &mut BehaviorData) -> bool {
    bdata.agent.action_state.timers[ActionStateInteractionTimers::TimerInteraction as usize] +=
        bdata.read_data.dt.0;
    false
}

/// Handles Talk event if the front of the agent's inbox contains one
pub fn handle_inbox_talk(bdata: &mut BehaviorData) -> bool {
    let BehaviorData {
        agent,
        agent_data,
        read_data,
        event_emitter,
        controller,
        ..
    } = bdata;

    if !matches!(agent.inbox.front(), Some(AgentEvent::Talk(_, _))) {
        return false;
    }

    if let Some(AgentEvent::Talk(by, subject)) = agent.inbox.pop_front() {
        if agent.allowed_to_speak() {
            if let Some(target) = get_entity_by_id(by.id(), read_data) {
                let target_pos = read_data.positions.get(target).map(|pos| pos.0);

                agent.target = Some(Target::new(
                    target,
                    false,
                    read_data.time.0,
                    false,
                    target_pos,
                ));

                if agent_data.look_toward(controller, read_data, target) {
                    controller.push_action(ControlAction::Stand);
                    controller.push_action(ControlAction::Talk);
                    controller.push_utterance(UtteranceKind::Greeting);

                    match subject {
                        Subject::Regular => {
                            if let Some(rtsim_entity) = &bdata.rtsim_entity {
                                if matches!(rtsim_entity.kind, RtSimEntityKind::Prisoner) {
                                    agent_data.chat_npc("npc-speech-prisoner", event_emitter);
                                } else if let (
                                    Some((_travel_to, destination_name)),
                                    Some(rtsim_entity),
                                ) =
                                    (&agent.rtsim_controller.travel_to, &&bdata.rtsim_entity)
                                {
                                    let personality = &rtsim_entity.brain.personality;
                                    let standard_response_msg = || -> String {
                                        if personality.will_ambush {
                                            format!(
                                                "I'm heading to {}! Want to come along? We'll \
                                                 make great travel buddies, hehe.",
                                                destination_name
                                            )
                                        } else if personality
                                            .personality_traits
                                            .contains(PersonalityTrait::Extroverted)
                                        {
                                            format!(
                                                "I'm heading to {}! Want to come along?",
                                                destination_name
                                            )
                                        } else if personality
                                            .personality_traits
                                            .contains(PersonalityTrait::Disagreeable)
                                        {
                                            "Hrm.".to_string()
                                        } else {
                                            "Hello!".to_string()
                                        }
                                    };
                                    let msg = if let Some(tgt_stats) = read_data.stats.get(target) {
                                        agent.rtsim_controller.events.push(RtSimEvent::AddMemory(
                                            Memory {
                                                item: MemoryItem::CharacterInteraction {
                                                    name: tgt_stats.name.clone(),
                                                },
                                                time_to_forget: read_data.time.0 + 600.0,
                                            },
                                        ));
                                        if rtsim_entity.brain.remembers_character(&tgt_stats.name) {
                                            if personality.will_ambush {
                                                "Just follow me a bit more, hehe.".to_string()
                                            } else if personality
                                                .personality_traits
                                                .contains(PersonalityTrait::Extroverted)
                                            {
                                                format!(
                                                    "Greetings fair {}! It has been far too long \
                                                     since last I saw you. I'm going to {} right \
                                                     now.",
                                                    &tgt_stats.name, destination_name
                                                )
                                            } else if personality
                                                .personality_traits
                                                .contains(PersonalityTrait::Disagreeable)
                                            {
                                                "Oh. It's you again.".to_string()
                                            } else {
                                                format!(
                                                    "Hi again {}! Unfortunately I'm in a hurry \
                                                     right now. See you!",
                                                    &tgt_stats.name
                                                )
                                            }
                                        } else {
                                            standard_response_msg()
                                        }
                                    } else {
                                        standard_response_msg()
                                    };
                                    agent_data.chat_npc(msg, event_emitter);
                                } else if agent
                                    .behavior
                                    .can_trade(agent_data.alignment.copied(), by)
                                {
                                    if !agent.behavior.is(BehaviorState::TRADING) {
                                        controller.push_initiate_invite(by, InviteKind::Trade);
                                        agent_data.chat_npc(
                                            "npc-speech-merchant_advertisement",
                                            event_emitter,
                                        );
                                    } else {
                                        let default_msg = "npc-speech-merchant_busy";
                                        let msg = &bdata.rtsim_entity.map_or(default_msg, |e| {
                                            if e.brain
                                                .personality
                                                .personality_traits
                                                .contains(PersonalityTrait::Disagreeable)
                                            {
                                                "npc-speech-merchant_busy_rude"
                                            } else {
                                                default_msg
                                            }
                                        });
                                        agent_data.chat_npc(msg, event_emitter);
                                    }
                                } else {
                                    let mut rng = thread_rng();
                                    if let Some(extreme_trait) = &bdata.rtsim_entity.and_then(|e| {
                                        e.brain.personality.random_chat_trait(&mut rng)
                                    }) {
                                        let msg = match extreme_trait {
                                            PersonalityTrait::Open => "npc-speech-villager_open",
                                            PersonalityTrait::Adventurous => {
                                                "npc-speech-villager_adventurous"
                                            },
                                            PersonalityTrait::Closed => {
                                                "npc-speech-villager_closed"
                                            },
                                            PersonalityTrait::Conscientious => {
                                                "npc-speech-villager_conscientious"
                                            },
                                            PersonalityTrait::Busybody => {
                                                "npc-speech-villager_busybody"
                                            },
                                            PersonalityTrait::Unconscientious => {
                                                "npc-speech-villager_unconscientious"
                                            },
                                            PersonalityTrait::Extroverted => {
                                                "npc-speech-villager_extroverted"
                                            },
                                            PersonalityTrait::Introverted => {
                                                "npc-speech-villager_introverted"
                                            },
                                            PersonalityTrait::Agreeable => {
                                                "npc-speech-villager_agreeable"
                                            },
                                            PersonalityTrait::Sociable => {
                                                "npc-speech-villager_sociable"
                                            },
                                            PersonalityTrait::Disagreeable => {
                                                "npc-speech-villager_disagreeable"
                                            },
                                            PersonalityTrait::Neurotic => {
                                                "npc-speech-villager_neurotic"
                                            },
                                            PersonalityTrait::Seeker => {
                                                "npc-speech-villager_seeker"
                                            },
                                            PersonalityTrait::SadLoner => {
                                                "npc-speech-villager_sad_loner"
                                            },
                                            PersonalityTrait::Worried => {
                                                "npc-speech-villager_worried"
                                            },
                                            PersonalityTrait::Stable => {
                                                "npc-speech-villager_stable"
                                            },
                                        };
                                        agent_data.chat_npc(msg, event_emitter);
                                    } else {
                                        agent_data.chat_npc("npc-speech-villager", event_emitter);
                                    }
                                }
                            }
                        },
                        Subject::Trade => {
                            if agent.behavior.can_trade(agent_data.alignment.copied(), by) {
                                if !agent.behavior.is(BehaviorState::TRADING) {
                                    controller.push_initiate_invite(by, InviteKind::Trade);
                                    agent_data.chat_npc_if_allowed_to_speak(
                                        "npc-speech-merchant_advertisement",
                                        agent,
                                        event_emitter,
                                    );
                                } else {
                                    agent_data.chat_npc_if_allowed_to_speak(
                                        "npc-speech-merchant_busy",
                                        agent,
                                        event_emitter,
                                    );
                                }
                            } else {
                                // TODO: maybe make some travellers willing to trade with
                                // simpler goods like potions
                                agent_data.chat_npc_if_allowed_to_speak(
                                    "npc-speech-villager_decline_trade",
                                    agent,
                                    event_emitter,
                                );
                            }
                        },
                        Subject::Mood => {
                            if let Some(rtsim_entity) = &bdata.rtsim_entity {
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
                                                    state: MoodState::Bad(MoodContext::GoodWeather),
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
                                    agent_data.chat_npc(msg, event_emitter);
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
                                agent_data.chat_npc(msg, event_emitter);
                            }
                        },
                        Subject::Person(person) => {
                            if let Some(src_pos) = read_data.positions.get(target) {
                                let msg = if let Some(person_pos) = person.origin {
                                    let distance =
                                        Distance::from_dir(person_pos.xy() - src_pos.0.xy());
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
                                                "{} ? I think he's gone visiting another town. \
                                                 Come back later!",
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
                                agent_data.chat_npc(msg, event_emitter);
                            }
                        },
                        Subject::Work => {},
                    }
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
        event_emitter,
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
                if let Some(target) = get_entity_by_id(with.id(), read_data) {
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
                    "npc-speech-merchant_busy",
                    agent,
                    event_emitter,
                );
            }
        } else {
            // TODO: Provide a hint where to find the closest merchant?
            controller.push_invite_response(InviteResponse::Decline);
            agent_data.chat_npc_if_allowed_to_speak(
                "npc-speech-villager_decline_trade",
                agent,
                event_emitter,
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
            if let Some(target) = get_entity_by_id(with.id(), read_data) {
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
        event_emitter,
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
                        "npc-speech-merchant_trade_successful",
                        agent,
                        event_emitter,
                    );
                },
                _ => {
                    agent_data.chat_npc_if_allowed_to_speak(
                        "npc-speech-merchant_trade_declined",
                        agent,
                        event_emitter,
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
        event_emitter,
        ..
    } = bdata;

    if !matches!(agent.inbox.front(), Some(AgentEvent::UpdatePendingTrade(_))) {
        return false;
    }

    if let Some(AgentEvent::UpdatePendingTrade(boxval)) = agent.inbox.pop_front() {
        let (tradeid, pending, prices, inventories) = *boxval;
        if agent.behavior.is(BehaviorState::TRADING) {
            let who = usize::from(!agent.behavior.is(BehaviorState::TRADING_ISSUER));
            match agent.behavior.trading_behavior {
                TradingBehavior::RequireBalanced { .. } => {
                    let balance0: f32 =
                        prices.balance(&pending.offers, &inventories, 1 - who, true);
                    let balance1: f32 = prices.balance(&pending.offers, &inventories, who, false);
                    if balance0 >= balance1 {
                        // If the trade is favourable to us, only send an accept message if we're
                        // not already accepting (since otherwise, spam-clicking the accept button
                        // results in lagging and moving to the review phase of an unfavorable trade
                        // (although since the phase is included in the message, this shouldn't
                        // result in fully accepting an unfavourable trade))
                        if !pending.accept_flags[who] && !pending.is_empty_trade() {
                            event_emitter.emit(ServerEvent::ProcessTradeAction(
                                *agent_data.entity,
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
                                        UnresolvedChatMsg::npc_tell(*agent_data.uid, *with, msg),
                                    ));
                                } else {
                                    event_emitter.emit(ServerEvent::Chat(
                                        UnresolvedChatMsg::npc_say(*agent_data.uid, msg),
                                    ));
                                }
                            } else {
                                event_emitter.emit(ServerEvent::Chat(UnresolvedChatMsg::npc_say(
                                    *agent_data.uid,
                                    msg,
                                )));
                            }
                        }
                        if pending.phase != TradePhase::Mutate {
                            // we got into the review phase but without balanced goods, decline
                            agent.behavior.unset(BehaviorState::TRADING);
                            agent.target = None;
                            event_emitter.emit(ServerEvent::ProcessTradeAction(
                                *agent_data.entity,
                                tradeid,
                                TradeAction::Decline,
                            ));
                        }
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
                        event_emitter.emit(ServerEvent::ProcessTradeAction(
                            *agent_data.entity,
                            tradeid,
                            TradeAction::Accept(pending.phase),
                        ));
                    }
                },
                TradingBehavior::None => {
                    agent.behavior.unset(BehaviorState::TRADING);
                    agent.target = None;
                    event_emitter.emit(ServerEvent::ProcessTradeAction(
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
        event_emitter,
        controller,
        ..
    } = bdata;

    if let Some(msg) = agent.inbox.front() {
        let used = match msg {
            AgentEvent::Talk(by, _) | AgentEvent::TradeAccepted(by) => {
                if let (Some(target), Some(speaker)) =
                    (agent.target, get_entity_by_id(by.id(), bdata.read_data))
                {
                    // in combat, speak to players that aren't the current target
                    if !target.hostile || target.target != speaker {
                        agent_data.chat_npc_if_allowed_to_speak(
                            "npc-speech-villager_busy",
                            agent,
                            event_emitter,
                        );
                    }
                }

                true
            },
            AgentEvent::TradeInvite(by) => {
                controller.push_invite_response(InviteResponse::Decline);
                if let (Some(target), Some(speaker)) =
                    (agent.target, get_entity_by_id(by.id(), bdata.read_data))
                {
                    // in combat, speak to players that aren't the current target
                    if !target.hostile || target.target != speaker {
                        if agent.behavior.can_trade(agent_data.alignment.copied(), *by) {
                            agent_data.chat_npc_if_allowed_to_speak(
                                "npc-speech-merchant_busy",
                                agent,
                                event_emitter,
                            );
                        } else {
                            agent_data.chat_npc_if_allowed_to_speak(
                                "npc-speech-villager_busy",
                                agent,
                                event_emitter,
                            );
                        }
                    }
                }
                true
            },
            AgentEvent::FinishedTrade(result) => {
                // copy pasted from recv_interaction
                // because the trade is not cancellable in this state
                if agent.behavior.is(BehaviorState::TRADING) {
                    match result {
                        TradeResult::Completed => {
                            agent_data.chat_npc_if_allowed_to_speak(
                                "npc-speech-merchant_trade_successful",
                                agent,
                                event_emitter,
                            );
                        },
                        _ => {
                            agent_data.chat_npc_if_allowed_to_speak(
                                "npc-speech-merchant_trade_declined",
                                agent,
                                event_emitter,
                            );
                        },
                    }
                    agent.behavior.unset(BehaviorState::TRADING);
                    agent.target = None;
                }
                true
            },
            AgentEvent::UpdatePendingTrade(boxval) => {
                // immediately cancel the trade
                let (tradeid, _pending, _prices, _inventories) = &**boxval;
                agent.behavior.unset(BehaviorState::TRADING);
                agent.target = None;
                event_emitter.emit(ServerEvent::ProcessTradeAction(
                    *agent_data.entity,
                    *tradeid,
                    TradeAction::Decline,
                ));
                agent_data.chat_npc_if_allowed_to_speak(
                    "npc-speech-merchant_trade_cancelled_hostile",
                    agent,
                    event_emitter,
                );
                true
            },
            AgentEvent::ServerSound(_) | AgentEvent::Hurt => false,
        };
        if used {
            agent.inbox.pop_front();
        }
    }
    false
}
