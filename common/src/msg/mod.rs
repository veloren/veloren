pub mod ecs_packet;
pub mod server;
pub mod client;

// Reexports
pub use self::server::{ServerMsg, RequestStateError};
pub use self::client::ClientMsg;
pub use self::ecs_packet::EcsPacket;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClientState {
    Disconnected,
    Spectator,
    Character,
}
