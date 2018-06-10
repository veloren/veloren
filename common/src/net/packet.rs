use bincode;
use nalgebra::Vector3;

use Uid;
use network::ClientMode;
use super::Error;

pub trait Packet {
    fn serialize(&self) -> Result<Vec<u8>, Error>;
    fn from(data: &[u8]) -> Result<Self, Error> where Self: Sized;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ServerPacket {
    Connected { entity_uid: Option<Uid>, version: String },
    Kicked { reason: String },
    Shutdown,
    Ping,
    RecvChatMsg { alias: String, msg: String },
    EntityUpdate { uid: Uid, pos: Vector3<f32> },
}

impl Packet for ServerPacket {
    fn from(data: &[u8]) -> Result<ServerPacket, Error> {
        bincode::deserialize(data).map_err(|_e| Error::CannotDeserialize)
    }

    fn serialize(&self) -> Result<Vec<u8>, Error> {
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
    PlayerEntityUpdate { pos: Vector3<f32> }
}

impl Packet for ClientPacket {
    fn from(data: &[u8]) -> Result<ClientPacket, Error> {
        bincode::deserialize(data).map_err(|_e| Error::CannotDeserialize)
    }

    fn serialize(&self) -> Result<Vec<u8>, Error> {
        bincode::serialize(&self).map_err(|_e| Error::CannotSerialize)
    }
}
