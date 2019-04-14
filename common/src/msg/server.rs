use vek::*;
use crate::terrain::TerrainChunk;
use super::EcsPacket;

#[derive(Clone, Serialize, Deserialize)]
pub enum ServerMsg {
    Handshake {
        ecs_state: sphynx::StatePackage<EcsPacket>,
        player_entity: u64,
    },
    Shutdown,
    Ping,
    Pong,
    Chat(String),
    SetPlayerEntity(u64),
    EcsSync(sphynx::SyncPackage<EcsPacket>),
    TerrainChunkUpdate {
        key: Vec3<i32>,
        chunk: Box<TerrainChunk>,
    },
}
