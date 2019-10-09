pub enum NetworkError {
    IoError(std::io::Error),
    SerdeError(serde_cbor::Error),
    EngineShutdownError,
    MailBoxEmpty,
}

pub type NetworkResult<T> = Result<T, NetworkError>;

impl From<std::io::Error> for NetworkError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}

impl From<serde_cbor::Error> for NetworkError {
    fn from(e: serde_cbor::Error) -> Self {
        Self::SerdeError(e)
    }
}

impl<T> From<crossbeam_channel::SendError<T>> for NetworkError {
    fn from(_e: crossbeam_channel::SendError<T>) -> Self {
        Self::EngineShutdownError
    }
}

impl From<crossbeam_channel::RecvError> for NetworkError {
    fn from(_e: crossbeam_channel::RecvError) -> Self {
        Self::EngineShutdownError
    }
}

impl From<crossbeam_channel::TryRecvError> for NetworkError {
    fn from(e: crossbeam_channel::TryRecvError) -> Self {
        match e {
            crossbeam_channel::TryRecvError::Empty => Self::MailBoxEmpty,
            crossbeam_channel::TryRecvError::Disconnected => Self::EngineShutdownError,
        }
    }
}
