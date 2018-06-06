use nalgebra::Vector2;

pub struct KeyState {
    pub right: bool,
    pub left: bool,
    pub up: bool,
    pub down: bool,
    pub fly: bool,
    pub fall: bool,
}

impl KeyState {
    pub fn new() -> KeyState {
        KeyState {
            right: false,
            left: false,
            up: false,
            down: false,
            fly: false,
            fall: false,
        }
    }

    pub fn dir_vec(&self) -> Vector2<f32> {
        Vector2::<f32>::new(
            if self.right { 1.0 } else { 0.0 } + if self.left { -1.0 } else { 0.0 },
            if self.up { 1.0 } else { 0.0 } + if self.down { -1.0 } else { 0.0 },
        )
    }

    pub fn fly_vec(&self) -> f32 {
        (if self.fly { 1.0 } else { 0.0 }) + (if self.fall { -1.0 } else { 0.0 })
    }
}
