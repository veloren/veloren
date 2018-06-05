use nalgebra::Vector3;

pub struct Entity {
    pos: Vector3<f32>,
}

impl Entity {
    pub fn new(pos: Vector3<f32>) -> Entity {
        Entity {
            pos,
        }
    }

    pub fn pos(&self) -> &Vector3<f32> {
        &self.pos
    }

    pub fn pos_mut<'a>(&'a mut self) -> &'a mut Vector3<f32> {
        &mut self.pos
    }
}
