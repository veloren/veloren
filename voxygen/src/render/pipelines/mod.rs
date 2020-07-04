pub mod figure;
pub mod fluid;
pub mod lod_terrain;
pub mod postprocess;
pub mod shadow;
pub mod skybox;
pub mod sprite;
pub mod terrain;
pub mod ui;

use crate::scene::camera::CameraMode;
use common::terrain::BlockKind;
use gfx::{self, gfx_constant_struct_meta, gfx_defines, gfx_impl_struct_meta};
use vek::*;

pub const MAX_POINT_LIGHT_COUNT: usize = 31;
pub const MAX_FIGURE_SHADOW_COUNT: usize = 24;
pub const MAX_DIRECTED_LIGHT_COUNT: usize = 6;

gfx_defines! {
    constant Globals {
        view_mat: [[f32; 4]; 4] = "view_mat",
        proj_mat: [[f32; 4]; 4] = "proj_mat",
        all_mat: [[f32; 4]; 4] = "all_mat",
        cam_pos: [f32; 4] = "cam_pos",
        focus_off: [f32; 4] = "focus_off",
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
        sun_dir: [f32; 4] = "sun_dir",
        moon_dir: [f32; 4] = "moon_dir",
        tick: [f32; 4] = "tick",
        /// x, y represent the resolution of the screen;
        /// w, z represent the near and far planes of the shadow map.
        screen_res: [f32; 4] = "screen_res",
        light_shadow_count: [u32; 4] = "light_shadow_count",
        shadow_proj_factors: [f32; 4] = "shadow_proj_factors",
        medium: [u32; 4] = "medium",
        select_pos: [i32; 4] = "select_pos",
        gamma: [f32; 4] = "gamma",
        cam_mode: u32 = "cam_mode",
        sprite_render_distance: f32 = "sprite_render_distance",
    }

    constant Light {
        pos: [f32; 4] = "light_pos",
        col: [f32; 4] = "light_col",
        // proj: [[f32; 4]; 4] = "light_proj";
    }

    constant Shadow {
        pos_radius: [f32; 4] = "shadow_pos_radius",
    }
}

impl Globals {
    /// Create global consts from the provided parameters.
    #[allow(clippy::or_fun_call)] // TODO: Pending review in #587
    #[allow(clippy::too_many_arguments)] // TODO: Pending review in #587
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
        shadow_planes: Vec2<f32>,
        light_count: usize,
        shadow_count: usize,
        directed_light_count: usize,
        medium: BlockKind,
        select_pos: Option<Vec3<i32>>,
        gamma: f32,
        cam_mode: CameraMode,
        sprite_render_distance: f32,
    ) -> Self {
        Self {
            view_mat: view_mat.into_col_arrays(),
            proj_mat: proj_mat.into_col_arrays(),
            all_mat: (proj_mat * view_mat).into_col_arrays(),
            cam_pos: Vec4::from(cam_pos).into_array(),
            focus_off: Vec4::from(focus_pos).map(|e: f32| e.trunc()).into_array(),
            focus_pos: Vec4::from(focus_pos).map(|e: f32| e.fract()).into_array(),
            view_distance: [view_distance, tgt_detail, map_bounds.x, map_bounds.y],
            time_of_day: [time_of_day as f32; 4],
            sun_dir: Vec4::from_direction(Self::get_sun_dir(time_of_day)).into_array(),
            moon_dir: Vec4::from_direction(Self::get_moon_dir(time_of_day)).into_array(),
            tick: [tick as f32; 4],
            // Provide the shadow map far plane as well.
            screen_res: [
                screen_res.x as f32,
                screen_res.y as f32,
                shadow_planes.x,
                shadow_planes.y,
            ],
            light_shadow_count: [
                (light_count % (MAX_POINT_LIGHT_COUNT + 1)) as u32,
                (shadow_count % (MAX_FIGURE_SHADOW_COUNT + 1)) as u32,
                (directed_light_count % (MAX_DIRECTED_LIGHT_COUNT + 1)) as u32,
                0,
            ],
            shadow_proj_factors: [
                (shadow_planes.y + shadow_planes.x) / (shadow_planes.y - shadow_planes.x),
                (2.0 * shadow_planes.y * shadow_planes.x) / (shadow_planes.y - shadow_planes.x),
                0.0,
                0.0,
            ],
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

    fn get_angle_rad(time_of_day: f64) -> f32 {
        const TIME_FACTOR: f32 = (std::f32::consts::PI * 2.0) / (3600.0 * 24.0);
        time_of_day as f32 * TIME_FACTOR
    }

    pub fn get_sun_dir(time_of_day: f64) -> Vec3<f32> {
        let angle_rad = Self::get_angle_rad(time_of_day);
        Vec3::new(angle_rad.sin(), 0.0, angle_rad.cos())
    }

    pub fn get_moon_dir(time_of_day: f64) -> Vec3<f32> {
        let angle_rad = Self::get_angle_rad(time_of_day);
        -Vec3::new(angle_rad.sin(), 0.0, angle_rad.cos() - 0.5)
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
            Vec2::new(1.0, 25.0),
            0,
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
