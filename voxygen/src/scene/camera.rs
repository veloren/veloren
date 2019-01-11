// Standard
use std::f32::consts::PI;

// Library
use vek::*;

const NEAR_PLANE: f32 = 0.1;
const FAR_PLANE: f32 = 10000.0;

pub struct Camera {
    focus: Vec3<f32>,
    ori: Vec3<f32>,
    dist: f32,
    fov: f32,
    aspect: f32,
}

impl Camera {
    /// Create a new `Camera` with default parameters.
    pub fn new() -> Self {
        Self {
            focus: Vec3::zero(),
            ori: Vec3::zero(),
            dist: 10.0,
            fov: 1.3,
            aspect: 1.618,
        }
    }

    /// Compute the transformation matrices (view matrix and projection matrix) for the camera.
    pub fn compute_matrices(&self) -> (Mat4<f32>, Mat4<f32>) {
        let view = Mat4::<f32>::identity()
            * Mat4::translation_3d(-Vec3::unit_z() * self.dist)
            * Mat4::rotation_z(self.ori.z)
            * Mat4::rotation_x(self.ori.y)
            * Mat4::rotation_y(self.ori.x)
            * Mat4::rotation_3d(PI / 2.0, -Vec4::unit_x())
            * Mat4::translation_3d(-self.focus);

        let proj = Mat4::perspective_rh_no(
            self.fov,
            self.aspect,
            NEAR_PLANE,
            FAR_PLANE,
        );

        (view, proj)
    }
}
