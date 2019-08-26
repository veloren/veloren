use specs::{Component, FlaggedStorage, NullStorage};
use specs_idvs::IDVStorage;
use vek::*;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CanBuild;
impl Component for CanBuild {
    type Storage = FlaggedStorage<Self, NullStorage<Self>>;
}
