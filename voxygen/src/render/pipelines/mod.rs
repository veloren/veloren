pub mod figure;
pub mod skybox;

// Library
use gfx::{
    self,
    // Macros
    gfx_defines,
    gfx_constant_struct_meta,
    gfx_impl_struct_meta,
};
use vek::*;

// Local
use super::util::arr_to_mat;

gfx_defines! {
    constant Globals {
        view_mat: [[f32; 4]; 4] = "view_mat",
        proj_mat: [[f32; 4]; 4] = "proj_mat",
        cam_pos: [f32; 4] = "cam_pos",
        focus_pos: [f32; 4] = "focus_pos",
        // TODO: Fix whatever alignment issue requires these uniforms to be aligned
        view_distance: [f32; 4] = "view_distance",
        time_of_day: [f32; 4] = "time_of_day", // TODO: Make this f64
        time: [f32; 4] = "time",
    }
}

impl Globals {
    /// Create global consts with default values.
    pub fn default() -> Self {
        Self {
            view_mat: arr_to_mat(Mat4::identity().into_col_array()),
            proj_mat: arr_to_mat(Mat4::identity().into_col_array()),
            cam_pos: [0.0; 4],
            focus_pos: [0.0; 4],
            view_distance: [0.0; 4],
            time_of_day: [0.0; 4],
            time: [0.0; 4],
        }
    }

    /// Create global consts from the provided parameters.
    pub fn new(
        view_mat: Mat4<f32>,
        proj_mat: Mat4<f32>,
        cam_pos: Vec3<f32>,
        focus_pos: Vec3<f32>,
        view_distance: f32,
        time_of_day: f64,
        time: f32,
    ) -> Self {
        Self {
            view_mat: arr_to_mat(view_mat.into_col_array()),
            proj_mat: arr_to_mat(proj_mat.into_col_array()),
            cam_pos: Vec4::from(cam_pos).into_array(),
            focus_pos: Vec4::from(focus_pos).into_array(),
            view_distance: [view_distance; 4],
            time_of_day: [time_of_day as f32; 4],
            time: [time; 4],
        }
    }
}
