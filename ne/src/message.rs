use crate::NetworkResult;
use crossbeam_channel::{Receiver, Sender};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::io::Write;
use std::io::{BufReader, BufWriter};
use std::net::TcpStream;

/// A reliability level specified per mail. These levels have different guarantees.
/// Those with stronger guarantees incur a grater latency cost.
pub enum Reliability {
    /// Special value. Should not be used.
    None,

    /// The reliable level ensures mail ordering and that it gets to its destination.
    Reliable,

    /// The unreliable level does not ensure ordering nor that it gets to it's destination.
    /// The only guarantee is that the message is not damaged if it is received.
    Unreliable,
}

pub struct InternalNetworkMessage<T> {
    result_sender: Option<Sender<NetworkResult<()>>>,
    data: T,
    reliability: Reliability,
    id: u32,
}

impl<T: Send + Serialize + DeserializeOwned> InternalNetworkMessage<T> {
    pub fn new(data: T, reliability: Reliability, id: u32) -> (Self, Receiver<NetworkResult<()>>) {
        let (result_sender, result_receiver) = crossbeam_channel::bounded(1);
        (
            Self {
                result_sender: Some(result_sender),
                data,
                reliability,
                id,
            },
            result_receiver,
        )
    }

    pub fn deconstruct(self) -> (u32, T) {
        (self.id, self.data)
    }

    pub fn send(&self, writer: &mut BufWriter<TcpStream>) {
        let result = self.send_raw(writer);
        let _ = self.result_sender.as_ref().unwrap().send(result);
    }

    pub fn send_raw(&self, writer: &mut BufWriter<TcpStream>) -> NetworkResult<()> {
        serde_cbor::to_writer(writer.by_ref(), &self.data)?;
        writer.flush()?;
        Ok(())
    }

    pub fn receive(reader: &mut BufReader<TcpStream>, id: u32) -> NetworkResult<Self> {
        let data: T = serde_cbor::from_reader(reader)?;

        Ok(Self {
            result_sender: None,
            data,
            reliability: Reliability::None,
            id,
        })
    }
}
