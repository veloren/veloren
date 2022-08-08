pub mod actor;
pub mod nature;

pub use self::{
    actor::{Actor, ActorId, Actors},
    nature::Nature,
};

use self::helper::Latest;
use ron::error::SpannedResult;
use serde::{Serialize, Deserialize};
use std::io::{Read, Write};

#[derive(Clone, Serialize, Deserialize)]
pub struct Data {
    pub nature: Nature,
    pub actors: Actors,
}

impl Data {
    pub fn from_reader<R: Read>(reader: R) -> SpannedResult<Self> {
        ron::de::from_reader(reader)
    }

    pub fn write_to<W: Write>(&self, writer: W) -> Result<(), ron::Error> {
        ron::ser::to_writer(writer, self)
    }
}
