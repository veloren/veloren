// TODO: rename this module or create new one for ID maps
use serde::{Deserialize, Serialize};
use std::{fmt, u64};

#[cfg(not(target_arch = "wasm32"))]
use {
    crate::character::CharacterId,
    crate::rtsim::RtSimEntity,
    hashbrown::HashMap,
    specs::{Component, Entity, FlaggedStorage, VecStorage},
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

    // NOTE: This is technically only needed by the server code server. Keeping here
    // for now since this is related to the other code here.
    #[derive(Debug)]
    pub struct UidAllocator {
        /// Next Uid.
        next_uid: u64,
    }

    impl UidAllocator {
        pub fn new() -> Self { Self { next_uid: 0 } }

        pub fn allocate(&mut self, entity: Entity, id: Option<Uid>) -> Uid {
            let id = id.unwrap_or_else(|| {
                let id = self.next_uid;
                self.next_uid += 1;
                Uid(id)
            });
            self.uid_mapping.insert(id, entity);
            id
        }
    }

    #[derive(Debug)]
    pub struct IdMaps {
        /// "Universal" IDs (used to communicate entity identity over the
        /// network).
        uid_mapping: HashMap<Uid, Entity>,

        // Maps below are only used on the server.
        /// Character IDs.
        cid_mapping: HashMap<CharacterId, Entity>,
        /// Rtsim Entities.
        rid_mapping: HashMap<RtsimEntity, Entity>,
    }

    impl IdManager {
        pub fn new() -> Self {
            Self {
                uid_mapping: HashMap::new(),
                cid_mapping: HashMap::new(),
                rid_mapping: HashMap::new(),
            }
        }

        /// Given a `Uid` retrieve the corresponding `Entity`
        pub fn uid_entity(&self, id: Uid) -> Option<Entity> { self.uid_mapping.get(&id).copied() }

        /// Given a `CharacterId` retrieve the corresponding `Entity`
        pub fn cid_entity(&self, id: CharacterId) -> Option<Entity> {
            self.uid_mapping.get(&id).copied()
        }

        /// Given a `Uid` retrieve the corresponding `Entity`
        pub fn rid_entity(&self, id: RtSimEntity) -> Option<Entity> {
            self.uid_mapping.get(&id).copied()
        }

        // NOTE: This is only used on the client? Do we not remove on the server?
        // NOTE: We need UID mapping also on the client but we don't need the other
        // mappings on the client!
        //
        // Useful for when a single entity is deleted because it doesn't reconstruct the
        // entire hashmap
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
                error!("{kind} mapped to an unexpected entity!");
            }
            #[cold]
            #[inline(never)]
            fn not_present<ID>() {
                error!("{kind} not mapped to any entity!");
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

        // TODO: we probably need separate methods here
        // TODO: document what methods are called only on the client or the server
        pub fn add_entity(
            &mut self,
            entity: Entity,
            uid: Uid,
            cid: Option<CharacterId>,
            rid: Option<RtSimEntity>,
        ) {
            #[cold]
            #[inline(never)]
            fn already_present<ID>() {
                let kind = core::any::type_name::<ID>();
                error!("{kind} already mapped to an entity!!!");
            }

            fn insert<ID: Hash + Eq>(
                mapping: &mut HashMap<ID, Entity>,
                new_id: ID,
                entity: Entity,
            ) {
                if let Some(_previous_entity) = mapping.insert(new_id, entity) {
                    already_present::<ID>();
                }
            }

            insert(&mut self.uid_mapping, uid, entity);
            if let Some(cid) = cid {
                insert(&mut self.cid_mapping, cid, entity);
            }
            if let Some(rid) = rid {
                insert(&mut self.rid_mapping, rid, entity);
            }
        }

        pub fn allocate(&mut self, entity: Entity) -> Uid {
            let id = id.unwrap_or_else(|| {
                let id = self.next_uid;
                self.next_uid += 1;
                Uid(id)
            });
            // TODO: not sure we want to insert here?
            self.uid_mapping.insert(id, entity);
            id
        }
    }

    impl Default for UidAllocator {
        fn default() -> Self { Self::new() }
    }

    impl Default for IdMaps {
        fn default() -> Self { Self::new() }
    }
}
