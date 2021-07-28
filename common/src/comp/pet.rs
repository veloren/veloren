use crate::comp::body::Body;
use crossbeam_utils::atomic::AtomicCell;
use serde::{Deserialize, Serialize};
use specs::Component;
use specs_idvs::IdvStorage;
use std::{num::NonZeroU64, sync::Arc};

pub type PetId = AtomicCell<Option<NonZeroU64>>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pet {
    #[serde(skip)]
    database_id: Arc<PetId>,
}

impl Pet {
    /// Not to be used outside of persistence - provides mutable access to the
    /// pet component's database ID which is required to save the pet's data
    /// from the persistence thread.
    #[doc(hidden)]
    pub fn get_database_id(&self) -> Arc<PetId> { Arc::clone(&self.database_id) }

    pub fn new_from_database(database_id: NonZeroU64) -> Self {
        Self {
            database_id: Arc::new(AtomicCell::new(Some(database_id))),
        }
    }
}

impl Default for Pet {
    fn default() -> Self {
        Self {
            database_id: Arc::new(AtomicCell::new(None)),
        }
    }
}

/// Determines whether an entity of a particular body variant is tameable.
pub fn is_tameable(body: &Body) -> bool {
    // Currently only Quadruped animals can be tamed pending further work
    // on the pets feature (allowing larger animals to be tamed will
    // require balance issues to be addressed).
    matches!(
        body,
        Body::QuadrupedLow(_) | Body::QuadrupedMedium(_) | Body::QuadrupedSmall(_)
    )
}

impl Component for Pet {
    type Storage = IdvStorage<Self>;
}
