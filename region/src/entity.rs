use nalgebra::Vector3;

pub struct Entity {
    pos: Vector3<f32>,
}

impl Entity {
    pub fn new() -> Entity {
        Entity {
            pos: Vector3::new(0.0, 0.0, 0.0),
        }
    }

    pub fn pos(&self) -> &Vector3<f32> {
        &self.pos
    }
}
