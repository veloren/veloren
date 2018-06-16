mod packet;
pub mod connection;
pub mod message;
mod tcp;

use std::io;

// Reexports
pub use self::message::{Message, ServerMessage, ClientMessage};
pub use self::connection::Connection;

#[derive(Debug)]
pub enum Error {
    NetworkErr(io::Error),
    CannotSerialize,
    CannotDeserialize,
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::NetworkErr(e)
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum ClientMode {
    Headless,
    Character,
}
