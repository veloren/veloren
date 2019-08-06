pub mod figure;
pub mod postprocess;
pub mod skybox;
pub mod terrain;
pub mod ui;
pub mod fluid;

use super::util::arr_to_mat;
use gfx::{
    self,
    gfx_constant_struct_meta,
    // Macros
    gfx_defines,
    gfx_impl_struct_meta,
};
use vek::*;

gfx_defines! {
    constant Globals {
        view_mat: [[f32; 4]; 4] = "view_mat",
        proj_mat: [[f32; 4]; 4] = "proj_mat",
        cam_pos: [f32; 4] = "cam_pos",
        focus_pos: [f32; 4] = "focus_pos",
        // TODO: Fix whatever alignment issue requires these uniforms to be aligned.
        view_distance: [f32; 4] = "view_distance",
        time_of_day: [f32; 4] = "time_of_day", // TODO: Make this f64.
        tick: [f32; 4] = "tick",
        screen_res: [f32; 4] = "screen_res",
        light_count: [u32; 4] = "light_count",
    }

    constant Light {
        pos: [f32; 4] = "light_pos",
        col: [f32; 4] = "light_col",
    }
}

impl Globals {
    /// Create global consts from the provided parameters.
    pub fn new(
        view_mat: Mat4<f32>,
        proj_mat: Mat4<f32>,
        cam_pos: Vec3<f32>,
        focus_pos: Vec3<f32>,
        view_distance: f32,
        time_of_day: f64,
        tick: f64,
        screen_res: Vec2<u16>,
        light_count: usize,
    ) -> Self {
        Self {
            view_mat: arr_to_mat(view_mat.into_col_array()),
            proj_mat: arr_to_mat(proj_mat.into_col_array()),
            cam_pos: Vec4::from(cam_pos).into_array(),
            focus_pos: Vec4::from(focus_pos).into_array(),
            view_distance: [view_distance; 4],
            time_of_day: [time_of_day as f32; 4],
            tick: [tick as f32; 4],
            screen_res: Vec4::from(screen_res.map(|e| e as f32)).into_array(),
            light_count: [light_count as u32; 4],
        }
    }
}

impl Default for Globals {
    fn default() -> Self {
        Self::new(
            Mat4::identity(),
            Mat4::identity(),
            Vec3::zero(),
            Vec3::zero(),
            0.0,
            0.0,
            0.0,
            Vec2::new(800, 500),
            0,
        )
    }
}

impl Light {
    pub fn new(pos: Vec3<f32>, col: Rgb<f32>, strength: f32) -> Self {
        Self {
            pos: Vec4::from(pos).into_array(),
            col: Rgba::new(col.r, col.g, col.b, strength).into_array(),
        }
    }
}

impl Default for Light {
    fn default() -> Self {
        Self::new(Vec3::zero(), Rgb::zero(), 0.0)
    }
}
