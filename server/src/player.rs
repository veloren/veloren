// Library
use coord::prelude::*;

// Project
use common::Uid;

pub struct Player {
    session_id: u32,
    uid: Uid,
    entity_uid: Option<Uid>,
    alias: String,
}

impl Player {
    pub fn new(session_id: u32, uid: Uid, entity_uid: Option<Uid>, alias: &str) -> Player {
        Player {
            session_id,
            uid,
            entity_uid,
            alias: alias.to_string(),
        }
    }

    pub fn alias<'a>(&'a self) -> &str {
        &self.alias
    }

    pub fn get_uid(&self) -> Uid { self.uid }

    pub fn get_session_id(&self) -> u32 { self.session_id }

    pub fn get_entity_uid(&self) -> Option<Uid> { self.entity_uid }
}
