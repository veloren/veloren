use crate::depot::Id;
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use specs_idvs::IdvStorage;
use std::collections::HashSet;
use vek::geom::Aabb;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CanBuild {
    pub enabled: bool,
    pub build_areas: HashSet<Id<Aabb<i32>>>,
}
impl Component for CanBuild {
    type Storage = DerefFlaggedStorage<Self, IdvStorage<Self>>;
}
