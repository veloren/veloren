use crate::comp::{body::Body, quadruped_medium};
use crossbeam_utils::atomic::AtomicCell;
use specs::Component;
use std::{num::NonZeroU64, sync::Arc};

use super::Mass;

pub type PetId = AtomicCell<Option<NonZeroU64>>;

// TODO: move to server crate
#[derive(Clone, Debug)]
pub struct Pet {
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
    match body {
        Body::QuadrupedMedium(quad_med) =>
        {
            !matches!(
                quad_med.species,
                quadruped_medium::Species::Catoblepas
                    | quadruped_medium::Species::Mammoth
                    | quadruped_medium::Species::Elephant
                    | quadruped_medium::Species::Hirdrasil
            )
        },
        Body::QuadrupedLow(_)
        | Body::QuadrupedSmall(_)
        | Body::BirdMedium(_)
        | Body::Crustacean(_) => true,
        _ => false,
    }
}

pub fn is_mountable(
    mount: &Body,
    mount_mass: &Mass,
    rider: Option<&Body>,
    rider_mass: Option<&Mass>,
) -> bool {
    let is_light_enough = rider_mass.is_some_and(|r| r.0 / mount_mass.0 < 0.7);

    match mount {
        Body::Humanoid(_) => matches!(rider, Some(Body::BirdMedium(_))) && is_light_enough,
        Body::Ship(_) => true,
        Body::Object(_) => false,
        Body::Item(_) => false,
        _ => is_light_enough,
    }
}

impl Component for Pet {
    // Using `DenseVecStorage` has a u64 space overhead per entity and `Pet` just
    // has an `Arc` pointer which is the same size on 64-bit platforms. So it
    // isn't worth using `DenseVecStorage` here.
    type Storage = specs::VecStorage<Self>;
}
