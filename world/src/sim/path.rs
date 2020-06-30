use vek::*;

#[derive(Copy, Clone, Debug)]
pub struct Path {
    pub width: f32,
}

#[derive(Debug)]
pub struct PathData {
    pub offset: Vec2<f32>, /* Offset from centre of chunk: must not be more than half chunk
                            * width in any direction */
    pub path: Path,
    pub neighbors: u8, // One bit for each neighbor
}

impl PathData {
    pub fn is_path(&self) -> bool { self.neighbors != 0 }
}

impl Default for PathData {
    fn default() -> Self {
        Self {
            offset: Vec2::zero(),
            path: Path {
                width: 5.0,
            },
            neighbors: 0,
        }
    }
}

impl Path {
    /// Return the number of blocks of headspace required at the given path distance
    pub fn head_space(&self, dist: f32) -> i32 {
        (8 - (dist * 0.25).powf(6.0).round() as i32).max(1)
    }
}
