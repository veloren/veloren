#![feature(trait_alias, try_trait, async_closure)]

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
