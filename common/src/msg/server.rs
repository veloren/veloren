use vek::*;
use crate::{
    comp,
    terrain::TerrainChunk,
};
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
    EntityPhysics {
        entity: u64,
        pos: comp::phys::Pos,
        vel: comp::phys::Vel,
        dir: comp::phys::Dir,
    },
    EntityAnimation {
        entity: u64,
        animation: comp::Animation,
    },
    TerrainChunkUpdate {
        key: Vec3<i32>,
        chunk: Box<TerrainChunk>,
    },
}
