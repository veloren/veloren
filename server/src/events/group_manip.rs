use crate::{client::Client, Server};
use common::{
    comp::{
        self,
        group::{ChangeNotification, Group, GroupManager},
        invite::{InviteKind, PendingInvites},
        ChatType, GroupManip,
    },
    uid::Uid,
};
use common_net::{msg::ServerGeneral, sync::WorldSyncExt};
use common_state::State;
use specs::{
    world::{Entity, WorldExt},
    ReadStorage, WriteStorage,
};

pub fn can_invite(
    state: &State,
    clients: &ReadStorage<'_, Client>,
    pending_invites: &mut WriteStorage<'_, PendingInvites>,
    max_group_size: u32,
    inviter: Entity,
    invitee: Entity,
) -> bool {
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
        return false;
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

// TODO: turn chat messages into enums
pub fn handle_group(server: &mut Server, entity: Entity, manip: GroupManip) {
    match manip {
        GroupManip::Leave => {
            let state = server.state_mut();
            let clients = state.ecs().read_storage::<Client>();
            let uids = state.ecs().read_storage::<Uid>();
            let mut group_manager = state.ecs().write_resource::<GroupManager>();
            let map_markers = state.ecs().read_storage::<comp::MapMarker>();
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

            let mut groups = state.ecs().write_storage::<Group>();
            let mut group_manager = state.ecs().write_resource::<GroupManager>();
            let map_markers = state.ecs().read_storage::<comp::MapMarker>();
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
                                        .try_map_ref(|e| uids.get(*e).copied())
                                        .map(|g| (g, c))
                                })
                                .map(|(g, c)| {
                                    update_map_markers(&map_markers, &uids, c, &group_change);
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
            let groups = state.ecs().read_storage::<Group>();
            let mut group_manager = state.ecs().write_resource::<GroupManager>();
            let map_markers = state.ecs().read_storage::<comp::MapMarker>();
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
                                        .try_map_ref(|e| uids.get(*e).copied())
                                        .map(|g| (g, c))
                                })
                                .map(|(g, c)| {
                                    update_map_markers(&map_markers, &uids, c, &group_change);
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
