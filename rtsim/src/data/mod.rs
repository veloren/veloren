pub mod helper;
pub mod version;

pub mod actor;
pub mod nature;

pub use self::{
    actor::{Actor, ActorId, Actors},
    nature::Nature,
};

use self::helper::Latest;
use ron::error::SpannedResult;
use serde::Deserialize;
use std::io::{Read, Write};

pub struct Data {
    pub nature: Nature,
    pub actors: Actors,
}

impl Data {
    pub fn from_reader<R: Read>(reader: R) -> SpannedResult<Self> {
        ron::de::from_reader(reader).map(version::LatestData::to_unversioned)
    }

    pub fn write_to<W: Write>(&self, writer: W) -> Result<(), ron::Error> {
        ron::ser::to_writer(writer, &version::LatestData::from_unversioned(self))
    }
}
