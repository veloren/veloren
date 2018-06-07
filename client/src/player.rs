use nalgebra::Vector3;
use common::Uid;

pub struct Player {
	pub alias: String,
	pub entity_uid: Option<Uid>,
	pub dir_vec: Vector3<f32>,
}

impl Player {
	pub fn new(alias: String) -> Player {
		Player {
			alias,
			entity_uid: None,
			dir_vec: Vector3::new(0.0, 0.0, 0.0),
		}
	}
}
