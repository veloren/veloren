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
        CharacterInfo, ChatTypeContext, DisconnectReason, InviteAnswer, Notification, PlayerInfo,
        PlayerListUpdate, RegisterError, SerializedTerrainChunk, ServerGeneral, ServerInfo,
        ServerInit, ServerMsg, ServerRegisterAnswer,
    },
    world_msg::WorldMapMsg,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PingMsg {
    Ping,
    Pong,
}
