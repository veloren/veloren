pub mod client;
pub mod ecs_packet;
pub mod server;
pub mod world_msg;

// Reexports
pub use self::{
    client::{ClientGeneral, ClientMsg, ClientRegister, ClientType},
    ecs_packet::EcsCompPacket,
    server::{
        CharacterInfo, DisconnectReason, InviteAnswer, Notification, PlayerInfo, PlayerListUpdate,
        RegisterError, ServerGeneral, ServerInfo, ServerInit, ServerMsg, ServerRegisterAnswer,
    },
    world_msg::WorldMapMsg,
};
use common::character::CharacterId;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use tracing::trace;

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

/// Wrapper for compressed, serialized data (for stuff that doesn't use the
/// default lz4 compression)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompressedData<T> {
    pub data: Vec<u8>,
    compressed: bool,
    _phantom: PhantomData<T>,
}

impl<T: Serialize + for<'a> Deserialize<'a>> CompressedData<T> {
    pub fn compress(t: &T, level: u32) -> Self {
        use flate2::{write::DeflateEncoder, Compression};
        use std::io::Write;
        let uncompressed = bincode::serialize(t)
            .expect("bincode serialization can only fail if a byte limit is set");

        if uncompressed.len() >= 32 {
            const EXPECT_MSG: &str =
                "compression only fails for fallible Read/Write impls (which Vec<u8> is not)";

            let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(level));
            encoder.write_all(&*uncompressed).expect(EXPECT_MSG);
            let compressed = encoder.finish().expect(EXPECT_MSG);
            trace!(
                "compressed {}, uncompressed {}, ratio {}",
                compressed.len(),
                uncompressed.len(),
                compressed.len() as f32 / uncompressed.len() as f32
            );
            CompressedData {
                data: compressed,
                compressed: true,
                _phantom: PhantomData,
            }
        } else {
            CompressedData {
                data: uncompressed,
                compressed: false,
                _phantom: PhantomData,
            }
        }
    }

    pub fn decompress(&self) -> Option<T> {
        use std::io::Read;
        if self.compressed {
            let mut uncompressed = Vec::new();
            flate2::read::DeflateDecoder::new(&*self.data)
                .read_to_end(&mut uncompressed)
                .ok()?;
            bincode::deserialize(&*uncompressed).ok()
        } else {
            bincode::deserialize(&*self.data).ok()
        }
    }
}
