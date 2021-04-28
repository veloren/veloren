pub mod client;
pub mod compression;
pub mod ecs_packet;
pub mod server;
pub mod world_msg;

// Reexports
pub use self::{
    client::{ClientGeneral, ClientMsg, ClientRegister, ClientType},
    compression::{
        CompressedData, GridLtrPacking, PackingFormula, QuadPngEncoding, TriPngEncoding,
        VoxelImageEncoding, WidePacking, WireChonk,
    },
    ecs_packet::EcsCompPacket,
    server::{
        CharacterInfo, DisconnectReason, InviteAnswer, Notification, PlayerInfo, PlayerListUpdate,
        RegisterError, SerializedTerrainChunk, ServerGeneral, ServerInfo, ServerInit, ServerMsg,
        ServerRegisterAnswer, TERRAIN_LOW_BANDWIDTH,
    },
    world_msg::WorldMapMsg,
};
use common::character::CharacterId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PresenceKind {
    Spectator,
    Character(CharacterId),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PingMsg {
    Ping,
    Pong,
}

pub const MAX_BYTES_CHAT_MSG: usize = 256;

pub enum ChatMsgValidationError {
    TooLong,
}

pub fn validate_chat_msg(msg: &str) -> Result<(), ChatMsgValidationError> {
    // TODO: Consider using grapheme cluster count instead of size in bytes
    if msg.len() <= MAX_BYTES_CHAT_MSG {
        Ok(())
    } else {
        Err(ChatMsgValidationError::TooLong)
    }
}
