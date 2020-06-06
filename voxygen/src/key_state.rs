use vek::Vec2;

pub struct KeyState {
    pub right: bool,
    pub left: bool,
    pub up: bool,
    pub down: bool,
    pub climb_up: bool,
    pub climb_down: bool,
    pub toggle_wield: bool,
    pub toggle_sit: bool,
    pub toggle_dance: bool,
    pub auto_walk: bool,
    pub swap_loadout: bool,
    pub respawn: bool,
    pub analog_matrix: Vec2<f32>,
}

impl KeyState {
    pub fn new() -> KeyState {
        KeyState {
            right: false,
            left: false,
            up: false,
            down: false,
            climb_up: false,
            climb_down: false,
            toggle_wield: false,
            toggle_sit: false,
            toggle_dance: false,
            auto_walk: false,
            swap_loadout: false,
            respawn: false,
            analog_matrix: Vec2::zero(),
        }
    }

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
