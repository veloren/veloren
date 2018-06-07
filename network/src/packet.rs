use bincode::{serialize, deserialize};
use nalgebra::Vector3;
use ClientMode;
use Error;
use common::Uid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ServerPacket {
    Connected { player_entity_uid: Option<Uid>, version: String },
    Kicked { reason: String },
    Shutdown,
    Ping,
    RecvChatMsg { alias: String, msg: String },
    EntityUpdate { uid: Uid, pos: Vector3<f32> },
}

impl ServerPacket {
    pub fn from(data: &[u8]) -> Result<ServerPacket, Error> {
        match deserialize(data) {
            Ok(sp) => Ok(sp),
            Err(_) => Err(Error::CannotDeserialize),
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>, Error> {
        match serialize(&self) {
            Ok(data) => Ok(data),
            Err(_) => Err(Error::CannotSerialize),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ClientPacket {
    Connect { mode: ClientMode, alias: String, version: String },
    Disconnect,
    Ping,
    SendChatMsg { msg: String },
    SendCmd { cmd: String },
    PlayerEntityUpdate { pos: Vector3<f32> }
}

impl ClientPacket {
    pub fn from(data: &[u8]) -> Result<ClientPacket, Error> {
        match deserialize(data) {
            Ok(sp) => Ok(sp),
            Err(_) => Err(Error::CannotDeserialize),
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>, Error> {
        match serialize(&self) {
            Ok(data) => Ok(data),
            Err(_) => Err(Error::CannotSerialize),
        }
    }
}
