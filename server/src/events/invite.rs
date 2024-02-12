use super::{
    event_dispatch,
    group_manip::{self, update_map_markers},
    ServerEvent,
};
use crate::{client::Client, Settings};
use common::{
    comp::{
        self,
        agent::{Agent, AgentEvent},
        group::GroupManager,
        invite::{Invite, InviteKind, InviteResponse, PendingInvites},
        ChatType, Group, Pos,
    },
    consts::MAX_TRADE_RANGE,
    event::{InitiateInviteEvent, InviteResponseEvent},
    trade::{TradeResult, Trades},
    uid::{IdMaps, Uid},
};
use common_net::msg::{InviteAnswer, ServerGeneral};
use specs::{
    shred, DispatcherBuilder, Entities, Entity, Read, ReadExpect, ReadStorage, SystemData, Write,
    WriteStorage,
};
use std::time::{Duration, Instant};
use tracing::{error, warn};

/// Time before invite times out
const INVITE_TIMEOUT_DUR: Duration = Duration::from_secs(31);
/// Reduced duration shown to the client to help alleviate latency issues
const PRESENTED_INVITE_TIMEOUT_DUR: Duration = Duration::from_secs(30);

pub(super) fn register_event_systems(builder: &mut DispatcherBuilder) {
    event_dispatch::<InitiateInviteEvent>(builder);
    event_dispatch::<InviteResponseEvent>(builder);
}

impl ServerEvent for InitiateInviteEvent {
    type SystemData<'a> = (
        Write<'a, Trades>,
        Read<'a, Settings>,
        Read<'a, IdMaps>,
        Read<'a, GroupManager>,
        WriteStorage<'a, PendingInvites>,
        WriteStorage<'a, Agent>,
        WriteStorage<'a, Invite>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, Client>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, Group>,
    );

    fn handle(
        events: impl ExactSizeIterator<Item = Self>,
        (
            mut trades,
            settings,
            id_maps,
            group_manager,
            mut pending_invites,
            mut agents,
            mut invites,
            uids,
            clients,
            positions,
            groups,
        ): Self::SystemData<'_>,
    ) {
        for InitiateInviteEvent(inviter, invitee_uid, kind) in events {
            let max_group_size = settings.max_player_group_size;
            let invitee = match id_maps.uid_entity(invitee_uid) {
                Some(t) => t,
                None => {
                    // Inform of failure
                    if let Some(client) = clients.get(inviter) {
                        client.send_fallible(ServerGeneral::server_msg(
                            ChatType::Meta,
                            "Invite failed, target does not exist.",
                        ));
                    }
                    continue;
                },
            };

            // Check if entity is trying to invite themselves
            if uids
                .get(inviter)
                .map_or(false, |inviter_uid| *inviter_uid == invitee_uid)
            {
                warn!("Entity tried to invite themselves into a group/trade");
                continue;
            }

            if matches!(kind, InviteKind::Trade) {
                // Check whether the inviter is in range of the invitee
                if !within_trading_range(positions.get(inviter), positions.get(invitee)) {
                    continue;
                }
            }

            if let InviteKind::Group = kind {
                if !group_manip::can_invite(
                    &clients,
                    &groups,
                    &group_manager,
                    &mut pending_invites,
                    max_group_size,
                    inviter,
                    invitee,
                ) {
                    continue;
                }
            } else {
                // cancel current trades for inviter before inviting someone else to trade
                if let Some(inviter_uid) = uids.get(inviter).copied() {
                    if let Some(active_trade) = trades.entity_trades.get(&inviter_uid).copied() {
                        trades
                            .decline_trade(active_trade, inviter_uid)
                            .and_then(|u| id_maps.uid_entity(u))
                            .map(|e| {
                                if let Some(client) = clients.get(e) {
                                    client.send_fallible(ServerGeneral::FinishedTrade(
                                        TradeResult::Declined,
                                    ));
                                }
                                if let Some(agent) = agents.get_mut(e) {
                                    agent.inbox.push_back(AgentEvent::FinishedTrade(
                                        TradeResult::Declined,
                                    ));
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
                continue;
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
            if let (Some(client), Some(inviter)) =
                (clients.get(invitee), uids.get(inviter).copied())
            {
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
    }
}

#[derive(SystemData)]
pub struct InviteResponseData<'a> {
    entities: Entities<'a>,
    group_manager: Write<'a, GroupManager>,
    trades: Write<'a, Trades>,
    index: ReadExpect<'a, world::IndexOwned>,
    id_maps: Read<'a, IdMaps>,
    invites: WriteStorage<'a, Invite>,
    pending_invites: WriteStorage<'a, PendingInvites>,
    groups: WriteStorage<'a, Group>,
    agents: WriteStorage<'a, comp::Agent>,
    uids: ReadStorage<'a, Uid>,
    clients: ReadStorage<'a, Client>,
    alignments: ReadStorage<'a, comp::Alignment>,
    map_markers: ReadStorage<'a, comp::MapMarker>,
}

impl ServerEvent for InviteResponseEvent {
    type SystemData<'a> = InviteResponseData<'a>;

    fn handle(events: impl ExactSizeIterator<Item = Self>, mut data: Self::SystemData<'_>) {
        for InviteResponseEvent(entity, response) in events {
            match response {
                InviteResponse::Accept => handle_invite_accept(&mut data, entity),
                InviteResponse::Decline => handle_invite_decline(&mut data, entity),
            }
        }
    }
}

pub fn handle_invite_accept(data: &mut InviteResponseData, entity: Entity) {
    if let Some((inviter, kind)) = get_inviter_and_kind(entity, data) {
        handle_invite_answer(data, inviter, entity, InviteAnswer::Accepted, kind);

        match kind {
            InviteKind::Group => {
                data.group_manager.add_group_member(
                    inviter,
                    entity,
                    &data.entities,
                    &mut data.groups,
                    &data.alignments,
                    &data.uids,
                    |entity, group_change| {
                        data.clients
                            .get(entity)
                            .and_then(|c| {
                                group_change
                                    .try_map_ref(|e| data.uids.get(*e).copied())
                                    .map(|g| (g, c))
                            })
                            .map(|(g, c)| {
                                update_map_markers(&data.map_markers, &data.uids, c, &group_change);
                                c.send_fallible(ServerGeneral::GroupUpdate(g));
                            });
                    },
                );
            },
            InviteKind::Trade => {
                if let (Some(inviter_uid), Some(invitee_uid)) = (
                    data.uids.get(inviter).copied(),
                    data.uids.get(entity).copied(),
                ) {
                    // check if the person that invited me has started a new trade since the
                    // invitation was sent
                    if data
                        .trades
                        .entity_trades
                        .get(&inviter_uid)
                        .copied()
                        .is_some()
                    {
                        for client in data
                            .clients
                            .get(entity)
                            .into_iter()
                            .chain(data.clients.get(inviter))
                        {
                            client.send_fallible(ServerGeneral::server_msg(
                                ChatType::Meta,
                                "Trade failed, inviter initiated new trade since sending trade \
                                 request.",
                            ));
                        }
                        return;
                    }
                    let id = data.trades.begin_trade(inviter_uid, invitee_uid);
                    let trade = data.trades.trades[&id].clone();
                    if let Some(agent) = data.agents.get_mut(inviter) {
                        agent
                            .inbox
                            .push_back(AgentEvent::TradeAccepted(invitee_uid));
                    }
                    #[cfg(feature = "worldgen")]
                    let pricing = data
                        .agents
                        .get(inviter)
                        .and_then(|a| {
                            a.behavior
                                .trade_site()
                                .and_then(|id| data.index.get_site_prices(id))
                        })
                        .or_else(|| {
                            data.agents.get(entity).and_then(|a| {
                                a.behavior
                                    .trade_site()
                                    .and_then(|id| data.index.get_site_prices(id))
                            })
                        });
                    #[cfg(not(feature = "worldgen"))]
                    let pricing = None;

                    data.clients.get(inviter).map(|c| {
                        c.send(ServerGeneral::UpdatePendingTrade(
                            id,
                            trade.clone(),
                            pricing.clone(),
                        ))
                    });
                    data.clients
                        .get(entity)
                        .map(|c| c.send(ServerGeneral::UpdatePendingTrade(id, trade, pricing)));
                }
            },
        }
    }
}

fn get_inviter_and_kind(
    entity: Entity,
    data: &mut InviteResponseData,
) -> Option<(Entity, InviteKind)> {
    data.invites.remove(entity).and_then(|invite| {
        let Invite { inviter, kind } = invite;
        let pending = &mut data.pending_invites.get_mut(inviter)?.0;
        // Check that inviter has a pending invite and remove it from the list
        let invite_index = pending.iter().position(|p| p.0 == entity)?;
        pending.swap_remove(invite_index);
        // If no pending invites remain remove the component
        if pending.is_empty() {
            data.pending_invites.remove(inviter);
        }

        Some((inviter, kind))
    })
}

fn handle_invite_answer(
    data: &mut InviteResponseData,
    inviter: Entity,
    entity: Entity,
    invite_answer: InviteAnswer,
    kind: InviteKind,
) {
    if matches!(kind, InviteKind::Trade) && matches!(invite_answer, InviteAnswer::Accepted) {
        // invitee must close current trade if one exists before accepting new one
        if let Some(invitee_uid) = data.uids.get(entity).copied() {
            if let Some(active_trade) = data.trades.entity_trades.get(&invitee_uid).copied() {
                data.trades
                    .decline_trade(active_trade, invitee_uid)
                    .and_then(|u| data.id_maps.uid_entity(u))
                    .map(|e| {
                        if let Some(client) = data.clients.get(e) {
                            client
                                .send_fallible(ServerGeneral::FinishedTrade(TradeResult::Declined));
                        }
                        if let Some(agent) = data.agents.get_mut(e) {
                            agent
                                .inbox
                                .push_back(AgentEvent::FinishedTrade(TradeResult::Declined));
                        }
                    });
            }
        };
    }
    if let (Some(client), Some(target)) =
        (data.clients.get(inviter), data.uids.get(entity).copied())
    {
        client.send_fallible(ServerGeneral::InviteComplete {
            target,
            answer: invite_answer,
            kind,
        });
    }
}

pub fn handle_invite_decline(data: &mut InviteResponseData, entity: Entity) {
    if let Some((inviter, kind)) = get_inviter_and_kind(entity, data) {
        // Inform inviter of rejection
        handle_invite_answer(data, inviter, entity, InviteAnswer::Declined, kind)
    }
}

fn within_trading_range(requester_position: Option<&Pos>, invitee_position: Option<&Pos>) -> bool {
    match (requester_position, invitee_position) {
        (Some(rpos), Some(ipos)) => rpos.0.distance_squared(ipos.0) < MAX_TRADE_RANGE.powi(2),
        _ => false,
    }
}
