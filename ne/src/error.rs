pub enum NetworkError {
    Io(std::io::Error),
    Serde(serde_cbor::Error),
    EngineShutdown,
}

pub type NetworkResult<T> = Result<T, NetworkError>;

impl From<std::io::Error> for NetworkError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_cbor::Error> for NetworkError {
    fn from(e: serde_cbor::Error) -> Self {
        Self::Serde(e)
    }
}

impl<T> From<crossbeam_channel::SendError<T>> for NetworkError {
    fn from(_e: crossbeam_channel::SendError<T>) -> Self {
        Self::EngineShutdown
    }
}

impl From<crossbeam_channel::RecvError> for NetworkError {
    fn from(_e: crossbeam_channel::RecvError) -> Self {
        Self::EngineShutdown
    }
}
