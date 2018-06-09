use common::Uid;
use nalgebra::Vector3;

pub struct Player {
    session_id: u32,
    uid: Uid,
    entity_id: Option<Uid>,
    alias: String,
}

impl Player {
    pub fn new(session_id: u32, uid: Uid, entity_id: Option<Uid>, alias: &str) -> Player {
        Player {
            session_id,
            uid,
            entity_id,
            alias: alias.to_string(),
        }
    }

    pub fn alias<'a>(&'a self) -> &str {
        &self.alias
    }

    pub fn get_uid(&self) -> Uid { self.uid }

    pub fn get_session_id(&self) -> u32 { self.session_id }

    pub fn get_entity_id(&self) -> Option<Uid> { self.entity_id }
}
