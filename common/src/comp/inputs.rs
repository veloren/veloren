use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use specs_idvs::IdvStorage;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CanBuild {
    pub building_is_on: bool,
}
impl Component for CanBuild {
    type Storage = DerefFlaggedStorage<Self, IdvStorage<Self>>;
}
