use protocol::Protocol;

pub(crate) const VERSION: u16 = 0;
pub(crate) const VELOREN_HEADER: [u8; 7] = [b'v', b'e', b'l', b'o', b'r', b'e', b'n'];
// The actual maximum size of packets will be `MAX_REQUEST_SIZE +
// VELOREN_HEADER.len() + 2` (2 added for currently unused version).
// NOTE: The actual maximum size must never exceed 1200 or we risk getting near
// MTU limits for some networks.
pub(crate) const MAX_REQUEST_SIZE: usize = 300;
pub(crate) const MAX_RESPONSE_SIZE: usize = 256;

#[derive(Protocol, Debug, Clone, Copy)]
pub(crate) struct RawQueryServerRequest {
    /// See comment on [`Init::p`]
    pub p: u64,
    pub request: QueryServerRequest,
}

#[derive(Protocol, Debug, Clone, Copy)]
#[protocol(discriminant = "integer")]
#[protocol(discriminator(u8))]
#[allow(clippy::large_enum_variant)]
pub enum QueryServerRequest {
    ServerInfo,
    Ping,
    // New requests should be added at the end to prevent breakage
}

#[derive(Protocol, Debug, Clone, Copy)]
pub(crate) struct Init {
    /// This is used as a challenge to prevent IP address spoofing by verifying
    /// that the client can receive from the source address.
    ///
    /// Any request to the server must include this value to be processed,
    /// otherwise this response will be returned (giving clients a value to pass
    /// for later requests).
    pub p: u64,
    /// The maximum supported protocol version by the server. The first request
    /// to a server must always be done in the V0 protocol to query this value.
    /// Following requests (when the version is known), can be done in the
    /// maximum version or below, responses will be sent in the same version as
    /// the requests.
    pub max_supported_version: u16,
}

#[derive(Protocol, Debug, Clone, Copy)]
#[protocol(discriminant = "integer")]
#[protocol(discriminator(u8))]
pub(crate) enum RawQueryServerResponse {
    Response(QueryServerResponse),
    Init(Init),
}

#[derive(Protocol, Debug, Clone, Copy)]
#[protocol(discriminant = "integer")]
#[protocol(discriminator(u8))]
pub enum QueryServerResponse {
    ServerInfo(ServerInfo),
    Pong,
    // New responses should be added at the end to prevent breakage
}

#[derive(Protocol, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerInfo {
    pub git_hash: u32,
    pub git_version: i64,
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
