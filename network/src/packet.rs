pub trait Serialize {
    fn serialize(&self) -> Vec<u8>;
}

#[derive(Clone)]
pub enum ServerPacket {
    Connected,
    Shutdown,
    Ping,
    RecvChatMsg { alias: String, msg: String },
}

impl Serialize for ServerPacket {
    fn serialize(&self) -> Vec<u8> {
        vec!()
    }
}

#[derive(Clone)]
pub enum ClientPacket {
    Connect { alias: String },
    Disconnect,
    Ping,
    SendChatMsg { msg: String },
}

impl Serialize for ClientPacket {
    fn serialize(&self) -> Vec<u8> {
        vec!()
    }
}
