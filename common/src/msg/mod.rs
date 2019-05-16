pub mod client;
pub mod ecs_packet;
pub mod server;

// Reexports
pub use self::client::ClientMsg;
pub use self::ecs_packet::{EcsCompPacket, EcsResPacket};
pub use self::server::{RequestStateError, ServerMsg};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ClientState {
    Connected,
    Registered,
    Spectator,
    Character,
}
