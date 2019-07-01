use common::net::PostError;

#[derive(Debug)]
pub enum Error {
    Network(PostError),
    ServerWentMad,
    ServerTimeout,
    ServerShutdown,
    TooManyPlayers,
    Other(String),
}

impl From<PostError> for Error {
    fn from(err: PostError) -> Self {
        Error::Network(err)
    }
}
