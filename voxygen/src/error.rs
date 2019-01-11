// Standard
use std::any;

// Crate
use crate::render::RenderError;

/// Represents any error that may be triggered by Voxygen
#[derive(Debug)]
pub enum Error {
    /// A miscellaneous error relating to a backend dependency
    BackendError(Box<any::Any>),
    /// An error relating the rendering subsystem
    RenderError(RenderError),
    // A miscellaneous error with an unknown or unspecified source
    Other(failure::Error),
}

impl From<RenderError> for Error {
    fn from(err: RenderError) -> Self {
        Error::RenderError(err)
    }
}
