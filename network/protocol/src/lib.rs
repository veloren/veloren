mod event;
mod frame;
mod handshake;
mod io;
mod message;
mod metrics;
mod mpsc;
mod prio;
mod tcp;
mod types;

pub use event::ProtocolEvent;
pub use io::{BaseDrain, BaseSink, UnreliableDrain, UnreliableSink};
pub use message::MessageBuffer;
pub use metrics::ProtocolMetricCache;
#[cfg(feature = "metrics")]
pub use metrics::ProtocolMetrics;
pub use mpsc::{MpscMsg, MpscRecvProtcol, MpscSendProtcol};
pub use tcp::{TcpRecvProtcol, TcpSendProtcol};
pub use types::{Bandwidth, Cid, Mid, Pid, Prio, Promises, Sid, VELOREN_NETWORK_VERSION};

///use at own risk, might change any time, for internal benchmarks
pub mod _internal {
    pub use crate::frame::Frame;
}

use async_trait::async_trait;

#[async_trait]
pub trait InitProtocol {
    async fn initialize(
        &mut self,
        initializer: bool,
        local_pid: Pid,
        secret: u128,
    ) -> Result<(Pid, Sid, u128), InitProtocolError>;
}

#[async_trait]
pub trait SendProtocol {
    //a stream MUST be bound to a specific Protocol, there will be a failover
    // feature comming for the case where a Protocol fails completly
    async fn send(&mut self, event: ProtocolEvent) -> Result<(), ProtocolError>;
    async fn flush(
        &mut self,
        bandwidth: Bandwidth,
        dt: std::time::Duration,
    ) -> Result<(), ProtocolError>;
}

#[async_trait]
pub trait RecvProtocol {
    async fn recv(&mut self) -> Result<ProtocolEvent, ProtocolError>;
}

#[derive(Debug, PartialEq)]
pub enum InitProtocolError {
    Closed,
    WrongMagicNumber([u8; 7]),
    WrongVersion([u32; 3]),
}

#[derive(Debug, PartialEq)]
/// When you return closed you must stay closed!
pub enum ProtocolError {
    Closed,
}

impl From<ProtocolError> for InitProtocolError {
    fn from(err: ProtocolError) -> Self {
        match err {
            ProtocolError::Closed => InitProtocolError::Closed,
        }
    }
}
