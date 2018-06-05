use ClientMode;
use nalgebra::Vector3;

pub struct Player {
    entity_uid: Option<u64>,
    mode: ClientMode,
    alias: String,
}

impl Player {
    pub fn new(entity_uid: Option<u64>, mode: ClientMode, alias: &str) -> Player {
        Player {
            entity_uid,
            mode,
            alias: alias.to_string(),
        }
    }

    pub fn alias<'a>(&'a self) -> &str {
        &self.alias
    }

    pub fn entity_uid(&self) -> Option<u64> {
        self.entity_uid
    }
}
