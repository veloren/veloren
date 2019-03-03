use common::net::PostError;

#[derive(Debug)]
pub enum Error {
    Network(PostError),
    ServerShutdown,
    Other(String),
}

impl From<PostError> for Error {
    fn from(err: PostError) -> Self {
        match err {
            PostError::Disconnected => Error::ServerShutdown,
            err => Error::Network(err),
        }
    }
}
