pub mod blit;
pub mod bloom;
pub mod clouds;
pub mod debug;
pub mod figure;
pub mod fluid;
pub mod lod_object;
pub mod lod_terrain;
pub mod particle;
pub mod postprocess;
pub mod rain_occlusion;
pub mod rope;
pub mod shadow;
pub mod skybox;
pub mod sprite;
pub mod terrain;
pub mod trail;
pub mod ui;

use super::{Consts, Renderer, Texture};
use crate::scene::camera::CameraMode;
use bytemuck::{Pod, Zeroable};
use common::{resources::TimeOfDay, terrain::BlockKind, util::srgb_to_linear};
use std::marker::PhantomData;
use vek::*;

pub use self::{figure::FigureSpriteAtlasData, terrain::TerrainAtlasData};

// TODO: auto insert these into shaders
pub const MAX_POINT_LIGHT_COUNT: usize = 20;
pub const MAX_FIGURE_SHADOW_COUNT: usize = 24;
pub const MAX_DIRECTED_LIGHT_COUNT: usize = 6;

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
pub struct Globals {
    /// Transformation from world coordinate space (with focus_off as the
    /// origin) to the camera space
    view_mat: [[f32; 4]; 4],
    proj_mat: [[f32; 4]; 4],
    /// proj_mat * view_mat
    all_mat: [[f32; 4]; 4],
    /// Offset of the camera from the focus position
    cam_pos: [f32; 4],
    /// Integer portion of the focus position in world coordinates
    focus_off: [f32; 4],
    /// Fractions portion of the focus position
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
    /// Direction of sunlight.
    sun_dir: [f32; 4],
    /// Direction of moonlight.
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
    last_lightning: [f32; 4],
    wind_vel: [f32; 2],
    ambiance: f32,
    cam_mode: u32,
    sprite_render_distance: f32,
    // To keep 16-byte-aligned.
    globals_dummy: [f32; 3],
}
/// Make sure Globals is 16-byte-aligned.
const _: () = assert!(core::mem::size_of::<Globals>() % 16 == 0);

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

pub const TIME_OVERFLOW: f64 = 300000.0;

impl Globals {
    /// Create global consts from the provided parameters.
    #[allow(clippy::too_many_arguments)]
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
        client_tick: f64,
        screen_res: Vec2<u16>,
        shadow_planes: Vec2<f32>,
        light_count: usize,
        shadow_count: usize,
        directed_light_count: usize,
        medium: BlockKind,
        select_pos: Option<Vec3<i32>>,
        gamma: f32,
        exposure: f32,
        last_lightning: (Vec3<f32>, f64),
        wind_vel: Vec2<f32>,
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
            time_of_day: [
                (time_of_day % (3600.0 * 24.0)) as f32,
                // TODO: Find a better way than just pure repetition. A solution like
                // the one applied to `tick` could work, but it would be used in hot
                // shader_code. So we might not want to use that method there.
                //
                // Repeats every 1000 ingame days. This increases by dt * (1 / 3600)
                // per tick on defualt server settings. So those per tick changes can't
                // really be fully represented at a value above `50.0`.
                (time_of_day / (3600.0 * 24.0) % 1000.0) as f32,
                0.0,
                0.0,
            ],
            sun_dir: Vec4::from_direction(TimeOfDay::new(time_of_day).get_sun_dir()).into_array(),
            moon_dir: Vec4::from_direction(TimeOfDay::new(time_of_day).get_moon_dir()).into_array(),
            tick: [
                (tick % TIME_OVERFLOW) as f32,
                (tick / TIME_OVERFLOW).floor() as f32,
                client_tick as f32,
                0.0,
            ],
            // Provide the shadow map far plane as well.
            screen_res: [
                screen_res.x as f32,
                screen_res.y as f32,
                shadow_planes.x,
                shadow_planes.y,
            ],
            // TODO: why do we accept values greater than the max?
            light_shadow_count: [
                usize::min(light_count, MAX_POINT_LIGHT_COUNT) as u32,
                usize::min(shadow_count, MAX_FIGURE_SHADOW_COUNT) as u32,
                usize::min(directed_light_count, MAX_DIRECTED_LIGHT_COUNT) as u32,
                0,
            ],
            shadow_proj_factors: [
                shadow_planes.y / (shadow_planes.y - shadow_planes.x),
                shadow_planes.y * shadow_planes.x / (shadow_planes.y - shadow_planes.x),
                0.0,
                0.0,
            ],
            medium: [if medium.is_liquid() {
                1
            } else if medium.is_filled() {
                2
            } else {
                0
            }; 4],
            select_pos: select_pos
                .map(|sp| Vec4::from(sp) + Vec4::unit_w())
                .unwrap_or_else(Vec4::zero)
                .into_array(),
            gamma_exposure: [gamma, exposure, 0.0, 0.0],
            last_lightning: last_lightning
                .0
                .with_w((last_lightning.1 % TIME_OVERFLOW) as f32)
                .into_array(),
            wind_vel: wind_vel.into_array(),
            ambiance: ambiance.clamped(0.0, 1.0),
            cam_mode: cam_mode as u32,
            sprite_render_distance,
            globals_dummy: [0.0; 3],
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
            (Vec3::zero(), -1000.0),
            Vec2::zero(),
            1.0,
            CameraMode::ThirdPerson,
            250.0,
        )
    }
}

impl Light {
    pub fn new(pos: Vec3<f32>, col: Rgb<f32>, strength: f32) -> Self {
        let linearized_col = srgb_to_linear(col);

        Self {
            pos: Vec4::from(pos).into_array(),
            col: (Rgba::new(linearized_col.r, linearized_col.g, linearized_col.b, 0.0) * strength)
                .into_array(),
        }
    }

    pub fn get_pos(&self) -> Vec3<f32> { Vec3::new(self.pos[0], self.pos[1], self.pos[2]) }

    #[must_use]
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
    // TODO: enforce that these are the lengths in the shaders??
    pub globals: Consts<Globals>,
    pub lights: Consts<Light>,
    pub shadows: Consts<Shadow>,
    pub shadow_mats: shadow::BoundLocals,
    pub rain_occlusion_mats: rain_occlusion::BoundLocals,
    pub point_light_matrices: Box<[shadow::PointLightMatrix; 126]>,
}

pub struct GlobalsBindGroup {
    pub(super) bind_group: wgpu::BindGroup,
}

pub struct ShadowTexturesBindGroup {
    pub(super) bind_group: wgpu::BindGroup,
}

pub struct GlobalsLayouts {
    pub globals: wgpu::BindGroupLayout,
    pub figure_sprite_atlas_layout: VoxelAtlasLayout<FigureSpriteAtlasData>,
    pub terrain_atlas_layout: VoxelAtlasLayout<TerrainAtlasData>,
    pub shadow_textures: wgpu::BindGroupLayout,
}

/// A type representing a set of textures that have the same atlas layout and
/// pertain to a greedy voxel structure.
pub struct AtlasTextures<Locals, S: AtlasData>
where
    [(); S::TEXTURES]:,
{
    pub(super) bind_group: wgpu::BindGroup,
    pub textures: [Texture; S::TEXTURES],
    phantom: std::marker::PhantomData<Locals>,
}

pub struct VoxelAtlasLayout<S: AtlasData>(wgpu::BindGroupLayout, PhantomData<S>);

impl<S: AtlasData> VoxelAtlasLayout<S> {
    pub fn new(device: &wgpu::Device) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &S::layout(),
        });

        Self(layout, PhantomData)
    }

    pub fn layout(&self) -> &wgpu::BindGroupLayout { &self.0 }
}

/// A trait implemented by texture atlas groups.
///
/// Terrain, figures, sprites, etc. all use texture atlases but have different
/// requirements, such as that layers provided by each atlas. This trait
/// abstracts over these cases.
pub trait AtlasData {
    /// The number of texture channels that this atlas has.
    const TEXTURES: usize;
    /// Abstracts over a slice into the texture data, as returned by
    /// [`AtlasData::slice_mut`].
    type SliceMut<'a>: Iterator
    where
        Self: 'a;

    /// Return blank atlas data upon which texels can be applied.
    fn blank_with_size(sz: Vec2<u16>) -> Self;

    /// Return an array of texture formats and data for each texture layer in
    /// the atlas.
    fn as_texture_data(&self) -> [(wgpu::TextureFormat, &[u8]); Self::TEXTURES];

    /// Return a layout entry that corresponds to the texture layers in the
    /// atlas.
    fn layout() -> Vec<wgpu::BindGroupLayoutEntry>;

    /// Take a sub-slice of the texture data for each layer in the atlas.
    fn slice_mut(&mut self, range: std::ops::Range<usize>) -> Self::SliceMut<'_>;

    /// Create textures on the GPU corresponding to the layers in the atlas.
    fn create_textures(
        &self,
        renderer: &mut Renderer,
        atlas_size: Vec2<u16>,
    ) -> [Texture; Self::TEXTURES] {
        self.as_texture_data().map(|(fmt, data)| {
            let texture_info = wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: u32::from(atlas_size.x),
                    height: u32::from(atlas_size.y),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: fmt,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            };

            let sampler_info = wgpu::SamplerDescriptor {
                label: None,
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                border_color: Some(wgpu::SamplerBorderColor::TransparentBlack),
                ..Default::default()
            };

            let view_info = wgpu::TextureViewDescriptor {
                label: None,
                format: Some(fmt),
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: None,
                base_array_layer: 0,
                array_layer_count: None,
            };

            renderer.create_texture_with_data_raw(&texture_info, &view_info, &sampler_info, data)
        })
    }
}

impl GlobalsLayouts {
    pub fn base_globals_layout() -> Vec<wgpu::BindGroupLayoutEntry> {
        vec![
            // Global uniform
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Noise tex
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            // Light uniform
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Shadow uniform
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // Alt texture
            wgpu::BindGroupLayoutEntry {
                binding: 5,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 6,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            // Horizon texture
            wgpu::BindGroupLayoutEntry {
                binding: 7,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 8,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            // light shadows (ie shadows from a light?)
            wgpu::BindGroupLayoutEntry {
                binding: 9,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                // TODO: is this relevant?
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // lod map (t_map)
            wgpu::BindGroupLayoutEntry {
                binding: 10,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 11,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            // clouds t_weather
            wgpu::BindGroupLayoutEntry {
                binding: 12,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 13,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            // rain occlusion
            wgpu::BindGroupLayoutEntry {
                binding: 14,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ]
    }

    pub fn new(device: &wgpu::Device) -> Self {
        let globals = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Globals layout"),
            entries: &Self::base_globals_layout(),
        });

        let shadow_textures = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                // point shadow_maps
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
                // directed shadow maps
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
                // Rain occlusion maps
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
            ],
        });

        Self {
            globals,
            figure_sprite_atlas_layout: VoxelAtlasLayout::new(device),
            terrain_atlas_layout: VoxelAtlasLayout::new(device),
            shadow_textures,
        }
    }

    // Note: this allocation serves the purpose of not having to duplicate code
    pub fn bind_base_globals<'a>(
        global_model: &'a GlobalModel,
        lod_data: &'a lod_terrain::LodData,
        noise: &'a Texture,
    ) -> Vec<wgpu::BindGroupEntry<'a>> {
        vec![
            // Global uniform
            wgpu::BindGroupEntry {
                binding: 0,
                resource: global_model.globals.buf().as_entire_binding(),
            },
            // Noise tex
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&noise.view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&noise.sampler),
            },
            // Light uniform
            wgpu::BindGroupEntry {
                binding: 3,
                resource: global_model.lights.buf().as_entire_binding(),
            },
            // Shadow uniform
            wgpu::BindGroupEntry {
                binding: 4,
                resource: global_model.shadows.buf().as_entire_binding(),
            },
            // Alt texture
            wgpu::BindGroupEntry {
                binding: 5,
                resource: wgpu::BindingResource::TextureView(&lod_data.alt.view),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: wgpu::BindingResource::Sampler(&lod_data.alt.sampler),
            },
            // Horizon texture
            wgpu::BindGroupEntry {
                binding: 7,
                resource: wgpu::BindingResource::TextureView(&lod_data.horizon.view),
            },
            wgpu::BindGroupEntry {
                binding: 8,
                resource: wgpu::BindingResource::Sampler(&lod_data.horizon.sampler),
            },
            // light shadows
            wgpu::BindGroupEntry {
                binding: 9,
                resource: global_model.shadow_mats.buf().as_entire_binding(),
            },
            // lod map (t_map)
            wgpu::BindGroupEntry {
                binding: 10,
                resource: wgpu::BindingResource::TextureView(&lod_data.map.view),
            },
            wgpu::BindGroupEntry {
                binding: 11,
                resource: wgpu::BindingResource::Sampler(&lod_data.map.sampler),
            },
            wgpu::BindGroupEntry {
                binding: 12,
                resource: wgpu::BindingResource::TextureView(&lod_data.weather.view),
            },
            wgpu::BindGroupEntry {
                binding: 13,
                resource: wgpu::BindingResource::Sampler(&lod_data.weather.sampler),
            },
            // rain occlusion
            wgpu::BindGroupEntry {
                binding: 14,
                resource: global_model.rain_occlusion_mats.buf().as_entire_binding(),
            },
        ]
    }

    pub fn bind(
        &self,
        device: &wgpu::Device,
        global_model: &GlobalModel,
        lod_data: &lod_terrain::LodData,
        noise: &Texture,
    ) -> GlobalsBindGroup {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.globals,
            entries: &Self::bind_base_globals(global_model, lod_data, noise),
        });

        GlobalsBindGroup { bind_group }
    }

    pub fn bind_shadow_textures(
        &self,
        device: &wgpu::Device,
        point_shadow_map: &Texture,
        directed_shadow_map: &Texture,
        rain_occlusion_map: &Texture,
    ) -> ShadowTexturesBindGroup {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.shadow_textures,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&point_shadow_map.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&point_shadow_map.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&directed_shadow_map.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&directed_shadow_map.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&rain_occlusion_map.view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&rain_occlusion_map.sampler),
                },
            ],
        });

        ShadowTexturesBindGroup { bind_group }
    }

    pub fn bind_atlas_textures<Locals, S: AtlasData>(
        &self,
        device: &wgpu::Device,
        layout: &VoxelAtlasLayout<S>,
        textures: [Texture; S::TEXTURES],
    ) -> AtlasTextures<Locals, S> {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: layout.layout(),
            entries: &textures
                .iter()
                .enumerate()
                .flat_map(|(i, tex)| {
                    [
                        wgpu::BindGroupEntry {
                            binding: i as u32 * 2,
                            resource: wgpu::BindingResource::TextureView(&tex.view),
                        },
                        wgpu::BindGroupEntry {
                            binding: i as u32 * 2 + 1,
                            resource: wgpu::BindingResource::Sampler(&tex.sampler),
                        },
                    ]
                })
                .collect::<Vec<_>>(),
        });

        AtlasTextures {
            textures,
            bind_group,
            phantom: std::marker::PhantomData,
        }
    }
}
