use client::Client;
use common::vol::{ReadVol, Vox};
use std::f32::consts::PI;
use treeculler::Frustum;
use vek::*;

const NEAR_PLANE: f32 = 0.5;
const FAR_PLANE: f32 = 100000.0;

const FIRST_PERSON_INTERP_TIME: f32 = 0.05;
const THIRD_PERSON_INTERP_TIME: f32 = 0.1;
pub const MIN_ZOOM: f32 = 0.1;

// Possible TODO: Add more modes
#[derive(PartialEq, Clone, Copy, Eq, Hash)]
pub enum CameraMode {
    FirstPerson,
    ThirdPerson,
}

impl Default for CameraMode {
    fn default() -> Self { Self::ThirdPerson }
}

pub struct Camera {
    tgt_focus: Vec3<f32>,
    focus: Vec3<f32>,
    ori: Vec3<f32>,
    tgt_dist: f32,
    dist: f32,
    fov: f32,
    aspect: f32,
    mode: CameraMode,

    last_time: Option<f64>,
}

impl Camera {
    /// Create a new `Camera` with default parameters.
    pub fn new(aspect: f32, mode: CameraMode) -> Self {
        Self {
            tgt_focus: Vec3::unit_z() * 10.0,
            focus: Vec3::unit_z() * 10.0,
            ori: Vec3::zero(),
            tgt_dist: 10.0,
            dist: 10.0,
            fov: 1.1,
            aspect,
            mode,

            last_time: None,
        }
    }

    /// Compute the transformation matrices (view matrix and projection matrix)
    /// and position of the camera.
    pub fn compute_dependents(&self, client: &Client) -> (Mat4<f32>, Mat4<f32>, Vec3<f32>) {
        let dist = {
            let (start, end) = (
                self.focus
                    + (Vec3::new(
                        -f32::sin(self.ori.x) * f32::cos(self.ori.y),
                        -f32::cos(self.ori.x) * f32::cos(self.ori.y),
                        f32::sin(self.ori.y),
                    ) * self.dist),
                self.focus,
            );

            match client
                .state()
                .terrain()
                .ray(start, end)
                .ignore_error()
                .max_iter(500)
                .until(|b| b.is_empty())
                .cast()
            {
                (d, Ok(Some(_))) => f32::min(self.dist - d - 0.03, self.dist),
                (_, Ok(None)) => self.dist,
                (_, Err(_)) => self.dist,
            }
            .max(0.0)
        };

        let view_mat = Mat4::<f32>::identity()
            * Mat4::translation_3d(-Vec3::unit_z() * dist)
            * Mat4::rotation_z(self.ori.z)
            * Mat4::rotation_x(self.ori.y)
            * Mat4::rotation_y(self.ori.x)
            * Mat4::rotation_3d(PI / 2.0, -Vec4::unit_x())
            * Mat4::translation_3d(-self.focus);

        let proj_mat = Mat4::perspective_rh_no(self.fov, self.aspect, NEAR_PLANE, FAR_PLANE);

        // TODO: Make this more efficient.
        let cam_pos = Vec3::from(view_mat.inverted() * Vec4::unit_w());

        (view_mat, proj_mat, cam_pos)
    }

    pub fn frustum(&self, client: &Client) -> Frustum<f32> {
        let (view_mat, proj_mat, _) = self.compute_dependents(client);

        Frustum::from_modelview_projection((proj_mat * view_mat).into_col_arrays())
    }

    /// Rotate the camera about its focus by the given delta, limiting the input
    /// accordingly.
    pub fn rotate_by(&mut self, delta: Vec3<f32>) {
        // Wrap camera yaw
        self.ori.x = (self.ori.x + delta.x) % (2.0 * PI);
        // Clamp camera pitch to the vertical limits
        self.ori.y = (self.ori.y + delta.y).min(PI / 2.0).max(-PI / 2.0);
        // Wrap camera roll
        self.ori.z = (self.ori.z + delta.z) % (2.0 * PI);
    }

    /// Set the orientation of the camera about its focus.
    pub fn set_orientation(&mut self, orientation: Vec3<f32>) {
        // Wrap camera yaw
        self.ori.x = orientation.x % (2.0 * PI);
        // Clamp camera pitch to the vertical limits
        self.ori.y = orientation.y.min(PI / 2.0).max(-PI / 2.0);
        // Wrap camera roll
        self.ori.z = orientation.z % (2.0 * PI);
    }

    /// Zoom the camera by the given delta, limiting the input accordingly.
    pub fn zoom_by(&mut self, delta: f32) {
        match self.mode {
            CameraMode::ThirdPerson => {
                // Clamp camera dist to the 2 <= x <= infinity range
                self.tgt_dist = (self.tgt_dist + delta).max(2.0);
            },
            CameraMode::FirstPerson => {},
        };
    }

    /// Zoom with the ability to switch between first and third-person mode.
    pub fn zoom_switch(&mut self, delta: f32) {
        if delta > 0_f32 || self.mode != CameraMode::FirstPerson {
            let t = self.tgt_dist + delta;
            match self.mode {
                CameraMode::ThirdPerson => {
                    if t < 1_f32 {
                        self.set_mode(CameraMode::FirstPerson);
                    } else {
                        self.tgt_dist = t;
                    }
                },
                CameraMode::FirstPerson => {
                    self.set_mode(CameraMode::ThirdPerson);
                    self.tgt_dist = 1_f32;
                },
            }
        }
    }

    /// Get the distance of the camera from the target
    pub fn get_distance(&self) -> f32 { self.tgt_dist }

    /// Set the distance of the camera from the target (i.e., zoom).
    pub fn set_distance(&mut self, dist: f32) { self.tgt_dist = dist; }

    pub fn update(&mut self, time: f64) {
        // This is horribly frame time dependent, but so is most of the game
        let delta = self.last_time.replace(time).map_or(0.0, |t| time - t);
        if (self.dist - self.tgt_dist).abs() > 0.01 {
            self.dist = f32::lerp(
                self.dist,
                self.tgt_dist,
                (delta as f32) / self.interp_time(),
            );
        }

        if (self.focus - self.tgt_focus).magnitude() > 0.01 {
            self.focus = Vec3::lerp(
                self.focus,
                self.tgt_focus,
                (delta as f32) / self.interp_time(),
            );
        }
    }

    pub fn interp_time(&self) -> f32 {
        match self.mode {
            CameraMode::FirstPerson => FIRST_PERSON_INTERP_TIME,
            CameraMode::ThirdPerson => THIRD_PERSON_INTERP_TIME,
        }
    }

    /// Get the focus position of the camera.
    pub fn get_focus_pos(&self) -> Vec3<f32> { self.tgt_focus }

    /// Set the focus position of the camera.
    pub fn set_focus_pos(&mut self, focus: Vec3<f32>) { self.tgt_focus = focus; }

    /// Get the aspect ratio of the camera.
    pub fn get_aspect_ratio(&self) -> f32 { self.aspect }

    /// Set the aspect ratio of the camera.
    pub fn set_aspect_ratio(&mut self, aspect: f32) {
        self.aspect = if aspect.is_normal() { aspect } else { 1.0 };
    }

    /// Get the orientation of the camera.
    pub fn get_orientation(&self) -> Vec3<f32> { self.ori }

    /// Get the field of view of the camera in radians.
    pub fn get_fov(&self) -> f32 { self.fov }

    /// Set the field of view of the camera in radians.
    pub fn set_fov(&mut self, fov: f32) { self.fov = fov; }

    /// Set the FOV in degrees
    pub fn set_fov_deg(&mut self, fov: u16) {
        //Magic value comes from pi/180; no use recalculating.
        self.set_fov((fov as f32) * 0.01745329)
    }

    /// Set the mode of the camera.
    pub fn set_mode(&mut self, mode: CameraMode) {
        if self.mode != mode {
            self.mode = mode;
            match self.mode {
                CameraMode::ThirdPerson => {
                    self.zoom_by(5.0);
                },
                CameraMode::FirstPerson => {
                    self.set_distance(MIN_ZOOM);
                },
            }
        }
    }

    /// Get the mode of the camera
    pub fn get_mode(&self) -> CameraMode { self.mode }
}
