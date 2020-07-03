use authc::AuthClientError;
pub use network::NetworkError;
use network::{ParticipantError, StreamError};

#[derive(Debug)]
pub enum Error {
    NetworkErr(NetworkError),
    ParticipantErr(ParticipantError),
    StreamErr(StreamError),
    ServerWentMad,
    ServerTimeout,
    ServerShutdown,
    TooManyPlayers,
    NotOnWhitelist,
    AlreadyLoggedIn,
    AuthErr(String),
    AuthClientError(AuthClientError),
    AuthServerNotTrusted,
    /// Persisted character data is invalid or missing
    InvalidCharacter,
    //TODO: InvalidAlias,
    Other(String),
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
