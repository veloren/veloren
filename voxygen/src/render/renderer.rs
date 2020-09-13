use super::{
    consts::Consts,
    instances::Instances,
    mesh::Mesh,
    model::Model,
    pipelines::{
        clouds, figure, fluid, lod_terrain, particle, postprocess, shadow, skybox, sprite, terrain,
        ui, GlobalsLayouts,
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

/// A type representing data that can be converted to an immutable texture map
/// of ColLight data (used for texture atlases created during greedy meshing).
pub type ColLightInfo = (Vec<[u8; 4]>, Vec2<u16>);

/// Load from a GLSL file.
pub struct Glsl(String);

impl From<String> for Glsl {
    fn from(s: String) -> Glsl { Glsl(s) }
}

impl assets::Asset for Glsl {
    type Loader = assets::LoadFrom<String, assets::StringLoader>;

    const EXTENSION: &'static str = "glsl";
}

struct Shaders {
    constants: AssetHandle<Glsl>,
    globals: AssetHandle<Glsl>,
    sky: AssetHandle<Glsl>,
    light: AssetHandle<Glsl>,
    srgb: AssetHandle<Glsl>,
    random: AssetHandle<Glsl>,
    lod: AssetHandle<Glsl>,
    shadows: AssetHandle<Glsl>,

    anti_alias_none: AssetHandle<Glsl>,
    anti_alias_fxaa: AssetHandle<Glsl>,
    anti_alias_msaa_x4: AssetHandle<Glsl>,
    anti_alias_msaa_x8: AssetHandle<Glsl>,
    anti_alias_msaa_x16: AssetHandle<Glsl>,
    cloud_none: AssetHandle<Glsl>,
    cloud_regular: AssetHandle<Glsl>,
    figure_vert: AssetHandle<Glsl>,

    terrain_point_shadow_vert: AssetHandle<Glsl>,
    terrain_directed_shadow_vert: AssetHandle<Glsl>,
    figure_directed_shadow_vert: AssetHandle<Glsl>,
    directed_shadow_frag: AssetHandle<Glsl>,

    skybox_vert: AssetHandle<Glsl>,
    skybox_frag: AssetHandle<Glsl>,
    figure_frag: AssetHandle<Glsl>,
    terrain_vert: AssetHandle<Glsl>,
    terrain_frag: AssetHandle<Glsl>,
    fluid_vert: AssetHandle<Glsl>,
    fluid_frag_cheap: AssetHandle<Glsl>,
    fluid_frag_shiny: AssetHandle<Glsl>,
    sprite_vert: AssetHandle<Glsl>,
    sprite_frag: AssetHandle<Glsl>,
    particle_vert: AssetHandle<Glsl>,
    particle_frag: AssetHandle<Glsl>,
    ui_vert: AssetHandle<Glsl>,
    ui_frag: AssetHandle<Glsl>,
    lod_terrain_vert: AssetHandle<Glsl>,
    lod_terrain_frag: AssetHandle<Glsl>,
    clouds_vert: AssetHandle<Glsl>,
    clouds_frag: AssetHandle<Glsl>,
    postprocess_vert: AssetHandle<Glsl>,
    postprocess_frag: AssetHandle<Glsl>,
    player_shadow_frag: AssetHandle<Glsl>,
    light_shadows_geom: AssetHandle<Glsl>,
    light_shadows_frag: AssetHandle<Glsl>,
}

impl assets::Compound for Shaders {
    // TODO: Taking the specifier argument as a base for shaders specifiers
    // would allow to use several shaders groups easily
    fn load<S: assets::source::Source>(
        _: &assets::AssetCache<S>,
        _: &str,
    ) -> Result<Shaders, assets::Error> {
        Ok(Shaders {
            constants: AssetExt::load("voxygen.shaders.include.constants")?,
            globals: AssetExt::load("voxygen.shaders.include.globals")?,
            sky: AssetExt::load("voxygen.shaders.include.sky")?,
            light: AssetExt::load("voxygen.shaders.include.light")?,
            srgb: AssetExt::load("voxygen.shaders.include.srgb")?,
            random: AssetExt::load("voxygen.shaders.include.random")?,
            lod: AssetExt::load("voxygen.shaders.include.lod")?,
            shadows: AssetExt::load("voxygen.shaders.include.shadows")?,

            anti_alias_none: AssetExt::load("voxygen.shaders.antialias.none")?,
            anti_alias_fxaa: AssetExt::load("voxygen.shaders.antialias.fxaa")?,
            anti_alias_msaa_x4: AssetExt::load("voxygen.shaders.antialias.msaa-x4")?,
            anti_alias_msaa_x8: AssetExt::load("voxygen.shaders.antialias.msaa-x8")?,
            anti_alias_msaa_x16: AssetExt::load("voxygen.shaders.antialias.msaa-x16")?,
            cloud_none: AssetExt::load("voxygen.shaders.include.cloud.none")?,
            cloud_regular: AssetExt::load("voxygen.shaders.include.cloud.regular")?,
            figure_vert: AssetExt::load("voxygen.shaders.figure-vert")?,

            terrain_point_shadow_vert: AssetExt::load("voxygen.shaders.light-shadows-vert")?,
            terrain_directed_shadow_vert: AssetExt::load(
                "voxygen.shaders.light-shadows-directed-vert",
            )?,
            figure_directed_shadow_vert: AssetExt::load(
                "voxygen.shaders.light-shadows-figure-vert",
            )?,
            directed_shadow_frag: AssetExt::load("voxygen.shaders.light-shadows-directed-frag")?,

            skybox_vert: AssetExt::load("voxygen.shaders.skybox-vert")?,
            skybox_frag: AssetExt::load("voxygen.shaders.skybox-frag")?,
            figure_frag: AssetExt::load("voxygen.shaders.figure-frag")?,
            terrain_vert: AssetExt::load("voxygen.shaders.terrain-vert")?,
            terrain_frag: AssetExt::load("voxygen.shaders.terrain-frag")?,
            fluid_vert: AssetExt::load("voxygen.shaders.fluid-vert")?,
            fluid_frag_cheap: AssetExt::load("voxygen.shaders.fluid-frag.cheap")?,
            fluid_frag_shiny: AssetExt::load("voxygen.shaders.fluid-frag.shiny")?,
            sprite_vert: AssetExt::load("voxygen.shaders.sprite-vert")?,
            sprite_frag: AssetExt::load("voxygen.shaders.sprite-frag")?,
            particle_vert: AssetExt::load("voxygen.shaders.particle-vert")?,
            particle_frag: AssetExt::load("voxygen.shaders.particle-frag")?,
            ui_vert: AssetExt::load("voxygen.shaders.ui-vert")?,
            ui_frag: AssetExt::load("voxygen.shaders.ui-frag")?,
            lod_terrain_vert: AssetExt::load("voxygen.shaders.lod-terrain-vert")?,
            lod_terrain_frag: AssetExt::load("voxygen.shaders.lod-terrain-frag")?,
            clouds_vert: AssetExt::load("voxygen.shaders.clouds-vert")?,
            clouds_frag: AssetExt::load("voxygen.shaders.clouds-frag")?,
            postprocess_vert: AssetExt::load("voxygen.shaders.postprocess-vert")?,
            postprocess_frag: AssetExt::load("voxygen.shaders.postprocess-frag")?,
            player_shadow_frag: AssetExt::load("voxygen.shaders.player-shadow-frag")?,
            light_shadows_geom: AssetExt::load("voxygen.shaders.light-shadows-geom")?,
            light_shadows_frag: AssetExt::load("voxygen.shaders.light-shadows-frag")?,
        })
    }
}

/// A type that holds shadow map data.  Since shadow mapping may not be
/// supported on all platforms, we try to keep it separate.
pub struct ShadowMapRenderer {
    // directed_encoder: gfx::Encoder<gfx_backend::Resources, gfx_backend::CommandBuffer>,
    // point_encoder: gfx::Encoder<gfx_backend::Resources, gfx_backend::CommandBuffer>,
    directed_depth_stencil_view: wgpu::TextureView,
    directed_sampler: wgpu::Sampler,

    point_depth_stencil_view: wgpu::TextureView,
    point_sampler: wgpu::Sampler,

    point_pipeline: shadow::ShadowPipeline,
    terrain_directed_pipeline: shadow::ShadowPipeline,
    figure_directed_pipeline: shadow::ShadowFigurePipeline,
    layout: shadow::ShadowLayout,
}

/// A type that stores all the layouts associated with this renderer.
pub struct Layouts {
    pub(self) global: GlobalsLayouts,

    pub(self) figure: figure::FigureLayout,
    pub(self) fluid: fluid::FluidLayout,
    pub(self) postprocess: postprocess::PostProcessLayout,
    pub(self) shadow: shadow::ShadowLayout,
    pub(self) sprite: sprite::SpriteLayout,
    pub(self) terrain: terrain::TerrainLayout,
    pub(self) ui: ui::UILayout,
}

/// A type that encapsulates rendering state. `Renderer` is central to Voxygen's
/// rendering subsystem and contains any state necessary to interact with the
/// GPU, along with pipeline state objects (PSOs) needed to renderer different
/// kinds of models to the screen.
pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    swap_chain: wgpu::SwapChain,
    sc_desc: wgpu::SwapChainDescriptor,

    win_depth_view: wgpu::TextureView,

    tgt_color_view: wgpu::TextureView,
    tgt_depth_stencil_view: wgpu::TextureView,
    // TODO: rename
    tgt_color_pp_view: wgpu::TextureView,

    sampler: wgpu::Sampler,

    shadow_map: Option<ShadowMapRenderer>,

    layouts: Layouts,

    figure_pipeline: figure::FigurePipeline,
    fluid_pipeline: fluid::FluidPipeline,
    lod_terrain_pipeline: lod_terrain::LodTerrainPipeline,
    particle_pipeline: particle::ParticlePipeline,
    //clouds_pipeline: wgpu::RenderPipeline,
    postprocess_pipeline: postprocess::PostProcessPipeline,
    // Consider reenabling at some time
    // player_shadow_pipeline: figure::FigurePipeline,
    skybox_pipeline: skybox::SkyboxPipeline,
    sprite_pipeline: sprite::SpritePipeline,
    terrain_pipeline: terrain::TerrainPipeline,
    ui_pipeline: ui::UIPipeline,

    shaders: AssetHandle<Shaders>,

    noise_tex: Texture,

    mode: RenderMode,
}

impl Renderer {
    /// Create a new `Renderer` from a variety of backend-specific components
    /// and the window targets.
    pub async fn new(
        window: &winit::window::Window,
        mode: RenderMode,
    ) -> Result<Self, RenderError> {
        // Enable seamless cubemaps globally, where available--they are essentially a
        // strict improvement on regular cube maps.
        //
        // Note that since we only have to enable this once globally, there is no point
        // in doing this on rerender.
        // Self::enable_seamless_cube_maps(&mut device);

        let dims = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY | wgpu::BackendBit::SECONDARY);

        // This is unsafe because the window handle must be valid, if you find a way to
        // have an invalid winit::Window then you have bigger issues
        #[allow(unsafe_code)]
        let surface = unsafe { instance.create_surface(window) };

        let adapter = instance
            .request_adapter(wgpu::RequestAdapterOptionsBase {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(surface),
            })
            .await
            .ok_or(RenderError::CouldNotFindAdapter)?;

        use wgpu::{Features, Limits};

        let (device, queue) = adapter
            .request_device(
                wgpu::DeviceDescriptor {
                    // TODO
                    features: Features::DEPTH_CLAMPING | Features::ADDRESS_MODE_CLAMP_TO_BORDER,
                    limits: Limits::default(),
                    shader_validation: true,
                },
                None,
            )
            .await?;

        let info = device.get_info();
        info!(
            ?info.name,
            ?info.vendor,
            ?info.backend,
            ?info.device,
            ?info.device_type,
            "selected graphics device"
        );

        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: dims.0,
            height: dims.1,
            present_mode: wgpu::PresentMode::Immediate,
        };

        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        let shadow_views = Self::create_shadow_views(
            &device,
            (dims.0, dims.1),
            &ShadowMapMode::try_from(mode.shadow).unwrap_or_default(),
        )
        .map_err(|err| {
            warn!("Could not create shadow map views: {:?}", err);
        })
        .ok();

        let shaders = Shaders::load_expect("");

        let layouts = {
            let global = GlobalsLayouts::new(&device);

            let figure = figure::FigureLayout::new(&device);
            let fluid = fluid::FluidLayout::new(&device);
            let postprocess = postprocess::PostProcessLayout::new(&device);
            let shadow = shadow::ShadowLayout::new(&device);
            let sprite = sprite::SpriteLayout::new(&device);
            let terrain = terrain::TerrainLayout::new(&device);
            let ui = ui::UILayout::new(&device);

            Layouts {
                global,

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
            skybox_pipeline,
            figure_pipeline,
            terrain_pipeline,
            fluid_pipeline,
            sprite_pipeline,
            particle_pipeline,
            ui_pipeline,
            lod_terrain_pipeline,
            clouds_pipeline,
            postprocess_pipeline,
            //player_shadow_pipeline,
            point_shadow_pipeline,
            terrain_directed_shadow_pipeline,
            figure_directed_shadow_pipeline,
        ) = create_pipelines(
            &device,
            &layouts & mode,
            shadow_views.is_some(),
        )?;

        let (tgt_color_view, tgt_depth_stencil_view, tgt_color_pp_view, win_depth_view) =
            Self::create_rt_views(&device, (dims.0, dims.1), &mode)?;

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
            let (
                point_depth_stencil_view,
                point_res,
                point_sampler,
                directed_depth_stencil_view,
                directed_res,
                directed_sampler,
            ) = shadow_views;

            let layout = shadow::ShadowLayout::new(&device);

            Some(ShadowMapRenderer {
                directed_depth_stencil_view,
                directed_sampler,

                // point_encoder: factory.create_command_buffer().into(),
                // directed_encoder: factory.create_command_buffer().into(),
                point_depth_stencil_view,
                point_sampler,

                point_pipeline,
                terrain_directed_pipeline,
                figure_directed_pipeline,

                layout,
            })
        } else {
            None
        };

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: None,
            ..Default::default()
        });

        let noise_tex = Texture::new(
            &device,
            &queue,
            &assets::Image::load_expect("voxygen.texture.noise").read().0,
            Some(wgpu::FilterMode::Linear),
            Some(wgpu::AddressMode::Repeat),
        )?;

        Ok(Self {
            device,
            queue,
            swap_chain,
            sc_desc,

            win_depth_view,

            tgt_color_view,
            tgt_depth_stencil_view,
            tgt_color_pp_view,

            sampler,

            shadow_map,

            layouts,

            skybox_pipeline,
            figure_pipeline,
            terrain_pipeline,
            fluid_pipeline,
            sprite_pipeline,
            particle_pipeline,
            ui_pipeline,
            lod_terrain_pipeline,
            clouds_pipeline,
            postprocess_pipeline,
            shaders,
            //player_shadow_pipeline,
            shader_reload_indicator,

            noise_tex,

            mode,
        })
    }

    /// Get references to the internal render target views that get rendered to
    /// before post-processing.
    #[allow(dead_code)]
    pub fn tgt_views(&self) -> (&wgpu::TextureView, &wgpu::TextureView) {
        (&self.tgt_color_view, &self.tgt_depth_stencil_view)
    }

    /// Get references to the internal render target views that get displayed
    /// directly by the window.
    #[allow(dead_code)]
    pub fn win_views(&self) -> &wgpu::TextureView { &self.win_depth_view }

    /// Change the render mode.
    pub fn set_render_mode(&mut self, mode: RenderMode) -> Result<(), RenderError> {
        self.mode = mode;

        // Recreate render target
        self.on_resize()?;

        // Recreate pipelines with the new AA mode
        self.recreate_pipelines();

        Ok(())
    }

    /// Get the render mode.
    pub fn render_mode(&self) -> &RenderMode { &self.mode }

    /// Resize internal render targets to match window render target dimensions.
    pub fn on_resize(&mut self) -> Result<(), RenderError> {
        let dims = self.win_color_view.get_dimensions();

        // Avoid panics when creating texture with w,h of 0,0.
        if dims.0 != 0 && dims.1 != 0 {
            let (
                tgt_color_view,
                tgt_depth_stencil_view,
                tgt_color_pp_view,
                tgt_color_res,
                tgt_depth_res,
                tgt_color_res_pp,
            ) = Self::create_rt_views(&mut self.factory, (dims.0, dims.1), &self.mode)?;
            self.tgt_color_res = tgt_color_res;
            self.tgt_depth_res = tgt_depth_res;
            self.tgt_color_res_pp = tgt_color_res_pp;
            self.tgt_color_view = tgt_color_view;
            self.tgt_depth_stencil_view = tgt_depth_stencil_view;
            self.tgt_color_pp_view = tgt_color_pp_view;
            if let (Some(shadow_map), ShadowMode::Map(mode)) =
                (self.shadow_map.as_mut(), self.mode.shadow)
            {
                match Self::create_shadow_views(&mut self.factory, (dims.0, dims.1), &mode) {
                    Ok((
                        point_depth_stencil_view,
                        point_res,
                        point_sampler,
                        directed_depth_stencil_view,
                        directed_res,
                        directed_sampler,
                    )) => {
                        shadow_map.point_depth_stencil_view = point_depth_stencil_view;
                        shadow_map.point_res = point_res;
                        shadow_map.point_sampler = point_sampler;

                        shadow_map.directed_depth_stencil_view = directed_depth_stencil_view;
                        shadow_map.directed_res = directed_res;
                        shadow_map.directed_sampler = directed_sampler;
                    },
                    Err(err) => {
                        warn!("Could not create shadow map views: {:?}", err);
                    },
                }
            }
        }

        Ok(())
    }

    fn create_rt_views(
        device: &wgpu::Device,
        size: (u16, u16),
        mode: &RenderMode,
    ) -> Result<
        (
            wgpu::TextureView,
            wgpu::TextureView,
            wgpu::TextureView,
            wgpu::TextureView,
        ),
        RenderError,
    > {
        let upscaled = Vec2::from(size)
            .map(|e: u16| (e as f32 * mode.upscale_mode.factor) as u16)
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

        let mut color_view = || {
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth: 1,
                },
                mip_level_count: levels,
                sample_count,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            });

            tex.create_view(&wgpu::TextureViewDescriptor {
                label: None,
                format: Some(wgpu::TextureFormat::Rgba8UnormSrgb),
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::Color,
                base_mip_level: 0,
                level_count: Some(levels),
                base_array_layer: 0,
                array_layer_count: None,
            })
        };

        let tgt_color_view = color_view();
        let tgt_color_pp_view = color_view();

        let tgt_depth_stencil_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width,
                height,
                depth: 1,
            },
            mip_level_count: levels,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        });
        let tgt_depth_stencil_view =
            tgt_depth_stencil_tex.create_view(&wgpu::TextureViewDescriptor {
                label: None,
                format: Some(wgpu::TextureFormat::Depth24Plus),
                dimension: Some(wgpu::TextureViewDimension::D2),
                aspect: wgpu::TextureAspect::DepthOnly,
                base_mip_level: 0,
                level_count: Some(levels),
                base_array_layer: 0,
                array_layer_count: None,
            });

        let win_depth_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: size.0,
                height: size.1,
                depth: 1,
            },
            mip_level_count: levels,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        });
        let win_depth_view = tgt_depth_stencil_tex.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            format: Some(wgpu::TextureFormat::Depth24Plus),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::DepthOnly,
            base_mip_level: 0,
            level_count: Some(levels),
            base_array_layer: 0,
            array_layer_count: None,
        });

        Ok((
            tgt_color_view,
            tgt_depth_stencil_view,
            tgt_color_pp_view,
            win_depth_view,
        ))
    }

    /// Create textures and views for shadow maps.
    // This is a one-use type and the two halves are not guaranteed to remain identical, so we
    // disable the type complexity lint.
    #[allow(clippy::type_complexity)]
    fn create_shadow_views(
        device: &wgpu::Device,
        size: (u16, u16),
        mode: &ShadowMapMode,
    ) -> Result<
        (
            wgpu::TextureView,
            wgpu::Sampler,
            wgpu::TextureView,
            wgpu::Sampler,
        ),
        RenderError,
    > {
        // (Attempt to) apply resolution factor to shadow map resolution.
        let resolution_factor = mode.resolution.clamped(0.25, 4.0);

        let max_texture_size = Self::max_texture_size_raw(device);
        // Limit to max texture size, rather than erroring.
        let size = Vec2::new(size.0, size.1).map(|e| {
            let size = f32::from(e) * resolution_factor;
            // NOTE: We know 0 <= e since we clamped the resolution factor to be between
            // 0.25 and 4.0.
            if size <= f32::from(max_texture_size) {
                size as u16
            } else {
                max_texture_size
            }
        });

        let levels = 1;
        // Limit to max texture size rather than erroring.
        let two_size = size.map(|e| {
            u16::checked_next_power_of_two(e)
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
                (diag_size as u16, diag_cross_size as u16)
            } else {
                // Limit to max texture resolution rather than error.
                (max_texture_size as u16, max_texture_size as u16)
            };
        let diag_two_size = u16::checked_next_power_of_two(diag_size)
            .filter(|&e| e <= max_texture_size)
            // Limit to max texture resolution rather than error.
            .unwrap_or(max_texture_size);

        let point_shadow_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: diag_two_size / 4,
                height: diag_two_size / 4,
                depth: 6,
            },
            mip_level_count: levels,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        });

        let point_tgt_shadow_view = point_shadow_tex.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            format: Some(wgpu::TextureFormat::Depth24Plus),
            dimension: Some(wgpu::TextureViewDimension::Cube),
            aspect: wgpu::TextureAspect::DepthOnly,
            base_mip_level: 0,
            level_count: Some(levels),
            base_array_layer: 0,
            array_layer_count: None,
        });

        let directed_shadow_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: diag_two_size,
                height: diag_two_size,
                depth: 1,
            },
            mip_level_count: levels,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        });

        let directed_tgt_shadow_view = point_shadow_tex.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            format: Some(wgpu::TextureFormat::Depth24Plus),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::DepthOnly,
            base_mip_level: 0,
            level_count: Some(levels),
            base_array_layer: 0,
            array_layer_count: None,
        });

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

        let point_shadow_tex_sampler = device.create_sampler(&sampler_info);
        let directed_shadow_tex_sampler = device.create_sampler(&sampler_info);

        Ok((
            point_tgt_shadow_view,
            point_shadow_tex_sampler,
            directed_tgt_shadow_view,
            directed_shadow_tex_sampler,
        ))
    }

    /// Get the resolution of the render target.
    /// Note: the change after a resize can be delayed so
    /// don't rely on this value being constant between resize events
    pub fn get_resolution(&self) -> Vec2<u16> {
        Vec2::new(
            self.win_color_view.get_dimensions().0,
            self.win_color_view.get_dimensions().1,
        )
    }

    /// Get the resolution of the shadow render target.
    pub fn get_shadow_resolution(&self) -> (Vec2<u16>, Vec2<u16>) {
        if let Some(shadow_map) = &self.shadow_map {
            let point_dims = shadow_map.point_depth_stencil_view.get_dimensions();
            let directed_dims = shadow_map.directed_depth_stencil_view.get_dimensions();
            (
                Vec2::new(point_dims.0, point_dims.1),
                Vec2::new(directed_dims.0, directed_dims.1),
            )
        } else {
            (Vec2::new(1, 1), Vec2::new(1, 1))
        }
    }

    /// Queue the clearing of the shadow targets ready for a new frame to be
    /// rendered.
    pub fn clear_shadows(&mut self) {
        span!(_guard, "clear_shadows", "Renderer::clear_shadows");
        if !self.mode.shadow.is_map() {
            return;
        }
        if let Some(shadow_map) = self.shadow_map.as_mut() {
            // let point_encoder = &mut shadow_map.point_encoder;
            let point_encoder = &mut self.encoder;
            point_encoder.clear_depth(&shadow_map.point_depth_stencil_view, 1.0);
            // let directed_encoder = &mut shadow_map.directed_encoder;
            let directed_encoder = &mut self.encoder;
            directed_encoder.clear_depth(&shadow_map.directed_depth_stencil_view, 1.0);
        }
    }

    /// NOTE: Supported by Vulkan (by default), DirectX 10+ (it seems--it's hard
    /// to find proof of this, but Direct3D 10 apparently does it by
    /// default, and 11 definitely does, so I assume it's natively supported
    /// by DirectX itself), OpenGL 3.2+, and Metal (done by default).  While
    /// there may be some GPUs that don't quite support it correctly, the
    /// impact is relatively small, so there is no reason not to enable it where
    /// available.
    #[allow(unsafe_code)]
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

    /// Queue the clearing of the depth target ready for a new frame to be
    /// rendered.
    pub fn clear(&mut self) {
        span!(_guard, "clear", "Renderer::clear");
        self.encoder.clear_depth(&self.tgt_depth_stencil_view, 1.0);
        // self.encoder.clear_stencil(&self.tgt_depth_stencil_view, 0);
        self.encoder.clear_depth(&self.win_depth_view, 1.0);
    }

    // /// Set up shadow rendering.
    // pub fn start_shadows(&mut self) {
    //     if !self.mode.shadow.is_map() {
    //         return;
    //     }
    //     if let Some(_shadow_map) = self.shadow_map.as_mut() {
    //         self.encoder.flush(&mut self.device);
    //         Self::set_depth_clamp(&mut self.device, true);
    //     }
    // }

    // /// Perform all queued draw calls for global.shadows.
    // pub fn flush_shadows(&mut self) {
    //     if !self.mode.shadow.is_map() {
    //         return;
    //     }
    //     if let Some(_shadow_map) = self.shadow_map.as_mut() {
    //         let point_encoder = &mut self.encoder;
    //         // let point_encoder = &mut shadow_map.point_encoder;
    //         point_encoder.flush(&mut self.device);
    //         // let directed_encoder = &mut shadow_map.directed_encoder;
    //         // directed_encoder.flush(&mut self.device);
    //         // Reset depth clamping.
    //         Self::set_depth_clamp(&mut self.device, false);
    //     }
    // }

    /// Perform all queued draw calls for this frame and clean up discarded
    /// items.
    pub fn flush(&mut self) {
        span!(_guard, "flush", "Renderer::flush");
        self.encoder.flush(&mut self.device);
        self.device.cleanup();

        // If the shaders files were changed attempt to recreate the shaders
        if self.shaders.reloaded() {
            self.recreate_pipelines();
        }
    }

    /// Recreate the pipelines
    fn recreate_pipelines(&mut self) {
        match create_pipelines(
            &mut self.factory,
            &self.shaders.read(),
            &self.mode,
            self.shadow_map.is_some(),
        ) {
            Ok((
                skybox_pipeline,
                figure_pipeline,
                terrain_pipeline,
                fluid_pipeline,
                sprite_pipeline,
                particle_pipeline,
                ui_pipeline,
                lod_terrain_pipeline,
                clouds_pipeline,
                postprocess_pipeline,
                player_shadow_pipeline,
                point_shadow_pipeline,
                terrain_directed_shadow_pipeline,
                figure_directed_shadow_pipeline,
            )) => {
                self.skybox_pipeline = skybox_pipeline;
                self.figure_pipeline = figure_pipeline;
                self.terrain_pipeline = terrain_pipeline;
                self.fluid_pipeline = fluid_pipeline;
                self.sprite_pipeline = sprite_pipeline;
                self.particle_pipeline = particle_pipeline;
                self.ui_pipeline = ui_pipeline;
                self.lod_terrain_pipeline = lod_terrain_pipeline;
                self.clouds_pipeline = clouds_pipeline;
                self.postprocess_pipeline = postprocess_pipeline;
                self.player_shadow_pipeline = player_shadow_pipeline;
                if let (
                    Some(point_pipeline),
                    Some(terrain_directed_pipeline),
                    Some(figure_directed_pipeline),
                    Some(shadow_map),
                ) = (
                    point_shadow_pipeline,
                    terrain_directed_shadow_pipeline,
                    figure_directed_shadow_pipeline,
                    self.shadow_map.as_mut(),
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
    pub fn create_consts<T: Copy + zerocopy::AsBytes>(
        &mut self,
        vals: &[T],
    ) -> Result<Consts<T>, RenderError> {
        let mut consts = Consts::new(&mut self.factory, vals.len());
        consts.update(&mut self.encoder, vals, 0)?;
        Ok(consts)
    }

    /// Update a set of constants with the provided values.
    pub fn update_consts<T: Copy + zerocopy::AsBytes>(
        &mut self,
        consts: &mut Consts<T>,
        vals: &[T],
    ) -> Result<(), RenderError> {
        consts.update(&mut self.encoder, vals, 0)
    }

    /// Create a new set of instances with the provided values.
    pub fn create_instances<T: Copy + zerocopy::AsBytes>(
        &mut self,
        vals: &[T],
    ) -> Result<Instances<T>, RenderError> {
        let mut instances = Instances::new(&mut self.factory, vals.len())?;
        instances.update(&mut self.encoder, vals)?;
        Ok(instances)
    }

    /// Create a new model from the provided mesh.
    pub fn create_model<V: Vertex>(&mut self, mesh: &Mesh<V>) -> Result<Model<V>, RenderError> {
        Ok(Model::new(&mut self.factory, mesh))
    }

    /// Create a new dynamic model with the specified size.
    pub fn create_dynamic_model<V: Vertex>(
        &mut self,
        size: usize,
    ) -> Result<Model<V>, RenderError> {
        Model::new(&self.device, size)
    }

    /// Update a dynamic model with a mesh and a offset.
    pub fn update_model<V: Vertex>(
        &mut self,
        model: &Model<V>,
        mesh: &Mesh<V>,
        offset: usize,
    ) -> Result<(), RenderError> {
        model.update(&mut self.encoder, mesh, offset)
    }

    /// Return the maximum supported texture size.
    pub fn max_texture_size(&self) -> u16 { Self::max_texture_size_raw(&self.factory) }

    /// Return the maximum supported texture size from the factory.
    fn max_texture_size_raw(device: &wgpu::Device) -> u16 {
        // This value is temporary as there are plans to include a way to get this in
        // wgpu this is just a sane standard for now
        8192
    }

    /// Create a new immutable texture from the provided image.
    pub fn create_texture_with_data_raw(
        &mut self,
        texture_info: wgpu::TextureDescriptor,
        sampler_info: wgpu::SamplerDescriptor,
        bytes_per_row: u32,
        data: &[u8],
    ) -> Texture {
        let tex = Texture::new_raw(&self.device, texture_info, sampler_info);

        tex.update(
            &self.device,
            &self.queue,
            [0; 2],
            [texture_info.size.x, texture_info.size.y],
            data,
            bytes_per_row,
        );

        tex
    }

    /// Create a new raw texture.
    pub fn create_texture_raw(
        &mut self,
        texture_info: wgpu::TextureDescriptor,
        sampler_info: wgpu::SamplerDescriptor,
    ) -> Texture {
        Texture::new_raw(&self.device, texture_info, sampler_info)
    }

    /// Create a new texture from the provided image.
    ///
    /// Currently only supports Rgba8Srgb
    pub fn create_texture(
        &mut self,
        image: &image::DynamicImage,
        filter_method: Option<FilterMode>,
        address_mode: Option<AddressMode>,
    ) -> Texture {
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
    pub fn create_dynamic_texture(&mut self, dims: Vec2<u16>) -> Texture {
        Texture::new_dynamic(&mut self.factory, dims.x, dims.y)
    }

    /// Update a texture with the provided offset, size, and data.
    ///
    /// Currently only supports Rgba8Srgb
    pub fn update_texture(
        &mut self,
        texture: &Texture, /* <T> */
        offset: [u16; 2],
        size: [u16; 2],
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
        bytes_per_row: u32,
    ) {
        texture.update(&mut self.encoder, offset, size, data, bytes_per_row)
    }

    /// Creates a download buffer, downloads the win_color_view, and converts to
    /// a image::DynamicImage.
    #[allow(clippy::map_clone)] // TODO: Pending review in #587
    pub fn create_screenshot(&mut self) -> Result<image::DynamicImage, RenderError> {
        todo!()
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
    // self.tgt_color_view.clone(),             tgt_depth_stencil:
    // (self.tgt_depth_stencil_view.clone()/* , (1, 1) */),         },
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
    // self.tgt_color_view.clone(),             tgt_depth_stencil:
    // (self.tgt_depth_stencil_view.clone()/* , (1, 1) */),         },
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
    // self.tgt_color_view.clone(),             tgt_depth_stencil:
    // (self.tgt_depth_stencil_view.clone()/* , (0, 0) */),         },
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
    // self.tgt_color_view.clone(),             tgt_depth_stencil:
    // (self.tgt_depth_stencil_view.clone()/* , (1, 1) */),         },
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
    // self.tgt_color_view.clone(),             tgt_depth_stencil:
    // (self.tgt_depth_stencil_view.clone()/* , (1, 1) */),         },
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
    //             tgt_depth_stencil: shadow_map.point_depth_stencil_view.clone(),
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
    //             tgt_depth_stencil:
    // shadow_map.directed_depth_stencil_view.clone(),         },
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
    //             tgt_depth_stencil:
    // shadow_map.directed_depth_stencil_view.clone(),         },
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
    // self.tgt_color_view.clone(),             tgt_depth_stencil:
    // (self.tgt_depth_stencil_view.clone()/* , (1, 1) */),         },
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
    // self.tgt_color_view.clone(),             tgt_depth_stencil:
    // (self.tgt_depth_stencil_view.clone()/* , (1, 1) */),         },
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
    // self.tgt_color_view.clone(),             tgt_depth_stencil:
    // (self.tgt_depth_stencil_view.clone()/* , (1, 1) */),         },
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
    // self.tgt_color_view.clone(),             tgt_depth_stencil:
    // (self.tgt_depth_stencil_view.clone()/* , (1, 1) */),         },
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
#[allow(clippy::type_complexity)] // TODO: Pending review in #587
fn create_pipelines(
    device: &wgpu::Device,
    layouts: &Layouts,
    shaders: &Shaders,
    mode: &RenderMode,
    sc_desc: &wgpu::SwapChainDescriptor,
    has_shadow_views: bool,
) -> Result<
    (
        skybox::SkyboxPipeline,
        figure::FigurePipeline,
        terrain::TerrainPipeline,
        fluid::FluidPipeline,
        sprite::SpritePipeline,
        particle::ParticlePipeline,
        ui::UIPipeline,
        lod_terrain::LodTerrainPipeline,
        // TODO: clouds
        postprocess::PostProcessPipeline,
        //figure::FigurePipeline,
        Option<shadow::ShadowPipeline>,
        Option<shadow::ShadowPipeline>,
        Option<shadow::ShadowFigurePipeline>,
    ),
    RenderError,
> {
    use shaderc::{CompileOptions, Compiler, OptimizationLevel, ResolvedInclude, ShaderKind};

    let constants = &shaders.constants.read().0;
    let globals = &shaders.globals.read().0;
    let sky = &shaders.sky.read().0;
    let light = &shaders.light.read().0;
    let srgb = &shaders.srgb.read().0;
    let random = &shaders.random.read().0;
    let lod = &shaders.lod.read().0;
    let shadows = &shaders.shadows.read().0;

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
        constants,
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

    let anti_alias = &match mode.aa {
        AaMode::None => shaders.anti_alias_none,
        AaMode::Fxaa => shaders.anti_alias_fxaa,
        AaMode::MsaaX4 => shaders.anti_alias_msaa_x4,
        AaMode::MsaaX8 => shaders.anti_alias_msaa_x8,
        AaMode::MsaaX16 => shaders.anti_alias_msaa_x16,
    };

    let cloud = &match mode.cloud {
        CloudMode::None => shaders.cloud_none,
        _ => shaders.cloud_regular,
    };

    let mut compiler = Compiler::new().ok_or(RenderError::ErrorInitializingCompiler)?;
    let mut options = CompileOptions::new().ok_or(RenderError::ErrorInitializingCompiler)?;
    options.set_optimization_level(OptimizationLevel::Performance);
    options.set_include_callback(move |name, _, shader_name, _| {
        Ok(ResolvedInclude {
            resolved_name: name,
            content: match name {
                "constants.glsl" => constants,
                "globals.glsl" => globals,
                "shadows.glsl" => shadows,
                "sky.glsl" => sky,
                "light.glsl" => light,
                "srgb.glsl" => srgb,
                "random.glsl" => &random,
                "lod.glsl" => &lod,
                "anti-aliasing.glsl" => &anti_alias,
                "cloud.glsl" => &cloud,
                other => return Err(format!("Include {} is not defined", other)),
            },
        })
    });

    let figure_vert = &shaders.figure_vert.read().0;

    let terrain_point_shadow_vert = &shaders.terrain_point_shadow_vert.read().0;

    let terrain_directed_shadow_vert = &shaders.terrain_directed_shadow_vert.read().0;

    let figure_directed_shadow_vert = &shadows.figure_directed_shadow_vert.read().0;

    let directed_shadow_frag = &shaders.directed_shadow_frag.read().0;

    let figure_vert_mod = create_shader_module(
        device,
        &mut compiler,
        figure_vert,
        ShaderKind::Vertex,
        "figure-vert.glsl",
        &options,
    )?;

    let terrain_point_shadow_vert_mod = create_shader_module(
        device,
        &mut compiler,
        terrain_point_shadow_vert,
        ShaderKind::Vertex,
        "light-shadows-vert.glsl",
        &options,
    )?;

    let terrain_directed_shadow_vert_mod = create_shader_module(
        device,
        &mut compiler,
        terrain_directed_shadow_vert,
        ShaderKind::Vertex,
        "light-shadows-directed-vert.glsl",
        &options,
    )?;

    let figure_directed_shadow_vert_mod = create_shader_module(
        device,
        &mut compiler,
        figure_directed_shadow_vert,
        ShaderKind::Vertex,
        "light-shadows-figure-vert.glsl",
        &options,
    )?;

    // TODO: closure to to make calling this easier

    let directed_shadow_frag_mod = create_shader_module(
        device,
        &mut compiler,
        directed_shadow_frag,
        ShaderKind::Fragment,
        "light-shadows-directed-frag.glsl",
        &options,
    )?;

    // Construct a pipeline for rendering skyboxes
    let skybox_pipeline = skybox::SkyboxPipeline::new(
        device,
        create_shader_module(
            device,
            &mut compiler,
            shaders.skybox_vert.read().0,
            ShaderKind::Vertex,
            "skybox-vert.glsl",
            &options,
        ),
        create_shader_module(
            device,
            &mut compiler,
            shaders.skybox_frag.read().0,
            ShaderKind::Fragment,
            "skybox-frag.glsl",
            &options,
        ),
        sc_desc,
        layouts,
        mode.aa,
    );

    // Construct a pipeline for rendering figures
    let figure_pipeline = figure::FigurePipeline::new(
        device,
        &figure_vert_mod,
        create_shader_module(
            device,
            &mut compiler,
            shaders.figure_frag.read().0,
            ShaderKind::Fragment,
            "figure-frag.glsl",
            &options,
        ),
        sc_desc,
        layouts,
        mode.aa,
    );

    // Construct a pipeline for rendering terrain
    let terrain_pipeline = terrain::TerrainPipeline::new(
        device,
        create_shader_module(
            device,
            &mut compiler,
            shaders.terrain_vert.read().0,
            ShaderKind::Vertex,
            "terrain-vert.glsl",
            &options,
        ),
        create_shader_module(
            device,
            &mut compiler,
            shaders.terrain_frag.read().0,
            ShaderKind::Fragment,
            "terrain-frag.glsl",
            &options,
        ),
        sc_desc,
        layouts,
        mode.aa,
    );

    // Construct a pipeline for rendering fluids
    let fluid_pipeline = fluid::FluidPipeline::new(
        device,
        create_shader_module(
            device,
            &mut compiler,
            shaders.fluid_vert.read().0,
            ShaderKind::Vertex,
            "terrain-vert.glsl",
            &options,
        ),
        create_shader_module(
            device,
            &mut compiler,
            match mode.fluid {
                FluidMode::Cheap => shaders.fluid_frag_cheap.read().0,
                FluidMode::Shiny => shaders.fluid_frag_shiny.read().0,
            },
            ShaderKind::Fragment,
            "fluid-frag.glsl",
            &options,
        ),
        sc_desc,
        layouts,
        mode.aa,
    );

    // Construct a pipeline for rendering sprites
    let sprite_pipeline = sprite::SpritePipeline::new(
        device,
        create_shader_module(
            device,
            &mut compiler,
            shaders.sprite_vert.read().0,
            ShaderKind::Vertex,
            "sprite-vert.glsl",
            &options,
        ),
        create_shader_module(
            device,
            &mut compiler,
            shaders.sprite_frag.read().0,
            ShaderKind::Fragment,
            "sprite-frag.glsl",
            &options,
        ),
        sc_desc,
        layouts,
        mode.aa,
    );

    // Construct a pipeline for rendering particles
    let particle_pipeline = particle::ParticlePipeline::new(
        device,
        create_shader_module(
            device,
            &mut compiler,
            shaders.particle_vert.read().0,
            ShaderKind::Vertex,
            "particle-vert.glsl",
            &options,
        ),
        create_shader_module(
            device,
            &mut compiler,
            shaders.particle_frag.read().0,
            ShaderKind::Fragment,
            "particle-frag.glsl",
            &options,
        ),
        sc_desc,
        layouts,
        mode.aa,
    );

    // Construct a pipeline for rendering UI elements
    let ui_pipeline = ui::UIPipeline::new(
        device,
        create_shader_module(
            device,
            &mut compiler,
            shaders.ui_vert.read().0,
            ShaderKind::Vertex,
            "ui-vert.glsl",
            &options,
        ),
        create_shader_module(
            device,
            &mut compiler,
            shaders.ui_frag.read().0,
            ShaderKind::Fragment,
            "ui-frag.glsl",
            &options,
        ),
        sc_desc,
        layouts,
        mode.aa,
    );

    // Construct a pipeline for rendering terrain
    let lod_terrain_pipeline = lod_terrain::LodTerrainPipeline::new(
        device,
        create_shader_module(
            device,
            &mut compiler,
            shaders.lod_terrain_vert.read().0,
            ShaderKind::Vertex,
            "lod-terrain-vert.glsl",
            &options,
        ),
        create_shader_module(
            device,
            &mut compiler,
            shaders.lod_terrain_frag.read().0,
            ShaderKind::Fragment,
            "lod-terrain-frag.glsl",
            &options,
        ),
        sc_desc,
        layouts,
        mode.aa,
    );

    // Construct a pipeline for rendering our clouds (a kind of post-processing)
    // let clouds_pipeline = create_pipeline(
    //     factory,
    //     clouds::pipe::new(),
    //     &Glsl::load_watched("voxygen.shaders.clouds-vert",
    // shader_reload_indicator).unwrap(),     &Glsl::load_watched("voxygen.
    // shaders.clouds-frag", shader_reload_indicator).unwrap(),
    //     &include_ctx,
    //     gfx::state::CullFace::Back,
    // )?;

    // Construct a pipeline for rendering our post-processing
    let postprocess_pipeline = postprocess::PostProcessPipeline::new(
        device,
        create_shader_module(
            device,
            &mut compiler,
            shaders.postprocess_vert.read().0,
            ShaderKind::Vertex,
            "postprocess-vert.glsl",
            &options,
        ),
        create_shader_module(
            device,
            &mut compiler,
            shaders.postprocess_frag.read().0,
            ShaderKind::Fragment,
            "postprocess-frag.glsl",
            &options,
        ),
        sc_desc,
        layouts,
        mode.aa,
    );

    // Consider reenabling at some time in the future
    //
    // // Construct a pipeline for rendering the player silhouette
    // let player_shadow_pipeline = create_pipeline(
    //     factory,
    //     figure::pipe::Init {
    //         tgt_depth_stencil: (gfx::preset::depth::PASS_TEST/*,
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

    // Sharp can fix it later ;)
    //
    // // Construct a pipeline for rendering point light terrain shadow maps.
    // let point_shadow_pipeline = match create_shadow_pipeline(
    //     factory,
    //     shadow::pipe::new(),
    //     &terrain_point_shadow_vert,
    //     Some(
    //         &Glsl::load_watched(
    //             "voxygen.shaders.light-shadows-geom",
    //             shader_reload_indicator,
    //         )
    //         .unwrap(),
    //     ),
    //     &Glsl::load_watched(
    //         "voxygen.shaders.light-shadows-frag",
    //         shader_reload_indicator,
    //     )
    //     .unwrap(),
    //     &include_ctx,
    //     gfx::state::CullFace::Back,
    //     None, // Some(gfx::state::Offset(2, 0))
    // ) {
    //     Ok(pipe) => Some(pipe),
    //     Err(err) => {
    //         warn!("Could not load point shadow map pipeline: {:?}", err);
    //         None
    //     },
    // };

    // // Construct a pipeline for rendering directional light terrain shadow maps.
    // let terrain_directed_shadow_pipeline = match create_shadow_pipeline(
    //     factory,
    //     shadow::pipe::new(),
    //     &terrain_directed_shadow_vert,
    //     None,
    //     &directed_shadow_frag,
    //     &include_ctx,
    //     gfx::state::CullFace::Back,
    //     None, // Some(gfx::state::Offset(2, 1))
    // ) {
    //     Ok(pipe) => Some(pipe),
    //     Err(err) => {
    //         warn!(
    //             "Could not load directed terrain shadow map pipeline: {:?}",
    //             err
    //         );
    //         None
    //     },
    // };

    // // Construct a pipeline for rendering directional light figure shadow maps.
    // let figure_directed_shadow_pipeline = match create_shadow_pipeline(
    //     factory,
    //     shadow::figure_pipe::new(),
    //     &figure_directed_shadow_vert,
    //     None,
    //     &directed_shadow_frag,
    //     &include_ctx,
    //     gfx::state::CullFace::Back,
    //     None, // Some(gfx::state::Offset(2, 1))
    // ) {
    //     Ok(pipe) => Some(pipe),
    //     Err(err) => {
    //         warn!(
    //             "Could not load directed figure shadow map pipeline: {:?}",
    //             err
    //         );
    //         None
    //     },
    // };

    Ok((
        skybox_pipeline,
        figure_pipeline,
        terrain_pipeline,
        fluid_pipeline,
        sprite_pipeline,
        particle_pipeline,
        ui_pipeline,
        lod_terrain_pipeline,
        clouds_pipeline,
        postprocess_pipeline,
        // player_shadow_pipeline,
        None,
        None,
        None,
    ))
}

pub fn create_shader_module(
    device: &wgpu::Device,
    compiler: &mut shaderc::Compiler,
    source: &str,
    kind: shaderc::ShaderKind,
    file_name: &str,
    options: &shaderc::CompileOptions,
) -> Result<wgpu::ShaderModule, RenderError> {
    use std::borrow::Cow;

    let spv = compiler.compile_into_spirv(source, kind, file_name, "main", Some(options))?;

    Ok(device.create_shader_module(wgpu::ShaderModule::SpirV(Cow::Bowrrowed(spv.as_binary()))))
}
