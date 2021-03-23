use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use specs_idvs::IdvStorage;
use vek::geom::Aabb;
use crate::store::Id;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CanBuild {
    pub building_is_on: bool,
    pub build_areas: Vec<Id<Aabb<i32>>>,
}
impl Component for CanBuild {
    type Storage = DerefFlaggedStorage<Self, IdvStorage<Self>>;
}
