use vek::Vec2;

pub struct KeyState {
    pub right: bool,
    pub left: bool,
    pub up: bool,
    pub down: bool,
    pub jump: bool,
}

impl KeyState {
    pub fn new() -> KeyState {
        KeyState {
            right: false,
            left: false,
            up: false,
            down: false,
            jump: false,
        }
    }

    pub fn dir_vec(&self) -> Vec2<f32> {
        let dir = Vec2::<f32>::new(
            if self.right { 1.0 } else { 0.0 } + if self.left { -1.0 } else { 0.0 },
            if self.up { 1.0 } else { 0.0 } + if self.down { -1.0 } else { 0.0 },
        );

        if dir.magnitude_squared() == 0.0 {
            dir
        } else {
            dir.normalized()
        }
    }
}
