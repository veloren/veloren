use crate::render::RenderError;
use std::fmt::Debug;

/// Represents any error that may be triggered by Voxygen.
#[derive(Debug)]
pub enum Error {
    /// An error relating to the internal client.
    ClientError(client::Error),
    /// A miscellaneous error relating to a backend dependency.
    BackendError(Box<dyn Debug>),
    /// An error relating the rendering subsystem.
    RenderError(RenderError),
    /// A miscellaneous error with an unknown or unspecified source.
    Other(failure::Error),
}

impl From<RenderError> for Error {
    fn from(err: RenderError) -> Self { Error::RenderError(err) }
}

impl From<client::Error> for Error {
    fn from(err: client::Error) -> Self { Error::ClientError(err) }
}
