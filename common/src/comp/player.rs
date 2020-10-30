use authc::Uuid;
use serde::{Deserialize, Serialize};
use specs::{Component, FlaggedStorage, NullStorage};
use specs_idvs::IdvStorage;

const MAX_ALIAS_LEN: usize = 32;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub alias: String,
    uuid: Uuid,
}

impl Player {
    pub fn new(alias: String, uuid: Uuid) -> Self { Self { alias, uuid } }

    pub fn is_valid(&self) -> bool { Self::alias_is_valid(&self.alias) }

    pub fn alias_is_valid(alias: &str) -> bool {
        // TODO: Expose auth name validation and use it here.
        // See https://gitlab.com/veloren/auth/-/blob/master/server/src/web.rs#L20
        alias
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
            && alias.len() <= MAX_ALIAS_LEN
    }

    /// Not to be confused with uid
    pub fn uuid(&self) -> Uuid { self.uuid }
}

impl Component for Player {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Respawn;
impl Component for Respawn {
    type Storage = NullStorage<Self>;
}
