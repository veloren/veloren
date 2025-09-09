use common::{
    comp::{
        BehaviorState, Content, ControlAction, Item, TradingBehavior, UnresolvedChatMsg,
        UtteranceKind,
        agent::{AgentEvent, Target, TimerAction},
        inventory::item::{ItemTag, MaterialStatManifest},
        invite::InviteResponse,
        tool::AbilityMap,
    },
    event::{ChatEvent, EmitExt, ProcessTradeActionEvent},
    rtsim::{Actor, NpcInput},
    trade::{TradeAction, TradePhase, TradeResult},
};
use rand::Rng;

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
                    if bdata.rng.random::<f32>() < 0.4 {
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

pub fn handle_inbox_dialogue(bdata: &mut BehaviorData) -> bool {
    let BehaviorData {
        agent, read_data, ..
    } = bdata;

    if !matches!(agent.inbox.front(), Some(AgentEvent::Dialogue(_, _))) {
        return false;
    }

    if let Some(AgentEvent::Dialogue(sender, dialogue)) = agent.inbox.pop_front()
        && let Some(rtsim_outbox) = &mut agent.rtsim_outbox
        && let Some(sender_entity) = read_data.id_maps.uid_entity(sender)
        && let Some(sender_actor) = read_data
            .presences
            .get(sender_entity)
            .and_then(|p| p.kind.character_id().map(Actor::Character))
            .or_else(|| Some(Actor::Npc(read_data.rtsim_entities.get(sender_entity)?.0)))
    {
        rtsim_outbox.push_back(NpcInput::Dialogue(sender_actor, dialogue));
        return false;
    }
    true
}

/// Handles Talk event if the front of the agent's inbox contains one
pub fn handle_inbox_talk(bdata: &mut BehaviorData) -> bool {
    let BehaviorData {
        agent, read_data, ..
    } = bdata;

    if !matches!(agent.inbox.front(), Some(AgentEvent::Talk(_))) {
        return false;
    }

    if let Some(AgentEvent::Talk(by)) = agent.inbox.pop_front() {
        let by_entity = get_entity_by_id(by, read_data);

        if let Some(rtsim_outbox) = &mut agent.rtsim_outbox
            && let Some(by_entity) = by_entity
            && let Some(actor) = read_data
                .presences
                .get(by_entity)
                .and_then(|p| p.kind.character_id().map(Actor::Character))
                .or_else(|| Some(Actor::Npc(read_data.rtsim_entities.get(by_entity)?.0)))
        {
            rtsim_outbox.push_back(NpcInput::Interaction(actor));
            return false;
        }

        if agent.allowed_to_speak()
            && let Some(target) = by_entity
        {
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
                    Content::localized("npc-speech-merchant_busy_trading"),
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

    if let Some(AgentEvent::TradeAccepted(with)) = agent.inbox.pop_front()
        && !agent.behavior.is(BehaviorState::TRADING)
    {
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

    if let Some(AgentEvent::FinishedTrade(result)) = agent.inbox.pop_front()
        && agent.behavior.is(BehaviorState::TRADING)
    {
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
                    emitters.emit(ChatEvent {
                        msg: UnresolvedChatMsg::npc_tell(*agent_data.uid, *with, content),
                        from_client: false,
                    });
                } else {
                    emitters.emit(ChatEvent {
                        msg: UnresolvedChatMsg::npc_say(*agent_data.uid, content),
                        from_client: false,
                    });
                }
            };
            match agent.behavior.trading_behavior {
                TradingBehavior::RequireBalanced { .. } => {
                    let balance0 = prices.balance(&pending.offers, &inventories, 1 - who, true);
                    let balance1 = prices.balance(&pending.offers, &inventories, who, false);
                    match (balance0, balance1) {
                        (_, None) => {
                            message(Content::localized("npc-speech-merchant_reject_sell_item"))
                        },
                        (None, _) => {
                            message(Content::localized("npc-speech-merchant_reject_buy_item"))
                        },
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
                                    message(Content::localized_with_args(
                                        "npc-speech-merchant_trade_balance",
                                        [(
                                            "percentage",
                                            format!("{:.0}", (balance0 / balance1 * 100.0).floor()),
                                        )],
                                    ));
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
                            if let Some(item) = ri.inventory.get(slot)
                                && let Ok(item) = Item::new_from_item_definition_id(
                                    item.name.as_ref(),
                                    &ability_map,
                                    &msm,
                                )
                                && !item.tags().contains(&ItemTag::Food)
                            {
                                only_food = false;
                                break;
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
            AgentEvent::Talk(by) | AgentEvent::TradeAccepted(by) | AgentEvent::Dialogue(by, _) => {
                if agent
                    .target
                    .zip(get_entity_by_id(*by, bdata.read_data))
                    // In combat, speak to players that aren't the current target.
                    .is_some_and(|(target, speaker)| !target.hostile || target.target != speaker)
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
                                Content::localized("npc-speech-merchant_busy_combat"),
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
