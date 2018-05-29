use bincode::{serialize, deserialize};

pub trait Serialize {
    fn serialize(&self) -> Option<Vec<u8>>;
}

#[derive(Clone, Serialize, Deserialize)]
pub enum ServerPacket {
    Connected,
    Shutdown,
    Ping,
    RecvChatMsg { alias: String, msg: String },
}

impl Serialize for ServerPacket {
    fn serialize(&self) -> Option<Vec<u8>> {
        serialize(&self).ok()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum ClientPacket {
    Connect { alias: String },
    Disconnect,
    Ping,
    SendChatMsg { msg: String },
}

impl Serialize for ClientPacket {
    fn serialize(&self) -> Option<Vec<u8>> {
        serialize(&self).ok()
    }
}
