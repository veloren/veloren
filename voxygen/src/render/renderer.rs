use super::{
    consts::Consts,
    gfx_backend,
    instances::Instances,
    mesh::Mesh,
    model::{DynamicModel, Model},
    pipelines::{
        clouds, figure, fluid, lod_terrain, particle, postprocess, shadow, skybox, sprite, terrain,
        ui, GlobalModel, Globals,
    },
    texture::Texture,
    AaMode, CloudMode, FilterMethod, FluidMode, LightingMode, Pipeline, RenderError, RenderMode,
    ShadowMapMode, ShadowMode, WrapMode,
};
use common::{
    assets::{self, watch::ReloadIndicator, Asset},
    span,
};
use core::convert::TryFrom;
use gfx::{
    self,
    handle::Sampler,
    state::Comparison,
    traits::{Device, Factory, FactoryExt},
};
use glsl_include::Context as IncludeContext;
use image::DynamicImage;
use std::{
    fs::File,
    io::{BufReader, Read},
};
use tracing::{error, warn};
use vek::*;

/// Represents the format of the pre-processed color target.
// TODO: `(gfx::format::R11_G11_B10, gfx::format::Float)` would be better in
// theory, but it doesn't seem to work
pub type TgtColorFmt = gfx::format::Rgba16F;
/// Represents the format of the pre-processed depth and stencil target.
pub type TgtDepthStencilFmt = gfx::format::Depth;

/// Represents the format of the window's color target.
pub type WinColorFmt = gfx::format::Srgba8;
/// Represents the format of the window's depth target.
pub type WinDepthFmt = gfx::format::Depth;

/// Represents the format of the pre-processed shadow depth target.
pub type ShadowDepthStencilFmt = gfx::format::Depth;

/// A handle to a pre-processed color target.
pub type TgtColorView = gfx::handle::RenderTargetView<gfx_backend::Resources, TgtColorFmt>;
/// A handle to a pre-processed depth target.
pub type TgtDepthStencilView =
    gfx::handle::DepthStencilView<gfx_backend::Resources, TgtDepthStencilFmt>;

/// A handle to a window color target.
pub type WinColorView = gfx::handle::RenderTargetView<gfx_backend::Resources, WinColorFmt>;
/// A handle to a window depth target.
pub type WinDepthView = gfx::handle::DepthStencilView<gfx_backend::Resources, WinDepthFmt>;

/// Represents the format of LOD shadows.
pub type LodTextureFmt = (gfx::format::R8_G8_B8_A8, gfx::format::Unorm);

/// Represents the format of LOD altitudes.
pub type LodAltFmt = (gfx::format::R16_G16, gfx::format::Unorm);

/// Represents the format of LOD map colors.
pub type LodColorFmt = (gfx::format::R8_G8_B8_A8, gfx::format::Srgb);

/// Represents the format of greedy meshed color-light textures.
pub type ColLightFmt = (gfx::format::R8_G8_B8_A8, gfx::format::Unorm);

/// A handle to a shadow depth target.
pub type ShadowDepthStencilView =
    gfx::handle::DepthStencilView<gfx_backend::Resources, ShadowDepthStencilFmt>;
/// A handle to a shadow depth target as a resource.
pub type ShadowResourceView = gfx::handle::ShaderResourceView<
    gfx_backend::Resources,
    <ShadowDepthStencilFmt as gfx::format::Formatted>::View,
>;

/// A handle to a render color target as a resource.
pub type TgtColorRes = gfx::handle::ShaderResourceView<
    gfx_backend::Resources,
    <TgtColorFmt as gfx::format::Formatted>::View,
>;

/// A handle to a render depth target as a resource.
pub type TgtDepthRes = gfx::handle::ShaderResourceView<
    gfx_backend::Resources,
    <TgtDepthStencilFmt as gfx::format::Formatted>::View,
>;

/// A handle to a greedy meshed color-light texture as a resource.
pub type ColLightRes = gfx::handle::ShaderResourceView<
    gfx_backend::Resources,
    <ColLightFmt as gfx::format::Formatted>::View,
>;
/// A type representing data that can be converted to an immutable texture map
/// of ColLight data (used for texture atlases created during greedy meshing).
pub type ColLightInfo = (
    Vec<<<ColLightFmt as gfx::format::Formatted>::Surface as gfx::format::SurfaceTyped>::DataType>,
    Vec2<u16>,
);

/// Load from a GLSL file.
pub struct Glsl;

impl Asset for Glsl {
    type Output = String;

    const ENDINGS: &'static [&'static str] = &["glsl"];

    fn parse(mut buf_reader: BufReader<File>, _specifier: &str) -> Result<String, assets::Error> {
        let mut string = String::new();
        buf_reader.read_to_string(&mut string)?;
        Ok(string)
    }
}

/// A type that holds shadow map data.  Since shadow mapping may not be
/// supported on all platforms, we try to keep it separate.
pub struct ShadowMapRenderer {
    // directed_encoder: gfx::Encoder<gfx_backend::Resources, gfx_backend::CommandBuffer>,
    // point_encoder: gfx::Encoder<gfx_backend::Resources, gfx_backend::CommandBuffer>,
    directed_depth_stencil_view: ShadowDepthStencilView,
    directed_res: ShadowResourceView,
    directed_sampler: Sampler<gfx_backend::Resources>,

    point_depth_stencil_view: ShadowDepthStencilView,
    point_res: ShadowResourceView,
    point_sampler: Sampler<gfx_backend::Resources>,

    point_pipeline: GfxPipeline<shadow::pipe::Init<'static>>,
    terrain_directed_pipeline: GfxPipeline<shadow::pipe::Init<'static>>,
    figure_directed_pipeline: GfxPipeline<shadow::figure_pipe::Init<'static>>,
}

/// A type that encapsulates rendering state. `Renderer` is central to Voxygen's
/// rendering subsystem and contains any state necessary to interact with the
/// GPU, along with pipeline state objects (PSOs) needed to renderer different
/// kinds of models to the screen.
pub struct Renderer {
    device: gfx_backend::Device,
    encoder: gfx::Encoder<gfx_backend::Resources, gfx_backend::CommandBuffer>,
    factory: gfx_backend::Factory,

    win_color_view: WinColorView,
    win_depth_view: WinDepthView,

    tgt_color_view: TgtColorView,
    tgt_depth_stencil_view: TgtDepthStencilView,
    tgt_color_view_pp: TgtColorView,

    tgt_color_res: TgtColorRes,
    tgt_depth_res: TgtDepthRes,
    tgt_color_res_pp: TgtColorRes,

    sampler: Sampler<gfx_backend::Resources>,

    shadow_map: Option<ShadowMapRenderer>,

    skybox_pipeline: GfxPipeline<skybox::pipe::Init<'static>>,
    figure_pipeline: GfxPipeline<figure::pipe::Init<'static>>,
    terrain_pipeline: GfxPipeline<terrain::pipe::Init<'static>>,
    fluid_pipeline: GfxPipeline<fluid::pipe::Init<'static>>,
    sprite_pipeline: GfxPipeline<sprite::pipe::Init<'static>>,
    particle_pipeline: GfxPipeline<particle::pipe::Init<'static>>,
    ui_pipeline: GfxPipeline<ui::pipe::Init<'static>>,
    lod_terrain_pipeline: GfxPipeline<lod_terrain::pipe::Init<'static>>,
    clouds_pipeline: GfxPipeline<clouds::pipe::Init<'static>>,
    postprocess_pipeline: GfxPipeline<postprocess::pipe::Init<'static>>,
    player_shadow_pipeline: GfxPipeline<figure::pipe::Init<'static>>,

    shader_reload_indicator: ReloadIndicator,

    noise_tex: Texture<(gfx::format::R8, gfx::format::Unorm)>,

    mode: RenderMode,
}

impl Renderer {
    /// Create a new `Renderer` from a variety of backend-specific components
    /// and the window targets.
    pub fn new(
        mut device: gfx_backend::Device,
        mut factory: gfx_backend::Factory,
        win_color_view: WinColorView,
        win_depth_view: WinDepthView,
        mode: RenderMode,
    ) -> Result<Self, RenderError> {
        // Enable seamless cubemaps globally, where available--they are essentially a
        // strict improvement on regular cube maps.
        //
        // Note that since we only have to enable this once globally, there is no point
        // in doing this on rerender.
        Self::enable_seamless_cube_maps(&mut device);

        let dims = win_color_view.get_dimensions();

        let mut shader_reload_indicator = ReloadIndicator::new();
        let shadow_views = Self::create_shadow_views(
            &mut factory,
            (dims.0, dims.1),
            &ShadowMapMode::try_from(mode.shadow).unwrap_or_default(),
        )
        .map_err(|err| {
            warn!("Could not create shadow map views: {:?}", err);
        })
        .ok();

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
            player_shadow_pipeline,
            point_shadow_pipeline,
            terrain_directed_shadow_pipeline,
            figure_directed_shadow_pipeline,
        ) = create_pipelines(
            &mut factory,
            &mode,
            shadow_views.is_some(),
            &mut shader_reload_indicator,
        )?;

        let (
            tgt_color_view,
            tgt_depth_stencil_view,
            tgt_color_view_pp,
            tgt_color_res,
            tgt_depth_res,
            tgt_color_res_pp,
        ) = Self::create_rt_views(&mut factory, (dims.0, dims.1), &mode)?;

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
            Some(ShadowMapRenderer {
                // point_encoder: factory.create_command_buffer().into(),
                // directed_encoder: factory.create_command_buffer().into(),
                point_depth_stencil_view,
                point_res,
                point_sampler,

                directed_depth_stencil_view,
                directed_res,
                directed_sampler,

                point_pipeline,
                terrain_directed_pipeline,
                figure_directed_pipeline,
            })
        } else {
            None
        };

        let sampler = factory.create_sampler(gfx::texture::SamplerInfo::new(
            gfx::texture::FilterMethod::Bilinear,
            gfx::texture::WrapMode::Clamp,
        ));

        let noise_tex = Texture::new(
            &mut factory,
            &DynamicImage::load_expect("voxygen.texture.noise"),
            Some(gfx::texture::FilterMethod::Trilinear),
            Some(gfx::texture::WrapMode::Tile),
            None,
        )?;

        Ok(Self {
            device,
            encoder: factory.create_command_buffer().into(),
            factory,

            win_color_view,
            win_depth_view,

            tgt_color_view,
            tgt_depth_stencil_view,
            tgt_color_view_pp,

            tgt_color_res,
            tgt_depth_res,
            tgt_color_res_pp,

            sampler,

            shadow_map,

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

            shader_reload_indicator,

            noise_tex,

            mode,
        })
    }

    /// Get references to the internal render target views that get rendered to
    /// before post-processing.
    #[allow(dead_code)]
    pub fn tgt_views(&self) -> (&TgtColorView, &TgtDepthStencilView) {
        (&self.tgt_color_view, &self.tgt_depth_stencil_view)
    }

    /// Get references to the internal render target views that get displayed
    /// directly by the window.
    #[allow(dead_code)]
    pub fn win_views(&self) -> (&WinColorView, &WinDepthView) {
        (&self.win_color_view, &self.win_depth_view)
    }

    /// Get mutable references to the internal render target views that get
    /// rendered to before post-processing.
    #[allow(dead_code)]
    pub fn tgt_views_mut(&mut self) -> (&mut TgtColorView, &mut TgtDepthStencilView) {
        (&mut self.tgt_color_view, &mut self.tgt_depth_stencil_view)
    }

    /// Get mutable references to the internal render target views that get
    /// displayed directly by the window.
    #[allow(dead_code)]
    pub fn win_views_mut(&mut self) -> (&mut WinColorView, &mut WinDepthView) {
        (&mut self.win_color_view, &mut self.win_depth_view)
    }

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
                tgt_color_view_pp,
                tgt_color_res,
                tgt_depth_res,
                tgt_color_res_pp,
            ) = Self::create_rt_views(&mut self.factory, (dims.0, dims.1), &self.mode)?;
            self.tgt_color_res = tgt_color_res;
            self.tgt_depth_res = tgt_depth_res;
            self.tgt_color_res_pp = tgt_color_res_pp;
            self.tgt_color_view = tgt_color_view;
            self.tgt_depth_stencil_view = tgt_depth_stencil_view;
            self.tgt_color_view_pp = tgt_color_view_pp;
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
        factory: &mut gfx_device_gl::Factory,
        size: (u16, u16),
        mode: &RenderMode,
    ) -> Result<
        (
            TgtColorView,
            TgtDepthStencilView,
            TgtColorView,
            TgtColorRes,
            TgtDepthRes,
            TgtColorRes,
        ),
        RenderError,
    > {
        let upscaled = Vec2::from(size)
            .map(|e: u16| (e as f32 * mode.upscale_mode.factor) as u16)
            .into_tuple();
        let kind = match mode.aa {
            AaMode::None | AaMode::Fxaa => {
                gfx::texture::Kind::D2(upscaled.0, upscaled.1, gfx::texture::AaMode::Single)
            },
            // TODO: Ensure sampling in the shader is exactly between the 4 texels
            AaMode::SsaaX4 => {
                // TODO: Figure out how to do upscaling correctly with SSAA
                gfx::texture::Kind::D2(size.0 * 2, size.1 * 2, gfx::texture::AaMode::Single)
            },
            AaMode::MsaaX4 => {
                gfx::texture::Kind::D2(upscaled.0, upscaled.1, gfx::texture::AaMode::Multi(4))
            },
            AaMode::MsaaX8 => {
                gfx::texture::Kind::D2(upscaled.0, upscaled.1, gfx::texture::AaMode::Multi(8))
            },
            AaMode::MsaaX16 => {
                gfx::texture::Kind::D2(upscaled.0, upscaled.1, gfx::texture::AaMode::Multi(16))
            },
        };
        let levels = 1;

        let color_cty = <<TgtColorFmt as gfx::format::Formatted>::Channel as gfx::format::ChannelTyped
                >::get_channel_type();
        let mut color_tex = || {
            factory.create_texture(
                kind,
                levels,
                gfx::memory::Bind::SHADER_RESOURCE | gfx::memory::Bind::RENDER_TARGET,
                gfx::memory::Usage::Data,
                Some(color_cty),
            )
        };
        let tgt_color_tex = color_tex()?;
        let tgt_color_tex_pp = color_tex()?;
        let mut color_res = |tex| {
            factory.view_texture_as_shader_resource::<TgtColorFmt>(
                tex,
                (0, levels - 1),
                gfx::format::Swizzle::new(),
            )
        };
        let tgt_color_res = color_res(&tgt_color_tex)?;
        let tgt_color_res_pp = color_res(&tgt_color_tex_pp)?;
        let tgt_color_view = factory.view_texture_as_render_target(&tgt_color_tex, 0, None)?;
        let tgt_color_view_pp =
            factory.view_texture_as_render_target(&tgt_color_tex_pp, 0, None)?;

        let depth_stencil_cty = <<TgtDepthStencilFmt as gfx::format::Formatted>::Channel as gfx::format::ChannelTyped>::get_channel_type();
        let tgt_depth_stencil_tex = factory.create_texture(
            kind,
            levels,
            gfx::memory::Bind::SHADER_RESOURCE | gfx::memory::Bind::DEPTH_STENCIL,
            gfx::memory::Usage::Data,
            Some(depth_stencil_cty),
        )?;
        let tgt_depth_res = factory.view_texture_as_shader_resource::<TgtDepthStencilFmt>(
            &tgt_depth_stencil_tex,
            (0, levels - 1),
            gfx::format::Swizzle::new(),
        )?;
        let tgt_depth_stencil_view =
            factory.view_texture_as_depth_stencil_trivial(&tgt_depth_stencil_tex)?;

        Ok((
            tgt_color_view,
            tgt_depth_stencil_view,
            tgt_color_view_pp,
            tgt_color_res,
            tgt_depth_res,
            tgt_color_res_pp,
        ))
    }

    /// Create textures and views for shadow maps.
    // This is a one-use type and the two halves are not guaranteed to remain identical, so we
    // disable the type complexity lint.
    #[allow(clippy::type_complexity)]
    fn create_shadow_views(
        factory: &mut gfx_device_gl::Factory,
        size: (u16, u16),
        mode: &ShadowMapMode,
    ) -> Result<
        (
            ShadowDepthStencilView,
            ShadowResourceView,
            Sampler<gfx_backend::Resources>,
            ShadowDepthStencilView,
            ShadowResourceView,
            Sampler<gfx_backend::Resources>,
        ),
        RenderError,
    > {
        // (Attempt to) apply resolution factor to shadow map resolution.
        let resolution_factor = mode.resolution.clamped(0.25, 4.0);

        let max_texture_size = Self::max_texture_size_raw(factory);
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
        let depth_stencil_cty = <<ShadowDepthStencilFmt as gfx::format::Formatted>::Channel as gfx::format::ChannelTyped>::get_channel_type();

        let point_shadow_tex = factory
            .create_texture(
                gfx::texture::Kind::Cube(diag_two_size / 4),
                levels as gfx::texture::Level,
                gfx::memory::Bind::SHADER_RESOURCE | gfx::memory::Bind::DEPTH_STENCIL,
                gfx::memory::Usage::Data,
                Some(depth_stencil_cty),
            )
            .map_err(|err| RenderError::CombinedError(gfx::CombinedError::Texture(err)))?;

        let point_tgt_shadow_view = factory
            .view_texture_as_depth_stencil::<ShadowDepthStencilFmt>(
                &point_shadow_tex,
                0,
                None,
                gfx::texture::DepthStencilFlags::empty(),
            )?;

        let point_tgt_shadow_res = factory
            .view_texture_as_shader_resource::<ShadowDepthStencilFmt>(
                &point_shadow_tex,
                (0, levels - 1),
                gfx::format::Swizzle::new(),
            )?;

        let directed_shadow_tex = factory
            .create_texture(
                gfx::texture::Kind::D2(diag_two_size, diag_two_size, gfx::texture::AaMode::Single),
                levels as gfx::texture::Level,
                gfx::memory::Bind::SHADER_RESOURCE | gfx::memory::Bind::DEPTH_STENCIL,
                gfx::memory::Usage::Data,
                Some(depth_stencil_cty),
            )
            .map_err(|err| RenderError::CombinedError(gfx::CombinedError::Texture(err)))?;
        let directed_tgt_shadow_view = factory
            .view_texture_as_depth_stencil::<ShadowDepthStencilFmt>(
                &directed_shadow_tex,
                0,
                None,
                gfx::texture::DepthStencilFlags::empty(),
            )?;
        let directed_tgt_shadow_res = factory
            .view_texture_as_shader_resource::<ShadowDepthStencilFmt>(
                &directed_shadow_tex,
                (0, levels - 1),
                gfx::format::Swizzle::new(),
            )?;

        let mut sampler_info = gfx::texture::SamplerInfo::new(
            gfx::texture::FilterMethod::Bilinear,
            // Lights should always be assumed to flood areas we can't see.
            gfx::texture::WrapMode::Border,
        );
        sampler_info.comparison = Some(Comparison::LessEqual);
        sampler_info.border = [1.0; 4].into();
        let point_shadow_tex_sampler = factory.create_sampler(sampler_info);
        let directed_shadow_tex_sampler = factory.create_sampler(sampler_info);

        Ok((
            point_tgt_shadow_view,
            point_tgt_shadow_res,
            point_shadow_tex_sampler,
            directed_tgt_shadow_view,
            directed_tgt_shadow_res,
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
    fn enable_seamless_cube_maps(device: &mut gfx_backend::Device) {
        unsafe {
            // NOTE: Currently just fail silently rather than complain if the computer is on
            // a version lower than 3.2, where seamless cubemaps were introduced.
            if !device.get_info().is_version_supported(3, 2) {
                return;
            }

            // NOTE: Safe because GL_TEXTURE_CUBE_MAP_SEAMLESS is supported by OpenGL 3.2+
            // (see https://www.khronos.org/opengl/wiki/Cubemap_Texture#Seamless_cubemap);
            // enabling seamless cube maps should always be safe regardless of the state of
            // the OpenGL context, so no further checks are needed.
            device.with_gl(|gl| {
                gl.Enable(gfx_gl::TEXTURE_CUBE_MAP_SEAMLESS);
            });
        }
    }

    /// NOTE: Supported by all but a handful of mobile GPUs
    /// (see https://github.com/gpuweb/gpuweb/issues/480)
    /// so wgpu should support it too.
    #[allow(unsafe_code)]
    fn set_depth_clamp(device: &mut gfx_backend::Device, depth_clamp: bool) {
        unsafe {
            // NOTE: Currently just fail silently rather than complain if the computer is on
            // a version lower than 3.3, though we probably will complain
            // elsewhere regardless, since shadow mapping is an optional feature
            // and having depth clamping disabled won't cause undefined
            // behavior, just incorrect shadowing from objects behind the viewer.
            if !device.get_info().is_version_supported(3, 3) {
                return;
            }

            // NOTE: Safe because glDepthClamp is (I believe) supported by
            // OpenGL 3.3, so we shouldn't have to check for other OpenGL versions which
            // may use different extensions.  Also, enabling depth clamping should
            // essentially always be safe regardless of the state of the OpenGL
            // context, so no further checks are needed.
            device.with_gl(|gl| {
                if depth_clamp {
                    gl.Enable(gfx_gl::DEPTH_CLAMP);
                } else {
                    gl.Disable(gfx_gl::DEPTH_CLAMP);
                }
            });
        }
    }

    /// Queue the clearing of the depth target ready for a new frame to be
    /// rendered.
    pub fn clear(&mut self) {
        span!(_guard, "clear", "Renderer::clear");
        self.encoder.clear_depth(&self.tgt_depth_stencil_view, 1.0);
        // self.encoder.clear_stencil(&self.tgt_depth_stencil_view, 0);
        self.encoder.clear_depth(&self.win_depth_view, 1.0);
    }

    /// Set up shadow rendering.
    pub fn start_shadows(&mut self) {
        if !self.mode.shadow.is_map() {
            return;
        }
        if let Some(_shadow_map) = self.shadow_map.as_mut() {
            self.encoder.flush(&mut self.device);
            Self::set_depth_clamp(&mut self.device, true);
        }
    }

    /// Perform all queued draw calls for global.shadows.
    pub fn flush_shadows(&mut self) {
        if !self.mode.shadow.is_map() {
            return;
        }
        if let Some(_shadow_map) = self.shadow_map.as_mut() {
            let point_encoder = &mut self.encoder;
            // let point_encoder = &mut shadow_map.point_encoder;
            point_encoder.flush(&mut self.device);
            // let directed_encoder = &mut shadow_map.directed_encoder;
            // directed_encoder.flush(&mut self.device);
            // Reset depth clamping.
            Self::set_depth_clamp(&mut self.device, false);
        }
    }

    /// Perform all queued draw calls for this frame and clean up discarded
    /// items.
    pub fn flush(&mut self) {
        span!(_guard, "flush", "Renderer::flush");
        self.encoder.flush(&mut self.device);
        self.device.cleanup();

        // If the shaders files were changed attempt to recreate the shaders
        if self.shader_reload_indicator.reloaded() {
            self.recreate_pipelines();
        }
    }

    /// Recreate the pipelines
    fn recreate_pipelines(&mut self) {
        match create_pipelines(
            &mut self.factory,
            &self.mode,
            self.shadow_map.is_some(),
            &mut self.shader_reload_indicator,
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
    pub fn create_consts<T: Copy + gfx::traits::Pod>(
        &mut self,
        vals: &[T],
    ) -> Result<Consts<T>, RenderError> {
        let mut consts = Consts::new(&mut self.factory, vals.len());
        consts.update(&mut self.encoder, vals, 0)?;
        Ok(consts)
    }

    /// Update a set of constants with the provided values.
    pub fn update_consts<T: Copy + gfx::traits::Pod>(
        &mut self,
        consts: &mut Consts<T>,
        vals: &[T],
    ) -> Result<(), RenderError> {
        consts.update(&mut self.encoder, vals, 0)
    }

    /// Create a new set of instances with the provided values.
    pub fn create_instances<T: Copy + gfx::traits::Pod>(
        &mut self,
        vals: &[T],
    ) -> Result<Instances<T>, RenderError> {
        let mut instances = Instances::new(&mut self.factory, vals.len())?;
        instances.update(&mut self.encoder, vals)?;
        Ok(instances)
    }

    /// Create a new model from the provided mesh.
    pub fn create_model<P: Pipeline>(&mut self, mesh: &Mesh<P>) -> Result<Model<P>, RenderError> {
        Ok(Model::new(&mut self.factory, mesh))
    }

    /// Create a new dynamic model with the specified size.
    pub fn create_dynamic_model<P: Pipeline>(
        &mut self,
        size: usize,
    ) -> Result<DynamicModel<P>, RenderError> {
        DynamicModel::new(&mut self.factory, size)
    }

    /// Update a dynamic model with a mesh and a offset.
    pub fn update_model<P: Pipeline>(
        &mut self,
        model: &DynamicModel<P>,
        mesh: &Mesh<P>,
        offset: usize,
    ) -> Result<(), RenderError> {
        model.update(&mut self.encoder, mesh, offset)
    }

    /// Return the maximum supported texture size.
    pub fn max_texture_size(&self) -> u16 { Self::max_texture_size_raw(&self.factory) }

    /// Return the maximum supported texture size from the factory.
    fn max_texture_size_raw(factory: &gfx_backend::Factory) -> u16 {
        /// NOTE: OpenGL requirement.
        const MAX_TEXTURE_SIZE_MIN: u16 = 1024;
        #[cfg(target_os = "macos")]
        /// NOTE: Because Macs lie about their max supported texture size.
        const MAX_TEXTURE_SIZE_MAX: u16 = 8192;
        #[cfg(not(target_os = "macos"))]
        /// NOTE: Apparently Macs aren't the only machines that lie.
        ///
        /// TODO: Find a way to let graphics cards that don't lie do better.
        const MAX_TEXTURE_SIZE_MAX: u16 = 8192;
        // NOTE: Many APIs for textures require coordinates to fit in u16, which is why
        // we perform this conversion.
        u16::try_from(factory.get_capabilities().max_texture_size)
            .unwrap_or(MAX_TEXTURE_SIZE_MIN)
            .min(MAX_TEXTURE_SIZE_MAX)
    }

    /// Create a new immutable texture from the provided image.
    pub fn create_texture_immutable_raw<F: gfx::format::Formatted>(
        &mut self,
        kind: gfx::texture::Kind,
        mipmap: gfx::texture::Mipmap,
        data: &[&[<F::Surface as gfx::format::SurfaceTyped>::DataType]],
        sampler_info: gfx::texture::SamplerInfo,
    ) -> Result<Texture<F>, RenderError>
    where
        F::Surface: gfx::format::TextureSurface,
        F::Channel: gfx::format::TextureChannel,
        <F::Surface as gfx::format::SurfaceTyped>::DataType: Copy,
    {
        Texture::new_immutable_raw(&mut self.factory, kind, mipmap, data, sampler_info)
    }

    /// Create a new raw texture.
    pub fn create_texture_raw<F: gfx::format::Formatted>(
        &mut self,
        kind: gfx::texture::Kind,
        max_levels: u8,
        bind: gfx::memory::Bind,
        usage: gfx::memory::Usage,
        levels: (u8, u8),
        swizzle: gfx::format::Swizzle,
        sampler_info: gfx::texture::SamplerInfo,
    ) -> Result<Texture<F>, RenderError>
    where
        F::Surface: gfx::format::TextureSurface,
        F::Channel: gfx::format::TextureChannel,
        <F::Surface as gfx::format::SurfaceTyped>::DataType: Copy,
    {
        Texture::new_raw(
            &mut self.device,
            &mut self.factory,
            kind,
            max_levels,
            bind,
            usage,
            levels,
            swizzle,
            sampler_info,
        )
    }

    /// Create a new texture from the provided image.
    pub fn create_texture<F: gfx::format::Formatted>(
        &mut self,
        image: &image::DynamicImage,
        filter_method: Option<FilterMethod>,
        wrap_mode: Option<WrapMode>,
        border: Option<gfx::texture::PackedColor>,
    ) -> Result<Texture<F>, RenderError>
    where
        F::Surface: gfx::format::TextureSurface,
        F::Channel: gfx::format::TextureChannel,
        <F::Surface as gfx::format::SurfaceTyped>::DataType: Copy,
    {
        Texture::new(&mut self.factory, image, filter_method, wrap_mode, border)
    }

    /// Create a new dynamic texture (gfx::memory::Usage::Dynamic) with the
    /// specified dimensions.
    pub fn create_dynamic_texture(&mut self, dims: Vec2<u16>) -> Result<Texture, RenderError> {
        Texture::new_dynamic(&mut self.factory, dims.x, dims.y)
    }

    /// Update a texture with the provided offset, size, and data.
    pub fn update_texture<T: gfx::format::Formatted>(
        &mut self,
        texture: &Texture<T>,
        offset: [u16; 2],
        size: [u16; 2],
        data: &[<<T as gfx::format::Formatted>::Surface as gfx::format::SurfaceTyped>::DataType],
    ) -> Result<(), RenderError>
    where
        <T as gfx::format::Formatted>::Surface: gfx::format::TextureSurface,
        <T as gfx::format::Formatted>::Channel: gfx::format::TextureChannel,
        <<T as gfx::format::Formatted>::Surface as gfx::format::SurfaceTyped>::DataType: Copy,
    {
        texture.update(&mut self.encoder, offset, size, data)
    }

    /// Creates a download buffer, downloads the win_color_view, and converts to
    /// a image::DynamicImage.
    #[allow(clippy::map_clone)] // TODO: Pending review in #587
    pub fn create_screenshot(&mut self) -> Result<image::DynamicImage, RenderError> {
        let (width, height) = self.get_resolution().into_tuple();
        use gfx::{
            format::{Formatted, SurfaceTyped},
            memory::Typed,
        };
        type WinSurfaceData = <<WinColorFmt as Formatted>::Surface as SurfaceTyped>::DataType;
        let download = self
            .factory
            .create_download_buffer::<WinSurfaceData>(width as usize * height as usize)?;
        self.encoder.copy_texture_to_buffer_raw(
            self.win_color_view.raw().get_texture(),
            None,
            gfx::texture::RawImageInfo {
                xoffset: 0,
                yoffset: 0,
                zoffset: 0,
                width,
                height,
                depth: 0,
                format: WinColorFmt::get_format(),
                mipmap: 0,
            },
            download.raw(),
            0,
        )?;
        self.flush();

        // Assumes that the format is Rgba8.
        let raw_data = self
            .factory
            .read_mapping(&download)?
            .chunks_exact(width as usize)
            .rev()
            .flatten()
            .flatten()
            .map(|&e| e)
            .collect::<Vec<_>>();
        Ok(image::DynamicImage::ImageRgba8(
            // Should not fail if the dimensions are correct.
            image::ImageBuffer::from_raw(width as u32, height as u32, raw_data).unwrap(),
        ))
    }

    /// Queue the rendering of the provided skybox model in the upcoming frame.
    pub fn render_skybox(
        &mut self,
        model: &Model<skybox::SkyboxPipeline>,
        global: &GlobalModel,
        locals: &Consts<skybox::Locals>,
        lod: &lod_terrain::LodData,
    ) {
        self.encoder.draw(
            &gfx::Slice {
                start: model.vertex_range().start,
                end: model.vertex_range().end,
                base_vertex: 0,
                instances: None,
                buffer: gfx::IndexBuffer::Auto,
            },
            &self.skybox_pipeline.pso,
            &skybox::pipe::Data {
                vbuf: model.vbuf.clone(),
                locals: locals.buf.clone(),
                globals: global.globals.buf.clone(),
                noise: (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
                alt: (lod.alt.srv.clone(), lod.alt.sampler.clone()),
                horizon: (lod.horizon.srv.clone(), lod.horizon.sampler.clone()),
                tgt_color: self.tgt_color_view.clone(),
                tgt_depth_stencil: (self.tgt_depth_stencil_view.clone()/* , (1, 1) */),
            },
        );
    }

    /// Queue the rendering of the provided figure model in the upcoming frame.
    pub fn render_figure(
        &mut self,
        model: &figure::FigureModel,
        col_lights: &Texture<ColLightFmt>,
        global: &GlobalModel,
        locals: &Consts<figure::Locals>,
        bones: &Consts<figure::BoneData>,
        lod: &lod_terrain::LodData,
    ) {
        let (point_shadow_maps, directed_shadow_maps) =
            if let Some(shadow_map) = &mut self.shadow_map {
                (
                    (
                        shadow_map.point_res.clone(),
                        shadow_map.point_sampler.clone(),
                    ),
                    (
                        shadow_map.directed_res.clone(),
                        shadow_map.directed_sampler.clone(),
                    ),
                )
            } else {
                (
                    (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
                    (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
                )
            };
        let model = &model.opaque;

        self.encoder.draw(
            &gfx::Slice {
                start: model.vertex_range().start,
                end: model.vertex_range().end,
                base_vertex: 0,
                instances: None,
                buffer: gfx::IndexBuffer::Auto,
            },
            &self.figure_pipeline.pso,
            &figure::pipe::Data {
                vbuf: model.vbuf.clone(),
                col_lights: (col_lights.srv.clone(), col_lights.sampler.clone()),
                locals: locals.buf.clone(),
                globals: global.globals.buf.clone(),
                bones: bones.buf.clone(),
                lights: global.lights.buf.clone(),
                shadows: global.shadows.buf.clone(),
                light_shadows: global.shadow_mats.buf.clone(),
                point_shadow_maps,
                directed_shadow_maps,
                noise: (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
                alt: (lod.alt.srv.clone(), lod.alt.sampler.clone()),
                horizon: (lod.horizon.srv.clone(), lod.horizon.sampler.clone()),
                tgt_color: self.tgt_color_view.clone(),
                tgt_depth_stencil: (self.tgt_depth_stencil_view.clone()/* , (1, 1) */),
            },
        );
    }

    /// Queue the rendering of the player silhouette in the upcoming frame.
    pub fn render_player_shadow(
        &mut self,
        _model: &figure::FigureModel,
        _col_lights: &Texture<ColLightFmt>,
        _global: &GlobalModel,
        _bones: &Consts<figure::BoneData>,
        _lod: &lod_terrain::LodData,
        _locals: &Consts<shadow::Locals>,
    ) {
        // FIXME: Consider reenabling at some point.
        /* let (point_shadow_maps, directed_shadow_maps) =
            if let Some(shadow_map) = &mut self.shadow_map {
                (
                    (
                        shadow_map.point_res.clone(),
                        shadow_map.point_sampler.clone(),
                    ),
                    (
                        shadow_map.directed_res.clone(),
                        shadow_map.directed_sampler.clone(),
                    ),
                )
            } else {
                (
                    (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
                    (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
                )
            };
        let model = &model.opaque;

        self.encoder.draw(
            &gfx::Slice {
                start: model.vertex_range().start,
                end: model.vertex_range().end,
                base_vertex: 0,
                instances: None,
                buffer: gfx::IndexBuffer::Auto,
            },
            &self.player_shadow_pipeline.pso,
            &figure::pipe::Data {
                vbuf: model.vbuf.clone(),
                col_lights: (col_lights.srv.clone(), col_lights.sampler.clone()),
                locals: locals.buf.clone(),
                globals: global.globals.buf.clone(),
                bones: bones.buf.clone(),
                lights: global.lights.buf.clone(),
                shadows: global.shadows.buf.clone(),
                light_shadows: global.shadow_mats.buf.clone(),
                point_shadow_maps,
                directed_shadow_maps,
                noise: (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
                alt: (lod.alt.srv.clone(), lod.alt.sampler.clone()),
                horizon: (lod.horizon.srv.clone(), lod.horizon.sampler.clone()),
                tgt_color: self.tgt_color_view.clone(),
                tgt_depth_stencil: (self.tgt_depth_stencil_view.clone()/* , (0, 0) */),
            },
        ); */
    }

    /// Queue the rendering of the player model in the upcoming frame.
    pub fn render_player(
        &mut self,
        model: &figure::FigureModel,
        col_lights: &Texture<ColLightFmt>,
        global: &GlobalModel,
        locals: &Consts<figure::Locals>,
        bones: &Consts<figure::BoneData>,
        lod: &lod_terrain::LodData,
    ) {
        let (point_shadow_maps, directed_shadow_maps) =
            if let Some(shadow_map) = &mut self.shadow_map {
                (
                    (
                        shadow_map.point_res.clone(),
                        shadow_map.point_sampler.clone(),
                    ),
                    (
                        shadow_map.directed_res.clone(),
                        shadow_map.directed_sampler.clone(),
                    ),
                )
            } else {
                (
                    (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
                    (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
                )
            };
        let model = &model.opaque;

        self.encoder.draw(
            &gfx::Slice {
                start: model.vertex_range().start,
                end: model.vertex_range().end,
                base_vertex: 0,
                instances: None,
                buffer: gfx::IndexBuffer::Auto,
            },
            &self.figure_pipeline.pso,
            &figure::pipe::Data {
                vbuf: model.vbuf.clone(),
                col_lights: (col_lights.srv.clone(), col_lights.sampler.clone()),
                locals: locals.buf.clone(),
                globals: global.globals.buf.clone(),
                bones: bones.buf.clone(),
                lights: global.lights.buf.clone(),
                shadows: global.shadows.buf.clone(),
                light_shadows: global.shadow_mats.buf.clone(),
                point_shadow_maps,
                directed_shadow_maps,
                noise: (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
                alt: (lod.alt.srv.clone(), lod.alt.sampler.clone()),
                horizon: (lod.horizon.srv.clone(), lod.horizon.sampler.clone()),
                tgt_color: self.tgt_color_view.clone(),
                tgt_depth_stencil: (self.tgt_depth_stencil_view.clone()/* , (1, 1) */),
            },
        );
    }

    /// Queue the rendering of the provided terrain chunk model in the upcoming
    /// frame.
    pub fn render_terrain_chunk(
        &mut self,
        model: &Model<terrain::TerrainPipeline>,
        col_lights: &Texture<ColLightFmt>,
        global: &GlobalModel,
        locals: &Consts<terrain::Locals>,
        lod: &lod_terrain::LodData,
    ) {
        let (point_shadow_maps, directed_shadow_maps) =
            if let Some(shadow_map) = &mut self.shadow_map {
                (
                    (
                        shadow_map.point_res.clone(),
                        shadow_map.point_sampler.clone(),
                    ),
                    (
                        shadow_map.directed_res.clone(),
                        shadow_map.directed_sampler.clone(),
                    ),
                )
            } else {
                (
                    (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
                    (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
                )
            };

        self.encoder.draw(
            &gfx::Slice {
                start: model.vertex_range().start,
                end: model.vertex_range().end,
                base_vertex: 0,
                instances: None,
                buffer: gfx::IndexBuffer::Auto,
            },
            &self.terrain_pipeline.pso,
            &terrain::pipe::Data {
                vbuf: model.vbuf.clone(),
                // TODO: Consider splitting out texture atlas data into a separate vertex buffer,
                // since we don't need it for things like global.shadows.
                col_lights: (col_lights.srv.clone(), col_lights.sampler.clone()),
                locals: locals.buf.clone(),
                globals: global.globals.buf.clone(),
                lights: global.lights.buf.clone(),
                shadows: global.shadows.buf.clone(),
                light_shadows: global.shadow_mats.buf.clone(),
                point_shadow_maps,
                directed_shadow_maps,
                noise: (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
                alt: (lod.alt.srv.clone(), lod.alt.sampler.clone()),
                horizon: (lod.horizon.srv.clone(), lod.horizon.sampler.clone()),
                tgt_color: self.tgt_color_view.clone(),
                tgt_depth_stencil: (self.tgt_depth_stencil_view.clone()/* , (1, 1) */),
            },
        );
    }

    /// Queue the rendering of a shadow map from a point light in the upcoming
    /// frame.
    pub fn render_shadow_point(
        &mut self,
        model: &Model<terrain::TerrainPipeline>,
        global: &GlobalModel,
        terrain_locals: &Consts<terrain::Locals>,
        locals: &Consts<shadow::Locals>,
    ) {
        if !self.mode.shadow.is_map() {
            return;
        }
        // NOTE: Don't render shadows if the shader is not supported.
        let shadow_map = if let Some(shadow_map) = &mut self.shadow_map {
            shadow_map
        } else {
            return;
        };

        // let point_encoder = &mut shadow_map.point_encoder;
        let point_encoder = &mut self.encoder;
        point_encoder.draw(
            &gfx::Slice {
                start: model.vertex_range().start,
                end: model.vertex_range().end,
                base_vertex: 0,
                instances: None,
                buffer: gfx::IndexBuffer::Auto,
            },
            &shadow_map.point_pipeline.pso,
            &shadow::pipe::Data {
                // Terrain vertex stuff
                vbuf: model.vbuf.clone(),
                locals: terrain_locals.buf.clone(),
                globals: global.globals.buf.clone(),

                // Shadow stuff
                light_shadows: locals.buf.clone(),
                tgt_depth_stencil: shadow_map.point_depth_stencil_view.clone(),
            },
        );
    }

    /// Queue the rendering of terrain shadow map from all directional lights in
    /// the upcoming frame.
    pub fn render_terrain_shadow_directed(
        &mut self,
        model: &Model<terrain::TerrainPipeline>,
        global: &GlobalModel,
        terrain_locals: &Consts<terrain::Locals>,
        locals: &Consts<shadow::Locals>,
    ) {
        if !self.mode.shadow.is_map() {
            return;
        }
        // NOTE: Don't render shadows if the shader is not supported.
        let shadow_map = if let Some(shadow_map) = &mut self.shadow_map {
            shadow_map
        } else {
            return;
        };

        // let directed_encoder = &mut shadow_map.directed_encoder;
        let directed_encoder = &mut self.encoder;
        directed_encoder.draw(
            &gfx::Slice {
                start: model.vertex_range().start,
                end: model.vertex_range().end,
                base_vertex: 0,
                instances: None,
                buffer: gfx::IndexBuffer::Auto,
            },
            &shadow_map.terrain_directed_pipeline.pso,
            &shadow::pipe::Data {
                // Terrain vertex stuff
                vbuf: model.vbuf.clone(),
                locals: terrain_locals.buf.clone(),
                globals: global.globals.buf.clone(),

                // Shadow stuff
                light_shadows: locals.buf.clone(),
                tgt_depth_stencil: shadow_map.directed_depth_stencil_view.clone(),
            },
        );
    }

    /// Queue the rendering of figure shadow map from all directional lights in
    /// the upcoming frame.
    pub fn render_figure_shadow_directed(
        &mut self,
        model: &figure::FigureModel,
        global: &GlobalModel,
        figure_locals: &Consts<figure::Locals>,
        bones: &Consts<figure::BoneData>,
        locals: &Consts<shadow::Locals>,
    ) {
        if !self.mode.shadow.is_map() {
            return;
        }
        // NOTE: Don't render shadows if the shader is not supported.
        let shadow_map = if let Some(shadow_map) = &mut self.shadow_map {
            shadow_map
        } else {
            return;
        };
        let model = &model.opaque;

        // let directed_encoder = &mut shadow_map.directed_encoder;
        let directed_encoder = &mut self.encoder;
        directed_encoder.draw(
            &gfx::Slice {
                start: model.vertex_range().start,
                end: model.vertex_range().end,
                base_vertex: 0,
                instances: None,
                buffer: gfx::IndexBuffer::Auto,
            },
            &shadow_map.figure_directed_pipeline.pso,
            &shadow::figure_pipe::Data {
                // Terrain vertex stuff
                vbuf: model.vbuf.clone(),
                locals: figure_locals.buf.clone(),
                bones: bones.buf.clone(),
                globals: global.globals.buf.clone(),

                // Shadow stuff
                light_shadows: locals.buf.clone(),
                tgt_depth_stencil: shadow_map.directed_depth_stencil_view.clone(),
            },
        );
    }

    /// Queue the rendering of the provided terrain chunk model in the upcoming
    /// frame.
    pub fn render_fluid_chunk(
        &mut self,
        model: &Model<fluid::FluidPipeline>,
        global: &GlobalModel,
        locals: &Consts<terrain::Locals>,
        lod: &lod_terrain::LodData,
        waves: &Texture,
    ) {
        let (point_shadow_maps, directed_shadow_maps) =
            if let Some(shadow_map) = &mut self.shadow_map {
                (
                    (
                        shadow_map.point_res.clone(),
                        shadow_map.point_sampler.clone(),
                    ),
                    (
                        shadow_map.directed_res.clone(),
                        shadow_map.directed_sampler.clone(),
                    ),
                )
            } else {
                (
                    (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
                    (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
                )
            };

        self.encoder.draw(
            &gfx::Slice {
                start: model.vertex_range().start,
                end: model.vertex_range().end,
                base_vertex: 0,
                instances: None,
                buffer: gfx::IndexBuffer::Auto,
            },
            &self.fluid_pipeline.pso,
            &fluid::pipe::Data {
                vbuf: model.vbuf.clone(),
                locals: locals.buf.clone(),
                globals: global.globals.buf.clone(),
                lights: global.lights.buf.clone(),
                shadows: global.shadows.buf.clone(),
                light_shadows: global.shadow_mats.buf.clone(),
                point_shadow_maps,
                directed_shadow_maps,
                alt: (lod.alt.srv.clone(), lod.alt.sampler.clone()),
                horizon: (lod.horizon.srv.clone(), lod.horizon.sampler.clone()),
                noise: (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
                waves: (waves.srv.clone(), waves.sampler.clone()),
                tgt_color: self.tgt_color_view.clone(),
                tgt_depth_stencil: (self.tgt_depth_stencil_view.clone()/* , (1, 1) */),
            },
        );
    }

    /// Queue the rendering of the provided terrain chunk model in the upcoming
    /// frame.
    pub fn render_sprites(
        &mut self,
        model: &Model<sprite::SpritePipeline>,
        col_lights: &Texture<ColLightFmt>,
        global: &GlobalModel,
        terrain_locals: &Consts<terrain::Locals>,
        locals: &Consts<sprite::Locals>,
        instances: &Instances<sprite::Instance>,
        lod: &lod_terrain::LodData,
    ) {
        let (point_shadow_maps, directed_shadow_maps) =
            if let Some(shadow_map) = &mut self.shadow_map {
                (
                    (
                        shadow_map.point_res.clone(),
                        shadow_map.point_sampler.clone(),
                    ),
                    (
                        shadow_map.directed_res.clone(),
                        shadow_map.directed_sampler.clone(),
                    ),
                )
            } else {
                (
                    (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
                    (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
                )
            };

        self.encoder.draw(
            &gfx::Slice {
                start: model.vertex_range().start,
                end: model.vertex_range().end,
                base_vertex: 0,
                instances: Some((instances.count() as u32, 0)),
                buffer: gfx::IndexBuffer::Auto,
            },
            &self.sprite_pipeline.pso,
            &sprite::pipe::Data {
                vbuf: model.vbuf.clone(),
                ibuf: instances.ibuf.clone(),
                col_lights: (col_lights.srv.clone(), col_lights.sampler.clone()),
                terrain_locals: terrain_locals.buf.clone(),
                // NOTE: It would be nice if this wasn't needed and we could use a constant buffer
                // offset into the sprite data.  Hopefully, when we switch to wgpu we can do this,
                // as it offers the exact API we want (the equivalent can be done in OpenGL using
                // glBindBufferOffset).
                locals: locals.buf.clone(),
                globals: global.globals.buf.clone(),
                lights: global.lights.buf.clone(),
                shadows: global.shadows.buf.clone(),
                light_shadows: global.shadow_mats.buf.clone(),
                point_shadow_maps,
                directed_shadow_maps,
                noise: (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
                alt: (lod.alt.srv.clone(), lod.alt.sampler.clone()),
                horizon: (lod.horizon.srv.clone(), lod.horizon.sampler.clone()),
                tgt_color: self.tgt_color_view.clone(),
                tgt_depth_stencil: (self.tgt_depth_stencil_view.clone()/* , (1, 1) */),
            },
        );
    }

    /// Queue the rendering of the provided LoD terrain model in the upcoming
    /// frame.
    pub fn render_lod_terrain(
        &mut self,
        model: &Model<lod_terrain::LodTerrainPipeline>,
        global: &GlobalModel,
        locals: &Consts<lod_terrain::Locals>,
        lod: &lod_terrain::LodData,
    ) {
        self.encoder.draw(
            &gfx::Slice {
                start: model.vertex_range().start,
                end: model.vertex_range().end,
                base_vertex: 0,
                instances: None,
                buffer: gfx::IndexBuffer::Auto,
            },
            &self.lod_terrain_pipeline.pso,
            &lod_terrain::pipe::Data {
                vbuf: model.vbuf.clone(),
                locals: locals.buf.clone(),
                globals: global.globals.buf.clone(),
                noise: (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
                map: (lod.map.srv.clone(), lod.map.sampler.clone()),
                alt: (lod.alt.srv.clone(), lod.alt.sampler.clone()),
                horizon: (lod.horizon.srv.clone(), lod.horizon.sampler.clone()),
                tgt_color: self.tgt_color_view.clone(),
                tgt_depth_stencil: (self.tgt_depth_stencil_view.clone()/* , (1, 1) */),
            },
        );
    }

    /// Queue the rendering of the provided particle in the upcoming frame.
    pub fn render_particles(
        &mut self,
        model: &Model<particle::ParticlePipeline>,
        global: &GlobalModel,
        instances: &Instances<particle::Instance>,
        lod: &lod_terrain::LodData,
    ) {
        let (point_shadow_maps, directed_shadow_maps) =
            if let Some(shadow_map) = &mut self.shadow_map {
                (
                    (
                        shadow_map.point_res.clone(),
                        shadow_map.point_sampler.clone(),
                    ),
                    (
                        shadow_map.directed_res.clone(),
                        shadow_map.directed_sampler.clone(),
                    ),
                )
            } else {
                (
                    (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
                    (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
                )
            };

        self.encoder.draw(
            &gfx::Slice {
                start: model.vertex_range().start,
                end: model.vertex_range().end,
                base_vertex: 0,
                instances: Some((instances.count() as u32, 0)),
                buffer: gfx::IndexBuffer::Auto,
            },
            &self.particle_pipeline.pso,
            &particle::pipe::Data {
                vbuf: model.vbuf.clone(),
                ibuf: instances.ibuf.clone(),
                globals: global.globals.buf.clone(),
                lights: global.lights.buf.clone(),
                shadows: global.shadows.buf.clone(),
                light_shadows: global.shadow_mats.buf.clone(),
                point_shadow_maps,
                directed_shadow_maps,
                noise: (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
                alt: (lod.alt.srv.clone(), lod.alt.sampler.clone()),
                horizon: (lod.horizon.srv.clone(), lod.horizon.sampler.clone()),
                tgt_color: self.tgt_color_view.clone(),
                tgt_depth_stencil: (self.tgt_depth_stencil_view.clone()/* , (1, 1) */),
            },
        );
    }

    /// Queue the rendering of the provided UI element in the upcoming frame.
    pub fn render_ui_element<F: gfx::format::Formatted<View = [f32; 4]>>(
        &mut self,
        model: Model<ui::UiPipeline>,
        tex: &Texture<F>,
        scissor: Aabr<u16>,
        globals: &Consts<Globals>,
        locals: &Consts<ui::Locals>,
    ) where
        F::Surface: gfx::format::TextureSurface,
        F::Channel: gfx::format::TextureChannel,
        <F::Surface as gfx::format::SurfaceTyped>::DataType: Copy,
    {
        let Aabr { min, max } = scissor;
        self.encoder.draw(
            &gfx::Slice {
                start: model.vertex_range.start,
                end: model.vertex_range.end,
                base_vertex: 0,
                instances: None,
                buffer: gfx::IndexBuffer::Auto,
            },
            &self.ui_pipeline.pso,
            &ui::pipe::Data {
                vbuf: model.vbuf,
                scissor: gfx::Rect {
                    x: min.x,
                    y: min.y,
                    w: max.x - min.x,
                    h: max.y - min.y,
                },
                tex: (tex.srv.clone(), tex.sampler.clone()),
                locals: locals.buf.clone(),
                globals: globals.buf.clone(),
                tgt_color: self.win_color_view.clone(),
                tgt_depth: self.win_depth_view.clone(),
            },
        );
    }

    pub fn render_clouds(
        &mut self,
        model: &Model<clouds::CloudsPipeline>,
        globals: &Consts<Globals>,
        locals: &Consts<clouds::Locals>,
        lod: &lod_terrain::LodData,
    ) {
        self.encoder.draw(
            &gfx::Slice {
                start: model.vertex_range().start,
                end: model.vertex_range().end,
                base_vertex: 0,
                instances: None,
                buffer: gfx::IndexBuffer::Auto,
            },
            &self.clouds_pipeline.pso,
            &clouds::pipe::Data {
                vbuf: model.vbuf.clone(),
                locals: locals.buf.clone(),
                globals: globals.buf.clone(),
                map: (lod.map.srv.clone(), lod.map.sampler.clone()),
                alt: (lod.alt.srv.clone(), lod.alt.sampler.clone()),
                horizon: (lod.horizon.srv.clone(), lod.horizon.sampler.clone()),
                color_sampler: (self.tgt_color_res.clone(), self.sampler.clone()),
                depth_sampler: (self.tgt_depth_res.clone(), self.sampler.clone()),
                noise: (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
                tgt_color: self.tgt_color_view_pp.clone(),
            },
        )
    }

    pub fn render_post_process(
        &mut self,
        model: &Model<postprocess::PostProcessPipeline>,
        globals: &Consts<Globals>,
        locals: &Consts<postprocess::Locals>,
        lod: &lod_terrain::LodData,
    ) {
        self.encoder.draw(
            &gfx::Slice {
                start: model.vertex_range().start,
                end: model.vertex_range().end,
                base_vertex: 0,
                instances: None,
                buffer: gfx::IndexBuffer::Auto,
            },
            &self.postprocess_pipeline.pso,
            &postprocess::pipe::Data {
                vbuf: model.vbuf.clone(),
                locals: locals.buf.clone(),
                globals: globals.buf.clone(),
                map: (lod.map.srv.clone(), lod.map.sampler.clone()),
                alt: (lod.alt.srv.clone(), lod.alt.sampler.clone()),
                horizon: (lod.horizon.srv.clone(), lod.horizon.sampler.clone()),
                color_sampler: (self.tgt_color_res_pp.clone(), self.sampler.clone()),
                depth_sampler: (self.tgt_depth_res.clone(), self.sampler.clone()),
                noise: (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
                tgt_color: self.win_color_view.clone(),
            },
        )
    }
}

struct GfxPipeline<P: gfx::pso::PipelineInit> {
    pso: gfx::pso::PipelineState<gfx_backend::Resources, P::Meta>,
}

/// Creates all the pipelines used to render.
#[allow(clippy::type_complexity)] // TODO: Pending review in #587
fn create_pipelines(
    factory: &mut gfx_backend::Factory,
    mode: &RenderMode,
    has_shadow_views: bool,
    shader_reload_indicator: &mut ReloadIndicator,
) -> Result<
    (
        GfxPipeline<skybox::pipe::Init<'static>>,
        GfxPipeline<figure::pipe::Init<'static>>,
        GfxPipeline<terrain::pipe::Init<'static>>,
        GfxPipeline<fluid::pipe::Init<'static>>,
        GfxPipeline<sprite::pipe::Init<'static>>,
        GfxPipeline<particle::pipe::Init<'static>>,
        GfxPipeline<ui::pipe::Init<'static>>,
        GfxPipeline<lod_terrain::pipe::Init<'static>>,
        GfxPipeline<clouds::pipe::Init<'static>>,
        GfxPipeline<postprocess::pipe::Init<'static>>,
        GfxPipeline<figure::pipe::Init<'static>>,
        Option<GfxPipeline<shadow::pipe::Init<'static>>>,
        Option<GfxPipeline<shadow::pipe::Init<'static>>>,
        Option<GfxPipeline<shadow::figure_pipe::Init<'static>>>,
    ),
    RenderError,
> {
    let constants =
        Glsl::load_watched("voxygen.shaders.include.constants", shader_reload_indicator).unwrap();
    let globals =
        Glsl::load_watched("voxygen.shaders.include.globals", shader_reload_indicator).unwrap();
    let sky = Glsl::load_watched("voxygen.shaders.include.sky", shader_reload_indicator).unwrap();
    let light =
        Glsl::load_watched("voxygen.shaders.include.light", shader_reload_indicator).unwrap();
    let srgb = Glsl::load_watched("voxygen.shaders.include.srgb", shader_reload_indicator).unwrap();
    let random =
        Glsl::load_watched("voxygen.shaders.include.random", shader_reload_indicator).unwrap();
    let lod = Glsl::load_watched("voxygen.shaders.include.lod", shader_reload_indicator).unwrap();
    let shadows =
        Glsl::load_watched("voxygen.shaders.include.shadows", shader_reload_indicator).unwrap();

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

    let anti_alias = Glsl::load_watched(
        &["voxygen.shaders.antialias.", match mode.aa {
            AaMode::None | AaMode::SsaaX4 => "none",
            AaMode::Fxaa => "fxaa",
            AaMode::MsaaX4 => "msaa-x4",
            AaMode::MsaaX8 => "msaa-x8",
            AaMode::MsaaX16 => "msaa-x16",
        }]
        .concat(),
        shader_reload_indicator,
    )
    .unwrap();

    let cloud = Glsl::load_watched(
        &["voxygen.shaders.include.cloud.", match mode.cloud {
            CloudMode::None => "none",
            _ => "regular",
        }]
        .concat(),
        shader_reload_indicator,
    )
    .unwrap();

    let mut include_ctx = IncludeContext::new();
    include_ctx.include("constants.glsl", &constants);
    include_ctx.include("globals.glsl", &globals);
    include_ctx.include("shadows.glsl", &shadows);
    include_ctx.include("sky.glsl", &sky);
    include_ctx.include("light.glsl", &light);
    include_ctx.include("srgb.glsl", &srgb);
    include_ctx.include("random.glsl", &random);
    include_ctx.include("lod.glsl", &lod);
    include_ctx.include("anti-aliasing.glsl", &anti_alias);
    include_ctx.include("cloud.glsl", &cloud);

    let figure_vert =
        Glsl::load_watched("voxygen.shaders.figure-vert", shader_reload_indicator).unwrap();

    let terrain_point_shadow_vert = Glsl::load_watched(
        "voxygen.shaders.light-shadows-vert",
        shader_reload_indicator,
    )
    .unwrap();

    let terrain_directed_shadow_vert = Glsl::load_watched(
        "voxygen.shaders.light-shadows-directed-vert",
        shader_reload_indicator,
    )
    .unwrap();

    let figure_directed_shadow_vert = Glsl::load_watched(
        "voxygen.shaders.light-shadows-figure-vert",
        shader_reload_indicator,
    )
    .unwrap();

    let directed_shadow_frag = Glsl::load_watched(
        "voxygen.shaders.light-shadows-directed-frag",
        shader_reload_indicator,
    )
    .unwrap();

    // Construct a pipeline for rendering skyboxes
    let skybox_pipeline = create_pipeline(
        factory,
        skybox::pipe::new(),
        &Glsl::load_watched("voxygen.shaders.skybox-vert", shader_reload_indicator).unwrap(),
        &Glsl::load_watched("voxygen.shaders.skybox-frag", shader_reload_indicator).unwrap(),
        &include_ctx,
        gfx::state::CullFace::Back,
    )?;

    // Construct a pipeline for rendering figures
    let figure_pipeline = create_pipeline(
        factory,
        figure::pipe::new(),
        &figure_vert,
        &Glsl::load_watched("voxygen.shaders.figure-frag", shader_reload_indicator).unwrap(),
        &include_ctx,
        gfx::state::CullFace::Back,
    )?;

    // Construct a pipeline for rendering terrain
    let terrain_pipeline = create_pipeline(
        factory,
        terrain::pipe::new(),
        &Glsl::load_watched("voxygen.shaders.terrain-vert", shader_reload_indicator).unwrap(),
        &Glsl::load_watched("voxygen.shaders.terrain-frag", shader_reload_indicator).unwrap(),
        &include_ctx,
        gfx::state::CullFace::Back,
    )?;

    // Construct a pipeline for rendering fluids
    let fluid_pipeline = create_pipeline(
        factory,
        fluid::pipe::new(),
        &Glsl::load_watched("voxygen.shaders.fluid-vert", shader_reload_indicator).unwrap(),
        &Glsl::load_watched(
            &["voxygen.shaders.fluid-frag.", match mode.fluid {
                FluidMode::Cheap => "cheap",
                FluidMode::Shiny => "shiny",
            }]
            .concat(),
            shader_reload_indicator,
        )
        .unwrap(),
        &include_ctx,
        gfx::state::CullFace::Nothing,
    )?;

    // Construct a pipeline for rendering sprites
    let sprite_pipeline = create_pipeline(
        factory,
        sprite::pipe::new(),
        &Glsl::load_watched("voxygen.shaders.sprite-vert", shader_reload_indicator).unwrap(),
        &Glsl::load_watched("voxygen.shaders.sprite-frag", shader_reload_indicator).unwrap(),
        &include_ctx,
        gfx::state::CullFace::Back,
    )?;

    // Construct a pipeline for rendering particles
    let particle_pipeline = create_pipeline(
        factory,
        particle::pipe::new(),
        &Glsl::load_watched("voxygen.shaders.particle-vert", shader_reload_indicator).unwrap(),
        &Glsl::load_watched("voxygen.shaders.particle-frag", shader_reload_indicator).unwrap(),
        &include_ctx,
        gfx::state::CullFace::Back,
    )?;

    // Construct a pipeline for rendering UI elements
    let ui_pipeline = create_pipeline(
        factory,
        ui::pipe::new(),
        &Glsl::load_watched("voxygen.shaders.ui-vert", shader_reload_indicator).unwrap(),
        &Glsl::load_watched("voxygen.shaders.ui-frag", shader_reload_indicator).unwrap(),
        &include_ctx,
        gfx::state::CullFace::Back,
    )?;

    // Construct a pipeline for rendering terrain
    let lod_terrain_pipeline = create_pipeline(
        factory,
        lod_terrain::pipe::new(),
        &Glsl::load_watched("voxygen.shaders.lod-terrain-vert", shader_reload_indicator).unwrap(),
        &Glsl::load_watched("voxygen.shaders.lod-terrain-frag", shader_reload_indicator).unwrap(),
        &include_ctx,
        gfx::state::CullFace::Back,
    )?;

    // Construct a pipeline for rendering our clouds (a kind of post-processing)
    let clouds_pipeline = create_pipeline(
        factory,
        clouds::pipe::new(),
        &Glsl::load_watched("voxygen.shaders.clouds-vert", shader_reload_indicator).unwrap(),
        &Glsl::load_watched("voxygen.shaders.clouds-frag", shader_reload_indicator).unwrap(),
        &include_ctx,
        gfx::state::CullFace::Back,
    )?;

    // Construct a pipeline for rendering our post-processing
    let postprocess_pipeline = create_pipeline(
        factory,
        postprocess::pipe::new(),
        &Glsl::load_watched("voxygen.shaders.postprocess-vert", shader_reload_indicator).unwrap(),
        &Glsl::load_watched("voxygen.shaders.postprocess-frag", shader_reload_indicator).unwrap(),
        &include_ctx,
        gfx::state::CullFace::Back,
    )?;

    // Construct a pipeline for rendering the player silhouette
    let player_shadow_pipeline = create_pipeline(
        factory,
        figure::pipe::Init {
            tgt_depth_stencil: (gfx::preset::depth::PASS_TEST/*,
            Stencil::new(
                Comparison::Equal,
                0xff,
                (StencilOp::Keep, StencilOp::Keep, StencilOp::Keep),
            ),*/),
            ..figure::pipe::new()
        },
        &figure_vert,
        &Glsl::load_watched(
            "voxygen.shaders.player-shadow-frag",
            shader_reload_indicator,
        )
        .unwrap(),
        &include_ctx,
        gfx::state::CullFace::Back,
    )?;

    // Construct a pipeline for rendering point light terrain shadow maps.
    let point_shadow_pipeline = match create_shadow_pipeline(
        factory,
        shadow::pipe::new(),
        &terrain_point_shadow_vert,
        Some(
            &Glsl::load_watched(
                "voxygen.shaders.light-shadows-geom",
                shader_reload_indicator,
            )
            .unwrap(),
        ),
        &Glsl::load_watched(
            "voxygen.shaders.light-shadows-frag",
            shader_reload_indicator,
        )
        .unwrap(),
        &include_ctx,
        gfx::state::CullFace::Back,
        None, // Some(gfx::state::Offset(2, 0))
    ) {
        Ok(pipe) => Some(pipe),
        Err(err) => {
            warn!("Could not load point shadow map pipeline: {:?}", err);
            None
        },
    };

    // Construct a pipeline for rendering directional light terrain shadow maps.
    let terrain_directed_shadow_pipeline = match create_shadow_pipeline(
        factory,
        shadow::pipe::new(),
        &terrain_directed_shadow_vert,
        None,
        &directed_shadow_frag,
        &include_ctx,
        gfx::state::CullFace::Back,
        None, // Some(gfx::state::Offset(2, 1))
    ) {
        Ok(pipe) => Some(pipe),
        Err(err) => {
            warn!(
                "Could not load directed terrain shadow map pipeline: {:?}",
                err
            );
            None
        },
    };

    // Construct a pipeline for rendering directional light figure shadow maps.
    let figure_directed_shadow_pipeline = match create_shadow_pipeline(
        factory,
        shadow::figure_pipe::new(),
        &figure_directed_shadow_vert,
        None,
        &directed_shadow_frag,
        &include_ctx,
        gfx::state::CullFace::Back,
        None, // Some(gfx::state::Offset(2, 1))
    ) {
        Ok(pipe) => Some(pipe),
        Err(err) => {
            warn!(
                "Could not load directed figure shadow map pipeline: {:?}",
                err
            );
            None
        },
    };

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
    ))
}

/// Create a new pipeline from the provided vertex shader and fragment shader.
fn create_pipeline<P: gfx::pso::PipelineInit>(
    factory: &mut gfx_backend::Factory,
    pipe: P,
    vs: &str,
    fs: &str,
    ctx: &IncludeContext,
    cull_face: gfx::state::CullFace,
) -> Result<GfxPipeline<P>, RenderError> {
    let vs = ctx.expand(vs)?;
    let fs = ctx.expand(fs)?;

    let program = factory.link_program(vs.as_bytes(), fs.as_bytes())?;

    let result = Ok(GfxPipeline {
        pso: factory.create_pipeline_from_program(
            &program,
            gfx::Primitive::TriangleList,
            gfx::state::Rasterizer {
                front_face: gfx::state::FrontFace::CounterClockwise,
                cull_face,
                method: gfx::state::RasterMethod::Fill,
                offset: None,
                samples: Some(gfx::state::MultiSample),
            },
            pipe,
        )?,
    });

    result
}

/// Create a new shadow map pipeline.
fn create_shadow_pipeline<P: gfx::pso::PipelineInit>(
    factory: &mut gfx_backend::Factory,
    pipe: P,
    vs: &str,
    gs: Option<&str>,
    fs: &str,
    ctx: &IncludeContext,
    cull_face: gfx::state::CullFace,
    offset: Option<gfx::state::Offset>,
) -> Result<GfxPipeline<P>, RenderError> {
    let vs = ctx.expand(vs)?;
    let gs = gs.map(|gs| ctx.expand(gs)).transpose()?;
    let fs = ctx.expand(fs)?;

    let shader_set = if let Some(gs) = gs {
        factory.create_shader_set_geometry(vs.as_bytes(), gs.as_bytes(), fs.as_bytes())?
    } else {
        factory.create_shader_set(vs.as_bytes(), fs.as_bytes())?
    };

    Ok(GfxPipeline {
        pso: factory.create_pipeline_state(
            &shader_set,
            gfx::Primitive::TriangleList,
            gfx::state::Rasterizer {
                front_face: gfx::state::FrontFace::CounterClockwise,
                // Second-depth shadow mapping: should help reduce z-fighting provided all objects
                // are "watertight" (every triangle edge is shared with at most one other
                // triangle); this *should* be true for Veloren.
                cull_face: match cull_face {
                    gfx::state::CullFace::Front => gfx::state::CullFace::Back,
                    gfx::state::CullFace::Back => gfx::state::CullFace::Front,
                    gfx::state::CullFace::Nothing => gfx::state::CullFace::Nothing,
                },
                method: gfx::state::RasterMethod::Fill,
                offset,
                samples: None,
            },
            pipe,
        )?,
    })
}
