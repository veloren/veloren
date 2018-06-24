// Standard
use std::io;

// Library
use bincode;
use coord::prelude::*;

// Local
use Uid;

// Parent
use super::ClientMode;

#[derive(Debug)]
pub enum Error {
    NetworkErr(io::Error),
    CannotSerialize,
    CannotDeserialize,
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::NetworkErr(e)
    }
}

pub trait Message {
    fn to_bytes(&self) -> Result<Vec<u8>, Error>;
    fn from_bytes(data: &[u8]) -> Result<Self, Error> where Self: Sized;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ServerMessage {
    Connected { entity_uid: Option<Uid>, version: String },
    Kicked { reason: String },
    Shutdown,
    Ping,
    RecvChatMsg { alias: String, msg: String },
    EntityUpdate { uid: Uid, pos: Vec3f, ori: f32 },
    ChunkData {},
}

impl Message for ServerMessage {
    fn from_bytes(data: &[u8]) -> Result<ServerMessage, Error> {
        bincode::deserialize(data).map_err(|_e| Error::CannotDeserialize)
    }

    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        bincode::serialize(&self).map_err(|_e| Error::CannotSerialize)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    Connect { mode: ClientMode, alias: String, version: String },
    Disconnect,
    Ping,
    ChatMsg { msg: String },
    SendCmd { cmd: String },
    PlayerEntityUpdate { pos: Vec3f, ori: f32 }
}

impl Message for ClientMessage {
    fn from_bytes(data: &[u8]) -> Result<ClientMessage, Error> {
        bincode::deserialize(data).map_err(|_e| Error::CannotDeserialize)
    }

    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        bincode::serialize(&self).map_err(|_e| Error::CannotSerialize)
    }
}
