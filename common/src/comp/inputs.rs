use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use specs_idvs::IdvStorage;
use vek::geom::Aabb;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CanBuild {
    pub building_is_on: bool,
    pub build_area: Aabb<i32>,
}
impl Component for CanBuild {
    type Storage = DerefFlaggedStorage<Self, IdvStorage<Self>>;
}
