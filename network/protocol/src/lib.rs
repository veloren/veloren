#![feature(drain_filter)]
//! Network Protocol
//!
//! a I/O-Free protocol for the veloren network crate.
//! This crate defines multiple different protocols over [`UnreliableDrain`] and
//! [`UnreliableSink`] traits, which allows it to define the behavior of a
//! protocol separated from the actual io.
//!
//! For example we define the TCP protocol on top of Drains and Sinks that can
//! send chunks of bytes. You can now implement your own Drain And Sink that
//! sends the data via tokio's or std's implementation. Or you just use a
//! std::mpsc::channel for unit tests without needing a actual tcp socket.
//!
//! This crate currently defines:
//!  - TCP
//!  - MPSC
//!  - QUIC
//!
//! eventually a pure UDP implementation will follow
//!
//! warning: don't mix protocol, using the TCP variant for actual UDP socket
//! will result in dropped data  using UDP with a TCP socket will be a waste of
//! resources.
//!
//! A *channel* in this crate is defined as a combination of *read* and *write*
//! protocol.
//!
//! # adding a protocol
//!
//! We start by defining our DataFormat. For most this is prob [`Vec<u8>`] or
//! [`Bytes`]. MPSC can directly send a msg without serialisation.
//!
//! Create 2 structs, one for the receiving and sending end. Based on a generic
//! Drain/Sink with your required DataFormat.
//! Implement the [`SendProtocol`] and [`RecvProtocol`] traits respectively.
//!
//! Implement the Handshake: [`InitProtocol`], alternatively you can also
//! implement `ReliableDrain` and `ReliableSink`, by this, you use the default
//! Handshake.
//!
//! This crate also contains consts and definitions for the network protocol.
//!
//! For an *example* see `TcpDrain` and `TcpSink` in the [tcp.rs](tcp.rs)
//!
//! [`UnreliableDrain`]: crate::UnreliableDrain
//! [`UnreliableSink`]: crate::UnreliableSink
//! [`Vec<u8>`]: std::vec::Vec
//! [`Bytes`]: bytes::Bytes
//! [`SendProtocol`]: crate::SendProtocol
//! [`RecvProtocol`]: crate::RecvProtocol
//! [`InitProtocol`]: crate::InitProtocol

mod error;
mod event;
mod frame;
mod handshake;
mod message;
mod metrics;
mod mpsc;
mod prio;
mod quic;
mod tcp;
mod types;
mod util;

pub use error::{InitProtocolError, ProtocolError};
pub use event::ProtocolEvent;
pub use metrics::ProtocolMetricCache;
#[cfg(feature = "metrics")]
pub use metrics::ProtocolMetrics;
pub use mpsc::{MpscMsg, MpscRecvProtocol, MpscSendProtocol};
pub use quic::{QuicDataFormat, QuicDataFormatStream, QuicRecvProtocol, QuicSendProtocol};
pub use tcp::{TcpRecvProtocol, TcpSendProtocol};
pub use types::{Bandwidth, Cid, Pid, Prio, Promises, Sid, HIGHEST_PRIO, VELOREN_NETWORK_VERSION};

///use at own risk, might change any time, for internal benchmarks
pub mod _internal {
    pub use crate::{
        frame::{ITFrame, OTFrame},
        util::SortedVec,
    };
}

use async_trait::async_trait;

/// Handshake: Used to connect 2 Channels.
#[async_trait]
pub trait InitProtocol {
    type CustomErr: std::fmt::Debug + Send;

    async fn initialize(
        &mut self,
        initializer: bool,
        local_pid: Pid,
        secret: u128,
    ) -> Result<(Pid, Sid, u128), InitProtocolError<Self::CustomErr>>;
}

/// Generic Network Send Protocol.
/// Implement this for your Protocol of choice ( tcp, udp, mpsc, quic)
/// Allows the creation/deletions of `Streams` and sending messages via
/// [`ProtocolEvent`].
///
/// A `Stream` MUST be bound to a specific Channel. You MUST NOT switch the
/// channel to send a stream mid air. We will provide takeover options for
/// Channel closure in the future to allow keeping a `Stream` over a broken
/// Channel.
///
/// [`ProtocolEvent`]: crate::ProtocolEvent
#[async_trait]
pub trait SendProtocol {
    type CustomErr: std::fmt::Debug + Send;

    /// YOU MUST inform the `SendProtocol` by any Stream Open BEFORE using it in
    /// `send` and Stream Close AFTER using it in `send` via this fn.
    fn notify_from_recv(&mut self, event: ProtocolEvent);
    /// Send a Event via this Protocol. The `SendProtocol` MAY require `flush`
    /// to be called before actual data is send to the respective `Sink`.
    async fn send(&mut self, event: ProtocolEvent) -> Result<(), ProtocolError<Self::CustomErr>>;
    /// Flush all buffered messages according to their [`Prio`] and
    /// [`Bandwidth`]. provide the current bandwidth budget (per second) as
    /// well as the `dt` since last call. According to the budget the
    /// respective messages will be flushed.
    ///
    /// [`Prio`]: crate::Prio
    /// [`Bandwidth`]: crate::Bandwidth
    async fn flush(
        &mut self,
        bandwidth: Bandwidth,
        dt: std::time::Duration,
    ) -> Result<Bandwidth, ProtocolError<Self::CustomErr>>;
}

/// Generic Network Recv Protocol. See: [`SendProtocol`]
///
/// [`SendProtocol`]: crate::SendProtocol
#[async_trait]
pub trait RecvProtocol {
    type CustomErr: std::fmt::Debug + Send;

    /// Either recv an event or fail the Protocol, once the Recv side is closed
    /// it cannot recover from the error.
    async fn recv(&mut self) -> Result<ProtocolEvent, ProtocolError<Self::CustomErr>>;
}

/// This crate makes use of UnreliableDrains, they are expected to provide the
/// same guarantees like their IO-counterpart. E.g. ordered messages for TCP and
/// nothing for UDP. The respective Protocol needs then to handle this.
/// This trait is an abstraction above multiple Drains, e.g. [`tokio`](https://tokio.rs) [`async-std`] [`std`] or even [`async-channel`]
///
/// [`async-std`]: async-std
/// [`std`]: std
/// [`async-channel`]: async-channel
#[async_trait]
pub trait UnreliableDrain: Send {
    type CustomErr: std::fmt::Debug + Send;
    type DataFormat;
    async fn send(&mut self, data: Self::DataFormat) -> Result<(), ProtocolError<Self::CustomErr>>;
}

/// Sink counterpart of [`UnreliableDrain`]
///
/// [`UnreliableDrain`]: crate::UnreliableDrain
#[async_trait]
pub trait UnreliableSink: Send {
    type CustomErr: std::fmt::Debug + Send;
    type DataFormat;
    async fn recv(&mut self) -> Result<Self::DataFormat, ProtocolError<Self::CustomErr>>;
}
