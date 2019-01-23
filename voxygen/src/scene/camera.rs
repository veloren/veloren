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
            focus: Vec3::unit_z() * 10.0,
            ori: Vec3::zero(),
            dist: 150.0,
            fov: 1.3,
            aspect: 1.618,
        }
    }

    /// Compute the transformation matrices (view matrix and projection matrix) and position of the
    /// camera.
    pub fn compute_dependents(&self) -> (Mat4<f32>, Mat4<f32>, Vec3<f32>) {
        let view_mat = Mat4::<f32>::identity()
            * Mat4::translation_3d(-Vec3::unit_z() * self.dist)
            * Mat4::rotation_z(self.ori.z)
            * Mat4::rotation_x(self.ori.y)
            * Mat4::rotation_y(self.ori.x)
            * Mat4::rotation_3d(PI / 2.0, -Vec4::unit_x())
            * Mat4::translation_3d(-self.focus);

        let proj_mat = Mat4::perspective_rh_no(
            self.fov,
            self.aspect,
            NEAR_PLANE,
            FAR_PLANE,
        );

        // TODO: Make this more efficient
        let cam_pos = Vec3::from(view_mat.inverted() * Vec4::unit_w());

        (view_mat, proj_mat, cam_pos)
    }

    /// Rotate the camera about its focus by the given delta, limiting the input accordingly.
    pub fn rotate_by(&mut self, delta: Vec3<f32>) {
        self.ori += delta;
        // Clamp camera pitch to the vertical limits
        self.ori.y = self.ori.y
            .min(PI / 2.0)
            .max(-PI / 2.0);
    }

    /// Get the focus position of the camera.
    pub fn get_focus_pos(&self) -> Vec3<f32> { self.focus }
    /// Set the focus position of the camera.
    pub fn set_focus_pos(&mut self, focus: Vec3<f32>) { self.focus = focus; }

    /// Get the aspect ratio of the camera.
    pub fn get_aspect_ratio(&self) -> f32 { self.aspect }
    /// Set the aspect ratio of the camera.
    pub fn set_aspect_ratio(&mut self, aspect: f32) { self.aspect = aspect; }
}
