use specs::{Component, NullStorage, FlaggedStorage, VecStorage};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub alias: String,
    pub view_distance: Option<u32>,
}

impl Player {
    pub fn new(alias: String, view_distance: Option<u32>) -> Self {
        Self {
            alias,
            view_distance,
        }
    }
}

impl Component for Player {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}


#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Respawn;
impl Component for Respawn {
    type Storage = NullStorage<Self>;
}
