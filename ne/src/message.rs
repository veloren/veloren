use crate::NetworkResult;
use serde::{Serialize, Deserialize};
use serde::de::DeserializeOwned;
use std::io::{BufWriter, BufReader};
use std::net::TcpStream;
use std::io::Write;

#[derive(Serialize, Deserialize)]
pub struct NetworkMessage<T> {
    data: T,
}

impl<T: Serialize + DeserializeOwned> NetworkMessage<T> {
    pub fn send(&self, writer: &mut BufWriter<TcpStream>) -> NetworkResult<()> {
        serde_cbor::to_writer(writer.by_ref(), self)?;
        writer.flush()?;
        Ok(())
    }

    pub fn decode(reader: &mut BufReader<TcpStream>) -> NetworkResult<Self> {
        let message = serde_cbor::from_reader(reader)?;
        Ok(message)
    }
}
