use crate::NetworkResult;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct NetworkMessage<T> {
    data: T,
}

impl<'a, T: Serialize + Deserialize<'a>> NetworkMessage<T> {
    pub fn encode(&self) -> NetworkResult<Vec<u8>> {
        Ok(serde_cbor::to_vec(self)?)
    }

    pub fn decode(bytes: &'a [u8]) -> NetworkResult<Self> {
        Ok(serde_cbor::from_slice(bytes)?)
    }
}
