use vek::Vec2;

pub struct KeyState {
    pub right: bool,
    pub left: bool,
    pub up: bool,
    pub down: bool,
    pub climb_up: bool,
    pub climb_down: bool,
    pub swim_up: bool,
    pub swim_down: bool,
    pub fly: bool,
    pub auto_walk: bool,
    pub trade: bool,
    pub analog_matrix: Vec2<f32>,
}

impl Default for KeyState {
    fn default() -> Self {
        Self {
            right: false,
            left: false,
            up: false,
            down: false,
            climb_up: false,
            climb_down: false,
            swim_up: false,
            swim_down: false,
            fly: false,
            auto_walk: false,
            trade: false,
            analog_matrix: Vec2::zero(),
        }
    }
}

impl KeyState {
    pub fn dir_vec(&self) -> Vec2<f32> {
        let dir = if self.analog_matrix == Vec2::zero() {
            Vec2::<f32>::new(
                if self.right { 1.0 } else { 0.0 } + if self.left { -1.0 } else { 0.0 },
                if self.up || self.auto_walk { 1.0 } else { 0.0 }
                    + if self.down { -1.0 } else { 0.0 },
            )
        } else {
            self.analog_matrix
        };

        if dir.magnitude_squared() <= 1.0 {
            dir
        } else {
            dir.normalized()
        }
    }

    pub fn climb(&self) -> Option<common::comp::Climb> {
        use common::comp::Climb;
        match (self.climb_up, self.climb_down) {
            (true, false) => Some(Climb::Up),
            (false, true) => Some(Climb::Down),
            (true, true) => Some(Climb::Hold),
            (false, false) => None,
        }
    }
}
