pub mod packet;
pub mod conn;
pub mod message;
pub mod manager;

use std::io;

// Reexports
pub use self::message::{Message, ServerMessage, ClientMessage};
pub use self::conn::Conn;
pub use self::manager::Manager;

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
