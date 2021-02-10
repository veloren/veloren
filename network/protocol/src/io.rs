use crate::ProtocolError;
use async_trait::async_trait;
use bytes::BytesMut;
use std::collections::VecDeque;
///! I/O-Free (Sans-I/O) protocol https://sans-io.readthedocs.io/how-to-sans-io.html

// Protocols should base on the Unrealiable variants to get something effective!
#[async_trait]
pub trait UnreliableDrain: Send {
    type DataFormat;
    async fn send(&mut self, data: Self::DataFormat) -> Result<(), ProtocolError>;
}

#[async_trait]
pub trait UnreliableSink: Send {
    type DataFormat;
    async fn recv(&mut self) -> Result<Self::DataFormat, ProtocolError>;
}

pub struct BaseDrain {
    data: VecDeque<BytesMut>,
}

pub struct BaseSink {
    data: VecDeque<BytesMut>,
}

impl BaseDrain {
    pub fn new() -> Self {
        Self {
            data: VecDeque::new(),
        }
    }
}

impl BaseSink {
    pub fn new() -> Self {
        Self {
            data: VecDeque::new(),
        }
    }
}

//TODO: Test Sinks that drop 20% by random and log that

#[async_trait]
impl UnreliableDrain for BaseDrain {
    type DataFormat = BytesMut;

    async fn send(&mut self, data: Self::DataFormat) -> Result<(), ProtocolError> {
        self.data.push_back(data);
        Ok(())
    }
}

#[async_trait]
impl UnreliableSink for BaseSink {
    type DataFormat = BytesMut;

    async fn recv(&mut self) -> Result<Self::DataFormat, ProtocolError> {
        self.data.pop_front().ok_or(ProtocolError::Closed)
    }
}
