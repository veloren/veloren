/// Messages server sends to client.
#[derive(Deserialize, Serialize, Debug)]
pub enum ServerMsg {
    // VersionInfo MUST always stay first in this struct.
    VersionInfo {},
}

/// Messages client sends to server.
#[derive(Deserialize, Serialize, Debug)]
pub enum ClientMsg {
    // VersionInfo MUST always stay first in this struct.
    VersionInfo {},
}

/// Control message type, used in [PostBox](super::PostBox) and
/// [PostOffice](super::PostOffice) to control threads.
pub enum ControlMsg {
    Shutdown,
}
