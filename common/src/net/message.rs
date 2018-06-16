use bincode;
use nalgebra::Vector3;

use Uid;
use net::ClientMode;
use super::Error;

pub trait Message {
    fn serialize(&self) -> Result<Vec<u8>, Error>;
    fn from(data: &[u8]) -> Result<Self, Error> where Self: Sized;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ServerMessage {
    Connected { entity_uid: Option<Uid>, version: String },
    Kicked { reason: String },
    Shutdown,
    Ping,
    RecvChatMsg { alias: String, msg: String },
    EntityUpdate { uid: Uid, pos: Vector3<f32> },
}

impl Message for ServerMessage {
    fn from(data: &[u8]) -> Result<ServerMessage, Error> {
        bincode::deserialize(data).map_err(|_e| Error::CannotDeserialize)
    }

    fn serialize(&self) -> Result<Vec<u8>, Error> {
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
    PlayerEntityUpdate { pos: Vector3<f32> }
}

impl Message for ClientMessage {
    fn from(data: &[u8]) -> Result<ClientMessage, Error> {
        bincode::deserialize(data).map_err(|_e| Error::CannotDeserialize)
    }

    fn serialize(&self) -> Result<Vec<u8>, Error> {
        bincode::serialize(&self).map_err(|_e| Error::CannotSerialize)
    }
}
