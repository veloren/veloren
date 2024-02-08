use crate::{comp::Alignment, uid::Uid};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use slab::Slab;
use specs::{storage::GenericReadStorage, Component, DerefFlaggedStorage, Join, LendJoin};
use tracing::{error, warn};

// Primitive group system
// Shortcomings include:
//  - no support for more complex group structures
//  - lack of npc group integration
//  - relies on careful management of groups to maintain a valid state
//  - the possession rod could probably wreck this
//  - clients don't know which pets are theirs (could be easy to solve by
//    putting owner uid in Role::Pet)

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct Group(u32);

// TODO: Hack
// Corresponds to Alignment::Enemy
pub const ENEMY: Group = Group(u32::MAX);
// Corresponds to Alignment::Npc | Alignment::Tame
pub const NPC: Group = Group(u32::MAX - 1);

impl Component for Group {
    type Storage = DerefFlaggedStorage<Self, specs::VecStorage<Self>>;
}

#[derive(Clone, Debug)]
pub struct GroupInfo {
    // TODO: what about enemy groups, either the leader will constantly change because they have to
    // be loaded or we create a dummy entity or this needs to be optional
    pub leader: specs::Entity,
    // Number of group members (excluding pets)
    pub num_members: u32,
    // Name of the group
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Member,
    Pet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeNotification<E> {
    // :D
    Added(E, Role),
    // :(
    Removed(E),
    NewLeader(E),
    // Use to put in a group overwriting existing group
    NewGroup { leader: E, members: Vec<(E, Role)> },
    // No longer in a group
    NoGroup,
}
// Note: now that we are dipping into uids here consider just using
// ChangeNotification<Uid> everywhere
// Also note when the same notification is sent to multiple destinations the
// mapping might be duplicated effort
impl<E> ChangeNotification<E> {
    pub fn try_map_ref<T>(&self, f: impl Fn(&E) -> Option<T>) -> Option<ChangeNotification<T>> {
        match self {
            Self::Added(e, r) => f(e).map(|t| ChangeNotification::Added(t, *r)),
            Self::Removed(e) => f(e).map(ChangeNotification::Removed),
            Self::NewLeader(e) => f(e).map(ChangeNotification::NewLeader),
            // Note just discards members that fail map
            Self::NewGroup { leader, members } => {
                f(leader).map(|leader| ChangeNotification::NewGroup {
                    leader,
                    members: members
                        .iter()
                        .filter_map(|(e, r)| f(e).map(|t| (t, *r)))
                        .collect(),
                })
            },
            Self::NoGroup => Some(ChangeNotification::NoGroup),
        }
    }
}

type GroupsMut<'a> = specs::WriteStorage<'a, Group>;
type Alignments<'a> = specs::ReadStorage<'a, Alignment>;
type Uids<'a> = specs::ReadStorage<'a, Uid>;

#[derive(Debug, Default)]
pub struct GroupManager {
    groups: Slab<GroupInfo>,
}

// Gather list of pets of the group member
// Note: iterating through all entities here could become slow at higher entity
// counts
fn pets(
    entity: specs::Entity,
    uid: Uid,
    alignments: &Alignments,
    entities: &specs::world::EntitiesRes,
) -> Vec<specs::Entity> {
    (entities, alignments)
        .join()
        .filter_map(|(e, a)| {
            matches!(a, Alignment::Owned(owner) if *owner == uid && e != entity).then_some(e)
        })
        .collect::<Vec<_>>()
}

/// Returns list of current members of a group
pub fn members<'a>(
    group: Group,
    groups: impl Join<Type = &'a Group> + 'a,
    entities: &'a specs::world::EntitiesRes,
    alignments: &'a Alignments,
    uids: &'a Uids,
) -> impl Iterator<Item = (specs::Entity, Role)> + 'a {
    (entities, groups, alignments, uids)
        .join()
        .filter(move |&(_e, g, _a, _u)| (*g == group))
        .map(|(e, _g, a, u)| {
            (
                e,
                if matches!(a, Alignment::Owned(owner) if owner != u) {
                    Role::Pet
                } else {
                    Role::Member
                },
            )
        })
}

// TODO: optimize add/remove for massive NPC groups
impl GroupManager {
    pub fn group_info(&self, group: Group) -> Option<&GroupInfo> {
        self.groups.get(group.0 as usize)
    }

    fn group_info_mut(&mut self, group: Group) -> Option<&mut GroupInfo> {
        self.groups.get_mut(group.0 as usize)
    }

    fn create_group(&mut self, leader: specs::Entity, num_members: u32) -> Group {
        Group(self.groups.insert(GroupInfo {
            leader,
            num_members,
            name: "Group".into(),
        }) as u32)
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

        let group = if let Some(group) = group {
            // Increment group size
            // Note: unwrap won't fail since we just retrieved the group successfully above
            self.group_info_mut(group).unwrap().num_members += 1;
            group
        } else {
            let new_group = self.create_group(leader, 2);
            // Unwrap should not fail since we just found these entities and they should
            // still exist Note: if there is an issue replace with a warn
            groups.insert(leader, new_group).unwrap();
            // Inform
            notifier(leader, ChangeNotification::NewLeader(leader));
            new_group
        };

        let new_pets = pets(new_member, new_member_uid, alignments, entities);

        // Inform
        members(group, &*groups, entities, alignments, uids).for_each(|(e, role)| match role {
            Role::Member => {
                notifier(e, ChangeNotification::Added(new_member, Role::Member));
                notifier(new_member, ChangeNotification::Added(e, Role::Member));

                new_pets.iter().for_each(|p| {
                    notifier(e, ChangeNotification::Added(*p, Role::Pet));
                })
            },
            Role::Pet => {
                notifier(new_member, ChangeNotification::Added(e, Role::Pet));
            },
        });
        notifier(new_member, ChangeNotification::NewLeader(leader));

        // Add group id for new member and pets
        // Unwrap should not fail since we just found these entities and they should
        // still exist
        // Note: if there is an issue replace with a warn
        let _ = groups.insert(new_member, group).unwrap();
        new_pets.iter().for_each(|e| {
            let _ = groups.insert(*e, group).unwrap();
        });
    }

    pub fn new_pet(
        &mut self,
        pet: specs::Entity,
        owner: specs::Entity,
        groups: &mut GroupsMut,
        entities: &specs::Entities,
        alignments: &Alignments,
        uids: &Uids,
        notifier: &mut impl FnMut(specs::Entity, ChangeNotification<specs::Entity>),
    ) {
        if !entities.is_alive(owner) {
            warn!("Tried to create new pet for non-existent owner {owner:?}");
        } else if !entities.is_alive(pet) {
            warn!("Tried to create new pet for non-existent pet {pet:?}");
        } else {
            let group = match groups.get(owner).copied() {
                Some(group) => group,
                None => {
                    let new_group = self.create_group(owner, 1);
                    // Unwrap can't fail, we checked that `owner` is alive above
                    groups.insert(owner, new_group).unwrap();
                    // Inform
                    notifier(owner, ChangeNotification::NewLeader(owner));
                    new_group
                },
            };

            // Inform
            members(group, &*groups, entities, alignments, uids).for_each(|(e, role)| match role {
                Role::Member => {
                    notifier(e, ChangeNotification::Added(pet, Role::Pet));
                },
                Role::Pet => {},
            });

            // Add
            // Unwrap can't fail, we checked that `pet` is alive above
            groups.insert(pet, group).unwrap();
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
        entities: &specs::world::EntitiesRes,
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
        entities: &specs::world::EntitiesRes,
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
                .filter(|(e, _, g, _)| **g == group && !(to_be_deleted && *e == member))
                .fold(
                    HashMap::<Uid, (Option<specs::Entity>, Vec<specs::Entity>)>::new(),
                    |mut acc, (e, uid, _, alignment)| {
                        if let Some(owner) = alignment.and_then(|a| match a {
                            Alignment::Owned(owner) if uid != owner => Some(owner),
                            _ => None,
                        }) {
                            // A pet
                            // Assumes owner will be in the group
                            acc.entry(*owner).or_default().1.push(e);
                        } else {
                            // Not a pet
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
                            let mut members =
                                pets.iter().map(|e| (*e, Role::Pet)).collect::<Vec<_>>();
                            members.push((owner, Role::Member));

                            // New group
                            let new_group = self.create_group(owner, 1);
                            for (member, _) in &members {
                                groups.insert(*member, new_group).unwrap();
                            }

                            notifier(owner, ChangeNotification::NewGroup {
                                leader: owner,
                                members,
                            });
                        } else {
                            // If no pets just remove group
                            groups.remove(owner);
                            notifier(owner, ChangeNotification::NoGroup)
                        }
                    } else {
                        // Owner not found, potentially the were removed from the world
                        pets.into_iter().for_each(|pet| {
                            groups.remove(pet);
                        });
                    }
                });
        } else {
            // Not leader
            let leaving_member_uid = if let Some(uid) = uids.get(member) {
                *uid
            } else {
                error!("Failed to retrieve uid for the leaving member");
                return;
            };

            let leaving_pets = pets(member, leaving_member_uid, alignments, entities);

            // If pets and not about to be deleted form new group
            if !leaving_pets.is_empty() && !to_be_deleted {
                let new_group = self.create_group(member, 1);

                notifier(member, ChangeNotification::NewGroup {
                    leader: member,
                    members: leaving_pets
                        .iter()
                        .map(|p| (*p, Role::Pet))
                        .chain(std::iter::once((member, Role::Member)))
                        .collect(),
                });

                let _ = groups.insert(member, new_group).unwrap();
                leaving_pets.iter().for_each(|&e| {
                    let _ = groups.insert(e, new_group).unwrap();
                });
            } else {
                let _ = groups.remove(member);
                notifier(member, ChangeNotification::NoGroup);
                leaving_pets.iter().for_each(|&e| {
                    let _ = groups.remove(e);
                });
            }

            if let Some(info) = self.group_info_mut(group) {
                // If not pet, decrement number of members
                if !matches!(alignments.get(member), Some(Alignment::Owned(owner)) if uids.get(member).map_or(true, |uid| uid != owner))
                {
                    if info.num_members > 0 {
                        info.num_members -= 1;
                    } else {
                        error!("Group with invalid number of members")
                    }
                }

                let mut remaining_count = 0; // includes pets
                // Inform remaining members
                members(group, &*groups, entities, alignments, uids).for_each(|(e, role)| {
                    remaining_count += 1;
                    match role {
                        Role::Member => {
                            notifier(e, ChangeNotification::Removed(member));
                            leaving_pets.iter().for_each(|p| {
                                notifier(e, ChangeNotification::Removed(*p));
                            })
                        },
                        Role::Pet => {},
                    }
                });
                // If leader is the last one left then disband the group
                // Assumes last member is the leader
                if remaining_count == 1 {
                    let leader = info.leader;
                    self.remove_group(group);
                    groups.remove(leader);
                    notifier(leader, ChangeNotification::NoGroup);
                } else if remaining_count == 0 {
                    error!("Somehow group has no members")
                }
            }
        }
    }

    // Assign new group leader
    // Does nothing if new leader is not part of a group
    pub fn assign_leader<'a>(
        &mut self,
        new_leader: specs::Entity,
        groups: impl GenericReadStorage<Component = Group> + Join<Type = &'a Group> + 'a,
        entities: &'a specs::Entities,
        alignments: &'a Alignments,
        uids: &'a Uids,
        mut notifier: impl FnMut(specs::Entity, ChangeNotification<specs::Entity>),
    ) {
        let group = match groups.get(new_leader) {
            Some(group) => *group,
            None => return,
        };

        // Set new leader
        self.groups[group.0 as usize].leader = new_leader;

        // Point to new leader
        members(group, groups, entities, alignments, uids).for_each(|(e, role)| match role {
            Role::Member => notifier(e, ChangeNotification::NewLeader(new_leader)),
            Role::Pet => {},
        });
    }
}
