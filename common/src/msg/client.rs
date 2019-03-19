use crate::comp::{
    Uid,
    phys,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ClientMsg {
    Ping,
    Pong,
    Chat(String),
    PlayerPhysics {
        pos: phys::Pos,
        vel: phys::Vel,
        dir: phys::Dir,
    },
    Disconnect,
}
