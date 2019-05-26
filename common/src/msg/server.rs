use super::{ClientState, EcsCompPacket, EcsResPacket};
use crate::{comp, terrain::TerrainChunk};
use vek::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RequestStateError {
    Denied,
    Already,
    Impossible,
    WrongMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMsg {
    InitialSync {
        ecs_state: sphynx::StatePackage<EcsCompPacket, EcsResPacket>,
        entity_uid: u64,
    },
    StateAnswer(Result<ClientState, (RequestStateError, ClientState)>),
    ForceState(ClientState),
    Ping,
    Pong,
    Chat(String),
    SetPlayerEntity(u64),
    EcsSync(sphynx::SyncPackage<EcsCompPacket, EcsResPacket>),
    EntityPhysics {
        entity: u64,
        pos: comp::phys::Pos,
        vel: comp::phys::Vel,
        dir: comp::phys::Dir,
    },
    EntityAnimation {
        entity: u64,
        animation_info: comp::AnimationInfo,
    },
    TerrainChunkUpdate {
        key: Vec2<i32>,
        chunk: Box<TerrainChunk>,
    },
    Disconnect,
    Shutdown,
}
