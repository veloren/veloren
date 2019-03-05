use crate::comp::{
    Uid,
    phys,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ServerMsg {
    Shutdown,
    Ping,
    Pong,
    Chat(String),
    SetPlayerEntity(Uid),
    EntityPhysics {
        uid: Uid,
        pos: phys::Pos,
        vel: phys::Vel,
        dir: phys::Dir,
    },
    EntityDeleted(Uid),
}
