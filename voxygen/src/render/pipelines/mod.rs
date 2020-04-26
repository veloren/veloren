pub mod figure;
pub mod fluid;
pub mod lod_terrain;
pub mod postprocess;
pub mod skybox;
pub mod sprite;
pub mod terrain;
pub mod ui;

use super::util::arr_to_mat;
use crate::scene::camera::CameraMode;
use common::terrain::BlockKind;
use gfx::{self, gfx_constant_struct_meta, gfx_defines, gfx_impl_struct_meta};
use vek::*;

gfx_defines! {
    constant Globals {
        view_mat: [[f32; 4]; 4] = "view_mat",
        proj_mat: [[f32; 4]; 4] = "proj_mat",
        all_mat: [[f32; 4]; 4] = "all_mat",
        cam_pos: [f32; 4] = "cam_pos",
        focus_pos: [f32; 4] = "focus_pos",
        /// NOTE: max_intensity is computed as the ratio between the brightest and least bright
        /// intensities among all lights in the scene.
        // hdr_ratio: [f32; 4] = "max_intensity",
        /// NOTE: view_distance.x is the horizontal view distance, view_distance.y is the LOD
        /// detail, view_distance.z is the
        /// minimum height over any land chunk (i.e. the sea level), and view_distance.w is the
        /// maximum height over this minimum height.
        ///
        /// TODO: Fix whatever alignment issue requires these uniforms to be aligned.
        view_distance: [f32; 4] = "view_distance",
        time_of_day: [f32; 4] = "time_of_day", // TODO: Make this f64.
        tick: [f32; 4] = "tick",
        screen_res: [f32; 4] = "screen_res",
        light_shadow_count: [u32; 4] = "light_shadow_count",
        medium: [u32; 4] = "medium",
        select_pos: [i32; 4] = "select_pos",
        gamma: [f32; 4] = "gamma",
        cam_mode: u32 = "cam_mode",
        sprite_render_distance: f32 = "sprite_render_distance",
    }

    constant Light {
        pos: [f32; 4] = "light_pos",
        col: [f32; 4] = "light_col",
    }

    constant Shadow {
        pos_radius: [f32; 4] = "shadow_pos_radius",
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
        tgt_detail: f32,
        map_bounds: Vec2<f32>,
        time_of_day: f64,
        tick: f64,
        screen_res: Vec2<u16>,
        light_count: usize,
        shadow_count: usize,
        medium: BlockKind,
        select_pos: Option<Vec3<i32>>,
        gamma: f32,
        cam_mode: CameraMode,
        sprite_render_distance: f32,
    ) -> Self {
        Self {
            view_mat: arr_to_mat(view_mat.into_col_array()),
            proj_mat: arr_to_mat(proj_mat.into_col_array()),
            all_mat: arr_to_mat((proj_mat * view_mat).into_col_array()),
            cam_pos: Vec4::from(cam_pos).into_array(),
            focus_pos: Vec4::from(focus_pos).into_array(),
            view_distance: [view_distance, tgt_detail, map_bounds.x, map_bounds.y],
            time_of_day: [time_of_day as f32; 4],
            tick: [tick as f32; 4],
            screen_res: Vec4::from(screen_res.map(|e| e as f32)).into_array(),
            light_shadow_count: [light_count as u32, shadow_count as u32, 0, 0],
            medium: [if medium.is_fluid() { 1 } else { 0 }; 4],
            select_pos: select_pos
                .map(|sp| Vec4::from(sp) + Vec4::unit_w())
                .unwrap_or(Vec4::zero())
                .into_array(),
            gamma: [gamma; 4],
            cam_mode: cam_mode as u32,
            sprite_render_distance,
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
            100.0,
            Vec2::new(140.0, 2048.0),
            0.0,
            0.0,
            Vec2::new(800, 500),
            0,
            0,
            BlockKind::Air,
            None,
            1.0,
            CameraMode::ThirdPerson,
            250.0,
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

    pub fn get_pos(&self) -> Vec3<f32> { Vec3::new(self.pos[0], self.pos[1], self.pos[2]) }
}

impl Default for Light {
    fn default() -> Self { Self::new(Vec3::zero(), Rgb::zero(), 0.0) }
}

impl Shadow {
    pub fn new(pos: Vec3<f32>, radius: f32) -> Self {
        Self {
            pos_radius: [pos.x, pos.y, pos.z, radius],
        }
    }

    pub fn get_pos(&self) -> Vec3<f32> {
        Vec3::new(self.pos_radius[0], self.pos_radius[1], self.pos_radius[2])
    }
}

impl Default for Shadow {
    fn default() -> Self { Self::new(Vec3::zero(), 0.0) }
}
