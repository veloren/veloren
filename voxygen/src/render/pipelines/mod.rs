pub mod clouds;
pub mod figure;
pub mod fluid;
pub mod lod_terrain;
pub mod particle;
pub mod postprocess;
pub mod shadow;
pub mod skybox;
pub mod sprite;
pub mod terrain;
pub mod ui;

use super::Consts;
use crate::scene::camera::CameraMode;
use bytemuck::{Pod, Zeroable};
use common::terrain::BlockKind;
use vek::*;

pub const MAX_POINT_LIGHT_COUNT: usize = 31;
pub const MAX_FIGURE_SHADOW_COUNT: usize = 24;
pub const MAX_DIRECTED_LIGHT_COUNT: usize = 6;

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct Globals {
    view_mat: [[f32; 4]; 4],
    proj_mat: [[f32; 4]; 4],
    all_mat: [[f32; 4]; 4],
    cam_pos: [f32; 4],
    focus_off: [f32; 4],
    focus_pos: [f32; 4],
    /// NOTE: view_distance.x is the horizontal view distance, view_distance.y
    /// is the LOD detail, view_distance.z is the
    /// minimum height over any land chunk (i.e. the sea level), and
    /// view_distance.w is the maximum height over this minimum height.
    ///
    /// TODO: Fix whatever alignment issue requires these uniforms to be
    /// aligned.
    view_distance: [f32; 4],
    time_of_day: [f32; 4], // TODO: Make this f64.
    sun_dir: [f32; 4],
    moon_dir: [f32; 4],
    tick: [f32; 4],
    /// x, y represent the resolution of the screen;
    /// w, z represent the near and far planes of the shadow map.
    screen_res: [f32; 4],
    light_shadow_count: [u32; 4],
    shadow_proj_factors: [f32; 4],
    medium: [u32; 4],
    select_pos: [i32; 4],
    gamma_exposure: [f32; 4],
    ambiance: f32,
    cam_mode: u32,
    sprite_render_distance: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct Light {
    pub pos: [f32; 4],
    pub col: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct Shadow {
    pos_radius: [f32; 4],
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
        exposure: f32,
        ambiance: f32,
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
            medium: [if medium.is_liquid() { 1 } else { 0 }; 4],
            select_pos: select_pos
                .map(|sp| Vec4::from(sp) + Vec4::unit_w())
                .unwrap_or(Vec4::zero())
                .into_array(),
            gamma_exposure: [gamma, exposure, 0.0, 0.0],
            ambiance,
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
        Vec3::new(-angle_rad.sin(), 0.0, angle_rad.cos())
    }

    pub fn get_moon_dir(time_of_day: f64) -> Vec3<f32> {
        let angle_rad = Self::get_angle_rad(time_of_day);
        -Vec3::new(-angle_rad.sin(), 0.0, angle_rad.cos() - 0.5).normalized()
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
            1.0,
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

    pub fn with_strength(mut self, strength: f32) -> Self {
        self.col = (Vec4::<f32>::from(self.col) * strength).into_array();
        self
    }
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

// Global scene data spread across several arrays.
pub struct GlobalModel {
    pub globals: Consts<Globals>,
    pub lights: Consts<Light>,
    pub shadows: Consts<Shadow>,
    pub shadow_mats: Consts<shadow::Locals>,
}

pub struct GlobalsLayouts {
    pub globals: wgpu::BindGroupLayout,
}

impl GlobalsLayouts {
    pub fn new(device: &wgpu::Device) -> Self {
        let globals = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Globals layout"),
            entries: &[
                // Global uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::UniformBuffer {
                        dynamic: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Noise tex
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::SampledTexture {
                        dimension: wgpu::TextureViewDimension::D2,
                        component_type: wgpu::TextureComponentType::Float,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler { comparison: false },
                    count: None,
                },
                // Light uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::UniformBuffer {
                        dynamic: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Shadow uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::UniformBuffer {
                        dynamic: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Alt texture
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::SampledTexture {
                        component_type: wgpu::TextureComponentType::Float,
                        dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler { comparison: false },
                    count: None,
                },
                // Horizon texture
                wgpu::BindGroupLayoutEntry {
                    binding: 7,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::SampledTexture {
                        component_type: wgpu::TextureComponentType::Float,
                        dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 8,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler { comparison: false },
                    count: None,
                },
                // light shadows
                wgpu::BindGroupLayoutEntry {
                    binding: 9,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::SampledTexture {
                        component_type: wgpu::TextureComponentType::Float,
                        dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 10,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler { comparison: false },
                    count: None,
                },
                // point shadow_maps
                wgpu::BindGroupLayoutEntry {
                    binding: 11,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::SampledTexture {
                        component_type: wgpu::TextureComponentType::Float,
                        dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 12,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler { comparison: false },
                    count: None,
                },
                // directed shadow maps
                wgpu::BindGroupLayoutEntry {
                    binding: 13,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::SampledTexture {
                        component_type: wgpu::TextureComponentType::Float,
                        dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 14,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler { comparison: false },
                    count: None,
                },
                // lod map (t_map)
                wgpu::BindGroupLayoutEntry {
                    binding: 15,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::SampledTexture {
                        component_type: wgpu::TextureComponentType::Float,
                        dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 16,
                    visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler { comparison: false },
                    count: None,
                },
            ],
        });

        Self { globals }
    }
}
