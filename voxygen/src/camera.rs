use euler::{Vec3, Mat4};

pub struct Camera {
    focus: Vec3,
    ori: Vec2,
    zoom: f32,
}

impl Camera {
    pub fn new() -> Camera {
        Camera {
            focus: vec3!(0, 0, 0),
            ori: vec2!(0, 0),
            zoom: 5.0,
        }
    }

    pub fn get_mat(&self) -> Mat4 {
        unimplemented!();
    }
}
