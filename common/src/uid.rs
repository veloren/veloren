// TODO: rename this module or create new one for ID maps
use serde::{Deserialize, Serialize};
use std::{fmt, u64};

#[cfg(not(target_arch = "wasm32"))]
use {
    crate::character::CharacterId,
    crate::rtsim::RtSimEntity,
    core::hash::Hash,
    hashbrown::HashMap,
    specs::{Component, Entity, FlaggedStorage, VecStorage},
    tracing::error,
};

// TODO: could we switch this to `NonZeroU64`?
#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct Uid(pub u64);

impl From<Uid> for u64 {
    fn from(uid: Uid) -> u64 { uid.0 }
}

impl From<u64> for Uid {
    fn from(uid: u64) -> Self { Self(uid) }
}

impl fmt::Display for Uid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{}", self.0) }
}

pub use not_wasm::*;
#[cfg(not(target_arch = "wasm32"))]
mod not_wasm {
    use super::*;

    impl Component for Uid {
        type Storage = FlaggedStorage<Self, VecStorage<Self>>;
    }

    #[derive(Debug)]
    struct UidAllocator {
        /// Next Uid.
        next_uid: u64,
    }

    impl UidAllocator {
        fn new() -> Self { Self { next_uid: 0 } }

        fn allocate(&mut self) -> Uid {
            let id = self.next_uid;
            self.next_uid += 1;
            Uid(id)
        }
    }

    #[derive(Debug)]
    pub struct IdMaps {
        /// "Universal" IDs (used to communicate entity identity over the
        /// network).
        uid_mapping: HashMap<Uid, Entity>,

        // -- Fields below only used on the server --
        uid_allocator: UidAllocator,

        // Maps below are only used on the server.
        /// Character IDs.
        cid_mapping: HashMap<CharacterId, Entity>,
        /// Rtsim Entities.
        rid_mapping: HashMap<RtsimEntity, Entity>,
    }

    impl IdMaps {
        pub fn new() -> Self {
            Self {
                uid_mapping: HashMap::new(),
                uid_allocator: UidAllocator::new(),
                cid_mapping: HashMap::new(),
                rid_mapping: HashMap::new(),
            }
        }

        /// Given a `Uid` retrieve the corresponding `Entity`.
        pub fn uid_entity(&self, id: Uid) -> Option<Entity> { self.uid_mapping.get(&id).copied() }

        /// Given a `CharacterId` retrieve the corresponding `Entity`.
        pub fn cid_entity(&self, id: CharacterId) -> Option<Entity> {
            self.uid_mapping.get(&id).copied()
        }

        /// Given a `RtSimEntity` retrieve the corresponding `Entity`.
        pub fn rid_entity(&self, id: RtSimEntity) -> Option<Entity> {
            self.uid_mapping.get(&id).copied()
        }

        // TODO: I think this is suitable to use on both the client and the server.
        // NOTE: This is only used on the client? Do we not remove on the server?
        // NOTE: We need UID mapping also on the client but we don't need the other
        // mappings on the client!
        //
        // Useful for when a single entity is deleted because it doesn't reconstruct the
        // entire hashmap
        /// Returns the `Entity` that the provided `Uid` was mapped to.
        pub fn remove_entity(
            &mut self,
            expected_entity: Option<Entity>,
            uid: Uid,
            cid: Option<CharacterId>,
            rid: Option<RtsimEntity>,
        ) -> Option<Entity> {
            #[cold]
            #[inline(never)]
            fn unexpected_entity<ID>() {
                error!("Provided was {kind} mapped to an unexpected entity!");
            }
            #[cold]
            #[inline(never)]
            fn not_present<ID>() {
                error!("Provided was {kind} not mapped to any entity!");
            }

            fn remove<ID: Hash + Eq>(
                mapping: &mut HashMap<ID, Entity>,
                id: Option<ID>,
                expected: Option<Entity>,
            ) -> Option<Entity> {
                if let Some(id) = id {
                    if let Some(e) = mapping.remove(id) {
                        if Some(expected) = expected && e != expected {
                            unexpected_entity::<ID>();
                        }
                        Some(e)
                    } else {
                        not_present::<ID>();
                        None
                    }
                } else {
                    None
                }
            }

            let maybe_entity = remove(&mut self.uid_mapping, Some(uid), expected_entity);
            let expected_entity = expected_entity.or(maybe_entity);
            remove(&mut self.cid_mapping, cid, expected_entity);
            remove(&mut self.rid_mapping, rid, expected_entity);
            maybe_entity
        }

        /// Only used on the client (server solely uses `Self::allocate` to
        /// allocate and add Uid mappings).
        pub fn add_entity(&mut self, uid: Uid, entity: Entity) {
            Self::insert(&mut self.uid_mapping, uid, entity);
        }

        /// Only used on the server.
        pub fn add_character(&mut self, cid: CharacterId, entity: Entity) {
            Self::insert(&mut self.cid_mapping, cid, entity);
        }

        /// Only used on the server.
        pub fn add_rtsim(&mut self, rid: RtSimEntity, entity: Entity) {
            Self::insert(&mut self.rid_mapping, rid, entity);
        }

        /// Allocates a new `Uid` and links it to the provided entity.
        ///
        /// Only used on the server.
        pub fn allocate(&mut self, entity: Entity) -> Uid {
            let uid = self.uid_allocator.allocate();
            self.uid_mapping.insert(uid, entity);
            uid
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

    impl Default for IdMaps {
        fn default() -> Self { Self::new() }
    }
}
