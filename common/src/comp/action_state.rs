use specs::{Component, FlaggedStorage, NullStorage, VecStorage};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ActionState {
    pub moving: bool,
    pub on_ground: bool,
    pub attacking: bool,
    pub rolling: bool,
    pub gliding: bool,
}

impl Default for ActionState {
    fn default() -> Self {
        Self {
            moving: false,
            on_ground: false,
            attacking: false,
            rolling: false,
            gliding: false,
        }
    }
}

impl Component for ActionState {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}
