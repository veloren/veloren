use common::net::PostError;

#[derive(Debug)]
pub enum Error {
    Network(PostError),
    Other(String),
}

impl From<PostError> for Error {
    fn from(err: PostError) -> Self { Error::Network(err) }
}
