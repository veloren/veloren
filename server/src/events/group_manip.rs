use crate::{client::Client, Server};
use common::{
    comp::{
        self,
        group::{self, Group, GroupManager, Invite, InviteKind, PendingInvites},
        ChatType, GroupManip,
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
        // Disallow inviting entity that is already in your group
        let groups = state.ecs().read_storage::<Group>();
        let group_manager = state.ecs().read_resource::<GroupManager>();
        let already_in_same_group = groups.get(inviter).map_or(false, |group| {
            group_manager
                .group_info(*group)
                .map_or(false, |g| g.leader == inviter)
                && groups.get(invitee) == Some(group)
        });
        if already_in_same_group {
            // Inform of failure
            if let Some(client) = clients.get(inviter) {
                client.send_fallible(ServerGeneral::server_msg(
                    ChatType::Meta,
                    "Invite failed, can't invite someone already in your group",
                ));
            }
            return;
        }

        // Check if group max size is already reached
        // Adding the current number of pending invites
        let group_size_limit_reached = state
            .ecs()
            .read_storage()
            .get(inviter)
            .copied()
            .and_then(|group| {
                // If entity is currently the leader of a full group then they can't invite
                // anyone else
                group_manager
                    .group_info(group)
                    .filter(|i| i.leader == inviter)
                    .map(|i| i.num_members)
            })
            .unwrap_or(1) as usize
            + pending_invites.get(inviter).map_or(0, |p| p.0.len())
            >= max_group_size as usize;
        if group_size_limit_reached {
            // Inform inviter that they have reached the group size limit
            if let Some(client) = clients.get(inviter) {
                client.send_fallible(ServerGeneral::server_msg(
                    ChatType::Meta,
                    "Invite failed, pending invites plus current group size have reached the \
                     group size limit"
                        .to_owned(),
                ));
            }
            return;
        }
    }

    let agents = state.ecs().read_storage::<comp::Agent>();
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
        match invites.insert(invitee, group::Invite { inviter, kind }) {
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
    } else if agents.contains(invitee) {
        send_invite();
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

// TODO: turn chat messages into enums
pub fn handle_group(server: &mut Server, entity: specs::Entity, manip: GroupManip) {
    match manip {
        GroupManip::Invite(uid) => {
            handle_invite(server, entity, uid, InviteKind::Group);
        },
        GroupManip::Accept => {
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
                if let (Some(client), Some(target)) =
                    (clients.get(inviter), uids.get(entity).copied())
                {
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
                            clients.get(inviter).map(|c| {
                                c.send(ServerGeneral::UpdatePendingTrade(id, trade.clone()))
                            });
                            clients
                                .get(entity)
                                .map(|c| c.send(ServerGeneral::UpdatePendingTrade(id, trade)));
                        }
                    },
                }
            }
        },
        GroupManip::Decline => {
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
                if let (Some(client), Some(target)) =
                    (clients.get(inviter), uids.get(entity).copied())
                {
                    client.send_fallible(ServerGeneral::InviteComplete {
                        target,
                        answer: InviteAnswer::Declined,
                        kind,
                    });
                }
            }
        },
        GroupManip::Leave => {
            let state = server.state_mut();
            let clients = state.ecs().read_storage::<Client>();
            let uids = state.ecs().read_storage::<Uid>();
            let mut group_manager = state.ecs().write_resource::<GroupManager>();
            group_manager.leave_group(
                entity,
                &mut state.ecs().write_storage(),
                &state.ecs().read_storage(),
                &uids,
                &state.ecs().entities(),
                &mut |entity, group_change| {
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
        GroupManip::Kick(uid) => {
            let state = server.state_mut();
            let clients = state.ecs().read_storage::<Client>();
            let uids = state.ecs().read_storage::<Uid>();
            let alignments = state.ecs().read_storage::<comp::Alignment>();

            let target = match state.ecs().entity_from_uid(uid.into()) {
                Some(t) => t,
                None => {
                    // Inform of failure
                    if let Some(client) = clients.get(entity) {
                        client.send_fallible(ServerGeneral::server_msg(
                            ChatType::Meta,
                            "Kick failed, target does not exist.",
                        ));
                    }
                    return;
                },
            };

            // Can't kick pet
            if matches!(alignments.get(target), Some(comp::Alignment::Owned(owner)) if uids.get(target).map_or(true, |u| u != owner))
            {
                if let Some(general_stream) = clients.get(entity) {
                    general_stream.send_fallible(ServerGeneral::server_msg(
                        ChatType::Meta,
                        "Kick failed, you can't kick pets.",
                    ));
                }
                return;
            }
            // Can't kick yourself
            if uids.get(entity).map_or(false, |u| *u == uid) {
                if let Some(client) = clients.get(entity) {
                    client.send_fallible(ServerGeneral::server_msg(
                        ChatType::Meta,
                        "Kick failed, you can't kick yourself.",
                    ));
                }
                return;
            }

            let mut groups = state.ecs().write_storage::<group::Group>();
            let mut group_manager = state.ecs().write_resource::<GroupManager>();
            // Make sure kicker is the group leader
            match groups
                .get(target)
                .and_then(|group| group_manager.group_info(*group))
            {
                Some(info) if info.leader == entity => {
                    // Remove target from group
                    group_manager.leave_group(
                        target,
                        &mut groups,
                        &state.ecs().read_storage(),
                        &uids,
                        &state.ecs().entities(),
                        &mut |entity, group_change| {
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

                    // Tell them the have been kicked
                    if let Some(client) = clients.get(target) {
                        client.send_fallible(ServerGeneral::server_msg(
                            ChatType::Meta,
                            "You were removed from the group.",
                        ));
                    }
                    // Tell kicker that they were succesful
                    if let Some(client) = clients.get(entity) {
                        client.send_fallible(ServerGeneral::server_msg(
                            ChatType::Meta,
                            "Player kicked.",
                        ));
                    }
                },
                Some(_) => {
                    // Inform kicker that they are not the leader
                    if let Some(client) = clients.get(entity) {
                        client.send_fallible(ServerGeneral::server_msg(
                            ChatType::Meta,
                            "Kick failed: You are not the leader of the target's group.",
                        ));
                    }
                },
                None => {
                    // Inform kicker that the target is not in a group
                    if let Some(client) = clients.get(entity) {
                        client.send_fallible(ServerGeneral::server_msg(
                            ChatType::Meta,
                            "Kick failed: Your target is not in a group.",
                        ));
                    }
                },
            }
        },
        GroupManip::AssignLeader(uid) => {
            let state = server.state_mut();
            let clients = state.ecs().read_storage::<Client>();
            let uids = state.ecs().read_storage::<Uid>();
            let target = match state.ecs().entity_from_uid(uid.into()) {
                Some(t) => t,
                None => {
                    // Inform of failure
                    if let Some(client) = clients.get(entity) {
                        client.send_fallible(ServerGeneral::server_msg(
                            ChatType::Meta,
                            "Leadership transfer failed, target does not exist",
                        ));
                    }
                    return;
                },
            };
            let groups = state.ecs().read_storage::<group::Group>();
            let mut group_manager = state.ecs().write_resource::<GroupManager>();
            // Make sure assigner is the group leader
            match groups
                .get(target)
                .and_then(|group| group_manager.group_info(*group))
            {
                Some(info) if info.leader == entity => {
                    // Assign target as group leader
                    group_manager.assign_leader(
                        target,
                        &groups,
                        &state.ecs().entities(),
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
                    // Tell them they are the leader
                    if let Some(client) = clients.get(target) {
                        client.send_fallible(ServerGeneral::server_msg(
                            ChatType::Meta,
                            "You are the group leader now.",
                        ));
                    }
                    // Tell the old leader that the transfer was succesful
                    if let Some(client) = clients.get(target) {
                        client.send_fallible(ServerGeneral::server_msg(
                            ChatType::Meta,
                            "You are no longer the group leader.",
                        ));
                    }
                },
                Some(_) => {
                    // Inform transferer that they are not the leader
                    if let Some(client) = clients.get(entity) {
                        client.send_fallible(ServerGeneral::server_msg(
                            ChatType::Meta,
                            "Transfer failed: You are not the leader of the target's group.",
                        ));
                    }
                },
                None => {
                    // Inform transferer that the target is not in a group
                    if let Some(client) = clients.get(entity) {
                        client.send_fallible(ServerGeneral::server_msg(
                            ChatType::Meta,
                            "Transfer failed: Your target is not in a group.",
                        ));
                    }
                },
            }
        },
    }
}
