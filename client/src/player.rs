use coord::prelude::*;
use common::Uid;

pub struct Player {
	pub alias: String,
	pub entity_uid: Option<Uid>,
	pub dir_vec: Vec3f,
}

impl Player {
	pub fn new(alias: String) -> Player {
		Player {
			alias,
			entity_uid: None,
			dir_vec: vec3!(0.0, 0.0, 0.0),
		}
	}
}
