pub enum NetworkError {
    SerializationError(serde_cbor::Error),
}

pub type NetworkResult<T> = Result<T, NetworkError>;

impl From<serde_cbor::Error> for NetworkError {
    fn from(e: serde_cbor::Error) -> Self {
        Self::SerializationError(e)
    }
}
