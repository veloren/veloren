use vek::*;

pub struct Input {
    // TODO: Use this type to manage client input
    pub move_dir: Vec2<f32>,
}

impl Default for Input {
    fn default() -> Self {
        Input {
            move_dir: Vec2::zero(),
        }
    }
}
