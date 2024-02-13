use crate::client::Client;
use common::{
    comp::{
        self,
        group::{ChangeNotification, Group, GroupManager},
        invite::{InviteKind, PendingInvites},
        ChatType, GroupManip,
    },
    event::GroupManipEvent,
    uid::{IdMaps, Uid},
};
use common_net::msg::ServerGeneral;
use specs::{world::Entity, DispatcherBuilder, Entities, Read, ReadStorage, Write, WriteStorage};

use super::{event_dispatch, ServerEvent};

pub(super) fn register_event_systems(builder: &mut DispatcherBuilder) {
    event_dispatch::<GroupManipEvent>(builder);
}

pub fn can_invite(
    clients: &ReadStorage<'_, Client>,
    groups: &ReadStorage<'_, Group>,
    group_manager: &GroupManager,
    pending_invites: &mut WriteStorage<'_, PendingInvites>,
    max_group_size: u32,
    inviter: Entity,
    invitee: Entity,
) -> bool {
    // Disallow inviting entity that is already in your group
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
        return false;
    }

    // Check if group max size is already reached
    // Adding the current number of pending invites
    let group_size_limit_reached = groups
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
        + pending_invites.get(inviter).map_or(0, |p| {
            p.0.iter()
                .filter(|(_, k, _)| *k == InviteKind::Group)
                .count()
        })
        >= max_group_size as usize;
    if group_size_limit_reached {
        // Inform inviter that they have reached the group size limit
        if let Some(client) = clients.get(inviter) {
            client.send_fallible(ServerGeneral::server_msg(
                ChatType::Meta,
                "Invite failed, pending invites plus current group size have reached the group \
                 size limit"
                    .to_owned(),
            ));
        }
        return false;
    }

    true
}

pub fn update_map_markers<'a>(
    map_markers: &ReadStorage<'a, comp::MapMarker>,
    uids: &ReadStorage<'a, Uid>,
    client: &Client,
    change: &ChangeNotification<Entity>,
) {
    use comp::group::ChangeNotification::*;
    let send_update = |entity| {
        if let (Some(map_marker), Some(uid)) = (map_markers.get(entity), uids.get(entity)) {
            client.send_fallible(ServerGeneral::MapMarker(
                comp::MapMarkerUpdate::GroupMember(
                    *uid,
                    comp::MapMarkerChange::Update(map_marker.0),
                ),
            ));
        }
    };
    match change {
        &Added(entity, _) => {
            send_update(entity);
        },
        NewGroup { leader: _, members } => {
            for (entity, _) in members {
                send_update(*entity);
            }
        },
        // Removed and NoGroup can be inferred by the client, NewLeader does not affect map markers
        Removed(_) | NoGroup | NewLeader(_) => {},
    }
}

impl ServerEvent for GroupManipEvent {
    type SystemData<'a> = (
        Entities<'a>,
        Write<'a, GroupManager>,
        Read<'a, IdMaps>,
        WriteStorage<'a, Group>,
        ReadStorage<'a, Client>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, comp::Alignment>,
        ReadStorage<'a, comp::MapMarker>,
    );

    fn handle(
        events: impl ExactSizeIterator<Item = Self>,
        (entities, mut group_manager, id_maps, mut groups, clients, uids, alignments, map_markers): Self::SystemData<'_>,
    ) {
        for GroupManipEvent(entity, manip) in events {
            match manip {
                GroupManip::Leave => {
                    group_manager.leave_group(
                        entity,
                        &mut groups,
                        &alignments,
                        &uids,
                        &entities,
                        &mut |entity, group_change| {
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
                GroupManip::Kick(uid) => {
                    let target = match id_maps.uid_entity(uid) {
                        Some(t) => t,
                        None => {
                            // Inform of failure
                            if let Some(client) = clients.get(entity) {
                                client.send_fallible(ServerGeneral::server_msg(
                                    ChatType::Meta,
                                    "Kick failed, target does not exist.",
                                ));
                            }
                            continue;
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
                        continue;
                    }
                    // Can't kick yourself
                    if uids.get(entity).map_or(false, |u| *u == uid) {
                        if let Some(client) = clients.get(entity) {
                            client.send_fallible(ServerGeneral::server_msg(
                                ChatType::Meta,
                                "Kick failed, you can't kick yourself.",
                            ));
                        }
                        continue;
                    }

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
                                &alignments,
                                &uids,
                                &entities,
                                &mut |entity, group_change| {
                                    clients
                                        .get(entity)
                                        .and_then(|c| {
                                            group_change
                                                .try_map_ref(|e| uids.get(*e).copied())
                                                .map(|g| (g, c))
                                        })
                                        .map(|(g, c)| {
                                            update_map_markers(
                                                &map_markers,
                                                &uids,
                                                c,
                                                &group_change,
                                            );
                                            c.send_fallible(ServerGeneral::GroupUpdate(g));
                                        });
                                },
                            );

                            // Tell them the have been kicked
                            if let Some(client) = clients.get(target) {
                                client.send_fallible(ServerGeneral::server_msg(
                                    ChatType::Meta,
                                    "You were removed from the group.",
                                ));
                            }
                            // Tell kicker that they were successful
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
                    let target = match id_maps.uid_entity(uid) {
                        Some(t) => t,
                        None => {
                            // Inform of failure
                            if let Some(client) = clients.get(entity) {
                                client.send_fallible(ServerGeneral::server_msg(
                                    ChatType::Meta,
                                    "Leadership transfer failed, target does not exist",
                                ));
                            }
                            continue;
                        },
                    };
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
                                &entities,
                                &alignments,
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
                                            update_map_markers(
                                                &map_markers,
                                                &uids,
                                                c,
                                                &group_change,
                                            );
                                            c.send_fallible(ServerGeneral::GroupUpdate(g));
                                        });
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
                            if let Some(client) = clients.get(entity) {
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
                                    "Transfer failed: You are not the leader of the target's \
                                     group.",
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
    }
}
