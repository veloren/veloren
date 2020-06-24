use authc::AuthClientError;
use common::net::PostError;

#[derive(Debug)]
pub enum Error {
    Network(PostError),
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

impl From<PostError> for Error {
    fn from(err: PostError) -> Self { Self::Network(err) }
}

impl From<AuthClientError> for Error {
    fn from(err: AuthClientError) -> Self { Self::AuthClientError(err) }
}
