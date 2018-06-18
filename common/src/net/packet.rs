use bincode;
use coord::prelude::*;

use Uid;
use super::{Error, ClientMode};

pub trait Packet: {
    fn from_bytes(data: &[u8]) -> Result<Self, Error> where Self: Sized;
    fn to_bytes(&self) -> Result<Vec<u8>, Error>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ServerPacket {
    Connected { entity_uid: Option<Uid>, version: String },
    Kicked { reason: String },
    Shutdown,
    Ping,
    RecvChatMsg { alias: String, msg: String },
    EntityUpdate { uid: Uid, pos: Vec3f, ori: Vec1f },
    ChunkData {},
}

impl Packet for ServerPacket {
    fn from_bytes(data: &[u8]) -> Result<ServerPacket, Error> {
        bincode::deserialize(data).map_err(|_e| Error::CannotDeserialize)
    }

    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        bincode::serialize(&self).map_err(|_e| Error::CannotSerialize)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ClientPacket {
    Connect { mode: ClientMode, alias: String, version: String },
    Disconnect,
    Ping,
    ChatMsg { msg: String },
    SendCmd { cmd: String },
    PlayerEntityUpdate { pos: Vec3f, ori: Vec1f }
}

impl Packet for ClientPacket {
    fn from_bytes(data: &[u8]) -> Result<ClientPacket, Error> {
        bincode::deserialize(data).map_err(|_e| Error::CannotDeserialize)
    }

    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        bincode::serialize(&self).map_err(|_e| Error::CannotSerialize)
    }
}
