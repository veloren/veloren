use protocol::Protocol;

pub const VELOREN_HEADER: [u8; 7] = [b'v', b'e', b'l', b'o', b'r', b'e', b'n'];
pub const MAX_REQUEST_SIZE: usize = 300;
pub const MAX_RESPONSE_SIZE: usize = 256;

#[derive(Protocol, Debug, Clone, Copy)]
#[protocol(discriminant = "integer")]
#[protocol(discriminator(u8))]
#[allow(clippy::large_enum_variant)]
pub enum QueryServerRequest {
    ServerInfo(ServerInfoRequest),
    // New requests should be added at the end to prevent breakage
}

#[derive(Protocol, Debug, Clone, Copy)]
#[protocol(discriminant = "integer")]
#[protocol(discriminator(u8))]
pub enum QueryServerResponse {
    ServerInfo(ServerInfo),
    // New responses should be added at the end to prevent breakage
}

#[derive(Protocol, Debug, Clone, Copy)]
pub struct ServerInfoRequest {
    // Padding to prevent amplification attacks
    pub _padding: [u8; 256],
}

#[derive(Protocol, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerInfo {
    pub git_hash: [char; 8],
    pub players_count: u16,
    pub player_cap: u16,
    pub battlemode: ServerBattleMode,
}

#[derive(Protocol, Debug, Clone, Copy, PartialEq, Eq)]
#[protocol(discriminant = "integer")]
#[protocol(discriminator(u8))]
#[repr(u8)]
pub enum ServerBattleMode {
    GlobalPvP,
    GlobalPvE,
    PerPlayer,
}

impl Default for ServerInfoRequest {
    fn default() -> Self { ServerInfoRequest { _padding: [0; 256] } }
}

impl ServerInfo {
    pub fn git_hash(&self) -> String { String::from_iter(&self.git_hash) }
}
