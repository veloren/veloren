use specs::{Component, FlaggedStorage, HashMapStorage};
use specs_idvs::IDVStorage;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActionState {
    pub moving: bool,
    pub on_ground: bool,
    pub attacking: bool,
    pub rolling: bool,
    pub gliding: bool,
    pub wielding: bool,
}

impl Default for ActionState {
    fn default() -> Self {
        Self {
            moving: false,
            on_ground: false,
            attacking: false,
            rolling: false,
            gliding: false,
            wielding: false,
        }
    }
}

impl Component for ActionState {
    type Storage = FlaggedStorage<Self, HashMapStorage<Self>>;
}
