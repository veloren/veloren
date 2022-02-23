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
        ServerRegisterAnswer,
    },
    world_msg::WorldMapMsg,
};
use common::{character::CharacterId, uid::Uid};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PresenceKind {
    Spectator,
    Character(CharacterId),
    Possessor(
        /// The original character Id before possession began. Used to revert
        /// back to original `Character` presence if original entity is
        /// re-possessed.
        CharacterId,
        /// The original entity Uid.
        Uid,
    ),
}

impl PresenceKind {
    /// Check if the presence represents a control of a character, and thus
    /// certain in-game messages from the client such as control inputs
    /// should be handled.
    pub fn controlling_char(&self) -> bool {
        matches!(self, Self::Character(_) | Self::Possessor(_, _))
    }
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
