#![feature(trait_alias, try_trait, async_closure)]

//! Crate to handle high level networking of messages with different
//! requirements and priorities over a number of protocols
//!
//! To start with the `veloren_network` crate you should focus on the 3
//! elementar structs [`Network`], [`Participant`] and [`Stream`].
//!
//! Say you have an application that wants to communicate with other application
//! over a Network or on the same computer. Now each application instances the
//! struct [`Network`] once with a new [`Pid`]. The Pid is necessary to identify
//! other [`Networks`] over the network protocols (e.g. TCP, UDP)
//!
//! To connect to another application, you must know it's [`Address`]. One side
//! will call [`connect`], the other [`connected`]. If successfull both
//! applications will now get a [`Arc<Participant>`].
//!
//! This [`Participant`] represents the connection between those 2 applications.
//! over the respective [`Address`] and with it the choosen network protocol.
//! However messages can't be send directly via [`Participants`], instead you
//! must open a [`Stream`] on it. Like above, one side has to call [`open`], the
//! other [`opened`]. [`Streams`] can have a different priority and
//! [`Promises`].
//!
//! You can now use the [`Stream`] to [`send`] and [`recv`] in both directions.
//! You can send all kind of messages that implement [`serde`].
//! As the receiving side needs to know the format, it sometimes is useful to
//! always send a specific Enum and then handling it with a big `match`
//! statement This create makes heavily use of `async`, except for [`send`]
//! which returns always directly.
//!
//! For best practices see the `examples` folder of this crate containing useful
//! code snippets, a simple client/server below. Of course due to the async
//! nature, no strict client server separation is necessary
//!
//! # Examples
//! ```rust
//! // Client
//! use futures::executor::block_on;
//! use veloren_network::{Network, Pid, PROMISES_CONSISTENCY, PROMISES_ORDERED};
//!
//! let network = Network::new(Pid::new(), ThreadPoolBuilder::new().build(), None);
//! block_on(async {
//!     let server = network
//!         .connect(Address::Tcp("127.0.0.1:12345".parse().unwrap()))
//!         .await?;
//!     let stream = server
//!         .open(10, PROMISES_ORDERED | PROMISES_CONSISTENCY)
//!         .await?;
//!     stream.send("Hello World")?;
//! });
//! ```
//!
//! ```rust
//! // Server
//! use futures::executor::block_on;
//! use veloren_network::{Network, Pid};
//!
//! let network = Network::new(Pid::new(), ThreadPoolBuilder::new().build(), None);
//! block_on(async {
//!     network
//!         .listen(Address::Tcp("127.0.0.1:12345".parse().unwrap()))
//!         .await?;
//!     let client = network.connected().await?;
//!     let stream = server.opened().await?;
//!     let msg: String = stream.recv().await?;
//!     println!("got message: {}", msg);
//! });
//! ```
//!
//! [`Network`]: crate::api::Network
//! [`Networks`]: crate::api::Network
//! [`connect`]: crate::api::Network::connect
//! [`connected`]: crate::api::Network::connected
//! [`Arc<Participant>`]: crate::api::Participant
//! [`Participant`]: crate::api::Participant
//! [`Participants`]: crate::api::Participant
//! [`open`]: crate::api::Participant::open
//! [`opened`]: crate::api::Participant::opened
//! [`Stream`]: crate::api::Stream
//! [`Streams`]: crate::api::Stream
//! [`send`]: crate::api::Stream::send
//! [`recv`]: crate::api::Stream::recv
//! [`Pid`]: crate::types::Pid
//! [`Address`]: crate::api::Address
//! [`Promises`]: crate::types::Promises

mod api;
mod channel;
mod message;
mod metrics;
mod participant;
mod prios;
mod protocols;
mod scheduler;
mod types;

pub use api::{Address, Network, NetworkError, Participant, ParticipantError, Stream, StreamError};
pub use message::MessageBuffer;
pub use types::{
    Pid, Promises, PROMISES_COMPRESSED, PROMISES_CONSISTENCY, PROMISES_ENCRYPTED,
    PROMISES_GUARANTEED_DELIVERY, PROMISES_NONE, PROMISES_ORDERED,
};
