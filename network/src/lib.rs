#![feature(trait_alias)]
mod api;
mod async_serde;
mod channel;
mod frames;
mod message;
mod metrics;
mod mpsc;
mod participant;
mod prios;
mod scheduler;
mod tcp;
mod types;
mod udp;

pub use api::{Address, Network};
pub use scheduler::Scheduler;
pub use types::{
    Pid, Promises, PROMISES_COMPRESSED, PROMISES_CONSISTENCY, PROMISES_ENCRYPTED,
    PROMISES_GUARANTEED_DELIVERY, PROMISES_NONE, PROMISES_ORDERED,
};

/*
pub use api::{
    Address, Network, NetworkError, Participant, ParticipantError, Promise, Stream, StreamError,
};
*/
