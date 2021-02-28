mod binding;
pub(super) mod drawer;
// Consts and bind groups for post-process and clouds
mod locals;
mod shaders;
mod shadow_map;

use locals::Locals;
use shaders::Shaders;
use shadow_map::{ShadowMap, ShadowMapRenderer};

use super::{
    buffer::Buffer,
    consts::Consts,
    instances::Instances,
    mesh::Mesh,
    model::{DynamicModel, Model},
    pipelines::{
        clouds, figure, fluid, lod_terrain, particle, postprocess, shadow, skybox, sprite, terrain,
        ui, GlobalsBindGroup, GlobalsLayouts, ShadowTexturesBindGroup,
    },
    texture::Texture,
    AaMode, AddressMode, CloudMode, FilterMode, FluidMode, LightingMode, RenderError, RenderMode,
    ShadowMapMode, ShadowMode, Vertex,
};
use common::assets::{self, AssetExt, AssetHandle};
use common_base::span;
use core::convert::TryFrom;
use tracing::{error, info, warn};
use vek::*;

// TODO: yeet this somewhere else
/// A type representing data that can be converted to an immutable texture map
/// of ColLight data (used for texture atlases created during greedy meshing).
// TODO: revert to u16
pub type ColLightInfo = (Vec<[u8; 4]>, Vec2<u32>);

const QUAD_INDEX_BUFFER_U16_START_VERT_LEN: u16 = 3000;
const QUAD_INDEX_BUFFER_U32_START_VERT_LEN: u32 = 3000;

/// A type that stores all the layouts associated with this renderer.
struct Layouts {
    global: GlobalsLayouts,

    clouds: clouds::CloudsLayout,
    figure: figure::FigureLayout,
    fluid: fluid::FluidLayout,
    postprocess: postprocess::PostProcessLayout,
    shadow: shadow::ShadowLayout,
    sprite: sprite::SpriteLayout,
    terrain: terrain::TerrainLayout,
    ui: ui::UiLayout,
}

/// A type that stores all the pipelines associated with this renderer.
struct Pipelines {
    figure: figure::FigurePipeline,
    fluid: fluid::FluidPipeline,
    lod_terrain: lod_terrain::LodTerrainPipeline,
    particle: particle::ParticlePipeline,
    clouds: clouds::CloudsPipeline,
    postprocess: postprocess::PostProcessPipeline,
    // Consider reenabling at some time
    // player_shadow: figure::FigurePipeline,
    skybox: skybox::SkyboxPipeline,
    sprite: sprite::SpritePipeline,
    terrain: terrain::TerrainPipeline,
    ui: ui::UiPipeline,
}

/// Render target views
struct Views {
    // NOTE: unused for now
    win_depth: wgpu::TextureView,

    tgt_color: wgpu::TextureView,
    tgt_depth: wgpu::TextureView,
    // TODO: rename
    tgt_color_pp: wgpu::TextureView,
}

/// Shadow rendering textures, layouts, pipelines, and bind groups
struct Shadow {
    map: ShadowMap,
    bind: ShadowTexturesBindGroup,
}

/// A type that encapsulates rendering state. `Renderer` is central to Voxygen's
/// rendering subsystem and contains any state necessary to interact with the
/// GPU, along with pipeline state objects (PSOs) needed to renderer different
/// kinds of models to the screen.
pub struct Renderer {
    // TODO: remove pub(super)
    pub(super) device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface,
    swap_chain: wgpu::SwapChain,
    sc_desc: wgpu::SwapChainDescriptor,

    sampler: wgpu::Sampler,
    depth_sampler: wgpu::Sampler,

    layouts: Layouts,
    pipelines: Pipelines,
    shadow: Shadow,
    // Note: we keep these here since their bind groups need to be updated if we resize the
    // color/depth textures
    locals: Locals,
    views: Views,
    noise_tex: Texture,

    quad_index_buffer_u16: Buffer<u16>,
    quad_index_buffer_u32: Buffer<u32>,

    shaders: AssetHandle<Shaders>,

    mode: RenderMode,
    resolution: Vec2<u32>,

    profiler: wgpu_profiler::GpuProfiler,
}

impl Renderer {
    /// Create a new `Renderer` from a variety of backend-specific components
    /// and the window targets.
    pub fn new(window: &winit::window::Window, mode: RenderMode) -> Result<Self, RenderError> {
        // Enable seamless cubemaps globally, where available--they are essentially a
        // strict improvement on regular cube maps.
        //
        // Note that since we only have to enable this once globally, there is no point
        // in doing this on rerender.
        // Self::enable_seamless_cube_maps(&mut device);

        // TODO: fix panic on wayland with opengl?
        // TODO: fix backend defaulting to opengl on wayland.
        let instance = wgpu::Instance::new(
            wgpu::BackendBit::PRIMARY, /* | wgpu::BackendBit::SECONDARY */
        );

        let dims = window.inner_size();

        // This is unsafe because the window handle must be valid, if you find a way to
        // have an invalid winit::Window then you have bigger issues
        #[allow(unsafe_code)]
        let surface = unsafe { instance.create_surface(window) };

        let adapter = futures::executor::block_on(instance.request_adapter(
            &wgpu::RequestAdapterOptionsBase {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
            },
        ))
        .ok_or(RenderError::CouldNotFindAdapter)?;

        let limits = wgpu::Limits {
            max_push_constant_size: 64,
            ..Default::default()
        };

        let (device, queue) = futures::executor::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                // TODO
                label: None,
                features: wgpu::Features::DEPTH_CLAMPING
                    | wgpu::Features::ADDRESS_MODE_CLAMP_TO_BORDER
                    | wgpu::Features::PUSH_CONSTANTS
                    // TODO: make optional based on enabling profiling
                    // NOTE: requires recreating the device/queue is this setting changes
                    // alternatively it could be a compile time feature toggle
                    | wgpu_profiler::GpuProfiler::REQUIRED_WGPU_FEATURES,
                limits,
            },
            None,
        ))?;

        let info = adapter.get_info();
        info!(
            ?info.name,
            ?info.vendor,
            ?info.backend,
            ?info.device,
            ?info.device_type,
            "selected graphics device"
        );

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: dims.width,
            height: dims.height,
            present_mode: mode.present_mode.into(),
        };

        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        let shadow_views = Self::create_shadow_views(
            &device,
            (dims.width, dims.height),
            &ShadowMapMode::try_from(mode.shadow).unwrap_or_default(),
        )
        .map_err(|err| {
            warn!("Could not create shadow map views: {:?}", err);
        })
        .ok();

        let shaders = Shaders::load_expect("");

        let layouts = {
            let global = GlobalsLayouts::new(&device);

            let clouds = clouds::CloudsLayout::new(&device);
            let figure = figure::FigureLayout::new(&device);
            let fluid = fluid::FluidLayout::new(&device);
            let postprocess = postprocess::PostProcessLayout::new(&device);
            let shadow = shadow::ShadowLayout::new(&device);
            let sprite = sprite::SpriteLayout::new(&device);
            let terrain = terrain::TerrainLayout::new(&device);
            let ui = ui::UiLayout::new(&device);

            Layouts {
                global,

                clouds,
                figure,
                fluid,
                postprocess,
                shadow,
                sprite,
                terrain,
                ui,
            }
        };

        let (
            pipelines,
            //player_shadow_pipeline,
            point_shadow_pipeline,
            terrain_directed_shadow_pipeline,
            figure_directed_shadow_pipeline,
        ) = create_pipelines(
            &device,
            &layouts,
            &shaders.read(),
            &mode,
            &sc_desc,
            shadow_views.is_some(),
        )?;

        let views = Self::create_rt_views(&device, (dims.width, dims.height), &mode)?;

        let shadow_map = if let (
            Some(point_pipeline),
            Some(terrain_directed_pipeline),
            Some(figure_directed_pipeline),
            Some(shadow_views),
        ) = (
            point_shadow_pipeline,
            terrain_directed_shadow_pipeline,
            figure_directed_shadow_pipeline,
            shadow_views,
        ) {
            let (point_depth, directed_depth) = shadow_views;

            let layout = shadow::ShadowLayout::new(&device);

            ShadowMap::Enabled(ShadowMapRenderer {
                // point_encoder: factory.create_command_buffer().into(),
                // directed_encoder: factory.create_command_buffer().into(),
                directed_depth,
                point_depth,

                point_pipeline,
                terrain_directed_pipeline,
                figure_directed_pipeline,

                layout,
            })
        } else {
            let (dummy_point, dummy_directed) = Self::create_dummy_shadow_tex(&device, &queue);
            ShadowMap::Disabled {
                dummy_point,
                dummy_directed,
            }
        };

        let shadow_bind = {
            let (point, directed) = shadow_map.textures();
            layouts
                .global
                .bind_shadow_textures(&device, point, directed)
        };

        let shadow = Shadow {
            map: shadow_map,
            bind: shadow_bind,
        };

        let create_sampler = |filter| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: None,
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: filter,
                min_filter: filter,
                mipmap_filter: wgpu::FilterMode::Nearest,
                compare: None,
                ..Default::default()
            })
        };

        let sampler = create_sampler(wgpu::FilterMode::Linear);
        let depth_sampler = create_sampler(wgpu::FilterMode::Nearest);

        let noise_tex = Texture::new(
            &device,
            &queue,
            &assets::Image::load_expect("voxygen.texture.noise").read().0,
            Some(wgpu::FilterMode::Linear),
            Some(wgpu::AddressMode::Repeat),
        )?;

        let clouds_locals =
            Self::create_consts_inner(&device, &queue, &[clouds::Locals::default()]);
        let postprocess_locals =
            Self::create_consts_inner(&device, &queue, &[postprocess::Locals::default()]);

        let locals = Locals::new(
            &device,
            &layouts,
            clouds_locals,
            postprocess_locals,
            &views.tgt_color,
            &views.tgt_depth,
            &views.tgt_color_pp,
            &sampler,
            &depth_sampler,
        );

        let quad_index_buffer_u16 =
            create_quad_index_buffer_u16(&device, QUAD_INDEX_BUFFER_U16_START_VERT_LEN as usize);
        let quad_index_buffer_u32 =
            create_quad_index_buffer_u32(&device, QUAD_INDEX_BUFFER_U32_START_VERT_LEN as usize);
        let mut profiler = wgpu_profiler::GpuProfiler::new(1, queue.get_timestamp_period());
        profiler.enable_timer = mode.profiler_enabled;
        profiler.enable_debug_marker = mode.profiler_enabled;

        Ok(Self {
            device,
            queue,
            surface,
            swap_chain,
            sc_desc,

            layouts,
            pipelines,
            shadow,
            locals,
            views,

            sampler,
            depth_sampler,
            noise_tex,

            quad_index_buffer_u16,
            quad_index_buffer_u32,

            shaders,

            mode,
            resolution: Vec2::new(dims.width, dims.height),

            profiler,
        })
    }

    /// Change the render mode.
    pub fn set_render_mode(&mut self, mode: RenderMode) -> Result<(), RenderError> {
        self.mode = mode;
        self.sc_desc.present_mode = self.mode.present_mode.into();

        // Enable/disable profiler
        self.profiler.enable_timer = self.mode.profiler_enabled;
        self.profiler.enable_debug_marker = self.mode.profiler_enabled;

        // Recreate render target
        self.on_resize(self.resolution)?;

        // Recreate pipelines with the new AA mode
        self.recreate_pipelines();

        Ok(())
    }

    /// Get the render mode.
    pub fn render_mode(&self) -> &RenderMode { &self.mode }

    /// Resize internal render targets to match window render target dimensions.
    pub fn on_resize(&mut self, dims: Vec2<u32>) -> Result<(), RenderError> {
        // Avoid panics when creating texture with w,h of 0,0.
        if dims.x != 0 && dims.y != 0 {
            // Resize swap chain
            self.resolution = dims;
            self.sc_desc.width = dims.x;
            self.sc_desc.height = dims.y;
            self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);

            // Resize other render targets
            self.views = Self::create_rt_views(&mut self.device, (dims.x, dims.y), &self.mode)?;
            // Rebind views to clouds/postprocess bind groups
            self.locals.rebind(
                &self.device,
                &self.layouts,
                &self.views.tgt_color,
                &self.views.tgt_depth,
                &self.views.tgt_color_pp,
                &self.sampler,
                &self.depth_sampler,
            );

            if let (ShadowMap::Enabled(shadow_map), ShadowMode::Map(mode)) =
                (&mut self.shadow.map, self.mode.shadow)
            {
                match Self::create_shadow_views(&mut self.device, (dims.x, dims.y), &mode) {
                    Ok((point_depth, directed_depth)) => {
                        shadow_map.point_depth = point_depth;
                        shadow_map.directed_depth = directed_depth;
                        self.shadow.bind = self.layouts.global.bind_shadow_textures(
                            &self.device,
                            &shadow_map.point_depth,
                            &shadow_map.directed_depth,
                        );
                    },
                    Err(err) => {
                        warn!("Could not create shadow map views: {:?}", err);
                    },
                }
            }
        }

        Ok(())
    }

    /// Create render target views
    fn create_rt_views(
        device: &wgpu::Device,
        size: (u32, u32),
        mode: &RenderMode,
    ) -> Result<Views, RenderError> {
        let upscaled = Vec2::<u32>::from(size)
            .map(|e| (e as f32 * mode.upscale_mode.factor) as u32)
            .into_tuple();
        let (width, height, sample_count) = match mode.aa {
            AaMode::None | AaMode::Fxaa => (upscaled.0, upscaled.1, 1),
            // TODO: Ensure sampling in the shader is exactly between the 4 texels
            // TODO: Figure out how to do upscaling correctly with SSAA
            AaMode::MsaaX4 => (upscaled.0, upscaled.1, 4),
            AaMode::MsaaX8 => (upscaled.0, upscaled.1, 8),
            AaMode::MsaaX16 => (upscaled.0, upscaled.1, 16),
        };
        let levels = 1;

        let color_view = || {
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: levels,
                sample_count,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::RENDER_ATTACHMENT,
            });

            tex.create_view(&wgpu::TextureViewDescriptor {
                label: None,
                format: Some(wgpu::TextureFormat::Bgra8UnormSrgb),
                dimension: Some(wgpu::TextureViewDimension::D2),
                // TODO: why is this not Color?
                aspect: wgpu::TextureAspect::All,
                base_mip_level: 0,
                level_count: None,
                base_array_layer: 0,
                array_layer_count: None,
            })
        };

        let tgt_color_view = color_view();
        let tgt_color_pp_view = color_view();

        let tgt_depth_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: levels,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::RENDER_ATTACHMENT,
        });
        let tgt_depth_view = tgt_depth_tex.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            format: Some(wgpu::TextureFormat::Depth32Float),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::DepthOnly,
            base_mip_level: 0,
            level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        let win_depth_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: levels,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
        });
        // TODO: Consider no depth buffer for the final draw to the window?
        let win_depth_view = tgt_depth_tex.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            format: Some(wgpu::TextureFormat::Depth32Float),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::DepthOnly,
            base_mip_level: 0,
            level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        Ok(Views {
            tgt_color: tgt_color_view,
            tgt_depth: tgt_depth_view,
            tgt_color_pp: tgt_color_pp_view,
            win_depth: win_depth_view,
        })
    }

    fn create_dummy_shadow_tex(device: &wgpu::Device, queue: &wgpu::Queue) -> (Texture, Texture) {
        let make_tex = |view_dim, depth| {
            let tex = wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: 4,
                    height: 4,
                    depth_or_array_layers: depth,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth24Plus,
                usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::RENDER_ATTACHMENT,
            };

            let view = wgpu::TextureViewDescriptor {
                label: None,
                format: Some(wgpu::TextureFormat::Depth24Plus),
                dimension: Some(view_dim),
                aspect: wgpu::TextureAspect::DepthOnly,
                base_mip_level: 0,
                level_count: None,
                base_array_layer: 0,
                array_layer_count: None,
            };

            let sampler_info = wgpu::SamplerDescriptor {
                label: None,
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                compare: Some(wgpu::CompareFunction::LessEqual),
                ..Default::default()
            };

            Texture::new_raw(device, &tex, &view, &sampler_info)
        };

        let cube_tex = make_tex(wgpu::TextureViewDimension::Cube, 6);
        let tex = make_tex(wgpu::TextureViewDimension::D2, 1);

        // Clear to 1.0
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Dummy shadow tex clearing encoder"),
        });
        let mut clear = |tex: &Texture| {
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear dummy shadow texture"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                    attachment: &tex.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
        };
        clear(&cube_tex);
        clear(&tex);
        drop(clear);
        queue.submit(std::iter::once(encoder.finish()));

        (cube_tex, tex)
    }

    /// Create textures and views for shadow maps.
    // This is a one-use type and the two halves are not guaranteed to remain identical, so we
    // disable the type complexity lint.
    #[allow(clippy::type_complexity)]
    fn create_shadow_views(
        device: &wgpu::Device,
        size: (u32, u32),
        mode: &ShadowMapMode,
    ) -> Result<(Texture, Texture), RenderError> {
        // (Attempt to) apply resolution factor to shadow map resolution.
        let resolution_factor = mode.resolution.clamped(0.25, 4.0);

        let max_texture_size = Self::max_texture_size_raw(device);
        // Limit to max texture size, rather than erroring.
        let size = Vec2::new(size.0, size.1).map(|e| {
            let size = e as f32 * resolution_factor;
            // NOTE: We know 0 <= e since we clamped the resolution factor to be between
            // 0.25 and 4.0.
            if size <= max_texture_size as f32 {
                size as u32
            } else {
                max_texture_size
            }
        });

        let levels = 1;
        // Limit to max texture size rather than erroring.
        let two_size = size.map(|e| {
            u32::checked_next_power_of_two(e)
                .filter(|&e| e <= max_texture_size)
                .unwrap_or(max_texture_size)
        });
        let min_size = size.reduce_min();
        let max_size = size.reduce_max();
        let _min_two_size = two_size.reduce_min();
        let _max_two_size = two_size.reduce_max();
        // For rotated shadow maps, the maximum size of a pixel along any axis is the
        // size of a diagonal along that axis.
        let diag_size = size.map(f64::from).magnitude();
        let diag_cross_size = f64::from(min_size) / f64::from(max_size) * diag_size;
        let (diag_size, _diag_cross_size) =
            if 0.0 < diag_size && diag_size <= f64::from(max_texture_size) {
                // NOTE: diag_cross_size must be non-negative, since it is the ratio of a
                // non-negative and a positive number (if max_size were zero,
                // diag_size would be 0 too).  And it must be <= diag_size,
                // since min_size <= max_size.  Therefore, if diag_size fits in a
                // u16, so does diag_cross_size.
                (diag_size as u32, diag_cross_size as u32)
            } else {
                // Limit to max texture resolution rather than error.
                (max_texture_size as u32, max_texture_size as u32)
            };
        let diag_two_size = u32::checked_next_power_of_two(diag_size)
            .filter(|&e| e <= max_texture_size)
            // Limit to max texture resolution rather than error.
            .unwrap_or(max_texture_size);

        let point_shadow_tex = wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: diag_two_size / 4,
                height: diag_two_size / 4,
                depth_or_array_layers: 6,
            },
            mip_level_count: levels,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::RENDER_ATTACHMENT,
        };

        //TODO: (0, levels - 1), ?? from master
        let point_shadow_view = wgpu::TextureViewDescriptor {
            label: None,
            format: Some(wgpu::TextureFormat::Depth24Plus),
            dimension: Some(wgpu::TextureViewDimension::Cube),
            aspect: wgpu::TextureAspect::DepthOnly,
            base_mip_level: 0,
            level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        };

        let directed_shadow_tex = wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: diag_two_size,
                height: diag_two_size,
                depth_or_array_layers: 1,
            },
            mip_level_count: levels,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::RENDER_ATTACHMENT,
        };

        let directed_shadow_view = wgpu::TextureViewDescriptor {
            label: None,
            format: Some(wgpu::TextureFormat::Depth24Plus),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::DepthOnly,
            base_mip_level: 0,
            level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        };

        let sampler_info = wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        };

        let point_shadow_tex =
            Texture::new_raw(device, &point_shadow_tex, &point_shadow_view, &sampler_info);
        let directed_shadow_tex = Texture::new_raw(
            device,
            &directed_shadow_tex,
            &directed_shadow_view,
            &sampler_info,
        );

        Ok((point_shadow_tex, directed_shadow_tex))
    }

    /// Get the resolution of the render target.
    pub fn resolution(&self) -> Vec2<u32> { self.resolution }

    /// Get the resolution of the shadow render target.
    pub fn get_shadow_resolution(&self) -> (Vec2<u32>, Vec2<u32>) {
        if let ShadowMap::Enabled(shadow_map) = &self.shadow.map {
            (
                shadow_map.point_depth.get_dimensions().xy(),
                shadow_map.directed_depth.get_dimensions().xy(),
            )
        } else {
            (Vec2::new(1, 1), Vec2::new(1, 1))
        }
    }

    // /// Queue the clearing of the shadow targets ready for a new frame to be
    // /// rendered.
    // pub fn clear_shadows(&mut self) {
    //     span!(_guard, "clear_shadows", "Renderer::clear_shadows");
    //     if !self.mode.shadow.is_map() {
    //         return;
    //     }
    //     if let Some(shadow_map) = self.shadow_map.as_mut() {
    //         // let point_encoder = &mut shadow_map.point_encoder;
    //         let point_encoder = &mut self.encoder;
    //         point_encoder.clear_depth(&shadow_map.point_depth_view, 1.0);
    //         // let directed_encoder = &mut shadow_map.directed_encoder;
    //         let directed_encoder = &mut self.encoder;
    //         directed_encoder.clear_depth(&shadow_map.directed_depth_view,
    // 1.0);     }
    // }

    /// NOTE: Supported by Vulkan (by default), DirectX 10+ (it seems--it's hard
    /// to find proof of this, but Direct3D 10 apparently does it by
    /// default, and 11 definitely does, so I assume it's natively supported
    /// by DirectX itself), OpenGL 3.2+, and Metal (done by default).  While
    /// there may be some GPUs that don't quite support it correctly, the
    /// impact is relatively small, so there is no reason not to enable it where
    /// available.
    fn enable_seamless_cube_maps() {
        todo!()
        // unsafe {
        //     // NOTE: Currently just fail silently rather than complain if the
        // computer is on     // a version lower than 3.2, where
        // seamless cubemaps were introduced.     if !device.get_info().
        // is_version_supported(3, 2) {         return;
        //     }

        //     // NOTE: Safe because GL_TEXTURE_CUBE_MAP_SEAMLESS is supported
        // by OpenGL 3.2+     // (see https://www.khronos.org/opengl/wiki/Cubemap_Texture#Seamless_cubemap);
        //     // enabling seamless cube maps should always be safe regardless
        // of the state of     // the OpenGL context, so no further
        // checks are needed.     device.with_gl(|gl| {
        //         gl.Enable(gfx_gl::TEXTURE_CUBE_MAP_SEAMLESS);
        //     });
        // }
    }

    /// Start recording the frame
    /// When the returned `Drawer` is dropped the recorded draw calls will be
    /// submitted to the queue
    /// If there is an intermittent issue with the swap chain then Ok(None) will
    /// be returned
    pub fn start_recording_frame<'a>(
        &'a mut self,
        globals: &'a GlobalsBindGroup,
    ) -> Result<Option<drawer::Drawer<'a>>, RenderError> {
        span!(
            _guard,
            "start_recording_frame",
            "Renderer::start_recording_frame"
        );

        // TODO: does this make sense here?
        self.device.poll(wgpu::Maintain::Poll);

        // If the shaders files were changed attempt to recreate the shaders
        if self.shaders.reloaded() {
            self.recreate_pipelines();
        }

        let tex = match self.swap_chain.get_current_frame() {
            Ok(frame) => frame.output,
            // If lost recreate the swap chain
            Err(err @ wgpu::SwapChainError::Lost) => {
                warn!("{}. Recreating swap chain. A frame will be missed", err);
                return self.on_resize(self.resolution).map(|()| None);
            },
            Err(wgpu::SwapChainError::Timeout) => {
                // This will probably be resolved on the next frame
                // NOTE: we don't log this because it happens very frequently with
                // PresentMode::Fifo on certain machines
                return Ok(None);
            },
            Err(err @ wgpu::SwapChainError::Outdated) => {
                warn!("{}. This will probably be resolved on the next frame", err);
                return Ok(None);
            },
            Err(err @ wgpu::SwapChainError::OutOfMemory) => return Err(err.into()),
        };
        let encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("A render encoder"),
            });

        Ok(Some(drawer::Drawer::new(encoder, self, tex, globals)))
    }

    /// Recreate the pipelines
    fn recreate_pipelines(&mut self) {
        match create_pipelines(
            &self.device,
            &self.layouts,
            &self.shaders.read(),
            &self.mode,
            &self.sc_desc,
            self.shadow.map.is_enabled(),
        ) {
            Ok((
                pipelines,
                //player_shadow_pipeline,
                point_shadow_pipeline,
                terrain_directed_shadow_pipeline,
                figure_directed_shadow_pipeline,
            )) => {
                self.pipelines = pipelines;
                //self.player_shadow_pipeline = player_shadow_pipeline;
                if let (
                    Some(point_pipeline),
                    Some(terrain_directed_pipeline),
                    Some(figure_directed_pipeline),
                    ShadowMap::Enabled(shadow_map),
                ) = (
                    point_shadow_pipeline,
                    terrain_directed_shadow_pipeline,
                    figure_directed_shadow_pipeline,
                    &mut self.shadow.map,
                ) {
                    shadow_map.point_pipeline = point_pipeline;
                    shadow_map.terrain_directed_pipeline = terrain_directed_pipeline;
                    shadow_map.figure_directed_pipeline = figure_directed_pipeline;
                }
            },
            Err(e) => error!(?e, "Could not recreate shaders from assets due to an error",),
        }
    }

    /// Create a new set of constants with the provided values.
    pub fn create_consts<T: Copy + bytemuck::Pod>(&mut self, vals: &[T]) -> Consts<T> {
        Self::create_consts_inner(&self.device, &self.queue, vals)
    }

    pub fn create_consts_inner<T: Copy + bytemuck::Pod>(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vals: &[T],
    ) -> Consts<T> {
        let mut consts = Consts::new(device, vals.len());
        consts.update(device, queue, vals, 0);
        consts
    }

    /// Update a set of constants with the provided values.
    pub fn update_consts<T: Copy + bytemuck::Pod>(&self, consts: &mut Consts<T>, vals: &[T]) {
        consts.update(&self.device, &self.queue, vals, 0)
    }

    pub fn update_clouds_locals(&mut self, new_val: clouds::Locals) {
        self.locals
            .clouds
            .update(&self.device, &self.queue, &[new_val], 0)
    }

    pub fn update_postprocess_locals(&mut self, new_val: postprocess::Locals) {
        self.locals
            .postprocess
            .update(&self.device, &self.queue, &[new_val], 0)
    }

    /// Create a new set of instances with the provided values.
    pub fn create_instances<T: Copy + bytemuck::Pod>(
        &mut self,
        vals: &[T],
    ) -> Result<Instances<T>, RenderError> {
        let mut instances = Instances::new(&self.device, vals.len());
        instances.update(&self.device, &self.queue, vals, 0);
        Ok(instances)
    }

    /// Ensure that the quad index buffer is large enough for a quad vertex
    /// buffer with this many vertices
    pub(super) fn ensure_sufficient_index_length<V: Vertex>(
        &mut self,
        // Length of the vert buffer with 4 verts per quad
        length: usize,
    ) {
        match V::QUADS_INDEX {
            Some(wgpu::IndexFormat::Uint16) => {
                // Make sure the global quad index buffer is large enough
                if self.quad_index_buffer_u16.len() < length {
                    // Make sure we aren't over the max
                    if length > u16::MAX as usize {
                        panic!(
                            "Vertex type: {} needs to use a larger index type, length: {}",
                            core::any::type_name::<V>(),
                            length
                        );
                    }
                    self.quad_index_buffer_u16 = create_quad_index_buffer_u16(&self.device, length);
                }
            },
            Some(wgpu::IndexFormat::Uint32) => {
                // Make sure the global quad index buffer is large enough
                if self.quad_index_buffer_u32.len() < length {
                    // Make sure we aren't over the max
                    if length > u32::MAX as usize {
                        panic!(
                            "More than u32::MAX({}) verts({}) for type({}) using an index buffer!",
                            u32::MAX,
                            length,
                            core::any::type_name::<V>()
                        );
                    }
                    self.quad_index_buffer_u32 = create_quad_index_buffer_u32(&self.device, length);
                }
            },
            None => {},
        }
    }

    /// Create a new model from the provided mesh.
    pub fn create_model<V: Vertex>(&mut self, mesh: &Mesh<V>) -> Result<Model<V>, RenderError> {
        self.ensure_sufficient_index_length::<V>(mesh.vertices().len());
        Ok(Model::new(&self.device, mesh))
    }

    /// Create a new dynamic model with the specified size.
    pub fn create_dynamic_model<V: Vertex>(&mut self, size: usize) -> DynamicModel<V> {
        DynamicModel::new(&self.device, size)
    }

    /// Update a dynamic model with a mesh and a offset.
    pub fn update_model<V: Vertex>(&self, model: &DynamicModel<V>, mesh: &Mesh<V>, offset: usize) {
        model.update(&self.device, &self.queue, mesh, offset)
    }

    /// Return the maximum supported texture size.
    pub fn max_texture_size(&self) -> u32 { Self::max_texture_size_raw(&self.device) }

    /// Return the maximum supported texture size from the factory.
    fn max_texture_size_raw(_device: &wgpu::Device) -> u32 {
        // This value is temporary as there are plans to include a way to get this in
        // wgpu this is just a sane standard for now
        8192
    }

    /// Create a new immutable texture from the provided image.
    pub fn create_texture_with_data_raw<const BYTES_PER_PIXEL: u32>(
        &mut self,
        texture_info: &wgpu::TextureDescriptor,
        view_info: &wgpu::TextureViewDescriptor,
        sampler_info: &wgpu::SamplerDescriptor,
        data: &[u8],
    ) -> Texture {
        let tex = Texture::new_raw(&self.device, &texture_info, &view_info, &sampler_info);

        tex.update::<BYTES_PER_PIXEL>(
            &self.device,
            &self.queue,
            [0; 2],
            [texture_info.size.width, texture_info.size.height],
            data,
        );

        tex
    }

    /// Create a new raw texture.
    pub fn create_texture_raw(
        &mut self,
        texture_info: &wgpu::TextureDescriptor,
        view_info: &wgpu::TextureViewDescriptor,
        sampler_info: &wgpu::SamplerDescriptor,
    ) -> Texture {
        Texture::new_raw(&self.device, texture_info, view_info, sampler_info)
    }

    /// Create a new texture from the provided image.
    ///
    /// Currently only supports Rgba8Srgb
    pub fn create_texture(
        &mut self,
        image: &image::DynamicImage,
        filter_method: Option<FilterMode>,
        address_mode: Option<AddressMode>,
    ) -> Result<Texture, RenderError> {
        Texture::new(
            &self.device,
            &self.queue,
            image,
            filter_method,
            address_mode,
        )
    }

    /// Create a new dynamic texture with the
    /// specified dimensions.
    ///
    /// Currently only supports Rgba8Srgb
    pub fn create_dynamic_texture(&mut self, dims: Vec2<u32>) -> Texture {
        Texture::new_dynamic(&self.device, dims.x, dims.y)
    }

    /// Update a texture with the provided offset, size, and data.
    ///
    /// Currently only supports Rgba8Srgb
    pub fn update_texture(
        &mut self,
        texture: &Texture, /* <T> */
        offset: [u32; 2],
        size: [u32; 2],
        // TODO
        //        data: &[<<T as gfx::format::Formatted>::Surface as
        // gfx::format::SurfaceTyped>::DataType],    ) -> Result<(), RenderError>
        //    where
        //        <T as gfx::format::Formatted>::Surface: gfx::format::TextureSurface,
        //        <T as gfx::format::Formatted>::Channel: gfx::format::TextureChannel,
        //        <<T as gfx::format::Formatted>::Surface as gfx::format::SurfaceTyped>::DataType:
        // Copy,    {
        //        texture.update(&mut self.encoder, offset, size, data)
        data: &[[u8; 4]],
    ) {
        texture.update::<4>(
            &self.device,
            &self.queue,
            offset,
            size,
            bytemuck::cast_slice(data),
        )
    }

    /// Creates a download buffer, downloads the win_color_view, and converts to
    /// a image::DynamicImage.
    //pub fn create_screenshot(&mut self) -> Result<image::DynamicImage,
    // RenderError> {
    pub fn create_screenshot(&mut self) {
        // TODO: check if profiler enabled
        // TODO: save alongside a screenshot
        // Ensure timestamp query data buffers are mapped
        self.device.poll(wgpu::Maintain::Wait);
        // Take profiler snapshot
        let profiling_data = if let Some(data) = self.profiler.process_finished_frame() {
            data
        } else {
            error!("Failed to retrieve profiling data");
            return;
        };

        let file_name = format!(
            "frame-trace_{}.json",
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        );

        wgpu_profiler::chrometrace::write_chrometrace(
            std::path::Path::new(&file_name),
            &profiling_data,
        );

        println!("{}", file_name);
        //todo!()
        // let (width, height) = self.get_resolution().into_tuple();

        // let download_buf = self
        //     .device
        //     .create_buffer(&wgpu::BufferDescriptor {
        //         label: None,
        //         size: width * height * 4,
        //         usage : wgpu::BufferUsage::COPY_DST,
        //         mapped_at_creation: true
        //     });

        // let encoder =
        // self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor
        // {label: None});

        //     encoder.copy_texture_to_buffer(&wgpu::TextureCopyViewBase {
        //         origin: &self.wi
        //     }, destination, copy_size)

        // self.encoder.copy_texture_to_buffer_raw(
        //     self.win_color_view.raw().get_texture(),
        //     None,
        //     gfx::texture::RawImageInfo {
        //         xoffset: 0,
        //         yoffset: 0,
        //         zoffset: 0,
        //         width,
        //         height,
        //         depth: 0,
        //         format: WinColorFmt::get_format(),
        //         mipmap: 0,
        //     },
        //     download.raw(),
        //     0,
        // )?;
        // self.flush();

        // // Assumes that the format is Rgba8.
        // let raw_data = self
        //     .factory
        //     .read_mapping(&download)?
        //     .chunks_exact(width as usize)
        //     .rev()
        //     .flatten()
        //     .flatten()
        //     .map(|&e| e)
        //     .collect::<Vec<_>>();
        // Ok(image::DynamicImage::ImageRgba8(
        //     // Should not fail if the dimensions are correct.
        //     image::ImageBuffer::from_raw(width as u32, height as u32,
        // raw_data).unwrap(), ))
    }

    // /// Queue the rendering of the provided skybox model in the upcoming frame.
    // pub fn render_skybox(
    //     &mut self,
    //     model: &Model<skybox::SkyboxPipeline>,
    //     global: &GlobalModel,
    //     locals: &Consts<skybox::Locals>,
    //     lod: &lod_terrain::LodData,
    // ) {
    //     self.encoder.draw(
    //         &gfx::Slice {
    //             start: model.vertex_range().start,
    //             end: model.vertex_range().end,
    //             base_vertex: 0,
    //             instances: None,
    //             buffer: gfx::IndexBuffer::Auto,
    //         },
    //         &self.skybox_pipeline.pso,
    //         &skybox::pipe::Data {
    //             vbuf: model.vbuf.clone(),
    //             locals: locals.buf.clone(),
    //             globals: global.globals.buf.clone(),
    //             noise: (self.noise_tex.srv.clone(),
    // self.noise_tex.sampler.clone()),             alt: (lod.alt.srv.clone(),
    // lod.alt.sampler.clone()),             horizon: (lod.horizon.srv.clone(),
    // lod.horizon.sampler.clone()),             tgt_color:
    // self.tgt_color_view.clone(),             tgt_depth:
    // (self.tgt_depth_view.clone()/* , (1, 1) */),         },
    //     );
    // }

    // /// Queue the rendering of the provided figure model in the upcoming frame.
    // pub fn render_figure(
    //     &mut self,
    //     model: &figure::FigureModel,
    //     col_lights: &Texture<ColLightFmt>,
    //     global: &GlobalModel,
    //     locals: &Consts<figure::Locals>,
    //     bones: &Consts<figure::BoneData>,
    //     lod: &lod_terrain::LodData,
    // ) {
    //     let (point_shadow_maps, directed_shadow_maps) =
    //         if let Some(shadow_map) = &mut self.shadow_map {
    //             (
    //                 (
    //                     shadow_map.point_res.clone(),
    //                     shadow_map.point_sampler.clone(),
    //                 ),
    //                 (
    //                     shadow_map.directed_res.clone(),
    //                     shadow_map.directed_sampler.clone(),
    //                 ),
    //             )
    //         } else {
    //             (
    //                 (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
    //                 (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
    //             )
    //         };
    //     let model = &model.opaque;

    //     self.encoder.draw(
    //         &gfx::Slice {
    //             start: model.vertex_range().start,
    //             end: model.vertex_range().end,
    //             base_vertex: 0,
    //             instances: None,
    //             buffer: gfx::IndexBuffer::Auto,
    //         },
    //         &self.figure_pipeline.pso,
    //         &figure::pipe::Data {
    //             vbuf: model.vbuf.clone(),
    //             col_lights: (col_lights.srv.clone(), col_lights.sampler.clone()),
    //             locals: locals.buf.clone(),
    //             globals: global.globals.buf.clone(),
    //             bones: bones.buf.clone(),
    //             lights: global.lights.buf.clone(),
    //             shadows: global.shadows.buf.clone(),
    //             light_shadows: global.shadow_mats.buf.clone(),
    //             point_shadow_maps,
    //             directed_shadow_maps,
    //             noise: (self.noise_tex.srv.clone(),
    // self.noise_tex.sampler.clone()),             alt: (lod.alt.srv.clone(),
    // lod.alt.sampler.clone()),             horizon: (lod.horizon.srv.clone(),
    // lod.horizon.sampler.clone()),             tgt_color:
    // self.tgt_color_view.clone(),             tgt_depth:
    // (self.tgt_depth_view.clone()/* , (1, 1) */),         },
    //     );
    // }

    // /// Queue the rendering of the player silhouette in the upcoming frame.
    // pub fn render_player_shadow(
    //     &mut self,
    //     _model: &figure::FigureModel,
    //     _col_lights: &Texture<ColLightFmt>,
    //     _global: &GlobalModel,
    //     _bones: &Consts<figure::BoneData>,
    //     _lod: &lod_terrain::LodData,
    //     _locals: &Consts<shadow::Locals>,
    // ) {
    //     // FIXME: Consider reenabling at some point.
    //     /* let (point_shadow_maps, directed_shadow_maps) =
    //         if let Some(shadow_map) = &mut self.shadow_map {
    //             (
    //                 (
    //                     shadow_map.point_res.clone(),
    //                     shadow_map.point_sampler.clone(),
    //                 ),
    //                 (
    //                     shadow_map.directed_res.clone(),
    //                     shadow_map.directed_sampler.clone(),
    //                 ),
    //             )
    //         } else {
    //             (
    //                 (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
    //                 (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
    //             )
    //         };
    //     let model = &model.opaque;

    //     self.encoder.draw(
    //         &gfx::Slice {
    //             start: model.vertex_range().start,
    //             end: model.vertex_range().end,
    //             base_vertex: 0,
    //             instances: None,
    //             buffer: gfx::IndexBuffer::Auto,
    //         },
    //         &self.player_shadow_pipeline.pso,
    //         &figure::pipe::Data {
    //             vbuf: model.vbuf.clone(),
    //             col_lights: (col_lights.srv.clone(), col_lights.sampler.clone()),
    //             locals: locals.buf.clone(),
    //             globals: global.globals.buf.clone(),
    //             bones: bones.buf.clone(),
    //             lights: global.lights.buf.clone(),
    //             shadows: global.shadows.buf.clone(),
    //             light_shadows: global.shadow_mats.buf.clone(),
    //             point_shadow_maps,
    //             directed_shadow_maps,
    //             noise: (self.noise_tex.srv.clone(),
    // self.noise_tex.sampler.clone()),             alt: (lod.alt.srv.clone(),
    // lod.alt.sampler.clone()),             horizon: (lod.horizon.srv.clone(),
    // lod.horizon.sampler.clone()),             tgt_color:
    // self.tgt_color_view.clone(),             tgt_depth:
    // (self.tgt_depth_view.clone()/* , (0, 0) */),         },
    //     ); */
    // }

    // /// Queue the rendering of the player model in the upcoming frame.
    // pub fn render_player(
    //     &mut self,
    //     model: &figure::FigureModel,
    //     col_lights: &Texture<ColLightFmt>,
    //     global: &GlobalModel,
    //     locals: &Consts<figure::Locals>,
    //     bones: &Consts<figure::BoneData>,
    //     lod: &lod_terrain::LodData,
    // ) {
    //     let (point_shadow_maps, directed_shadow_maps) =
    //         if let Some(shadow_map) = &mut self.shadow_map {
    //             (
    //                 (
    //                     shadow_map.point_res.clone(),
    //                     shadow_map.point_sampler.clone(),
    //                 ),
    //                 (
    //                     shadow_map.directed_res.clone(),
    //                     shadow_map.directed_sampler.clone(),
    //                 ),
    //             )
    //         } else {
    //             (
    //                 (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
    //                 (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
    //             )
    //         };
    //     let model = &model.opaque;

    //     self.encoder.draw(
    //         &gfx::Slice {
    //             start: model.vertex_range().start,
    //             end: model.vertex_range().end,
    //             base_vertex: 0,
    //             instances: None,
    //             buffer: gfx::IndexBuffer::Auto,
    //         },
    //         &self.figure_pipeline.pso,
    //         &figure::pipe::Data {
    //             vbuf: model.vbuf.clone(),
    //             col_lights: (col_lights.srv.clone(), col_lights.sampler.clone()),
    //             locals: locals.buf.clone(),
    //             globals: global.globals.buf.clone(),
    //             bones: bones.buf.clone(),
    //             lights: global.lights.buf.clone(),
    //             shadows: global.shadows.buf.clone(),
    //             light_shadows: global.shadow_mats.buf.clone(),
    //             point_shadow_maps,
    //             directed_shadow_maps,
    //             noise: (self.noise_tex.srv.clone(),
    // self.noise_tex.sampler.clone()),             alt: (lod.alt.srv.clone(),
    // lod.alt.sampler.clone()),             horizon: (lod.horizon.srv.clone(),
    // lod.horizon.sampler.clone()),             tgt_color:
    // self.tgt_color_view.clone(),             tgt_depth:
    // (self.tgt_depth_view.clone()/* , (1, 1) */),         },
    //     );
    // }

    // /// Queue the rendering of the provided terrain chunk model in the upcoming
    // /// frame.
    // pub fn render_terrain_chunk(
    //     &mut self,
    //     model: &Model<terrain::TerrainPipeline>,
    //     col_lights: &Texture<ColLightFmt>,
    //     global: &GlobalModel,
    //     locals: &Consts<terrain::Locals>,
    //     lod: &lod_terrain::LodData,
    // ) {
    //     let (point_shadow_maps, directed_shadow_maps) =
    //         if let Some(shadow_map) = &mut self.shadow_map {
    //             (
    //                 (
    //                     shadow_map.point_res.clone(),
    //                     shadow_map.point_sampler.clone(),
    //                 ),
    //                 (
    //                     shadow_map.directed_res.clone(),
    //                     shadow_map.directed_sampler.clone(),
    //                 ),
    //             )
    //         } else {
    //             (
    //                 (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
    //                 (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
    //             )
    //         };

    //     self.encoder.draw(
    //         &gfx::Slice {
    //             start: model.vertex_range().start,
    //             end: model.vertex_range().end,
    //             base_vertex: 0,
    //             instances: None,
    //             buffer: gfx::IndexBuffer::Auto,
    //         },
    //         &self.terrain_pipeline.pso,
    //         &terrain::pipe::Data {
    //             vbuf: model.vbuf.clone(),
    //             // TODO: Consider splitting out texture atlas data into a
    // separate vertex buffer,             // since we don't need it for things
    // like global.shadows.             col_lights: (col_lights.srv.clone(),
    // col_lights.sampler.clone()),             locals: locals.buf.clone(),
    //             globals: global.globals.buf.clone(),
    //             lights: global.lights.buf.clone(),
    //             shadows: global.shadows.buf.clone(),
    //             light_shadows: global.shadow_mats.buf.clone(),
    //             point_shadow_maps,
    //             directed_shadow_maps,
    //             noise: (self.noise_tex.srv.clone(),
    // self.noise_tex.sampler.clone()),             alt: (lod.alt.srv.clone(),
    // lod.alt.sampler.clone()),             horizon: (lod.horizon.srv.clone(),
    // lod.horizon.sampler.clone()),             tgt_color:
    // self.tgt_color_view.clone(),             tgt_depth:
    // (self.tgt_depth_view.clone()/* , (1, 1) */),         },
    //     );
    // }

    // /// Queue the rendering of a shadow map from a point light in the upcoming
    // /// frame.
    // pub fn render_shadow_point(
    //     &mut self,
    //     model: &Model<terrain::TerrainPipeline>,
    //     global: &GlobalModel,
    //     terrain_locals: &Consts<terrain::Locals>,
    //     locals: &Consts<shadow::Locals>,
    // ) {
    //     if !self.mode.shadow.is_map() {
    //         return;
    //     }
    //     // NOTE: Don't render shadows if the shader is not supported.
    //     let shadow_map = if let Some(shadow_map) = &mut self.shadow_map {
    //         shadow_map
    //     } else {
    //         return;
    //     };

    //     // let point_encoder = &mut shadow_map.point_encoder;
    //     let point_encoder = &mut self.encoder;
    //     point_encoder.draw(
    //         &gfx::Slice {
    //             start: model.vertex_range().start,
    //             end: model.vertex_range().end,
    //             base_vertex: 0,
    //             instances: None,
    //             buffer: gfx::IndexBuffer::Auto,
    //         },
    //         &shadow_map.point_pipeline.pso,
    //         &shadow::pipe::Data {
    //             // Terrain vertex stuff
    //             vbuf: model.vbuf.clone(),
    //             locals: terrain_locals.buf.clone(),
    //             globals: global.globals.buf.clone(),

    //             // Shadow stuff
    //             light_shadows: locals.buf.clone(),
    //             tgt_depth: shadow_map.point_depth_view.clone(),
    //         },
    //     );
    // }

    // /// Queue the rendering of terrain shadow map from all directional lights in
    // /// the upcoming frame.
    // pub fn render_terrain_shadow_directed(
    //     &mut self,
    //     model: &Model<terrain::TerrainPipeline>,
    //     global: &GlobalModel,
    //     terrain_locals: &Consts<terrain::Locals>,
    //     locals: &Consts<shadow::Locals>,
    // ) {
    //     if !self.mode.shadow.is_map() {
    //         return;
    //     }
    //     // NOTE: Don't render shadows if the shader is not supported.
    //     let shadow_map = if let Some(shadow_map) = &mut self.shadow_map {
    //         shadow_map
    //     } else {
    //         return;
    //     };

    //     // let directed_encoder = &mut shadow_map.directed_encoder;
    //     let directed_encoder = &mut self.encoder;
    //     directed_encoder.draw(
    //         &gfx::Slice {
    //             start: model.vertex_range().start,
    //             end: model.vertex_range().end,
    //             base_vertex: 0,
    //             instances: None,
    //             buffer: gfx::IndexBuffer::Auto,
    //         },
    //         &shadow_map.terrain_directed_pipeline.pso,
    //         &shadow::pipe::Data {
    //             // Terrain vertex stuff
    //             vbuf: model.vbuf.clone(),
    //             locals: terrain_locals.buf.clone(),
    //             globals: global.globals.buf.clone(),

    //             // Shadow stuff
    //             light_shadows: locals.buf.clone(),
    //             tgt_depth:
    // shadow_map.directed_depth_view.clone(),         },
    //     );
    // }

    // /// Queue the rendering of figure shadow map from all directional lights in
    // /// the upcoming frame.
    // pub fn render_figure_shadow_directed(
    //     &mut self,
    //     model: &figure::FigureModel,
    //     global: &GlobalModel,
    //     figure_locals: &Consts<figure::Locals>,
    //     bones: &Consts<figure::BoneData>,
    //     locals: &Consts<shadow::Locals>,
    // ) {
    //     if !self.mode.shadow.is_map() {
    //         return;
    //     }
    //     // NOTE: Don't render shadows if the shader is not supported.
    //     let shadow_map = if let Some(shadow_map) = &mut self.shadow_map {
    //         shadow_map
    //     } else {
    //         return;
    //     };
    //     let model = &model.opaque;

    //     // let directed_encoder = &mut shadow_map.directed_encoder;
    //     let directed_encoder = &mut self.encoder;
    //     directed_encoder.draw(
    //         &gfx::Slice {
    //             start: model.vertex_range().start,
    //             end: model.vertex_range().end,
    //             base_vertex: 0,
    //             instances: None,
    //             buffer: gfx::IndexBuffer::Auto,
    //         },
    //         &shadow_map.figure_directed_pipeline.pso,
    //         &shadow::figure_pipe::Data {
    //             // Terrain vertex stuff
    //             vbuf: model.vbuf.clone(),
    //             locals: figure_locals.buf.clone(),
    //             bones: bones.buf.clone(),
    //             globals: global.globals.buf.clone(),

    //             // Shadow stuff
    //             light_shadows: locals.buf.clone(),
    //             tgt_depth:
    // shadow_map.directed_depth_view.clone(),         },
    //     );
    // }

    // /// Queue the rendering of the provided terrain chunk model in the upcoming
    // /// frame.
    // pub fn render_fluid_chunk(
    //     &mut self,
    //     model: &Model<fluid::FluidPipeline>,
    //     global: &GlobalModel,
    //     locals: &Consts<terrain::Locals>,
    //     lod: &lod_terrain::LodData,
    //     waves: &Texture,
    // ) {
    //     let (point_shadow_maps, directed_shadow_maps) =
    //         if let Some(shadow_map) = &mut self.shadow_map {
    //             (
    //                 (
    //                     shadow_map.point_res.clone(),
    //                     shadow_map.point_sampler.clone(),
    //                 ),
    //                 (
    //                     shadow_map.directed_res.clone(),
    //                     shadow_map.directed_sampler.clone(),
    //                 ),
    //             )
    //         } else {
    //             (
    //                 (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
    //                 (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
    //             )
    //         };

    //     self.encoder.draw(
    //         &gfx::Slice {
    //             start: model.vertex_range().start,
    //             end: model.vertex_range().end,
    //             base_vertex: 0,
    //             instances: None,
    //             buffer: gfx::IndexBuffer::Auto,
    //         },
    //         &self.fluid_pipeline.pso,
    //         &fluid::pipe::Data {
    //             vbuf: model.vbuf.clone(),
    //             locals: locals.buf.clone(),
    //             globals: global.globals.buf.clone(),
    //             lights: global.lights.buf.clone(),
    //             shadows: global.shadows.buf.clone(),
    //             light_shadows: global.shadow_mats.buf.clone(),
    //             point_shadow_maps,
    //             directed_shadow_maps,
    //             alt: (lod.alt.srv.clone(), lod.alt.sampler.clone()),
    //             horizon: (lod.horizon.srv.clone(), lod.horizon.sampler.clone()),
    //             noise: (self.noise_tex.srv.clone(),
    // self.noise_tex.sampler.clone()),             waves: (waves.srv.clone(),
    // waves.sampler.clone()),             tgt_color:
    // self.tgt_color_view.clone(),             tgt_depth:
    // (self.tgt_depth_view.clone()/* , (1, 1) */),         },
    //     );
    // }

    // /// Queue the rendering of the provided terrain chunk model in the upcoming
    // /// frame.
    // pub fn render_sprites(
    //     &mut self,
    //     model: &Model<sprite::SpritePipeline>,
    //     col_lights: &Texture<ColLightFmt>,
    //     global: &GlobalModel,
    //     terrain_locals: &Consts<terrain::Locals>,
    //     locals: &Consts<sprite::Locals>,
    //     instances: &Instances<sprite::Instance>,
    //     lod: &lod_terrain::LodData,
    // ) {
    //     let (point_shadow_maps, directed_shadow_maps) =
    //         if let Some(shadow_map) = &mut self.shadow_map {
    //             (
    //                 (
    //                     shadow_map.point_res.clone(),
    //                     shadow_map.point_sampler.clone(),
    //                 ),
    //                 (
    //                     shadow_map.directed_res.clone(),
    //                     shadow_map.directed_sampler.clone(),
    //                 ),
    //             )
    //         } else {
    //             (
    //                 (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
    //                 (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
    //             )
    //         };

    //     self.encoder.draw(
    //         &gfx::Slice {
    //             start: model.vertex_range().start,
    //             end: model.vertex_range().end,
    //             base_vertex: 0,
    //             instances: Some((instances.count() as u32, 0)),
    //             buffer: gfx::IndexBuffer::Auto,
    //         },
    //         &self.sprite_pipeline.pso,
    //         &sprite::pipe::Data {
    //             vbuf: model.vbuf.clone(),
    //             ibuf: instances.ibuf.clone(),
    //             col_lights: (col_lights.srv.clone(), col_lights.sampler.clone()),
    //             terrain_locals: terrain_locals.buf.clone(),
    //             // NOTE: It would be nice if this wasn't needed and we could use
    // a constant buffer             // offset into the sprite data.  Hopefully,
    // when we switch to wgpu we can do this,             // as it offers the
    // exact API we want (the equivalent can be done in OpenGL using
    // // glBindBufferOffset).             locals: locals.buf.clone(),
    //             globals: global.globals.buf.clone(),
    //             lights: global.lights.buf.clone(),
    //             shadows: global.shadows.buf.clone(),
    //             light_shadows: global.shadow_mats.buf.clone(),
    //             point_shadow_maps,
    //             directed_shadow_maps,
    //             noise: (self.noise_tex.srv.clone(),
    // self.noise_tex.sampler.clone()),             alt: (lod.alt.srv.clone(),
    // lod.alt.sampler.clone()),             horizon: (lod.horizon.srv.clone(),
    // lod.horizon.sampler.clone()),             tgt_color:
    // self.tgt_color_view.clone(),             tgt_depth:
    // (self.tgt_depth_view.clone()/* , (1, 1) */),         },
    //     );
    // }

    // /// Queue the rendering of the provided LoD terrain model in the upcoming
    // /// frame.
    // pub fn render_lod_terrain(
    //     &mut self,
    //     model: &Model<lod_terrain::LodTerrainPipeline>,
    //     global: &GlobalModel,
    //     locals: &Consts<lod_terrain::Locals>,
    //     lod: &lod_terrain::LodData,
    // ) {
    //     self.encoder.draw(
    //         &gfx::Slice {
    //             start: model.vertex_range().start,
    //             end: model.vertex_range().end,
    //             base_vertex: 0,
    //             instances: None,
    //             buffer: gfx::IndexBuffer::Auto,
    //         },
    //         &self.lod_terrain_pipeline.pso,
    //         &lod_terrain::pipe::Data {
    //             vbuf: model.vbuf.clone(),
    //             locals: locals.buf.clone(),
    //             globals: global.globals.buf.clone(),
    //             noise: (self.noise_tex.srv.clone(),
    // self.noise_tex.sampler.clone()),             map: (lod.map.srv.clone(),
    // lod.map.sampler.clone()),             alt: (lod.alt.srv.clone(),
    // lod.alt.sampler.clone()),             horizon: (lod.horizon.srv.clone(),
    // lod.horizon.sampler.clone()),             tgt_color:
    // self.tgt_color_view.clone(),             tgt_depth:
    // (self.tgt_depth_view.clone()/* , (1, 1) */),         },
    //     );
    // }

    // /// Queue the rendering of the provided particle in the upcoming frame.
    // pub fn render_particles(
    //     &mut self,
    //     model: &Model<particle::ParticlePipeline>,
    //     global: &GlobalModel,
    //     instances: &Instances<particle::Instance>,
    //     lod: &lod_terrain::LodData,
    // ) {
    //     let (point_shadow_maps, directed_shadow_maps) =
    //         if let Some(shadow_map) = &mut self.shadow_map {
    //             (
    //                 (
    //                     shadow_map.point_res.clone(),
    //                     shadow_map.point_sampler.clone(),
    //                 ),
    //                 (
    //                     shadow_map.directed_res.clone(),
    //                     shadow_map.directed_sampler.clone(),
    //                 ),
    //             )
    //         } else {
    //             (
    //                 (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
    //                 (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
    //             )
    //         };

    //     self.encoder.draw(
    //         &gfx::Slice {
    //             start: model.vertex_range().start,
    //             end: model.vertex_range().end,
    //             base_vertex: 0,
    //             instances: Some((instances.count() as u32, 0)),
    //             buffer: gfx::IndexBuffer::Auto,
    //         },
    //         &self.particle_pipeline.pso,
    //         &particle::pipe::Data {
    //             vbuf: model.vbuf.clone(),
    //             ibuf: instances.ibuf.clone(),
    //             globals: global.globals.buf.clone(),
    //             lights: global.lights.buf.clone(),
    //             shadows: global.shadows.buf.clone(),
    //             light_shadows: global.shadow_mats.buf.clone(),
    //             point_shadow_maps,
    //             directed_shadow_maps,
    //             noise: (self.noise_tex.srv.clone(),
    // self.noise_tex.sampler.clone()),             alt: (lod.alt.srv.clone(),
    // lod.alt.sampler.clone()),             horizon: (lod.horizon.srv.clone(),
    // lod.horizon.sampler.clone()),             tgt_color:
    // self.tgt_color_view.clone(),             tgt_depth:
    // (self.tgt_depth_view.clone()/* , (1, 1) */),         },
    //     );
    // }

    // /// Queue the rendering of the provided UI element in the upcoming frame.
    // pub fn render_ui_element<F: gfx::format::Formatted<View = [f32; 4]>>(
    //     &mut self,
    //     model: Model<ui::UiPipeline>,
    //     tex: &Texture<F>,
    //     scissor: Aabr<u16>,
    //     globals: &Consts<Globals>,
    //     locals: &Consts<ui::Locals>,
    // ) where
    //     F::Surface: gfx::format::TextureSurface,
    //     F::Channel: gfx::format::TextureChannel,
    //     <F::Surface as gfx::format::SurfaceTyped>::DataType: Copy,
    // {
    //     let Aabr { min, max } = scissor;
    //     self.encoder.draw(
    //         &gfx::Slice {
    //             start: model.vertex_range.start,
    //             end: model.vertex_range.end,
    //             base_vertex: 0,
    //             instances: None,
    //             buffer: gfx::IndexBuffer::Auto,
    //         },
    //         &self.ui_pipeline.pso,
    //         &ui::pipe::Data {
    //             vbuf: model.vbuf,
    //             scissor: gfx::Rect {
    //                 x: min.x,
    //                 y: min.y,
    //                 w: max.x - min.x,
    //                 h: max.y - min.y,
    //             },
    //             tex: (tex.srv.clone(), tex.sampler.clone()),
    //             locals: locals.buf.clone(),
    //             globals: globals.buf.clone(),
    //             tgt_color: self.win_color_view.clone(),
    //             tgt_depth: self.win_depth_view.clone(),
    //         },
    //     );
    // }

    // pub fn render_clouds(
    //     &mut self,
    //     model: &Model<clouds::CloudsPipeline>,
    //     globals: &Consts<Globals>,
    //     locals: &Consts<clouds::Locals>,
    //     lod: &lod_terrain::LodData,
    // ) {
    //     self.encoder.draw(
    //         &gfx::Slice {
    //             start: model.vertex_range().start,
    //             end: model.vertex_range().end,
    //             base_vertex: 0,
    //             instances: None,
    //             buffer: gfx::IndexBuffer::Auto,
    //         },
    //         &self.clouds_pipeline.pso,
    //         &clouds::pipe::Data {
    //             vbuf: model.vbuf.clone(),
    //             locals: locals.buf.clone(),
    //             globals: globals.buf.clone(),
    //             map: (lod.map.srv.clone(), lod.map.sampler.clone()),
    //             alt: (lod.alt.srv.clone(), lod.alt.sampler.clone()),
    //             horizon: (lod.horizon.srv.clone(), lod.horizon.sampler.clone()),
    //             color_sampler: (self.tgt_color_res.clone(),
    // self.sampler.clone()),             depth_sampler:
    // (self.tgt_depth_res.clone(), self.sampler.clone()),             noise:
    // (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
    // tgt_color: self.tgt_color_pp_view.clone(),         },
    //     )
    // }

    // pub fn render_post_process(
    //     &mut self,
    //     model: &Model<postprocess::PostProcessPipeline>,
    //     globals: &Consts<Globals>,
    //     locals: &Consts<postprocess::Locals>,
    //     lod: &lod_terrain::LodData,
    // ) {
    //     self.encoder.draw(
    //         &gfx::Slice {
    //             start: model.vertex_range().start,
    //             end: model.vertex_range().end,
    //             base_vertex: 0,
    //             instances: None,
    //             buffer: gfx::IndexBuffer::Auto,
    //         },
    //         &self.postprocess_pipeline.pso,
    //         &postprocess::pipe::Data {
    //             vbuf: model.vbuf.clone(),
    //             locals: locals.buf.clone(),
    //             globals: globals.buf.clone(),
    //             map: (lod.map.srv.clone(), lod.map.sampler.clone()),
    //             alt: (lod.alt.srv.clone(), lod.alt.sampler.clone()),
    //             horizon: (lod.horizon.srv.clone(), lod.horizon.sampler.clone()),
    //             color_sampler: (self.tgt_color_res_pp.clone(),
    // self.sampler.clone()),             depth_sampler:
    // (self.tgt_depth_res.clone(), self.sampler.clone()),             noise:
    // (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
    // tgt_color: self.win_color_view.clone(),         },
    //     )
    // }
}

/// Creates all the pipelines used to render.
fn create_pipelines(
    device: &wgpu::Device,
    layouts: &Layouts,
    shaders: &Shaders,
    mode: &RenderMode,
    sc_desc: &wgpu::SwapChainDescriptor,
    has_shadow_views: bool,
) -> Result<
    (
        Pipelines,
        //figure::FigurePipeline,
        Option<shadow::PointShadowPipeline>,
        Option<shadow::ShadowPipeline>,
        Option<shadow::ShadowFigurePipeline>,
    ),
    RenderError,
> {
    use shaderc::{CompileOptions, Compiler, OptimizationLevel, ResolvedInclude, ShaderKind};

    let constants = shaders.get("include.constants").unwrap();
    let globals = shaders.get("include.globals").unwrap();
    let sky = shaders.get("include.sky").unwrap();
    let light = shaders.get("include.light").unwrap();
    let srgb = shaders.get("include.srgb").unwrap();
    let random = shaders.get("include.random").unwrap();
    let lod = shaders.get("include.lod").unwrap();
    let shadows = shaders.get("include.shadows").unwrap();

    // We dynamically add extra configuration settings to the constants file.
    let constants = format!(
        r#"
{}

#define VOXYGEN_COMPUTATION_PREFERENCE {}
#define FLUID_MODE {}
#define CLOUD_MODE {}
#define LIGHTING_ALGORITHM {}
#define SHADOW_MODE {}

"#,
        &constants.0,
        // TODO: Configurable vertex/fragment shader preference.
        "VOXYGEN_COMPUTATION_PREFERENCE_FRAGMENT",
        match mode.fluid {
            FluidMode::Cheap => "FLUID_MODE_CHEAP",
            FluidMode::Shiny => "FLUID_MODE_SHINY",
        },
        match mode.cloud {
            CloudMode::None => "CLOUD_MODE_NONE",
            CloudMode::Minimal => "CLOUD_MODE_MINIMAL",
            CloudMode::Low => "CLOUD_MODE_LOW",
            CloudMode::Medium => "CLOUD_MODE_MEDIUM",
            CloudMode::High => "CLOUD_MODE_HIGH",
            CloudMode::Ultra => "CLOUD_MODE_ULTRA",
        },
        match mode.lighting {
            LightingMode::Ashikhmin => "LIGHTING_ALGORITHM_ASHIKHMIN",
            LightingMode::BlinnPhong => "LIGHTING_ALGORITHM_BLINN_PHONG",
            LightingMode::Lambertian => "LIGHTING_ALGORITHM_LAMBERTIAN",
        },
        match mode.shadow {
            ShadowMode::None => "SHADOW_MODE_NONE",
            ShadowMode::Map(_) if has_shadow_views => "SHADOW_MODE_MAP",
            ShadowMode::Cheap | ShadowMode::Map(_) => "SHADOW_MODE_CHEAP",
        },
    );

    let anti_alias = shaders
        .get(match mode.aa {
            AaMode::None => "antialias.none",
            AaMode::Fxaa => "antialias.fxaa",
            AaMode::MsaaX4 => "antialias.msaa-x4",
            AaMode::MsaaX8 => "antialias.msaa-x8",
            AaMode::MsaaX16 => "antialias.msaa-x16",
        })
        .unwrap();

    let cloud = shaders
        .get(match mode.cloud {
            CloudMode::None => "include.cloud.none",
            _ => "include.cloud.regular",
        })
        .unwrap();

    let mut compiler = Compiler::new().ok_or(RenderError::ErrorInitializingCompiler)?;
    let mut options = CompileOptions::new().ok_or(RenderError::ErrorInitializingCompiler)?;
    options.set_optimization_level(OptimizationLevel::Performance);
    options.set_forced_version_profile(430, shaderc::GlslProfile::Core);
    options.set_include_callback(move |name, _, shader_name, _| {
        Ok(ResolvedInclude {
            resolved_name: name.to_string(),
            content: match name {
                "constants.glsl" => constants.clone(),
                "globals.glsl" => globals.0.to_owned(),
                "shadows.glsl" => shadows.0.to_owned(),
                "sky.glsl" => sky.0.to_owned(),
                "light.glsl" => light.0.to_owned(),
                "srgb.glsl" => srgb.0.to_owned(),
                "random.glsl" => random.0.to_owned(),
                "lod.glsl" => lod.0.to_owned(),
                "anti-aliasing.glsl" => anti_alias.0.to_owned(),
                "cloud.glsl" => cloud.0.to_owned(),
                other => return Err(format!("Include {} is not defined", other)),
            },
        })
    });

    let mut create_shader = |name, kind| {
        let glsl = &shaders
            .get(name)
            .unwrap_or_else(|| panic!("Can't retrieve shader: {}", name))
            .0;
        let file_name = format!("{}.glsl", name);
        create_shader_module(device, &mut compiler, glsl, kind, &file_name, &options)
    };

    let figure_vert_mod = create_shader("figure-vert", ShaderKind::Vertex)?;

    // let terrain_point_shadow_vert_mod = create_shader("Point-light-shadows-vert",
    // ShaderKind::Vertex)?;

    let terrain_directed_shadow_vert_mod =
        create_shader("light-shadows-directed-vert", ShaderKind::Vertex)?;

    let figure_directed_shadow_vert_mod =
        create_shader("light-shadows-figure-vert", ShaderKind::Vertex)?;

    let directed_shadow_frag_mod =
        create_shader("light-shadows-directed-frag", ShaderKind::Fragment)?;

    // Construct a pipeline for rendering skyboxes
    let skybox_pipeline = skybox::SkyboxPipeline::new(
        device,
        &create_shader("skybox-vert", ShaderKind::Vertex)?,
        &create_shader("skybox-frag", ShaderKind::Fragment)?,
        sc_desc,
        &layouts.global,
        mode.aa,
    );

    // Construct a pipeline for rendering figures
    let figure_pipeline = figure::FigurePipeline::new(
        device,
        &figure_vert_mod,
        &create_shader("figure-frag", ShaderKind::Fragment)?,
        sc_desc,
        &layouts.global,
        &layouts.figure,
        mode.aa,
    );

    let terrain_vert = create_shader("terrain-vert", ShaderKind::Vertex)?;
    // Construct a pipeline for rendering terrain
    let terrain_pipeline = terrain::TerrainPipeline::new(
        device,
        &terrain_vert,
        &create_shader("terrain-frag", ShaderKind::Fragment)?,
        sc_desc,
        &layouts.global,
        &layouts.terrain,
        mode.aa,
    );

    // Construct a pipeline for rendering fluids
    let selected_fluid_shader = ["fluid-frag.", match mode.fluid {
        FluidMode::Cheap => "cheap",
        FluidMode::Shiny => "shiny",
    }]
    .concat();
    let fluid_pipeline = fluid::FluidPipeline::new(
        device,
        &create_shader("fluid-vert", ShaderKind::Vertex)?,
        &create_shader(&selected_fluid_shader, ShaderKind::Fragment)?,
        sc_desc,
        &layouts.global,
        &layouts.fluid,
        &layouts.terrain,
        mode.aa,
    );

    // Construct a pipeline for rendering sprites
    let sprite_pipeline = sprite::SpritePipeline::new(
        device,
        &create_shader("sprite-vert", ShaderKind::Vertex)?,
        &create_shader("sprite-frag", ShaderKind::Fragment)?,
        sc_desc,
        &layouts.global,
        &layouts.sprite,
        &layouts.terrain,
        mode.aa,
    );

    // Construct a pipeline for rendering particles
    let particle_pipeline = particle::ParticlePipeline::new(
        device,
        &create_shader("particle-vert", ShaderKind::Vertex)?,
        &create_shader("particle-frag", ShaderKind::Fragment)?,
        sc_desc,
        &layouts.global,
        mode.aa,
    );

    // Construct a pipeline for rendering UI elements
    let ui_pipeline = ui::UiPipeline::new(
        device,
        &create_shader("ui-vert", ShaderKind::Vertex)?,
        &create_shader("ui-frag", ShaderKind::Fragment)?,
        sc_desc,
        &layouts.global,
        &layouts.ui,
    );

    // Construct a pipeline for rendering terrain
    let lod_terrain_pipeline = lod_terrain::LodTerrainPipeline::new(
        device,
        &create_shader("lod-terrain-vert", ShaderKind::Vertex)?,
        &create_shader("lod-terrain-frag", ShaderKind::Fragment)?,
        sc_desc,
        &layouts.global,
        mode.aa,
    );

    // Construct a pipeline for rendering our clouds (a kind of post-processing)
    let clouds_pipeline = clouds::CloudsPipeline::new(
        device,
        &create_shader("clouds-vert", ShaderKind::Vertex)?,
        &create_shader("clouds-frag", ShaderKind::Fragment)?,
        // TODO: pass in format of intermediate color buffer
        &layouts.global,
        sc_desc,
        &layouts.clouds,
        mode.aa,
    );

    // Construct a pipeline for rendering our post-processing
    let postprocess_pipeline = postprocess::PostProcessPipeline::new(
        device,
        &create_shader("postprocess-vert", ShaderKind::Vertex)?,
        &create_shader("postprocess-frag", ShaderKind::Fragment)?,
        sc_desc,
        &layouts.global,
        &layouts.postprocess,
    );

    // Consider reenabling at some time in the future
    //
    // // Construct a pipeline for rendering the player silhouette
    // let player_shadow_pipeline = create_pipeline(
    //     factory,
    //     figure::pipe::Init {
    //         tgt_depth: (gfx::preset::depth::PASS_TEST/*,
    //         Stencil::new(
    //             Comparison::Equal,
    //             0xff,
    //             (StencilOp::Keep, StencilOp::Keep, StencilOp::Keep),
    //         ),*/),
    //         ..figure::pipe::new()
    //     },
    //     &figure_vert,
    //     &Glsl::load_watched(
    //         "voxygen.shaders.player-shadow-frag",
    //         shader_reload_indicator,
    //     )
    //     .unwrap(),
    //     &include_ctx,
    //     gfx::state::CullFace::Back,
    // )?;

    // Construct a pipeline for rendering point light terrain shadow maps.
    let point_shadow_pipeline = shadow::PointShadowPipeline::new(
        device,
        &create_shader("point-light-shadows-vert", ShaderKind::Vertex)?,
        &create_shader("light-shadows-frag", ShaderKind::Fragment)?,
        &layouts.global,
        &layouts.terrain,
        mode.aa,
    );

    // Construct a pipeline for rendering directional light terrain shadow maps.
    let terrain_directed_shadow_pipeline = shadow::ShadowPipeline::new(
        device,
        &terrain_directed_shadow_vert_mod,
        &directed_shadow_frag_mod,
        &layouts.global,
        &layouts.terrain,
        mode.aa,
    );

    // Construct a pipeline for rendering directional light figure shadow maps.
    let figure_directed_shadow_pipeline = shadow::ShadowFigurePipeline::new(
        device,
        &figure_directed_shadow_vert_mod,
        &directed_shadow_frag_mod,
        &layouts.global,
        &layouts.figure,
        mode.aa,
    );

    Ok((
        Pipelines {
            skybox: skybox_pipeline,
            figure: figure_pipeline,
            terrain: terrain_pipeline,
            fluid: fluid_pipeline,
            sprite: sprite_pipeline,
            particle: particle_pipeline,
            ui: ui_pipeline,
            lod_terrain: lod_terrain_pipeline,
            clouds: clouds_pipeline,
            postprocess: postprocess_pipeline,
        },
        // player_shadow_pipeline,
        Some(point_shadow_pipeline),
        Some(terrain_directed_shadow_pipeline),
        Some(figure_directed_shadow_pipeline),
    ))
}

fn create_shader_module(
    device: &wgpu::Device,
    compiler: &mut shaderc::Compiler,
    source: &str,
    kind: shaderc::ShaderKind,
    file_name: &str,
    options: &shaderc::CompileOptions,
) -> Result<wgpu::ShaderModule, RenderError> {
    use std::borrow::Cow;

    let spv = compiler
        .compile_into_spirv(source, kind, file_name, "main", Some(options))
        .map_err(|e| (file_name, e))?;

    Ok(device.create_shader_module(&wgpu::ShaderModuleDescriptor {
        label: Some(source),
        source: wgpu::ShaderSource::SpirV(Cow::Borrowed(spv.as_binary())),
        flags: wgpu::ShaderFlags::empty(), // TODO: renable wgpu::ShaderFlags::VALIDATION,
    }))
}

fn create_quad_index_buffer_u16(device: &wgpu::Device, vert_length: usize) -> Buffer<u16> {
    assert!(vert_length <= u16::MAX as usize);
    let indices = [0, 1, 2, 2, 1, 3]
        .iter()
        .cycle()
        .copied()
        .take(vert_length / 4 * 6)
        .enumerate()
        .map(|(i, b)| (i / 6 * 4 + b) as u16)
        .collect::<Vec<_>>();

    Buffer::new(device, wgpu::BufferUsage::INDEX, &indices)
}

fn create_quad_index_buffer_u32(device: &wgpu::Device, vert_length: usize) -> Buffer<u32> {
    assert!(vert_length <= u32::MAX as usize);
    let indices = [0, 1, 2, 2, 1, 3]
        .iter()
        .cycle()
        .copied()
        .take(vert_length / 4 * 6)
        .enumerate()
        .map(|(i, b)| (i / 6 * 4 + b) as u32)
        .collect::<Vec<_>>();

    Buffer::new(device, wgpu::BufferUsage::INDEX, &indices)
}
