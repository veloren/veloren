#![allow(non_local_definitions)] // necessary because of the Protocol derive macro
use protocol::Protocol;

pub(crate) const VERSION: u16 = 0;
pub(crate) const VELOREN_HEADER: [u8; 7] = [b'v', b'e', b'l', b'o', b'r', b'e', b'n'];
pub(crate) const MAX_REQUEST_CONTENT_SIZE: usize = 300;
// NOTE: The actual maximum size must never exceed 1200 or we risk getting near
// MTU limits for some networks.
pub(crate) const MAX_REQUEST_SIZE: usize = MAX_REQUEST_CONTENT_SIZE + VELOREN_HEADER.len() + 2;
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
    /// This requests exists mostly for backwards-compatibilty reasons. As the
    /// first message sent to the server should always be in the V0 version
    /// of the protocol, if future versions of the protocol have more
    /// requests than server info it may be confusing to request `P` and the max
    /// version with a `QueryServerRequest::ServerInfo` request (the request
    /// will still be dropped as the supplied `P` value is invalid).
    Init,
    ServerInfo,
    // New requests should be added at the end to prevent breakage.
    // NOTE: Any new (sub-)variants must be added to the `check_request_sizes` test at the end of
    // this file
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
    // New responses should be added at the end to prevent breakage
}

#[derive(Protocol, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerInfo {
    pub git_hash: u32,
    pub git_timestamp: i64,
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

impl RawQueryServerRequest {
    #[cfg(any(feature = "client", test))]
    pub fn serialize(&self) -> Result<Vec<u8>, protocol::Error> {
        use protocol::Parcel;

        let mut buf = Vec::with_capacity(MAX_REQUEST_SIZE);

        // 2 extra bytes for version information, currently unused
        buf.extend(VERSION.to_le_bytes());
        buf.extend({
            let request_data =
                <RawQueryServerRequest as Parcel>::raw_bytes(self, &Default::default())?;
            if request_data.len() > MAX_REQUEST_CONTENT_SIZE {
                panic!(
                    "Attempted to send request larger than the max size (size: {}, max size: \
                     {MAX_REQUEST_CONTENT_SIZE}, request: {self:?})",
                    request_data.len()
                );
            }
            request_data
        });
        const _: () = assert!(MAX_RESPONSE_SIZE + VELOREN_HEADER.len() <= MAX_REQUEST_SIZE);
        buf.resize(MAX_RESPONSE_SIZE.max(buf.len()), 0);
        buf.extend(VELOREN_HEADER);
        Ok(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::{QueryServerRequest, RawQueryServerRequest};

    #[test]
    fn check_request_sizes() {
        const ALL_REQUESTS: &[QueryServerRequest] =
            &[QueryServerRequest::ServerInfo, QueryServerRequest::Init];
        for request in ALL_REQUESTS {
            let request = RawQueryServerRequest {
                p: 0,
                request: *request,
            };
            request.serialize().unwrap(); // This will panic if the size is above MAX_REQUEST_SIZE
        }
    }
}
