use ClientMode;
use nalgebra::Vector3;

pub struct Player {
    mode: ClientMode,
    position: Vector3<f32>, // Should be moved into some sort of Entity struct.
    alias: String,
}

impl Player {
    pub fn new(mode: ClientMode, alias: &str, x: f32, y: f32, z: f32) -> Player {
        Player {
            mode,
            alias: alias.to_string(),
            position: Vector3::new(x, y, z),
        }
    }

    pub fn alias<'a>(&'a self) -> &str {
        &self.alias
    }

    pub fn position<'a>(&'a self) -> &Vector3<f32> {
        &self.position
    }

    pub fn move_by(&mut self, dx: f32, dy: f32, dz: f32) {
        self.position += Vector3::new(dx, dy, dz);
    }
}
