use bincode::{serialize, deserialize};

#[derive(Clone, Serialize, Deserialize)]
pub enum ServerPacket {
    Connected,
    Shutdown,
    Ping,
    RecvChatMsg { alias: String, msg: String },
}

impl ServerPacket {
    pub fn from(data: &[u8]) -> Option<ServerPacket> {
        deserialize(data).ok() // TODO: Handle error?
    }

    pub fn serialize(&self) -> Option<Vec<u8>> {
        serialize(&self).ok() // TODO: Handle error?
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum ClientPacket {
    Connect { alias: String },
    Disconnect,
    Ping,
    SendChatMsg { msg: String },
}

impl ClientPacket {
    pub fn from(data: &[u8]) -> Option<ClientPacket> {
        deserialize(data).ok() // TODO: Handle error?
    }

    pub fn serialize(&self) -> Option<Vec<u8>> {
        serialize(&self).ok() // TODO: Handle error?
    }
}
