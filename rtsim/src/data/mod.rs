pub mod actor;
pub mod nature;

pub use self::{
    actor::{Actor, ActorId, Actors},
    nature::Nature,
};

use self::helper::Latest;
use serde::{Serialize, Deserialize};
use std::io::{Read, Write};

#[derive(Clone, Serialize, Deserialize)]
pub struct Data {
    pub nature: Nature,
    pub actors: Actors,
}

pub type ReadError = rmp_serde::decode::Error;
pub type WriteError = rmp_serde::encode::Error;

impl Data {
    pub fn from_reader<R: Read>(reader: R) -> Result<Self, ReadError> {
        rmp_serde::decode::from_read(reader)
    }

    pub fn write_to<W: Write>(&self, mut writer: W) -> Result<(), WriteError> {
        rmp_serde::encode::write(&mut writer, self)
    }
}
