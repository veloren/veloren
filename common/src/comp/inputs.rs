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
