use bincode::{serialize, deserialize};

use ClientMode;
use Error;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ServerPacket {
    Connected,
    Shutdown,
    Ping,
    RecvChatMsg { alias: String, msg: String },
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
    Connect { mode: ClientMode, alias: String },
    Disconnect,
    Ping,
    SendChatMsg { msg: String },
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
