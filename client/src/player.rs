use coord::prelude::*;
use common::Uid;

pub struct Player {
	pub alias: String,
	pub entity_uid: Option<Uid>,
}

impl Player {
	pub fn new(alias: String) -> Player {
		Player {
			alias,
			entity_uid: None,
		}
	}
}
