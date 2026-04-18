use crate::depot::Id;
use hashbrown::HashSet;
use serde::{Deserialize, Serialize};
use specs::{Component, DenseVecStorage, DerefFlaggedStorage};
use vek::geom::Aabb;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanBuild {
    pub enabled: bool,
    pub build_areas: HashSet<Id<Aabb<i32>>>,
}
impl Component for CanBuild {
    type Storage = DerefFlaggedStorage<Self, DenseVecStorage<Self>>;
}

/// Marks a player that currently has build mode active on the client side.
/// The server uses `CanBuild` to verify permissions; this component simply
/// records that the client toggled build mode on so it can be persisted and
/// restored on reconnect.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerPlot {
    /// The AABB of the plot claimed by this player.
    pub area: Aabb<i32>,
    /// Human-readable name for this plot (set by the player or auto-generated).
    pub name: String,
}

impl Component for PlayerPlot {
    type Storage = DenseVecStorage<Self>;
}
