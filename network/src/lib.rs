#![deny(unsafe_code)]
#![cfg_attr(test, deny(rust_2018_idioms))]
#![cfg_attr(test, deny(warnings))]
#![deny(clippy::clone_on_ref_ptr)]
#![feature(assert_matches)]

//! Crate to handle high level networking of messages with different
//! requirements and priorities over a number of protocols
//!
//! To start with the `veloren_network` crate you should focus on the 3
//! elementar structs [`Network`], [`Participant`] and [`Stream`].
//!
//! Say you have an application that wants to communicate with other application
//! over a Network or on the same computer. Now each application instances the
//! struct [`Network`] once with a new [`Pid`]. The Pid is necessary to identify
//! other [`Networks`] over the network protocols (e.g. TCP, UDP, QUIC, MPSC)
//!
//! To connect to another application, you must know it's [`ConnectAddr`]. One
//! side will call [`connect`], the other [`connected`]. If successful both
//! applications will now get a [`Participant`].
//!
//! This [`Participant`] represents the connection between those 2 applications.
//! over the respective [`ConnectAddr`] and with it the chosen network
//! protocol. However messages can't be send directly via [`Participants`],
//! instead you must open a [`Stream`] on it. Like above, one side has to call
//! [`open`], the other [`opened`]. [`Streams`] can have a different priority
//! and [`Promises`].
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
//! use std::sync::Arc;
//! use tokio::{join, runtime::Runtime, time::sleep};
//! use veloren_network::{ConnectAddr, ListenAddr, Network, Pid, Promises};
//!
//! // Client
//! async fn client(runtime: &Runtime) -> Result<(), Box<dyn std::error::Error>> {
//!     sleep(std::time::Duration::from_secs(1)).await; // `connect` MUST be after `listen`
//!     let client_network = Network::new(Pid::new(), runtime);
//!     let server = client_network
//!         .connect(ConnectAddr::Tcp("127.0.0.1:12345".parse().unwrap()))
//!         .await?;
//!     let mut stream = server
//!         .open(4, Promises::ORDERED | Promises::CONSISTENCY, 0)
//!         .await?;
//!     stream.send("Hello World")?;
//!     Ok(())
//! }
//!
//! // Server
//! async fn server(runtime: &Runtime) -> Result<(), Box<dyn std::error::Error>> {
//!     let mut server_network = Network::new(Pid::new(), runtime);
//!     server_network
//!         .listen(ListenAddr::Tcp("127.0.0.1:12345".parse().unwrap()))
//!         .await?;
//!     let mut client = server_network.connected().await?;
//!     let mut stream = client.opened().await?;
//!     let msg: String = stream.recv().await?;
//!     println!("Got message: {}", msg);
//!     assert_eq!(msg, "Hello World");
//!     Ok(())
//! }
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let runtime = Runtime::new().unwrap();
//!     runtime.block_on(async {
//!         let (result_c, result_s) = join!(client(&runtime), server(&runtime),);
//!         result_c?;
//!         result_s?;
//!         Ok(())
//!     })
//! }
//! ```
//!
//! [`Network`]: crate::api::Network
//! [`Networks`]: crate::api::Network
//! [`connect`]: crate::api::Network::connect
//! [`connected`]: crate::api::Network::connected
//! [`Participant`]: crate::api::Participant
//! [`Participants`]: crate::api::Participant
//! [`open`]: crate::api::Participant::open
//! [`opened`]: crate::api::Participant::opened
//! [`Stream`]: crate::api::Stream
//! [`Streams`]: crate::api::Stream
//! [`send`]: crate::api::Stream::send
//! [`recv`]: crate::api::Stream::recv
//! [`Pid`]: network_protocol::Pid
//! [`ListenAddr`]: crate::api::ListenAddr
//! [`ConnectAddr`]: crate::api::ConnectAddr
//! [`Promises`]: network_protocol::Promises

mod api;
mod channel;
mod message;
mod metrics;
mod participant;
mod scheduler;
mod util;

pub use api::{
    ConnectAddr, ListenAddr, Network, NetworkConnectError, NetworkError, Participant,
    ParticipantError, ParticipantEvent, Stream, StreamError, StreamParams,
};
pub use message::Message;
pub use network_protocol::{InitProtocolError, Pid, Promises};
