use crate::comp::phys;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ServerMsg {
    Shutdown,
    Chat(String),
    EntityPhysics {
        uid: u64,
        pos: phys::Pos,
        vel: phys::Vel,
        dir: phys::Dir,
    },
}
