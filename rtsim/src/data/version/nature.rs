use super::*;
use crate::data::Nature;

impl Latest<crate::data::Nature> for NatureV0 {
    fn to_unversioned(self) -> Nature { Nature {} }

    fn from_unversioned(nature: &Nature) -> Self { Self {} }
}

#[derive(Serialize, Deserialize)]
pub struct NatureV0 {}

impl Version for NatureV0 {
    type Prev = Bottom;

    fn migrate(x: Self::Prev) -> Self { match x {} }
}
