// # Hey, you! Yes, you!
//
// Don't touch anything in this module, or any sub-modules. No, really. Bad
// stuff will happen.
//
// You're only an exception to this rule if you fulfil the following criteria:
//
// - You *really* understand exactly how the versioning system in `helper.rs`
//   works, what assumptions it makes, and how all of this can go badly wrong.
//
// - You are creating a new version of a data structure, and *not* modifying an
//   existing one.
//
// - You've thought really carefully about things and you've come to the
//   conclusion that there's just no way to add the feature you want to add
//   without creating a new version of the data structure in question.
//
// That said, here's how to make a change to one of the structures in this
// module, or submodules.
//
// 1) Duplicate the latest version of the data structure  and the `Version` impl
// for it (later    versions should be kept at the top of each file).
//
// 2) Rename the duplicated version, incrementing the version number (i.e: V0
// becomes V1).
//
// 3) Change the `type Prev =` associated type in the new `Version` impl to the
// previous    versions' type. You will need to write an implementation of
// `migrate` that migrates from the    old version to the new version.
//
// 4) *Change* the existing `Latest` impl so that it uses the new version you
// have created.
//
// 5) If your data structure is contained within another data structure, you
// will need to similarly    update the parent data structure too, also
// following these instructions.
//
// The *golden rule* is that, once merged to master, an old version's type must
// not be changed!

pub mod world;

use super::{
    helper::{Bottom, Latest, Version, V},
    Data,
};
use serde::{Deserialize, Serialize};

impl Latest<Data> for DataV0 {
    fn to_unversioned(self) -> Data {
        Data {
            world: self.world.to_unversioned(),
        }
    }

    fn from_unversioned(data: Data) -> Self {
        Self {
            world: Latest::from_unversioned(data.world),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct DataV0 {
    world: V<world::WorldV0>,
}

impl Version for DataV0 {
    type Prev = Bottom;

    fn migrate(x: Self::Prev) -> Self { match x {} }
}
