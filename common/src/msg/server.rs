use super::EcsPacket;

#[derive(Clone, Debug, Serialize, Deserialize)]
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
}
