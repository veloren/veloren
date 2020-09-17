use network::{NetworkError, ParticipantError, StreamError};

use std::fmt::{self, Display};

#[derive(Debug)]
pub enum Error {
    NetworkErr(NetworkError),
    ParticipantErr(ParticipantError),
    StreamErr(StreamError),
    DatabaseErr(diesel::result::Error),
    Other(String),
}

impl From<NetworkError> for Error {
    fn from(err: NetworkError) -> Self { Error::NetworkErr(err) }
}

impl From<ParticipantError> for Error {
    fn from(err: ParticipantError) -> Self { Error::ParticipantErr(err) }
}

impl From<StreamError> for Error {
    fn from(err: StreamError) -> Self { Error::StreamErr(err) }
}

impl From<diesel::result::Error> for Error {
    fn from(err: diesel::result::Error) -> Self { Error::DatabaseErr(err) }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::NetworkErr(err) => write!(f, "Network Error: {}", err),
            Self::ParticipantErr(err) => write!(f, "Participant Error: {}", err),
            Self::StreamErr(err) => write!(f, "Stream Error: {}", err),
            Self::DatabaseErr(err) => write!(f, "Database Error: {}", err),
            Self::Other(err) => write!(f, "Error: {}", err),
        }
    }
}
