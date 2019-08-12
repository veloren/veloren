use specs::{Component, FlaggedStorage, NullStorage};
use specs_idvs::IDVStorage;

const MAX_ALIAS_LEN: usize = 32;

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

    pub fn is_valid(&self) -> bool {
        self.alias.chars().all(|c| c.is_alphanumeric() || c == '_')
            && self.alias.len() <= MAX_ALIAS_LEN
    }
}

impl Component for Player {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}
