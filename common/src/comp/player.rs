use specs::{Component, FlaggedStorage, VecStorage};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub alias: String,
}

impl Player {
    pub fn new(alias: String) -> Self {
        Self { alias }
    }
}

impl Component for Player {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}
