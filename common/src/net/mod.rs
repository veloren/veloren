mod packet;
pub mod connection;
pub mod message;
mod tcp;
#[cfg(test)]
mod tests;

// Reexports
pub use self::message::{Message, ServerMessage, ClientMessage, Error};
pub use self::connection::Connection;
pub use self::connection::Callback;

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum ClientMode {
    Headless,
    Character,
}
