// In common/src/comp/ephemeral_mount.rs

use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};

/// A tag component indicating that an entity should be deleted when dismounted.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct EphemeralMount;

impl Component for EphemeralMount {
    type Storage = DerefFlaggedStorage<Self, specs::VecStorage<Self>>;
}