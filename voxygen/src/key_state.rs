use vek::Vec2;

pub const GIVE_UP_HOLD_TIME: f32 = 2.0;

pub struct KeyState {
    pub right: bool,
    pub left: bool,
    pub up: bool,
    pub down: bool,
    pub swim_up: bool,
    pub swim_down: bool,
    pub fly: bool,
    pub auto_walk: bool,
    pub speed_mul: f32,
    pub trade: bool,
    pub give_up: Option<f32>,
    pub analog_matrix: Vec2<f32>,
}

impl Default for KeyState {
    fn default() -> Self {
        Self {
            right: false,
            left: false,
            up: false,
            down: false,
            swim_up: false,
            swim_down: false,
            fly: false,
            auto_walk: false,
            speed_mul: 1.0,
            trade: false,
            give_up: None,
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
            .try_normalized()
            .unwrap_or_default()
        } else {
            self.analog_matrix
        } * self.speed_mul;

        if dir.magnitude_squared() <= 1.0 {
            dir
        } else {
            dir.normalized()
        }
    }
}
