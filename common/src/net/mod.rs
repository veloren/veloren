mod packet;
pub mod connection;
pub mod message;
mod protocol;
mod tcp;
mod udp;
pub mod udpmgr;
#[cfg(test)]
mod tests;

// Reexports
pub use self::message::{Message, ServerMessage, ClientMessage, Error};
pub use self::connection::Connection;
pub use self::connection::Callback;
pub use self::udpmgr::UdpMgr;

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum ClientMode {
    Headless,
    Character,
}
