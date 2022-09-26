use super::group_manip::{self, update_map_markers};
use crate::{client::Client, Server};
use common::{
    comp::{
        self,
        agent::{Agent, AgentEvent},
        group::GroupManager,
        invite::{Invite, InviteKind, InviteResponse, PendingInvites},
        ChatType, Pos,
    },
    consts::MAX_TRADE_RANGE,
    trade::{TradeResult, Trades},
    uid::Uid,
};
use common_net::{
    msg::{InviteAnswer, ServerGeneral},
    sync::WorldSyncExt,
};
use common_state::State;
use specs::{world::WorldExt, Entity};
use std::time::{Duration, Instant};
use tracing::{error, warn};

/// Time before invite times out
const INVITE_TIMEOUT_DUR: Duration = Duration::from_secs(31);
/// Reduced duration shown to the client to help alleviate latency issues
const PRESENTED_INVITE_TIMEOUT_DUR: Duration = Duration::from_secs(30);

pub fn handle_invite(server: &mut Server, inviter: Entity, invitee_uid: Uid, kind: InviteKind) {
    let max_group_size = server.settings().max_player_group_size;
    let state = server.state_mut();
    let clients = state.ecs().read_storage::<Client>();
    let invitee = match state.ecs().entity_from_uid(invitee_uid.into()) {
        Some(t) => t,
        None => {
            // Inform of failure
            if let Some(client) = clients.get(inviter) {
                client.send_fallible(ServerGeneral::server_msg(
                    ChatType::Meta,
                    "Invite failed, target does not exist.",
                ));
            }
            return;
        },
    };

    let uids = state.ecs().read_storage::<Uid>();

    // Check if entity is trying to invite themselves
    if uids
        .get(inviter)
        .map_or(false, |inviter_uid| *inviter_uid == invitee_uid)
    {
        warn!("Entity tried to invite themselves into a group/trade");
        return;
    }

    let mut pending_invites = state.ecs().write_storage::<PendingInvites>();
    let mut agents = state.ecs().write_storage::<Agent>();
    let mut invites = state.ecs().write_storage::<Invite>();

    if let InviteKind::Trade = kind {
        // Check whether the inviter is in range of the invitee
        let positions = state.ecs().read_storage::<Pos>();
        if !within_trading_range(positions.get(inviter), positions.get(invitee)) {
            return;
        }
    }

    if let InviteKind::Group = kind {
        if !group_manip::can_invite(
            state,
            &clients,
            &mut pending_invites,
            max_group_size,
            inviter,
            invitee,
        ) {
            return;
        }
    } else {
        // cancel current trades for inviter before inviting someone else to trade
        let mut trades = state.ecs().write_resource::<Trades>();
        if let Some(inviter_uid) = uids.get(inviter).copied() {
            if let Some(active_trade) = trades.entity_trades.get(&inviter_uid).copied() {
                trades
                    .decline_trade(active_trade, inviter_uid)
                    .and_then(|u| state.ecs().entity_from_uid(u.0))
                    .map(|e| {
                        if let Some(client) = clients.get(e) {
                            client
                                .send_fallible(ServerGeneral::FinishedTrade(TradeResult::Declined));
                        }
                        if let Some(agent) = agents.get_mut(e) {
                            agent
                                .inbox
                                .push_back(AgentEvent::FinishedTrade(TradeResult::Declined));
                        }
                    });
            }
        };
    }

    if invites.contains(invitee) {
        // Inform inviter that there is already an invite
        if let Some(client) = clients.get(inviter) {
            client.send_fallible(ServerGeneral::server_msg(
                ChatType::Meta,
                "This player already has a pending invite.",
            ));
        }
        return;
    }

    let mut invite_sent = false;
    // Returns true if insertion was successful
    let mut send_invite = || {
        match invites.insert(invitee, Invite { inviter, kind }) {
            Err(err) => {
                error!("Failed to insert Invite component: {:?}", err);
                false
            },
            Ok(_) => {
                match pending_invites.entry(inviter) {
                    Ok(entry) => {
                        entry.or_insert_with(|| PendingInvites(Vec::new())).0.push((
                            invitee,
                            kind,
                            Instant::now() + INVITE_TIMEOUT_DUR,
                        ));
                        invite_sent = true;
                        true
                    },
                    Err(err) => {
                        error!(
                            "Failed to get entry for pending invites component: {:?}",
                            err
                        );
                        // Cleanup
                        invites.remove(invitee);
                        false
                    },
                }
            },
        }
    };

    // If client comp
    if let (Some(client), Some(inviter)) = (clients.get(invitee), uids.get(inviter).copied()) {
        if send_invite() {
            client.send_fallible(ServerGeneral::Invite {
                inviter,
                timeout: PRESENTED_INVITE_TIMEOUT_DUR,
                kind,
            });
        }
    } else if let Some(agent) = agents.get_mut(invitee) {
        if send_invite() {
            if let Some(inviter) = uids.get(inviter) {
                agent.inbox.push_back(AgentEvent::TradeInvite(*inviter));
                invite_sent = true;
            }
        }
    } else if let Some(client) = clients.get(inviter) {
        client.send_fallible(ServerGeneral::server_msg(
            ChatType::Meta,
            "Can't invite, not a player or npc",
        ));
    }

    // Notify inviter that the invite is pending
    if invite_sent {
        if let Some(client) = clients.get(inviter) {
            client.send_fallible(ServerGeneral::InvitePending(invitee_uid));
        }
    }
}
pub fn handle_invite_response(server: &mut Server, entity: Entity, response: InviteResponse) {
    match response {
        InviteResponse::Accept => handle_invite_accept(server, entity),
        InviteResponse::Decline => handle_invite_decline(server, entity),
    }
}

pub fn handle_invite_accept(server: &mut Server, entity: Entity) {
    let index = server.index.clone();
    let state = server.state_mut();
    if let Some((inviter, kind)) = get_inviter_and_kind(entity, state) {
        handle_invite_answer(state, inviter, entity, InviteAnswer::Accepted, kind);
        let clients = state.ecs().read_storage::<Client>();
        let uids = state.ecs().read_storage::<Uid>();
        let mut agents = state.ecs().write_storage::<Agent>();

        match kind {
            InviteKind::Group => {
                let map_markers = state.ecs().read_storage::<comp::MapMarker>();
                let mut group_manager = state.ecs().write_resource::<GroupManager>();
                group_manager.add_group_member(
                    inviter,
                    entity,
                    &state.ecs().entities(),
                    &mut state.ecs().write_storage(),
                    &state.ecs().read_storage(),
                    &uids,
                    |entity, group_change| {
                        clients
                            .get(entity)
                            .and_then(|c| {
                                group_change
                                    .try_map_ref(|e| uids.get(*e).copied())
                                    .map(|g| (g, c))
                            })
                            .map(|(g, c)| {
                                update_map_markers(&map_markers, &uids, c, &group_change);
                                c.send_fallible(ServerGeneral::GroupUpdate(g));
                            });
                    },
                );
            },
            InviteKind::Trade => {
                if let (Some(inviter_uid), Some(invitee_uid)) =
                    (uids.get(inviter).copied(), uids.get(entity).copied())
                {
                    let mut trades = state.ecs().write_resource::<Trades>();
                    // check if the person that invited me has started a new trade since the
                    // invitation was sent
                    if trades.entity_trades.get(&inviter_uid).copied().is_some() {
                        for client in clients.get(entity).into_iter().chain(clients.get(inviter)) {
                            client.send_fallible(ServerGeneral::server_msg(
                                ChatType::Meta,
                                "Trade failed, inviter initiated new trade since sending trade \
                                 request.",
                            ));
                        }
                        return;
                    }
                    let id = trades.begin_trade(inviter_uid, invitee_uid);
                    let trade = trades.trades[&id].clone();
                    if let Some(agent) = agents.get_mut(inviter) {
                        agent
                            .inbox
                            .push_back(AgentEvent::TradeAccepted(invitee_uid));
                    }
                    #[cfg(feature = "worldgen")]
                    let pricing = agents
                        .get(inviter)
                        .and_then(|a| {
                            a.behavior
                                .trade_site()
                                .and_then(|id| index.get_site_prices(id))
                        })
                        .or_else(|| {
                            agents.get(entity).and_then(|a| {
                                a.behavior
                                    .trade_site()
                                    .and_then(|id| index.get_site_prices(id))
                            })
                        });
                    #[cfg(not(feature = "worldgen"))]
                    let pricing = None;

                    clients.get(inviter).map(|c| {
                        c.send(ServerGeneral::UpdatePendingTrade(
                            id,
                            trade.clone(),
                            pricing.clone(),
                        ))
                    });
                    clients
                        .get(entity)
                        .map(|c| c.send(ServerGeneral::UpdatePendingTrade(id, trade, pricing)));
                }
            },
        }
    }
}

fn get_inviter_and_kind(entity: Entity, state: &mut State) -> Option<(Entity, InviteKind)> {
    let mut invites = state.ecs().write_storage::<Invite>();
    invites.remove(entity).and_then(|invite| {
        let Invite { inviter, kind } = invite;
        let mut pending_invites = state.ecs().write_storage::<PendingInvites>();
        let pending = &mut pending_invites.get_mut(inviter)?.0;
        // Check that inviter has a pending invite and remove it from the list
        let invite_index = pending.iter().position(|p| p.0 == entity)?;
        pending.swap_remove(invite_index);
        // If no pending invites remain remove the component
        if pending.is_empty() {
            pending_invites.remove(inviter);
        }

        Some((inviter, kind))
    })
}

fn handle_invite_answer(
    state: &mut State,
    inviter: Entity,
    entity: Entity,
    invite_answer: InviteAnswer,
    kind: InviteKind,
) {
    let clients = state.ecs().read_storage::<Client>();
    let uids = state.ecs().read_storage::<Uid>();
    if matches!(kind, InviteKind::Trade) && matches!(invite_answer, InviteAnswer::Accepted) {
        // invitee must close current trade if one exists before accepting new one
        if let Some(invitee_uid) = uids.get(entity).copied() {
            let mut trades = state.ecs().write_resource::<Trades>();
            if let Some(active_trade) = trades.entity_trades.get(&invitee_uid).copied() {
                trades
                    .decline_trade(active_trade, invitee_uid)
                    .and_then(|u| state.ecs().entity_from_uid(u.0))
                    .map(|e| {
                        if let Some(client) = clients.get(e) {
                            client
                                .send_fallible(ServerGeneral::FinishedTrade(TradeResult::Declined));
                        }
                        if let Some(agent) = state.ecs().write_storage::<Agent>().get_mut(e) {
                            agent
                                .inbox
                                .push_back(AgentEvent::FinishedTrade(TradeResult::Declined));
                        }
                    });
            }
        };
    }
    if let (Some(client), Some(target)) = (clients.get(inviter), uids.get(entity).copied()) {
        client.send_fallible(ServerGeneral::InviteComplete {
            target,
            answer: invite_answer,
            kind,
        });
    }
}

pub fn handle_invite_decline(server: &mut Server, entity: Entity) {
    let state = server.state_mut();
    if let Some((inviter, kind)) = get_inviter_and_kind(entity, state) {
        // Inform inviter of rejection
        handle_invite_answer(state, inviter, entity, InviteAnswer::Declined, kind)
    }
}

fn within_trading_range(requester_position: Option<&Pos>, invitee_position: Option<&Pos>) -> bool {
    match (requester_position, invitee_position) {
        (Some(rpos), Some(ipos)) => rpos.0.distance_squared(ipos.0) < MAX_TRADE_RANGE.powi(2),
        _ => false,
    }
}
