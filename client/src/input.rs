use vek::*;

pub enum InputEvent {
    Jump,
}

pub struct Input {
    pub move_dir: Vec2<f32>,
    pub jumping: bool,
    pub events: Vec<InputEvent>,
}

impl Default for Input {
    fn default() -> Self {
        Input {
            move_dir: Vec2::zero(),
            jumping: false,
            events: Vec::new(),
        }
    }
}
