pub mod client;
pub mod ecs_packet;
pub mod server;
pub mod world_packet;

// Reexports
pub use self::{
    client::{
        ClientCharacterScreen, ClientGeneral, ClientInGame, ClientMsg, ClientRegister, ClientType,
    },
    ecs_packet::EcsCompPacket,
    server::{
        CharacterInfo, DisconnectReason, InviteAnswer, Notification, PlayerInfo, PlayerListUpdate,
        RegisterError, ServerCharacterScreen, ServerGeneral, ServerInGame, ServerInfo, ServerInit,
        ServerMsg, ServerRegisterAnswer,
    },
    world_packet::WorldMapMsg,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ClientIngame {
    Spectator,
    Character,
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
