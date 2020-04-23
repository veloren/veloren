use vek::*;

#[derive(Debug)]
pub struct PathData {
    pub offset: Vec2<f32>, /* Offset from centre of chunk: must not be more than half chunk
                            * width in any direction */
    pub neighbors: u8, // One bit for each neighbor
}

impl PathData {
    pub fn is_path(&self) -> bool { self.neighbors != 0 }
}

impl Default for PathData {
    fn default() -> Self {
        Self {
            offset: Vec2::zero(),
            neighbors: 0,
        }
    }
}
