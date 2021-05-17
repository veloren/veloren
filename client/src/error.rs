use authc::AuthClientError;
pub use network::{InitProtocolError, NetworkConnectError, NetworkError};
use network::{ParticipantError, StreamError};
use specs::error::Error as SpecsError;

#[derive(Debug)]
pub enum Error {
    Kicked(String),
    NetworkErr(NetworkError),
    ParticipantErr(ParticipantError),
    StreamErr(StreamError),
    ServerTimeout,
    ServerShutdown,
    TooManyPlayers,
    NotOnWhitelist,
    AuthErr(String),
    AuthClientError(AuthClientError),
    AuthServerUrlInvalid(String),
    AuthServerNotTrusted,
    HostnameLookupFailed(std::io::Error),
    Banned(String),
    /// Persisted character data is invalid or missing
    InvalidCharacter,
    //TODO: InvalidAlias,
    Other(String),
    SpecsErr(SpecsError),
}

impl From<SpecsError> for Error {
    fn from(err: SpecsError) -> Self { Self::SpecsErr(err) }
}

impl From<NetworkError> for Error {
    fn from(err: NetworkError) -> Self { Self::NetworkErr(err) }
}

impl From<ParticipantError> for Error {
    fn from(err: ParticipantError) -> Self { Self::ParticipantErr(err) }
}

impl From<StreamError> for Error {
    fn from(err: StreamError) -> Self { Self::StreamErr(err) }
}

impl From<AuthClientError> for Error {
    fn from(err: AuthClientError) -> Self { Self::AuthClientError(err) }
}
