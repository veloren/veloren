use super::group_manip;
use crate::{client::Client, Server};
use common::{
    comp::{
        self,
        agent::AgentEvent,
        group::GroupManager,
        invite::{Invite, InviteKind, InviteResponse, PendingInvites},
        ChatType,
    },
    trade::Trades,
    uid::Uid,
};
use common_net::{
    msg::{InviteAnswer, ServerGeneral},
    sync::WorldSyncExt,
};
use specs::world::WorldExt;
use std::time::{Duration, Instant};
use tracing::{error, warn};

/// Time before invite times out
const INVITE_TIMEOUT_DUR: Duration = Duration::from_secs(31);
/// Reduced duration shown to the client to help alleviate latency issues
const PRESENTED_INVITE_TIMEOUT_DUR: Duration = Duration::from_secs(30);

pub fn handle_invite(
    server: &mut Server,
    inviter: specs::Entity,
    invitee_uid: Uid,
    kind: InviteKind,
) {
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
    }

    let mut agents = state.ecs().write_storage::<comp::Agent>();
    let mut invites = state.ecs().write_storage::<Invite>();

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
    // Returns true if insertion was succesful
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
                agent.inbox.push_front(AgentEvent::TradeInvite(*inviter));
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
pub fn handle_invite_response(
    server: &mut Server,
    entity: specs::Entity,
    response: InviteResponse,
) {
    match response {
        InviteResponse::Accept => handle_invite_accept(server, entity),
        InviteResponse::Decline => handle_invite_decline(server, entity),
    }
}

pub fn handle_invite_accept(server: &mut Server, entity: specs::Entity) {
    let state = server.state_mut();
    let clients = state.ecs().read_storage::<Client>();
    let uids = state.ecs().read_storage::<Uid>();
    let mut invites = state.ecs().write_storage::<Invite>();
    if let Some((inviter, kind)) = invites.remove(entity).and_then(|invite| {
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
    }) {
        if let (Some(client), Some(target)) = (clients.get(inviter), uids.get(entity).copied()) {
            client.send_fallible(ServerGeneral::InviteComplete {
                target,
                answer: InviteAnswer::Accepted,
                kind,
            });
        }
        match kind {
            InviteKind::Group => {
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
                                    .try_map(|e| uids.get(e).copied())
                                    .map(|g| (g, c))
                            })
                            .map(|(g, c)| c.send(ServerGeneral::GroupUpdate(g)));
                    },
                );
            },
            InviteKind::Trade => {
                if let (Some(inviter_uid), Some(invitee_uid)) =
                    (uids.get(inviter).copied(), uids.get(entity).copied())
                {
                    let mut trades = state.ecs().write_resource::<Trades>();
                    let id = trades.begin_trade(inviter_uid, invitee_uid);
                    let trade = trades.trades[&id].clone();
                    clients
                        .get(inviter)
                        .map(|c| c.send(ServerGeneral::UpdatePendingTrade(id, trade.clone())));
                    clients
                        .get(entity)
                        .map(|c| c.send(ServerGeneral::UpdatePendingTrade(id, trade)));
                }
            },
        }
    }
}

pub fn handle_invite_decline(server: &mut Server, entity: specs::Entity) {
    let state = server.state_mut();
    let clients = state.ecs().read_storage::<Client>();
    let uids = state.ecs().read_storage::<Uid>();
    let mut invites = state.ecs().write_storage::<Invite>();
    if let Some((inviter, kind)) = invites.remove(entity).and_then(|invite| {
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
    }) {
        // Inform inviter of rejection
        if let (Some(client), Some(target)) = (clients.get(inviter), uids.get(entity).copied()) {
            client.send_fallible(ServerGeneral::InviteComplete {
                target,
                answer: InviteAnswer::Declined,
                kind,
            });
        }
    }
}
