use crate::{comp::Alignment, sync::Uid};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use slab::Slab;
use specs::{Component, FlaggedStorage, Join};
use specs_idvs::IdvStorage;
use tracing::{error, warn};

// Primitive group system
// Shortcomings include:
//  - no support for more complex group structures
//  - lack of complex enemy npc integration
//  - relies on careful management of groups to maintain a valid state
//  - clients don't know what entities are their pets
//  - the possesion rod could probably wreck this

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Group(u32);

// TODO: Hack
// Corresponds to Alignment::Enemy
pub const ENEMY: Group = Group(u32::MAX);
// Corresponds to Alignment::Npc | Alignment::Tame
pub const NPC: Group = Group(u32::MAX - 1);

impl Component for Group {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}

#[derive(Copy, Clone, Debug)]
pub struct GroupInfo {
    // TODO: what about enemy groups, either the leader will constantly change because they have to
    // be loaded or we create a dummy entity or this needs to be optional
    pub leader: specs::Entity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeNotification<E> {
    // :D
    Added(E),
    // :(
    Removed(E),
    NewLeader(E),
    // Use to put in a group overwriting existing group
    NewGroup { leader: E, members: Vec<E> },
    // No longer in a group
    NoGroup,
}
// Note: now that we are dipping into uids here consider just using
// ChangeNotification<Uid> everywhere
// Also note when the same notification is sent to multiple destinations the
// maping might be duplicated effort
impl<E> ChangeNotification<E> {
    pub fn try_map<T>(self, f: impl Fn(E) -> Option<T>) -> Option<ChangeNotification<T>> {
        match self {
            Self::Added(e) => f(e).map(ChangeNotification::Added),
            Self::Removed(e) => f(e).map(ChangeNotification::Removed),
            Self::NewLeader(e) => f(e).map(ChangeNotification::NewLeader),
            // Note just discards members that fail map
            Self::NewGroup { leader, members } => {
                f(leader).map(|leader| ChangeNotification::NewGroup {
                    leader,
                    members: members.into_iter().filter_map(f).collect(),
                })
            },
            Self::NoGroup => Some(ChangeNotification::NoGroup),
        }
    }
}

type GroupsMut<'a> = specs::WriteStorage<'a, Group>;
type Groups<'a> = specs::ReadStorage<'a, Group>;
type Alignments<'a> = specs::ReadStorage<'a, Alignment>;
type Uids<'a> = specs::ReadStorage<'a, Uid>;

#[derive(Debug, Default)]
pub struct GroupManager {
    groups: Slab<GroupInfo>,
}

// Gather list of pets of the group member + member themselves
// Note: iterating through all entities here could become slow at higher entity
// counts
fn with_pets(
    entity: specs::Entity,
    uid: Uid,
    alignments: &Alignments,
    entities: &specs::Entities,
) -> Vec<specs::Entity> {
    let mut list = (entities, alignments)
        .join()
        .filter_map(|(e, a)| {
            matches!(a, Alignment::Owned(owner) if *owner == uid && e != entity).then_some(e)
        })
        .collect::<Vec<_>>();
    list.push(entity);
    list
}

/// Returns list of current members of a group
pub fn members<'a>(
    group: Group,
    groups: impl Join<Type = &'a Group> + 'a,
    entities: &'a specs::Entities,
) -> impl Iterator<Item = specs::Entity> + 'a {
    (entities, groups)
        .join()
        .filter_map(move |(e, g)| (*g == group).then_some(e))
}

// TODO: optimize add/remove for massive NPC groups
impl GroupManager {
    pub fn group_info(&self, group: Group) -> Option<GroupInfo> {
        self.groups.get(group.0 as usize).copied()
    }

    fn create_group(&mut self, leader: specs::Entity) -> Group {
        Group(self.groups.insert(GroupInfo { leader }) as u32)
    }

    fn remove_group(&mut self, group: Group) { self.groups.remove(group.0 as usize); }

    // Add someone to a group
    // Also used to create new groups
    pub fn add_group_member(
        &mut self,
        leader: specs::Entity,
        new_member: specs::Entity,
        entities: &specs::Entities,
        groups: &mut GroupsMut,
        alignments: &Alignments,
        uids: &Uids,
        mut notifier: impl FnMut(specs::Entity, ChangeNotification<specs::Entity>),
    ) {
        // Ensure leader is not inviting themselves
        if leader == new_member {
            warn!("Attempt to form group with leader as the only member (this is disallowed)");
            return;
        }

        // Get uid
        let new_member_uid = if let Some(uid) = uids.get(new_member) {
            *uid
        } else {
            error!("Failed to retrieve uid for the new group member");
            return;
        };

        // If new member is a member of a different group remove that
        if groups
            .get(new_member)
            .and_then(|g| self.group_info(*g))
            .is_some()
        {
            self.leave_group(
                new_member,
                groups,
                alignments,
                uids,
                entities,
                &mut notifier,
            )
        }

        let group = match groups.get(leader).copied() {
            Some(id)
                if self
                    .group_info(id)
                    .map(|info| info.leader == leader)
                    .unwrap_or(false) =>
            {
                Some(id)
            },
            // Member of an existing group can't be a leader
            // If the lead is a member of another group leave that group first
            Some(_) => {
                self.leave_group(leader, groups, alignments, uids, entities, &mut notifier);
                None
            },
            None => None,
        };

        let group = group.unwrap_or_else(|| {
            let new_group = self.create_group(leader);
            // Unwrap should not fail since we just found these entities and they should
            // still exist Note: if there is an issue replace with a warn
            groups.insert(leader, new_group).unwrap();
            // Inform
            notifier(leader, ChangeNotification::NewLeader(leader));
            new_group
        });

        let member_plus_pets = with_pets(new_member, new_member_uid, alignments, entities);

        // Inform
        members(group, &*groups, entities).for_each(|a| {
            member_plus_pets.iter().for_each(|b| {
                notifier(a, ChangeNotification::Added(*b));
                notifier(*b, ChangeNotification::Added(a));
            })
        });
        // Note: pets not informed
        notifier(new_member, ChangeNotification::NewLeader(leader));

        // Add group id for new member and pets
        // Unwrap should not fail since we just found these entities and they should
        // still exist
        // Note: if there is an issue replace with a warn
        member_plus_pets.iter().for_each(|e| {
            let _ = groups.insert(*e, group).unwrap();
        });
    }

    pub fn new_pet(
        &mut self,
        pet: specs::Entity,
        owner: specs::Entity,
        groups: &mut GroupsMut,
        entities: &specs::Entities,
        notifier: &mut impl FnMut(specs::Entity, ChangeNotification<specs::Entity>),
    ) {
        let group = match groups.get(owner).copied() {
            Some(group) => group,
            None => {
                let new_group = self.create_group(owner);
                groups.insert(owner, new_group).unwrap();
                // Inform
                notifier(owner, ChangeNotification::NewLeader(owner));
                new_group
            },
        };

        // Inform
        members(group, &*groups, entities).for_each(|a| {
            notifier(a, ChangeNotification::Added(pet));
            notifier(pet, ChangeNotification::Added(a));
        });

        // Add
        groups.insert(pet, group).unwrap();

        if let Some(info) = self.group_info(group) {
            notifier(pet, ChangeNotification::NewLeader(info.leader));
        }
    }

    pub fn leave_group(
        &mut self,
        member: specs::Entity,
        groups: &mut GroupsMut,
        alignments: &Alignments,
        uids: &Uids,
        entities: &specs::Entities,
        notifier: &mut impl FnMut(specs::Entity, ChangeNotification<specs::Entity>),
    ) {
        // Pets can't leave
        if matches!(alignments.get(member), Some(Alignment::Owned(uid)) if uids.get(member).map_or(true, |u| u != uid))
        {
            return;
        }
        self.remove_from_group(member, groups, alignments, uids, entities, notifier, false);

        // Set NPC back to their group
        if let Some(alignment) = alignments.get(member) {
            match alignment {
                Alignment::Npc => {
                    let _ = groups.insert(member, NPC);
                },
                Alignment::Enemy => {
                    let _ = groups.insert(member, ENEMY);
                },
                _ => {},
            }
        }
    }

    pub fn entity_deleted(
        &mut self,
        member: specs::Entity,
        groups: &mut GroupsMut,
        alignments: &Alignments,
        uids: &Uids,
        entities: &specs::Entities,
        notifier: &mut impl FnMut(specs::Entity, ChangeNotification<specs::Entity>),
    ) {
        self.remove_from_group(member, groups, alignments, uids, entities, notifier, true);
    }

    // Remove someone from a group if they are in one
    // Don't need to check if they are in a group before calling this
    // Also removes pets (ie call this if the pet no longer exists)
    fn remove_from_group(
        &mut self,
        member: specs::Entity,
        groups: &mut GroupsMut,
        alignments: &Alignments,
        uids: &Uids,
        entities: &specs::Entities,
        notifier: &mut impl FnMut(specs::Entity, ChangeNotification<specs::Entity>),
        to_be_deleted: bool,
    ) {
        let group = match groups.get(member) {
            Some(group) => *group,
            None => return,
        };

        // If leaving entity was the leader disband the group
        if self
            .group_info(group)
            .map(|info| info.leader == member)
            .unwrap_or(false)
        {
            // Remove group
            self.remove_group(group);

            (entities, uids, &*groups, alignments.maybe())
                .join()
                .filter(|(e, _, g, _)| **g == group && (!to_be_deleted || *e == member))
                .fold(
                    HashMap::<Uid, (Option<specs::Entity>, Vec<specs::Entity>)>::new(),
                    |mut acc, (e, uid, _, alignment)| {
                        if let Some(owner) = alignment.and_then(|a| match a {
                            Alignment::Owned(owner) if uid != owner => Some(owner),
                            _ => None,
                        }) {
                            // Assumes owner will be in the group
                            acc.entry(*owner).or_default().1.push(e);
                        } else {
                            acc.entry(*uid).or_default().0 = Some(e);
                        }

                        acc
                    },
                )
                .into_iter()
                .map(|(_, v)| v)
                .for_each(|(owner, pets)| {
                    if let Some(owner) = owner {
                        if !pets.is_empty() {
                            let mut members = pets.clone();
                            members.push(owner);

                            // New group
                            let new_group = self.create_group(owner);
                            for &member in &members {
                                groups.insert(member, new_group).unwrap();
                            }

                            let notification = ChangeNotification::NewGroup {
                                leader: owner,
                                members,
                            };

                            // TODO: don't clone
                            notifier(owner, notification.clone());
                            pets.into_iter()
                                .for_each(|pet| notifier(pet, notification.clone()));
                        } else {
                            // If no pets just remove group
                            groups.remove(owner);
                            notifier(owner, ChangeNotification::NoGroup)
                        }
                    } else {
                        pets.into_iter()
                            .for_each(|pet| notifier(pet, ChangeNotification::NoGroup));
                    }
                });
        } else {
            // Not leader
            let leaving_member_uid = if let Some(uid) = uids.get(member) {
                *uid
            } else {
                error!("Failed to retrieve uid for the new group member");
                return;
            };

            let leaving = with_pets(member, leaving_member_uid, alignments, entities);

            // If pets and not about to be deleted form new group
            if leaving.len() > 1 && !to_be_deleted {
                let new_group = self.create_group(member);

                let notification = ChangeNotification::NewGroup {
                    leader: member,
                    members: leaving.clone(),
                };

                leaving.iter().for_each(|&e| {
                    let _ = groups.insert(e, new_group).unwrap();
                    notifier(e, notification.clone());
                });
            } else {
                leaving.iter().for_each(|&e| {
                    let _ = groups.remove(e);
                    notifier(e, ChangeNotification::NoGroup);
                });
            }

            if let Some(info) = self.group_info(group) {
                // Inform remaining members
                let mut num_members = 0;
                members(group, &*groups, entities).for_each(|a| {
                    num_members += 1;
                    leaving.iter().for_each(|b| {
                        notifier(a, ChangeNotification::Removed(*b));
                    })
                });
                // If leader is the last one left then disband the group
                // Assumes last member is the leader
                if num_members == 1 {
                    let leader = info.leader;
                    self.remove_group(group);
                    groups.remove(leader);
                    notifier(leader, ChangeNotification::NoGroup);
                } else if num_members == 0 {
                    error!("Somehow group has no members")
                }
            }
        }
    }

    // Assign new group leader
    // Does nothing if new leader is not part of a group
    pub fn assign_leader(
        &mut self,
        new_leader: specs::Entity,
        groups: &Groups,
        entities: &specs::Entities,
        mut notifier: impl FnMut(specs::Entity, ChangeNotification<specs::Entity>),
    ) {
        let group = match groups.get(new_leader) {
            Some(group) => *group,
            None => return,
        };

        // Set new leader
        self.groups[group.0 as usize].leader = new_leader;

        // Point to new leader
        members(group, groups, entities).for_each(|e| {
            notifier(e, ChangeNotification::NewLeader(new_leader));
        });
    }
}
