use crate::{
    character::CharacterId,
    rtsim::{Actor, RtSimEntity},
};
use core::hash::Hash;
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use specs::{Component, Entity, FlaggedStorage, VecStorage};
use std::{fmt, num::NonZeroU64};
use tracing::error;

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct Uid(pub NonZeroU64);

impl From<Uid> for NonZeroU64 {
    fn from(uid: Uid) -> NonZeroU64 { uid.0 }
}

impl From<NonZeroU64> for Uid {
    fn from(uid: NonZeroU64) -> Self { Self(uid) }
}

impl fmt::Display for Uid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{}", self.0) }
}

impl Component for Uid {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}

#[derive(Debug)]
struct UidAllocator {
    /// Next Uid.
    next_uid: u64,
}

impl UidAllocator {
    fn new() -> Self { Self { next_uid: 1 } }

    fn allocate(&mut self) -> Uid {
        let id = self.next_uid;
        self.next_uid += 1;
        Uid(NonZeroU64::new(id).expect("Uid cannot be zero"))
    }
}

/// Mappings from various Id types to `Entity`s.
#[derive(Default, Debug)]
pub struct IdMaps {
    /// "Universal" IDs (used to communicate entity identity over the
    /// network).
    uid_mapping: HashMap<Uid, Entity>,

    // -- Fields below are only used on the server --
    uid_allocator: UidAllocator,

    /// Character IDs.
    character_to_ecs: HashMap<CharacterId, Entity>,
    /// Rtsim Entities.
    rtsim_to_ecs: HashMap<RtSimEntity, Entity>,
}

impl IdMaps {
    pub fn new() -> Self { Default::default() }

    /// Given a `Uid` retrieve the corresponding `Entity`.
    pub fn uid_entity(&self, id: Uid) -> Option<Entity> { self.uid_mapping.get(&id).copied() }

    /// Given a `CharacterId` retrieve the corresponding `Entity`.
    pub fn character_entity(&self, id: CharacterId) -> Option<Entity> {
        self.character_to_ecs.get(&id).copied()
    }

    /// Given a `RtSimEntity` retrieve the corresponding `Entity`.
    pub fn rtsim_entity(&self, id: RtSimEntity) -> Option<Entity> {
        self.rtsim_to_ecs.get(&id).copied()
    }

    pub fn actor_entity(&self, actor: Actor) -> Option<Entity> {
        match actor {
            Actor::Character(character_id) => self.character_entity(character_id),
            Actor::Npc(npc_id) => self.rtsim_entity(npc_id),
        }
    }

    /// Removes mappings for the provided Id(s).
    ///
    /// Returns the `Entity` that the provided `Uid` was mapped to.
    ///
    /// Used on both the client and the server when deleting entities,
    /// although the client only ever provides a Some value for the
    /// `Uid` parameter since the other mappings are not used on the
    /// client.
    #[track_caller]
    pub fn remove_entity(
        &mut self,
        expected_entity: Option<Entity>,
        uid: Option<Uid>,
        cid: Option<CharacterId>,
        rid: Option<RtSimEntity>,
    ) -> Option<Entity> {
        use std::fmt::Debug;
        #[cold]
        #[inline(never)]
        fn unexpected_entity<ID>() {
            let kind = core::any::type_name::<ID>();
            error!("Provided {kind} was mapped to an unexpected entity!");
        }
        #[cold]
        #[inline(never)]
        #[track_caller]
        fn not_present<ID: Debug>(id: ID) {
            let kind = core::any::type_name::<ID>();
            error!(
                "Provided {kind} {id:?} was not mapped to any entity! Caller: {}",
                std::panic::Location::caller()
            );
        }

        #[track_caller]
        fn remove<ID: Hash + Eq + Debug>(
            mapping: &mut HashMap<ID, Entity>,
            id: Option<ID>,
            expected: Option<Entity>,
        ) -> Option<Entity> {
            if let Some(id) = id {
                if let Some(e) = mapping.remove(&id) {
                    if expected.is_some_and(|expected| e != expected) {
                        unexpected_entity::<ID>();
                    }
                    Some(e)
                } else {
                    not_present::<ID>(id);
                    None
                }
            } else {
                None
            }
        }

        let maybe_entity = remove(&mut self.uid_mapping, uid, expected_entity);
        let expected_entity = expected_entity.or(maybe_entity);
        remove(&mut self.character_to_ecs, cid, expected_entity);
        remove(&mut self.rtsim_to_ecs, rid, expected_entity);
        maybe_entity
    }

    /// Only used on the client (server solely uses `Self::allocate` to
    /// allocate and add Uid mappings and `Self::remap` to move the `Uid` to
    /// a different entity).
    pub fn add_entity(&mut self, uid: Uid, entity: Entity) {
        Self::insert(&mut self.uid_mapping, uid, entity);
    }

    /// Only used on the server.
    pub fn add_character(&mut self, cid: CharacterId, entity: Entity) {
        Self::insert(&mut self.character_to_ecs, cid, entity);
    }

    /// Only used on the server.
    pub fn add_rtsim(&mut self, rid: RtSimEntity, entity: Entity) {
        Self::insert(&mut self.rtsim_to_ecs, rid, entity);
    }

    /// Allocates a new `Uid` and links it to the provided entity.
    ///
    /// Only used on the server.
    pub fn allocate(&mut self, entity: Entity) -> Uid {
        let uid = self.uid_allocator.allocate();
        self.uid_mapping.insert(uid, entity);
        uid
    }

    /// Links an existing `Uid` to a new entity.
    ///
    /// Only used on the server.
    ///
    /// Used for `handle_exit_ingame` which moves the same `Uid` to a new
    /// entity.
    pub fn remap_entity(&mut self, uid: Uid, new_entity: Entity) {
        if self.uid_mapping.insert(uid, new_entity).is_none() {
            error!("Uid {uid:?} remaped but there was no existing entry for it!");
        }
    }

    #[cold]
    #[inline(never)]
    fn already_present<ID>() {
        let kind = core::any::type_name::<ID>();
        error!("Provided {kind} was already mapped to an entity!!!");
    }

    fn insert<ID: Hash + Eq>(mapping: &mut HashMap<ID, Entity>, new_id: ID, entity: Entity) {
        if let Some(_previous_entity) = mapping.insert(new_id, entity) {
            Self::already_present::<ID>();
        }
    }
}

impl Default for UidAllocator {
    fn default() -> Self { Self::new() }
}
