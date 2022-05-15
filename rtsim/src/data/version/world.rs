use super::*;
use crate::data::World;

impl Latest<crate::data::World> for WorldV0 {
    fn to_unversioned(self) -> World { World {} }

    fn from_unversioned(world: World) -> Self { Self {} }
}

#[derive(Serialize, Deserialize)]
pub struct WorldV0 {}

impl Version for WorldV0 {
    type Prev = Bottom;

    fn migrate(x: Self::Prev) -> Self { match x {} }
}
