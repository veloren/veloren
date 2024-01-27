mod binding;
pub(super) mod drawer;
// Consts and bind groups for post-process and clouds
mod locals;
mod pipeline_creation;
mod rain_occlusion_map;
mod screenshot;
mod shaders;
mod shadow_map;

use locals::Locals;
use pipeline_creation::{
    IngameAndShadowPipelines, InterfacePipelines, PipelineCreation, Pipelines, ShadowPipelines,
};
use shaders::Shaders;
use shadow_map::{ShadowMap, ShadowMapRenderer};

use self::{pipeline_creation::RainOcclusionPipelines, rain_occlusion_map::RainOcclusionMap};

use super::{
    buffer::Buffer,
    consts::Consts,
    instances::Instances,
    mesh::Mesh,
    model::{DynamicModel, Model},
    pipelines::{
        blit, bloom, clouds, debug, figure, postprocess, rain_occlusion, rope, shadow, sprite,
        terrain, ui, GlobalsBindGroup, GlobalsLayouts, ShadowTexturesBindGroup,
    },
    texture::Texture,
    AddressMode, FilterMode, OtherModes, PipelineModes, PresentMode, RenderError, RenderMode,
    ShadowMapMode, ShadowMode, Vertex,
};
use common::assets::{self, AssetExt, AssetHandle, ReloadWatcher};
use common_base::span;
use core::convert::TryFrom;
use std::sync::Arc;
use tracing::{error, info, warn};
use vek::*;

const QUAD_INDEX_BUFFER_U16_START_VERT_LEN: u16 = 3000;
const QUAD_INDEX_BUFFER_U32_START_VERT_LEN: u32 = 3000;

/// A type that stores all the layouts associated with this renderer that never
/// change when the RenderMode is modified.
struct ImmutableLayouts {
    global: GlobalsLayouts,

    debug: debug::DebugLayout,
    figure: figure::FigureLayout,
    shadow: shadow::ShadowLayout,
    rain_occlusion: rain_occlusion::RainOcclusionLayout,
    sprite: sprite::SpriteLayout,
    terrain: terrain::TerrainLayout,
    rope: rope::RopeLayout,
    clouds: clouds::CloudsLayout,
    bloom: bloom::BloomLayout,
    ui: ui::UiLayout,
    premultiply_alpha: ui::PremultiplyAlphaLayout,
    blit: blit::BlitLayout,
}

/// A type that stores all the layouts associated with this renderer.
struct Layouts {
    immutable: Arc<ImmutableLayouts>,

    postprocess: Arc<postprocess::PostProcessLayout>,
}

impl core::ops::Deref for Layouts {
    type Target = ImmutableLayouts;

    fn deref(&self) -> &Self::Target { &self.immutable }
}

/// Render target views
struct Views {
    // NOTE: unused for now, maybe... we will want it for something
    _win_depth: wgpu::TextureView,

    tgt_color: wgpu::TextureView,
    tgt_mat: wgpu::TextureView,
    tgt_depth: wgpu::TextureView,

    bloom_tgts: Option<[wgpu::TextureView; bloom::NUM_SIZES]>,
    // TODO: rename
    tgt_color_pp: wgpu::TextureView,
}

/// Shadow rendering textures, layouts, pipelines, and bind groups
struct Shadow {
    rain_map: RainOcclusionMap,
    map: ShadowMap,
    bind: ShadowTexturesBindGroup,
}

/// Represent two states of the renderer:
/// 1. Only interface pipelines created
/// 2. All of the pipelines have been created
#[allow(clippy::large_enum_variant)] // They are both pretty large
enum State {
    // NOTE: this is used as a transient placeholder for moving things out of State temporarily
    Nothing,
    Interface {
        pipelines: InterfacePipelines,
        shadow_views: Option<(Texture, Texture)>,
        rain_occlusion_view: Option<Texture>,
        // In progress creation of the remaining pipelines in the background
        creating: PipelineCreation<IngameAndShadowPipelines>,
    },
    Complete {
        pipelines: Pipelines,
        shadow: Shadow,
        recreating: Option<(
            PipelineModes,
            PipelineCreation<
                Result<
                    (
                        Pipelines,
                        ShadowPipelines,
                        RainOcclusionPipelines,
                        Arc<postprocess::PostProcessLayout>,
                    ),
                    RenderError,
                >,
            >,
        )>,
    },
}

/// A type that encapsulates rendering state. `Renderer` is central to Voxygen's
/// rendering subsystem and contains any state necessary to interact with the
/// GPU, along with pipeline state objects (PSOs) needed to renderer different
/// kinds of models to the screen.
pub struct Renderer {
    device: Arc<wgpu::Device>,
    queue: wgpu::Queue,
    surface: wgpu::Surface,
    surface_config: wgpu::SurfaceConfiguration,

    sampler: wgpu::Sampler,
    depth_sampler: wgpu::Sampler,

    state: State,
    // Some if there is a pending need to recreate the pipelines (e.g. RenderMode change or shader
    // hotloading)
    recreation_pending: Option<PipelineModes>,

    layouts: Layouts,
    // Note: we keep these here since their bind groups need to be updated if we resize the
    // color/depth textures
    locals: Locals,
    views: Views,
    noise_tex: Texture,

    quad_index_buffer_u16: Buffer<u16>,
    quad_index_buffer_u32: Buffer<u32>,

    shaders: AssetHandle<Shaders>,
    shaders_watcher: ReloadWatcher,

    pipeline_modes: PipelineModes,
    other_modes: OtherModes,
    resolution: Vec2<u32>,

    // If this is Some then a screenshot will be taken and passed to the handler here
    take_screenshot: Option<screenshot::ScreenshotFn>,

    profiler: wgpu_profiler::GpuProfiler,
    profile_times: Vec<wgpu_profiler::GpuTimerScopeResult>,
    profiler_features_enabled: bool,

    ui_premultiply_uploads: ui::BatchedUploads,

    #[cfg(feature = "egui-ui")]
    egui_renderpass: egui_wgpu_backend::RenderPass,

    // This checks is added because windows resizes the window to 0,0 when
    // minimizing and this causes a bunch of validation errors
    is_minimized: bool,

    // To remember the backend info after initialization for debug purposes
    graphics_backend: String,

    /// The texture format used for the intermediate rendering passes
    intermediate_format: wgpu::TextureFormat,

    present_modes: Vec<PresentMode>,
}

impl Renderer {
    /// Create a new `Renderer` from a variety of backend-specific components
    /// and the window targets.
    pub fn new(
        window: &winit::window::Window,
        mode: RenderMode,
        runtime: &tokio::runtime::Runtime,
    ) -> Result<Self, RenderError> {
        let (pipeline_modes, mut other_modes) = mode.split();
        // Enable seamless cubemaps globally, where available--they are essentially a
        // strict improvement on regular cube maps.
        //
        // Note that since we only have to enable this once globally, there is no point
        // in doing this on rerender.
        // Self::enable_seamless_cube_maps(&mut device);

        // TODO: fix panic on wayland with opengl?
        // TODO: fix backend defaulting to opengl on wayland.
        let backends = std::env::var("WGPU_BACKEND")
            .ok()
            .and_then(|backend| match backend.to_lowercase().as_str() {
                "vulkan" | "vk" => Some(wgpu::Backends::VULKAN),
                "metal" => Some(wgpu::Backends::METAL),
                "dx12" => Some(wgpu::Backends::DX12),
                "primary" => Some(wgpu::Backends::PRIMARY),
                "opengl" | "gl" => Some(wgpu::Backends::GL),
                "dx11" => Some(wgpu::Backends::DX11),
                "secondary" => Some(wgpu::Backends::SECONDARY),
                "all" => Some(wgpu::Backends::all()),
                _ => None,
            })
            .unwrap_or(wgpu::Backends::PRIMARY | wgpu::Backends::SECONDARY);

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            dx12_shader_compiler: wgpu::Dx12Compiler::Fxc,
            gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
            // TODO: Look into what we want here.
            flags: wgpu::InstanceFlags::from_build_config().with_env(),
        });

        let dims = window.inner_size();

        // This is unsafe because the window handle must be valid, if you find a way to
        // have an invalid winit::Window then you have bigger issues
        #[allow(unsafe_code)]
        let surface =
            unsafe { instance.create_surface(window) }.expect("Failed to create a surface");

        let adapters = instance
            .enumerate_adapters(backends)
            .enumerate()
            .collect::<Vec<_>>();

        for (i, adapter) in adapters.iter() {
            let info = adapter.get_info();
            info!(
                ?info.name,
                ?info.vendor,
                ?info.backend,
                ?info.device,
                ?info.device_type,
                "graphics device #{}", i,
            );
        }

        let adapter = match std::env::var("WGPU_ADAPTER").ok() {
            Some(filter) if !filter.is_empty() => adapters.into_iter().find_map(|(i, adapter)| {
                let info = adapter.get_info();

                let full_name = format!("#{} {} {:?}", i, info.name, info.device_type,);

                full_name.contains(&filter).then_some(adapter)
            }),
            Some(_) | None => {
                runtime.block_on(instance.request_adapter(&wgpu::RequestAdapterOptionsBase {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    compatible_surface: Some(&surface),
                    force_fallback_adapter: false,
                }))
            },
        }
        .ok_or(RenderError::CouldNotFindAdapter)?;

        let info = adapter.get_info();
        info!(
            ?info.name,
            ?info.vendor,
            ?info.backend,
            ?info.device,
            ?info.device_type,
            "selected graphics device"
        );
        let graphics_backend = format!("{:?}", &info.backend);

        let limits = wgpu::Limits {
            max_push_constant_size: 64,
            ..Default::default()
        };

        let trace_env = std::env::var_os("WGPU_TRACE_DIR");
        let trace_path = trace_env.as_ref().map(|v| {
            let path = std::path::Path::new(v);
            // We don't want to continue if we can't actually collect the api trace
            assert!(
                path.exists(),
                "WGPU_TRACE_DIR is set to the path \"{}\" which doesn't exist",
                path.display()
            );
            assert!(
                path.is_dir(),
                "WGPU_TRACE_DIR is set to the path \"{}\" which is not a directory",
                path.display()
            );
            assert!(
                path.read_dir()
                    .expect("Could not read the directory that is specified by WGPU_TRACE_DIR")
                    .next()
                    .is_none(),
                "WGPU_TRACE_DIR is set to the path \"{}\" which already contains other files",
                path.display()
            );

            path
        });

        let (device, queue) = runtime.block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                // TODO
                label: None,
                features: wgpu::Features::DEPTH_CLIP_CONTROL
                    | wgpu::Features::ADDRESS_MODE_CLAMP_TO_BORDER
                    | wgpu::Features::PUSH_CONSTANTS
                    | (adapter.features() & wgpu_profiler::GpuProfiler::ALL_WGPU_TIMER_FEATURES),
                limits,
            },
            trace_path,
        ))?;

        // Set error handler for wgpu errors
        // This is better for use than their default because it includes the error in
        // the panic message
        device.on_uncaptured_error(Box::new(move |error| {
            error!("{}", &error);
            panic!(
                "wgpu error (handling all wgpu errors as fatal):\n{:?}\n{:?}",
                &error, &info,
            );
        }));

        let profiler_features_enabled = device
            .features()
            .intersects(wgpu_profiler::GpuProfiler::ALL_WGPU_TIMER_FEATURES);
        if !profiler_features_enabled {
            info!(
                "The features for GPU profiling (timestamp queries) are not available on this \
                 adapter"
            );
        }

        let surface_capabilities = surface.get_capabilities(&adapter);
        let format = surface_capabilities.formats[0];
        info!("Using {:?} as the surface format", format);

        let present_mode = other_modes.present_mode.into();
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: dims.width,
            height: dims.height,
            present_mode: if surface_capabilities.present_modes.contains(&present_mode) {
                present_mode
            } else {
                *surface_capabilities
                    .present_modes
                    .iter()
                    .find(|mode| PresentMode::try_from(**mode).is_ok())
                    .expect("There should never be no supported present modes")
            },
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            view_formats: Vec::new(),
        };

        let supported_internal_formats = [wgpu::TextureFormat::Rgba16Float, format];
        let intermediate_format = supported_internal_formats
            .into_iter()
            .find(|format| {
                use wgpu::TextureUsages as Usages;
                use wgpu::TextureFormatFeatureFlags as Flags;
                use super::AaMode;

                let features = adapter
                    .get_texture_format_features(*format);

                let usage_ok = features
                    .allowed_usages
                    .contains(Usages::RENDER_ATTACHMENT | Usages::COPY_SRC | Usages::TEXTURE_BINDING);

                let msaa_flags = match pipeline_modes.aa {
                    AaMode::None | AaMode::Fxaa | AaMode::Hqx | AaMode::FxUpscale | AaMode::Bilinear => Flags::empty(),
                    AaMode::MsaaX4 => Flags::MULTISAMPLE_X4,
                    AaMode::MsaaX8 => Flags::MULTISAMPLE_X8,
                    AaMode::MsaaX16 => Flags::MULTISAMPLE_X8, // TODO?
                };

                let flags_ok = features.flags.contains(Flags::FILTERABLE | msaa_flags);

                usage_ok && flags_ok
            })
            // This should be unreachable as the surface format should always support the
            // needed capabilities
            .expect("No supported intermediate format");
        info!("Using {:?} as the intermediate format", intermediate_format);

        surface.configure(&device, &surface_config);

        let shadow_views = ShadowMap::create_shadow_views(
            &device,
            (dims.width, dims.height),
            &ShadowMapMode::try_from(pipeline_modes.shadow).unwrap_or_default(),
        )
        .map_err(|err| {
            warn!("Could not create shadow map views: {:?}", err);
        })
        .ok();

        let rain_occlusion_view =
            RainOcclusionMap::create_view(&device, &pipeline_modes.rain_occlusion)
                .map_err(|err| {
                    warn!("Could not create rain occlusion map views: {:?}", err);
                })
                .ok();

        let shaders = Shaders::load_expect("");
        let shaders_watcher = shaders.reload_watcher();

        let layouts = {
            let global = GlobalsLayouts::new(&device);

            let debug = debug::DebugLayout::new(&device);
            let figure = figure::FigureLayout::new(&device);
            let shadow = shadow::ShadowLayout::new(&device);
            let rain_occlusion = rain_occlusion::RainOcclusionLayout::new(&device);
            let sprite = sprite::SpriteLayout::new(&device);
            let terrain = terrain::TerrainLayout::new(&device);
            let rope = rope::RopeLayout::new(&device);
            let clouds = clouds::CloudsLayout::new(&device);
            let bloom = bloom::BloomLayout::new(&device);
            let postprocess = Arc::new(postprocess::PostProcessLayout::new(
                &device,
                &pipeline_modes,
            ));
            let ui = ui::UiLayout::new(&device);
            let premultiply_alpha = ui::PremultiplyAlphaLayout::new(&device);
            let blit = blit::BlitLayout::new(&device);

            let immutable = Arc::new(ImmutableLayouts {
                global,

                debug,
                figure,
                shadow,
                rain_occlusion,
                sprite,
                terrain,
                rope,
                clouds,
                bloom,
                ui,
                premultiply_alpha,
                blit,
            });

            Layouts {
                immutable,
                postprocess,
            }
        };

        // Arcify the device
        let device = Arc::new(device);

        let (interface_pipelines, creating) = pipeline_creation::initial_create_pipelines(
            Arc::clone(&device),
            Layouts {
                immutable: Arc::clone(&layouts.immutable),
                postprocess: Arc::clone(&layouts.postprocess),
            },
            shaders.cloned(),
            pipeline_modes.clone(),
            surface_config.clone(), // Note: cheap clone
            shadow_views.is_some(),
            intermediate_format,
        )?;

        let state = State::Interface {
            pipelines: interface_pipelines,
            shadow_views,
            rain_occlusion_view,
            creating,
        };

        let (views, bloom_sizes) = Self::create_rt_views(
            &device,
            (dims.width, dims.height),
            &pipeline_modes,
            &other_modes,
            intermediate_format,
        );

        let create_sampler = |filter| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: None,
                address_mode_u: AddressMode::ClampToEdge,
                address_mode_v: AddressMode::ClampToEdge,
                address_mode_w: AddressMode::ClampToEdge,
                mag_filter: filter,
                min_filter: filter,
                mipmap_filter: FilterMode::Nearest,
                compare: None,
                ..Default::default()
            })
        };

        let sampler = create_sampler(FilterMode::Linear);
        let depth_sampler = create_sampler(FilterMode::Nearest);

        let noise_tex = Texture::new(
            &device,
            &queue,
            &assets::Image::load_expect("voxygen.texture.noise").read().0,
            Some(FilterMode::Linear),
            Some(AddressMode::Repeat),
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
            &views.tgt_mat,
            &views.tgt_depth,
            views.bloom_tgts.as_ref().map(|tgts| locals::BloomParams {
                locals: bloom_sizes.map(|size| {
                    Self::create_consts_inner(&device, &queue, &[bloom::Locals::new(size)])
                }),
                src_views: [&views.tgt_color_pp, &tgts[1], &tgts[2], &tgts[3], &tgts[4]],
                final_tgt_view: &tgts[0],
            }),
            &views.tgt_color_pp,
            &sampler,
            &depth_sampler,
        );

        let quad_index_buffer_u16 =
            create_quad_index_buffer_u16(&device, QUAD_INDEX_BUFFER_U16_START_VERT_LEN as usize);
        let quad_index_buffer_u32 =
            create_quad_index_buffer_u32(&device, QUAD_INDEX_BUFFER_U32_START_VERT_LEN as usize);
        other_modes.profiler_enabled &= profiler_features_enabled;
        let profiler = wgpu_profiler::GpuProfiler::new(wgpu_profiler::GpuProfilerSettings {
            enable_timer_scopes: other_modes.profiler_enabled,
            enable_debug_groups: other_modes.profiler_enabled,
            max_num_pending_frames: 4,
        })
        .map_err(RenderError::ProfilerCreationError)?;

        #[cfg(feature = "egui-ui")]
        let egui_renderpass = egui_wgpu_backend::RenderPass::new(&device, format, 1);

        let present_modes = surface
            .get_capabilities(&adapter)
            .present_modes
            .into_iter()
            .filter_map(|present_mode| PresentMode::try_from(present_mode).ok())
            .collect();

        Ok(Self {
            device,
            queue,
            surface,
            surface_config,

            state,
            recreation_pending: None,

            layouts,
            locals,
            views,

            sampler,
            depth_sampler,
            noise_tex,

            quad_index_buffer_u16,
            quad_index_buffer_u32,

            shaders,
            shaders_watcher,

            pipeline_modes,
            other_modes,
            resolution: Vec2::new(dims.width, dims.height),

            take_screenshot: None,

            profiler,
            profile_times: Vec::new(),
            profiler_features_enabled,

            ui_premultiply_uploads: Default::default(),

            #[cfg(feature = "egui-ui")]
            egui_renderpass,

            is_minimized: false,

            graphics_backend,

            intermediate_format,
            present_modes,
        })
    }

    /// Get the graphics backend being used
    pub fn graphics_backend(&self) -> &str { &self.graphics_backend }

    /// Check the status of the intial pipeline creation
    /// Returns `None` if complete
    /// Returns `Some((total, complete))` if in progress
    pub fn pipeline_creation_status(&self) -> Option<(usize, usize)> {
        if let State::Interface { creating, .. } = &self.state {
            Some(creating.status())
        } else {
            None
        }
    }

    /// Check the status the pipeline recreation
    /// Returns `None` if pipelines are currently not being recreated
    /// Returns `Some((total, complete))` if in progress
    pub fn pipeline_recreation_status(&self) -> Option<(usize, usize)> {
        if let State::Complete { recreating, .. } = &self.state {
            recreating.as_ref().map(|(_, c)| c.status())
        } else {
            None
        }
    }

    /// Change the render mode.
    pub fn set_render_mode(&mut self, mode: RenderMode) -> Result<(), RenderError> {
        let (pipeline_modes, other_modes) = mode.split();

        if self.other_modes != other_modes {
            self.other_modes = other_modes;

            // Update present mode in swap chain descriptor
            self.surface_config.present_mode =
                if self.present_modes.contains(&self.other_modes.present_mode) {
                    self.other_modes.present_mode
                } else {
                    *self
                        .present_modes
                        .first()
                        .expect("There should never be no supported present modes")
                }
                .into();

            // Only enable profiling if the wgpu features are enabled
            self.other_modes.profiler_enabled &= self.profiler_features_enabled;
            // Enable/disable profiler
            if !self.other_modes.profiler_enabled {
                // Clear the times if disabled
                core::mem::take(&mut self.profile_times);
            }
            self.profiler
                .change_settings(wgpu_profiler::GpuProfilerSettings {
                    enable_timer_scopes: self.other_modes.profiler_enabled,
                    enable_debug_groups: self.other_modes.profiler_enabled,
                    max_num_pending_frames: 4,
                })
                .map_err(|err| {
                    RenderError::ProfilerCreationError(
                        wgpu_profiler::CreationError::InvalidSettings(err),
                    )
                })?;

            // Recreate render target
            self.on_resize(self.resolution);
        }

        // We can't cancel the pending recreation even if the new settings are equal
        // to the current ones becuase the recreation could be triggered by something
        // else like shader hotloading
        if self.pipeline_modes != pipeline_modes
            || self
                .recreation_pending
                .as_ref()
                .map_or(false, |modes| modes != &pipeline_modes)
        {
            // Recreate pipelines with new modes
            self.recreate_pipelines(pipeline_modes);
        }

        Ok(())
    }

    /// Get the pipelines mode.
    pub fn pipeline_modes(&self) -> &PipelineModes { &self.pipeline_modes }

    /// Get the supported present modes.
    pub fn present_modes(&self) -> &[PresentMode] { &self.present_modes }

    /// Get the current profiling times
    /// Nested timings immediately follow their parent
    /// Returns Vec<(how nested this timing is, label, length in seconds)>
    pub fn timings(&self) -> Vec<(u8, &str, f64)> {
        use wgpu_profiler::GpuTimerScopeResult;
        fn recursive_collect<'a>(
            vec: &mut Vec<(u8, &'a str, f64)>,
            result: &'a GpuTimerScopeResult,
            nest_level: u8,
        ) {
            vec.push((
                nest_level,
                &result.label,
                result.time.end - result.time.start,
            ));
            result
                .nested_scopes
                .iter()
                .for_each(|child| recursive_collect(vec, child, nest_level + 1));
        }
        let mut vec = Vec::new();
        self.profile_times
            .iter()
            .for_each(|child| recursive_collect(&mut vec, child, 0));
        vec
    }

    /// Resize internal render targets to match window render target dimensions.
    pub fn on_resize(&mut self, dims: Vec2<u32>) {
        // Avoid panics when creating texture with w,h of 0,0.
        if dims.x != 0 && dims.y != 0 {
            self.is_minimized = false;
            // Resize swap chain
            self.resolution = dims;
            self.surface_config.width = dims.x;
            self.surface_config.height = dims.y;
            self.surface.configure(&self.device, &self.surface_config);

            // Resize other render targets
            let (views, bloom_sizes) = Self::create_rt_views(
                &self.device,
                (dims.x, dims.y),
                &self.pipeline_modes,
                &self.other_modes,
                self.intermediate_format,
            );
            self.views = views;

            let bloom_params = self
                .views
                .bloom_tgts
                .as_ref()
                .map(|tgts| locals::BloomParams {
                    locals: bloom_sizes.map(|size| {
                        Self::create_consts_inner(&self.device, &self.queue, &[bloom::Locals::new(
                            size,
                        )])
                    }),
                    src_views: [
                        &self.views.tgt_color_pp,
                        &tgts[1],
                        &tgts[2],
                        &tgts[3],
                        &tgts[4],
                    ],
                    final_tgt_view: &tgts[0],
                });

            self.locals.rebind(
                &self.device,
                &self.layouts,
                &self.views.tgt_color,
                &self.views.tgt_mat,
                &self.views.tgt_depth,
                bloom_params,
                &self.views.tgt_color_pp,
                &self.sampler,
                &self.depth_sampler,
            );

            // Get mutable reference to shadow views out of the current state
            let shadow_views = match &mut self.state {
                State::Interface {
                    shadow_views,
                    rain_occlusion_view,
                    ..
                } => shadow_views
                    .as_mut()
                    .map(|s| (&mut s.0, &mut s.1))
                    .zip(rain_occlusion_view.as_mut()),
                State::Complete {
                    shadow:
                        Shadow {
                            map: ShadowMap::Enabled(shadow_map),
                            rain_map: RainOcclusionMap::Enabled(rain_occlusion_map),
                            ..
                        },
                    ..
                } => Some((
                    (&mut shadow_map.point_depth, &mut shadow_map.directed_depth),
                    &mut rain_occlusion_map.depth,
                )),
                State::Complete { .. } => None,
                State::Nothing => None, // Should never hit this
            };

            let mut update_shadow_bind = false;
            let (shadow_views, rain_views) = shadow_views.unzip();

            if let (Some((point_depth, directed_depth)), ShadowMode::Map(mode)) =
                (shadow_views, self.pipeline_modes.shadow)
            {
                match ShadowMap::create_shadow_views(&self.device, (dims.x, dims.y), &mode) {
                    Ok((new_point_depth, new_directed_depth)) => {
                        *point_depth = new_point_depth;
                        *directed_depth = new_directed_depth;

                        update_shadow_bind = true;
                    },
                    Err(err) => {
                        warn!("Could not create shadow map views: {:?}", err);
                    },
                }
            }
            if let Some(rain_depth) = rain_views {
                match RainOcclusionMap::create_view(
                    &self.device,
                    &self.pipeline_modes.rain_occlusion,
                ) {
                    Ok(new_rain_depth) => {
                        *rain_depth = new_rain_depth;

                        update_shadow_bind = true;
                    },
                    Err(err) => {
                        warn!("Could not create rain occlusion map view: {:?}", err);
                    },
                }
            }
            if update_shadow_bind {
                // Recreate the shadow bind group if needed
                if let State::Complete {
                    shadow:
                        Shadow {
                            bind,
                            map: ShadowMap::Enabled(shadow_map),
                            rain_map: RainOcclusionMap::Enabled(rain_occlusion_map),
                            ..
                        },
                    ..
                } = &mut self.state
                {
                    *bind = self.layouts.global.bind_shadow_textures(
                        &self.device,
                        &shadow_map.point_depth,
                        &shadow_map.directed_depth,
                        &rain_occlusion_map.depth,
                    );
                }
            }
        } else {
            self.is_minimized = true;
        }
    }

    pub fn maintain(&self) {
        if self.is_minimized {
            self.queue.submit(std::iter::empty());
        }

        self.device.poll(wgpu::Maintain::Poll);
    }

    /// Create render target views
    fn create_rt_views(
        device: &wgpu::Device,
        size: (u32, u32),
        pipeline_modes: &PipelineModes,
        other_modes: &OtherModes,
        format: wgpu::TextureFormat,
    ) -> (Views, [Vec2<f32>; bloom::NUM_SIZES]) {
        let upscaled = Vec2::<u32>::from(size)
            .map(|e| (e as f32 * other_modes.upscale_mode.factor) as u32)
            .into_tuple();
        let (width, height) = upscaled;
        let sample_count = pipeline_modes.aa.samples();
        let levels = 1;

        let color_view = |width, height, format| {
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
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });

            tex.create_view(&wgpu::TextureViewDescriptor {
                label: None,
                format: Some(format),
                dimension: Some(wgpu::TextureViewDimension::D2),
                // TODO: why is this not Color?
                aspect: wgpu::TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: None,
                base_array_layer: 0,
                array_layer_count: None,
            })
        };

        let tgt_color_view = color_view(width, height, format);
        let tgt_color_pp_view = color_view(width, height, format);

        let tgt_mat_view = color_view(width, height, wgpu::TextureFormat::Rgba8Uint);

        let mut size_shift = 0;
        // TODO: skip creating bloom stuff when it is disabled
        let bloom_sizes = [(); bloom::NUM_SIZES].map(|()| {
            // .max(1) to ensure we don't create zero sized textures
            let size = Vec2::new(width, height).map(|e| (e >> size_shift).max(1));
            size_shift += 1;
            size
        });

        let bloom_tgt_views = pipeline_modes
            .bloom
            .is_on()
            .then(|| bloom_sizes.map(|size| color_view(size.x, size.y, format)));

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
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let tgt_depth_view = tgt_depth_tex.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            format: Some(wgpu::TextureFormat::Depth32Float),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::DepthOnly,
            base_mip_level: 0,
            mip_level_count: None,
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
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        // TODO: Consider no depth buffer for the final draw to the window?
        let win_depth_view = win_depth_tex.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            format: Some(wgpu::TextureFormat::Depth32Float),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::DepthOnly,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        (
            Views {
                tgt_color: tgt_color_view,
                tgt_mat: tgt_mat_view,
                tgt_depth: tgt_depth_view,
                bloom_tgts: bloom_tgt_views,
                tgt_color_pp: tgt_color_pp_view,
                _win_depth: win_depth_view,
            },
            bloom_sizes.map(|s| s.map(|e| e as f32)),
        )
    }

    /// Get the resolution of the render target.
    pub fn resolution(&self) -> Vec2<u32> { self.resolution }

    /// Get the resolution of the shadow render target.
    pub fn get_shadow_resolution(&self) -> (Vec2<u32>, Vec2<u32>) {
        match &self.state {
            State::Interface { shadow_views, .. } => shadow_views.as_ref().map(|s| (&s.0, &s.1)),
            State::Complete {
                shadow:
                    Shadow {
                        map: ShadowMap::Enabled(shadow_map),
                        ..
                    },
                ..
            } => Some((&shadow_map.point_depth, &shadow_map.directed_depth)),
            State::Complete { .. } | State::Nothing => None,
        }
        .map(|(point, directed)| (point.get_dimensions().xy(), directed.get_dimensions().xy()))
        .unwrap_or_else(|| (Vec2::new(1, 1), Vec2::new(1, 1)))
    }

    // TODO: Seamless is potentially the default with wgpu but we need further
    // investigation into whether this is actually turned on for the OpenGL
    // backend
    //
    /// NOTE: Supported by Vulkan (by default), DirectX 10+ (it seems--it's hard
    /// to find proof of this, but Direct3D 10 apparently does it by
    /// default, and 11 definitely does, so I assume it's natively supported
    /// by DirectX itself), OpenGL 3.2+, and Metal (done by default).  While
    /// there may be some GPUs that don't quite support it correctly, the
    /// impact is relatively small, so there is no reason not to enable it where
    /// available.
    //fn enable_seamless_cube_maps() {
    //todo!()
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
    //}

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

        if self.is_minimized {
            return Ok(None);
        }

        // Try to get the latest profiling results
        if self.other_modes.profiler_enabled {
            // Note: this lags a few frames behind
            let timestamp_period = self.queue.get_timestamp_period();
            if let Some(profile_times) = self.profiler.process_finished_frame(timestamp_period) {
                self.profile_times = profile_times;
            }
        }

        // Handle polling background pipeline creation/recreation
        // Temporarily set to nothing and then replace in the statement below
        let state = core::mem::replace(&mut self.state, State::Nothing);
        // Indicator for if pipeline recreation finished and we need to recreate bind
        // groups / render targets (handling defered so that State will be valid
        // when calling Self::on_resize)
        let mut trigger_on_resize = false;
        // If still creating initial pipelines, check if complete
        self.state = if let State::Interface {
            pipelines: interface,
            shadow_views,
            rain_occlusion_view,
            creating,
        } = state
        {
            match creating.try_complete() {
                Ok(pipelines) => {
                    let IngameAndShadowPipelines {
                        ingame,
                        shadow,
                        rain_occlusion,
                    } = pipelines;

                    let pipelines = Pipelines::consolidate(interface, ingame);

                    let shadow_map = ShadowMap::new(
                        &self.device,
                        &self.queue,
                        shadow.point,
                        shadow.directed,
                        shadow.figure,
                        shadow.debug,
                        shadow_views,
                    );

                    let rain_occlusion_map = RainOcclusionMap::new(
                        &self.device,
                        &self.queue,
                        rain_occlusion.terrain,
                        rain_occlusion.figure,
                        rain_occlusion_view,
                    );

                    let shadow_bind = {
                        let (point, directed) = shadow_map.textures();
                        self.layouts.global.bind_shadow_textures(
                            &self.device,
                            point,
                            directed,
                            rain_occlusion_map.texture(),
                        )
                    };

                    let shadow = Shadow {
                        rain_map: rain_occlusion_map,
                        map: shadow_map,
                        bind: shadow_bind,
                    };

                    State::Complete {
                        pipelines,
                        shadow,
                        recreating: None,
                    }
                },
                // Not complete
                Err(creating) => State::Interface {
                    pipelines: interface,
                    shadow_views,
                    rain_occlusion_view,
                    creating,
                },
            }
        // If recreating the pipelines, check if that is complete
        } else if let State::Complete {
            pipelines,
            mut shadow,
            recreating: Some((new_pipeline_modes, pipeline_creation)),
        } = state
        {
            match pipeline_creation.try_complete() {
                Ok(Ok((
                    pipelines,
                    shadow_pipelines,
                    rain_occlusion_pipelines,
                    postprocess_layout,
                ))) => {
                    if let (
                        Some(point_pipeline),
                        Some(terrain_directed_pipeline),
                        Some(figure_directed_pipeline),
                        Some(debug_directed_pipeline),
                        ShadowMap::Enabled(shadow_map),
                    ) = (
                        shadow_pipelines.point,
                        shadow_pipelines.directed,
                        shadow_pipelines.figure,
                        shadow_pipelines.debug,
                        &mut shadow.map,
                    ) {
                        shadow_map.point_pipeline = point_pipeline;
                        shadow_map.terrain_directed_pipeline = terrain_directed_pipeline;
                        shadow_map.figure_directed_pipeline = figure_directed_pipeline;
                        shadow_map.debug_directed_pipeline = debug_directed_pipeline;
                    }

                    if let (
                        Some(terrain_directed_pipeline),
                        Some(figure_directed_pipeline),
                        RainOcclusionMap::Enabled(rain_occlusion_map),
                    ) = (
                        rain_occlusion_pipelines.terrain,
                        rain_occlusion_pipelines.figure,
                        &mut shadow.rain_map,
                    ) {
                        rain_occlusion_map.terrain_pipeline = terrain_directed_pipeline;
                        rain_occlusion_map.figure_pipeline = figure_directed_pipeline;
                    }

                    self.pipeline_modes = new_pipeline_modes;
                    self.layouts.postprocess = postprocess_layout;
                    // TODO: we have the potential to skip recreating bindings / render targets on
                    // pipeline recreation trigged by shader reloading (would need to ensure new
                    // postprocess_layout is not created...)
                    trigger_on_resize = true;

                    State::Complete {
                        pipelines,
                        shadow,
                        recreating: None,
                    }
                },
                Ok(Err(e)) => {
                    error!(?e, "Could not recreate shaders from assets due to an error");
                    State::Complete {
                        pipelines,
                        shadow,
                        recreating: None,
                    }
                },
                // Not complete
                Err(pipeline_creation) => State::Complete {
                    pipelines,
                    shadow,
                    recreating: Some((new_pipeline_modes, pipeline_creation)),
                },
            }
        } else {
            state
        };

        // Call on_resize to recreate render targets and their bind groups if the
        // pipelines were recreated with a new postprocess layout and or changes in the
        // render modes
        if trigger_on_resize {
            self.on_resize(self.resolution);
        }

        // If the shaders files were changed attempt to recreate the shaders
        if self.shaders_watcher.reloaded() {
            self.recreate_pipelines(self.pipeline_modes.clone());
        }

        // Or if we have a recreation pending
        if matches!(&self.state, State::Complete {
            recreating: None,
            ..
        }) {
            if let Some(new_pipeline_modes) = self.recreation_pending.take() {
                self.recreate_pipelines(new_pipeline_modes);
            }
        }

        let texture = match self.surface.get_current_texture() {
            Ok(texture) => texture,
            // If lost recreate the swap chain
            Err(err @ wgpu::SurfaceError::Lost) => {
                warn!("{}. Recreating swap chain. A frame will be missed", err);
                self.on_resize(self.resolution);
                return Ok(None);
            },
            Err(wgpu::SurfaceError::Timeout) => {
                // This will probably be resolved on the next frame
                // NOTE: we don't log this because it happens very frequently with
                // PresentMode::Fifo and unlimited FPS on certain machines
                return Ok(None);
            },
            Err(err @ wgpu::SurfaceError::Outdated) => {
                warn!("{}. Recreating the swapchain", err);
                self.surface.configure(&self.device, &self.surface_config);
                return Ok(None);
            },
            Err(err @ wgpu::SurfaceError::OutOfMemory) => return Err(err.into()),
        };
        let encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("A render encoder"),
            });

        Ok(Some(drawer::Drawer::new(encoder, self, texture, globals)))
    }

    /// Recreate the pipelines
    fn recreate_pipelines(&mut self, pipeline_modes: PipelineModes) {
        match &mut self.state {
            State::Complete { recreating, .. } if recreating.is_some() => {
                // Defer recreation so that we are not building multiple sets of pipelines in
                // the background at once
                self.recreation_pending = Some(pipeline_modes);
            },
            State::Complete {
                recreating, shadow, ..
            } => {
                *recreating = Some((
                    pipeline_modes.clone(),
                    pipeline_creation::recreate_pipelines(
                        Arc::clone(&self.device),
                        Arc::clone(&self.layouts.immutable),
                        self.shaders.cloned(),
                        pipeline_modes,
                        // NOTE: if present_mode starts to be used to configure pipelines then it
                        // needs to become a part of the pipeline modes
                        // (note here since the present mode is accessible
                        // through the swap chain descriptor)
                        self.surface_config.clone(), // Note: cheap clone
                        shadow.map.is_enabled(),
                        self.intermediate_format,
                    ),
                ));
            },
            State::Interface { .. } => {
                // Defer recreation so that we are not building multiple sets of pipelines in
                // the background at once
                self.recreation_pending = Some(pipeline_modes);
            },
            State::Nothing => {},
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
        consts.update(queue, vals, 0);
        consts
    }

    /// Update a set of constants with the provided values.
    pub fn update_consts<T: Copy + bytemuck::Pod>(&self, consts: &mut Consts<T>, vals: &[T]) {
        consts.update(&self.queue, vals, 0)
    }

    pub fn update_clouds_locals(&mut self, new_val: clouds::Locals) {
        self.locals.clouds.update(&self.queue, &[new_val], 0)
    }

    pub fn update_postprocess_locals(&mut self, new_val: postprocess::Locals) {
        self.locals.postprocess.update(&self.queue, &[new_val], 0)
    }

    /// Create a new set of instances with the provided values.
    pub fn create_instances<T: Copy + bytemuck::Pod>(&mut self, vals: &[T]) -> Instances<T> {
        let mut instances = Instances::new(&self.device, vals.len());
        instances.update(&self.queue, vals, 0);
        instances
    }

    /// Ensure that the quad index buffer is large enough for a quad vertex
    /// buffer with this many vertices
    pub(super) fn ensure_sufficient_index_length<V: Vertex>(
        &mut self,
        // Length of the vert buffer with 4 verts per quad
        vert_length: usize,
    ) {
        let quad_index_length = vert_length / 4 * 6;

        match V::QUADS_INDEX {
            Some(wgpu::IndexFormat::Uint16) => {
                // Make sure the global quad index buffer is large enough
                if self.quad_index_buffer_u16.len() < quad_index_length {
                    // Make sure we aren't over the max
                    if vert_length > u16::MAX as usize {
                        panic!(
                            "Vertex type: {} needs to use a larger index type, length: {}",
                            core::any::type_name::<V>(),
                            vert_length
                        );
                    }
                    self.quad_index_buffer_u16 =
                        create_quad_index_buffer_u16(&self.device, vert_length);
                }
            },
            Some(wgpu::IndexFormat::Uint32) => {
                // Make sure the global quad index buffer is large enough
                if self.quad_index_buffer_u32.len() < quad_index_length {
                    // Make sure we aren't over the max
                    if vert_length > u32::MAX as usize {
                        panic!(
                            "More than u32::MAX({}) verts({}) for type({}) using an index buffer!",
                            u32::MAX,
                            vert_length,
                            core::any::type_name::<V>()
                        );
                    }
                    self.quad_index_buffer_u32 =
                        create_quad_index_buffer_u32(&self.device, vert_length);
                }
            },
            None => {},
        }
    }

    pub fn create_sprite_verts(&mut self, mesh: Mesh<sprite::Vertex>) -> sprite::SpriteVerts {
        self.ensure_sufficient_index_length::<sprite::Vertex>(sprite::VERT_PAGE_SIZE as usize);
        sprite::create_verts_buffer(&self.device, mesh)
    }

    /// Create a new model from the provided mesh.
    /// If the provided mesh is empty this returns None
    pub fn create_model<V: Vertex>(&mut self, mesh: &Mesh<V>) -> Option<Model<V>> {
        self.ensure_sufficient_index_length::<V>(mesh.vertices().len());
        Model::new(&self.device, mesh)
    }

    /// Create a new dynamic model with the specified size.
    pub fn create_dynamic_model<V: Vertex>(&mut self, size: usize) -> DynamicModel<V> {
        self.ensure_sufficient_index_length::<V>(size);
        DynamicModel::new(&self.device, size)
    }

    /// Update a dynamic model with a mesh and a offset.
    pub fn update_model<V: Vertex>(&self, model: &DynamicModel<V>, mesh: &Mesh<V>, offset: usize) {
        model.update(&self.queue, mesh, offset)
    }

    /// Return the maximum supported texture size.
    pub fn max_texture_size(&self) -> u32 { Self::max_texture_size_raw(&self.device) }

    fn max_texture_size_raw(device: &wgpu::Device) -> u32 {
        device.limits().max_texture_dimension_2d
    }

    /// Create a new immutable texture from the provided image.
    /// # Panics
    /// If the provided data doesn't completely fill the texture this function
    /// will panic.
    pub fn create_texture_with_data_raw(
        &mut self,
        texture_info: &wgpu::TextureDescriptor,
        view_info: &wgpu::TextureViewDescriptor,
        sampler_info: &wgpu::SamplerDescriptor,
        data: &[u8],
    ) -> Texture {
        let tex = Texture::new_raw(&self.device, texture_info, view_info, sampler_info);

        let size = texture_info.size;
        let block_size = texture_info.format.block_size(None).unwrap();
        assert_eq!(
            size.width as usize
                * size.height as usize
                * size.depth_or_array_layers as usize
                * block_size as usize,
            data.len(),
            "Provided data length {} does not fill the provided texture size {:?}",
            data.len(),
            size,
        );

        tex.update(
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
        let texture = Texture::new_raw(&self.device, texture_info, view_info, sampler_info);
        texture.clear(&self.queue); // Needs to be fully initialized for partial writes to work on Dx12 AMD
        texture
    }

    /// Create a new texture from the provided image.
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

    /// Create a new dynamic texture with the specified dimensions.
    ///
    /// Currently only supports Rgba8Srgb.
    pub fn create_dynamic_texture(&mut self, dims: Vec2<u32>) -> Texture {
        Texture::new_dynamic(&self.device, &self.queue, dims.x, dims.y)
    }

    /// Update a texture with the provided offset, size, and data.
    pub fn update_texture<T: bytemuck::Pod>(
        &mut self,
        texture: &Texture,
        offset: [u32; 2],
        size: [u32; 2],
        data: &[T],
    ) {
        texture.update(&self.queue, offset, size, bytemuck::cast_slice(data))
    }

    /// See docs on [`ui::BatchedUploads::submit`].
    pub fn ui_premultiply_upload(
        &mut self,
        target_texture: &Arc<Texture>,
        batch: ui::UploadBatchId,
        image: &image::RgbaImage,
        offset: Vec2<u16>,
    ) -> ui::UploadBatchId {
        let upload = ui::PremultiplyUpload::prepare(
            &self.device,
            &self.queue,
            &self.layouts.premultiply_alpha,
            image,
            offset,
        );
        self.ui_premultiply_uploads
            .submit(target_texture, batch, upload)
    }

    /// Queue to obtain a screenshot on the next frame render
    pub fn create_screenshot(
        &mut self,
        screenshot_handler: impl FnOnce(Result<image::RgbImage, String>) + Send + 'static,
    ) {
        // Queue screenshot
        self.take_screenshot = Some(Box::new(screenshot_handler));
        // Take profiler snapshot
        if self.other_modes.profiler_enabled {
            let file_name = format!(
                "frame-trace_{}.json",
                std::time::SystemTime::now()
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0)
            );

            if let Err(err) = wgpu_profiler::chrometrace::write_chrometrace(
                std::path::Path::new(&file_name),
                &self.profile_times,
            ) {
                error!(?err, "Failed to save GPU timing snapshot");
            } else {
                info!("Saved GPU timing snapshot as: {}", file_name);
            }
        }
    }

    // Consider reenabling at some time
    //
    // /// Queue the rendering of the player silhouette in the upcoming frame.
    // pub fn render_player_shadow(
    //     &mut self,
    //     _model: &figure::FigureModel,
    //     _col_lights: &Texture<ColLightFmt>,
    //     _global: &GlobalModel,
    //     _bones: &Consts<figure::BoneData>,
    //     _lod: &lod_terrain::LodData,
    //     _locals: &Consts<shadow::Locals>,
    // ) { // FIXME: Consider reenabling at some point. /* let (point_shadow_maps,
    //   directed_shadow_maps) = if let Some(shadow_map) = &mut self.shadow_map { (
    //   ( shadow_map.point_res.clone(), shadow_map.point_sampler.clone(), ), (
    //   shadow_map.directed_res.clone(), shadow_map.directed_sampler.clone(), ), )
    //   } else { ( (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
    //   (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()), ) }; let
    //   model = &model.opaque;

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

    Buffer::new(device, wgpu::BufferUsages::INDEX, &indices)
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

    Buffer::new(device, wgpu::BufferUsages::INDEX, &indices)
}

/// Terrain-related buffers segment themselves by depth to allow us to do
/// primitive occlusion culling based on whether the camera is underground or
/// not. This struct specifies the buffer offsets at which various layers start
/// and end.
///
/// 'Deep' structures appear within the range `0..deep_end`.
///
/// 'Shallow' structures appear within the range `deep_end..underground_end`.
///
/// 'Surface' structures appear within the range `underground_end..`.
pub struct AltIndices {
    pub deep_end: usize,
    pub underground_end: usize,
}

/// The mode with which culling based on the camera position relative to the
/// terrain is performed.
#[derive(Copy, Clone, Default)]
pub enum CullingMode {
    /// We need to render all elements of the given structure
    #[default]
    None,
    /// We only need to render surface and shallow (i.e: in the overlapping
    /// region) elements of the structure
    Surface,
    /// We only need to render shallow (i.e: in the overlapping region) and deep
    /// elements of the structure
    Underground,
}
