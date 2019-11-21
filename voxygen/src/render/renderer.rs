use super::{
    consts::Consts,
    gfx_backend,
    instances::Instances,
    mesh::Mesh,
    model::{DynamicModel, Model},
    pipelines::{
        figure, fluid, lod_terrain, postprocess, skybox, sprite, terrain, ui, Globals, Light,
        Shadow,
    },
    texture::Texture,
    AaMode, CloudMode, FluidMode, Pipeline, RenderError,
};
use common::assets::{self, watch::ReloadIndicator};
use gfx::{
    self,
    handle::Sampler,
    traits::{Device, Factory, FactoryExt},
};
use glsl_include::Context as IncludeContext;
use log::error;
use vek::*;

/// Represents the format of the pre-processed color target.
pub type TgtColorFmt = gfx::format::Srgba8;
/// Represents the format of the pre-processed depth target.
pub type TgtDepthFmt = gfx::format::Depth;

/// Represents the format of the window's color target.
pub type WinColorFmt = gfx::format::Srgba8;
/// Represents the format of the window's depth target.
pub type WinDepthFmt = gfx::format::Depth;

/// A handle to a pre-processed color target.
pub type TgtColorView = gfx::handle::RenderTargetView<gfx_backend::Resources, TgtColorFmt>;
/// A handle to a pre-processed depth target.
pub type TgtDepthView = gfx::handle::DepthStencilView<gfx_backend::Resources, TgtDepthFmt>;

/// A handle to a window color target.
pub type WinColorView = gfx::handle::RenderTargetView<gfx_backend::Resources, WinColorFmt>;
/// A handle to a window depth target.
pub type WinDepthView = gfx::handle::DepthStencilView<gfx_backend::Resources, WinDepthFmt>;

/// A handle to a render color target as a resource.
pub type TgtColorRes = gfx::handle::ShaderResourceView<
    gfx_backend::Resources,
    <TgtColorFmt as gfx::format::Formatted>::View,
>;

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
    tgt_depth_view: TgtDepthView,

    tgt_color_res: TgtColorRes,

    sampler: Sampler<gfx_backend::Resources>,

    skybox_pipeline: GfxPipeline<skybox::pipe::Init<'static>>,
    figure_pipeline: GfxPipeline<figure::pipe::Init<'static>>,
    terrain_pipeline: GfxPipeline<terrain::pipe::Init<'static>>,
    fluid_pipeline: GfxPipeline<fluid::pipe::Init<'static>>,
    sprite_pipeline: GfxPipeline<sprite::pipe::Init<'static>>,
    ui_pipeline: GfxPipeline<ui::pipe::Init<'static>>,
    lod_terrain_pipeline: GfxPipeline<lod_terrain::pipe::Init<'static>>,
    postprocess_pipeline: GfxPipeline<postprocess::pipe::Init<'static>>,

    shader_reload_indicator: ReloadIndicator,

    noise_tex: Texture<(gfx::format::R8, gfx::format::Unorm)>,

    aa_mode: AaMode,
    cloud_mode: CloudMode,
    fluid_mode: FluidMode,
}

impl Renderer {
    /// Create a new `Renderer` from a variety of backend-specific components
    /// and the window targets.
    pub fn new(
        device: gfx_backend::Device,
        mut factory: gfx_backend::Factory,
        win_color_view: WinColorView,
        win_depth_view: WinDepthView,
        aa_mode: AaMode,
        cloud_mode: CloudMode,
        fluid_mode: FluidMode,
    ) -> Result<Self, RenderError> {
        let mut shader_reload_indicator = ReloadIndicator::new();

        let (
            skybox_pipeline,
            figure_pipeline,
            terrain_pipeline,
            fluid_pipeline,
            sprite_pipeline,
            ui_pipeline,
            lod_terrain_pipeline,
            postprocess_pipeline,
        ) = create_pipelines(
            &mut factory,
            aa_mode,
            cloud_mode,
            fluid_mode,
            &mut shader_reload_indicator,
        )?;

        let dims = win_color_view.get_dimensions();
        let (tgt_color_view, tgt_depth_view, tgt_color_res) =
            Self::create_rt_views(&mut factory, (dims.0, dims.1), aa_mode)?;

        let sampler = factory.create_sampler_linear();

        let noise_tex = Texture::new(
            &mut factory,
            &assets::load_expect("voxygen.texture.noise"),
            Some(gfx::texture::FilterMethod::Trilinear),
            Some(gfx::texture::WrapMode::Tile),
        )?;

        Ok(Self {
            device,
            encoder: factory.create_command_buffer().into(),
            factory,

            win_color_view,
            win_depth_view,

            tgt_color_view,
            tgt_depth_view,

            tgt_color_res,
            sampler,

            skybox_pipeline,
            figure_pipeline,
            terrain_pipeline,
            fluid_pipeline,
            sprite_pipeline,
            ui_pipeline,
            lod_terrain_pipeline,
            postprocess_pipeline,

            shader_reload_indicator,

            noise_tex,

            aa_mode,
            cloud_mode,
            fluid_mode,
        })
    }

    /// Get references to the internal render target views that get rendered to
    /// before post-processing.
    #[allow(dead_code)]
    pub fn tgt_views(&self) -> (&TgtColorView, &TgtDepthView) {
        (&self.tgt_color_view, &self.tgt_depth_view)
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
    pub fn tgt_views_mut(&mut self) -> (&mut TgtColorView, &mut TgtDepthView) {
        (&mut self.tgt_color_view, &mut self.tgt_depth_view)
    }

    /// Get mutable references to the internal render target views that get
    /// displayed directly by the window.
    #[allow(dead_code)]
    pub fn win_views_mut(&mut self) -> (&mut WinColorView, &mut WinDepthView) {
        (&mut self.win_color_view, &mut self.win_depth_view)
    }

    /// Change the anti-aliasing mode
    pub fn set_aa_mode(&mut self, aa_mode: AaMode) -> Result<(), RenderError> {
        self.aa_mode = aa_mode;

        // Recreate render target
        self.on_resize()?;

        // Recreate pipelines with the new AA mode
        self.recreate_pipelines();

        Ok(())
    }

    /// Change the cloud rendering mode
    pub fn set_cloud_mode(&mut self, cloud_mode: CloudMode) -> Result<(), RenderError> {
        self.cloud_mode = cloud_mode;

        // Recreate render target
        self.on_resize()?;

        // Recreate pipelines with the new cloud mode
        self.recreate_pipelines();

        Ok(())
    }

    /// Change the fluid rendering mode
    pub fn set_fluid_mode(&mut self, fluid_mode: FluidMode) -> Result<(), RenderError> {
        self.fluid_mode = fluid_mode;

        // Recreate render target
        self.on_resize()?;

        // Recreate pipelines with the new fluid mode
        self.recreate_pipelines();

        Ok(())
    }

    /// Resize internal render targets to match window render target dimensions.
    pub fn on_resize(&mut self) -> Result<(), RenderError> {
        let dims = self.win_color_view.get_dimensions();

        // Avoid panics when creating texture with w,h of 0,0.
        if dims.0 != 0 && dims.1 != 0 {
            let (tgt_color_view, tgt_depth_view, tgt_color_res) =
                Self::create_rt_views(&mut self.factory, (dims.0, dims.1), self.aa_mode)?;
            self.tgt_color_res = tgt_color_res;
            self.tgt_color_view = tgt_color_view;
            self.tgt_depth_view = tgt_depth_view;
        }

        Ok(())
    }

    fn create_rt_views(
        factory: &mut gfx_device_gl::Factory,
        size: (u16, u16),
        aa_mode: AaMode,
    ) -> Result<(TgtColorView, TgtDepthView, TgtColorRes), RenderError> {
        let kind = match aa_mode {
            AaMode::None | AaMode::Fxaa => {
                gfx::texture::Kind::D2(size.0, size.1, gfx::texture::AaMode::Single)
            },
            // TODO: Ensure sampling in the shader is exactly between the 4 texels
            AaMode::SsaaX4 => {
                gfx::texture::Kind::D2(size.0 * 2, size.1 * 2, gfx::texture::AaMode::Single)
            },
            AaMode::MsaaX4 => {
                gfx::texture::Kind::D2(size.0, size.1, gfx::texture::AaMode::Multi(4))
            },
            AaMode::MsaaX8 => {
                gfx::texture::Kind::D2(size.0, size.1, gfx::texture::AaMode::Multi(8))
            },
            AaMode::MsaaX16 => {
                gfx::texture::Kind::D2(size.0, size.1, gfx::texture::AaMode::Multi(16))
            },
        };
        let levels = 1;

        let color_cty = <<TgtColorFmt as gfx::format::Formatted>::Channel as gfx::format::ChannelTyped
                >::get_channel_type();
        let tgt_color_tex = factory.create_texture(
            kind,
            levels,
            gfx::memory::Bind::SHADER_RESOURCE | gfx::memory::Bind::RENDER_TARGET,
            gfx::memory::Usage::Data,
            Some(color_cty),
        )?;
        let tgt_color_res = factory.view_texture_as_shader_resource::<TgtColorFmt>(
            &tgt_color_tex,
            (0, levels - 1),
            gfx::format::Swizzle::new(),
        )?;
        let tgt_color_view = factory.view_texture_as_render_target(&tgt_color_tex, 0, None)?;

        let depth_cty = <<TgtDepthFmt as gfx::format::Formatted>::Channel as gfx::format::ChannelTyped>::get_channel_type();
        let tgt_depth_tex = factory.create_texture(
            kind,
            levels,
            gfx::memory::Bind::DEPTH_STENCIL,
            gfx::memory::Usage::Data,
            Some(depth_cty),
        )?;
        let tgt_depth_view = factory.view_texture_as_depth_stencil_trivial(&tgt_depth_tex)?;

        Ok((tgt_color_view, tgt_depth_view, tgt_color_res))
    }

    /// Get the resolution of the render target.
    pub fn get_resolution(&self) -> Vec2<u16> {
        Vec2::new(
            self.win_color_view.get_dimensions().0,
            self.win_color_view.get_dimensions().1,
        )
    }

    /// Queue the clearing of the depth target ready for a new frame to be
    /// rendered.
    pub fn clear(&mut self) {
        self.encoder.clear_depth(&self.tgt_depth_view, 1.0);
        self.encoder.clear_depth(&self.win_depth_view, 1.0);
    }

    /// Perform all queued draw calls for this frame and clean up discarded
    /// items.
    pub fn flush(&mut self) {
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
            self.aa_mode,
            self.cloud_mode,
            self.fluid_mode,
            &mut self.shader_reload_indicator,
        ) {
            Ok((
                skybox_pipeline,
                figure_pipeline,
                terrain_pipeline,
                fluid_pipeline,
                sprite_pipeline,
                ui_pipeline,
                lod_terrain_pipeline,
                postprocess_pipeline,
            )) => {
                self.skybox_pipeline = skybox_pipeline;
                self.figure_pipeline = figure_pipeline;
                self.terrain_pipeline = terrain_pipeline;
                self.fluid_pipeline = fluid_pipeline;
                self.sprite_pipeline = sprite_pipeline;
                self.ui_pipeline = ui_pipeline;
                self.lod_terrain_pipeline = lod_terrain_pipeline;
                self.postprocess_pipeline = postprocess_pipeline;
            },
            Err(e) => error!(
                "Could not recreate shaders from assets due to an error: {:#?}",
                e
            ),
        }
    }

    /// Create a new set of constants with the provided values.
    pub fn create_consts<T: Copy + gfx::traits::Pod>(
        &mut self,
        vals: &[T],
    ) -> Result<Consts<T>, RenderError> {
        let mut consts = Consts::new(&mut self.factory, vals.len());
        consts.update(&mut self.encoder, vals)?;
        Ok(consts)
    }

    /// Update a set of constants with the provided values.
    pub fn update_consts<T: Copy + gfx::traits::Pod>(
        &mut self,
        consts: &mut Consts<T>,
        vals: &[T],
    ) -> Result<(), RenderError> {
        consts.update(&mut self.encoder, vals)
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
    pub fn max_texture_size(&self) -> usize { self.factory.get_capabilities().max_texture_size }

    /// Create a new texture from the provided image.
    pub fn create_texture(
        &mut self,
        image: &image::DynamicImage,
        filter_method: Option<gfx::texture::FilterMethod>,
        wrap_mode: Option<gfx::texture::WrapMode>,
    ) -> Result<Texture, RenderError> {
        Texture::new(&mut self.factory, image, filter_method, wrap_mode)
    }

    /// Create a new dynamic texture (gfx::memory::Usage::Dynamic) with the
    /// specified dimensions.
    pub fn create_dynamic_texture(&mut self, dims: Vec2<u16>) -> Result<Texture, RenderError> {
        Texture::new_dynamic(&mut self.factory, dims.x, dims.y)
    }

    /// Update a texture with the provided offset, size, and data.
    pub fn update_texture(
        &mut self,
        texture: &Texture,
        offset: [u16; 2],
        size: [u16; 2],
        data: &[[u8; 4]],
    ) -> Result<(), RenderError> {
        texture.update(&mut self.encoder, offset, size, data)
    }

    /// Creates a download buffer, downloads the win_color_view, and converts to
    /// a image::DynamicImage.
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
        globals: &Consts<Globals>,
        locals: &Consts<skybox::Locals>,
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
                globals: globals.buf.clone(),
                noise: (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
                tgt_color: self.tgt_color_view.clone(),
                tgt_depth: self.tgt_depth_view.clone(),
            },
        );
    }

    /// Queue the rendering of the provided figure model in the upcoming frame.
    pub fn render_figure(
        &mut self,
        model: &Model<figure::FigurePipeline>,
        globals: &Consts<Globals>,
        locals: &Consts<figure::Locals>,
        bones: &Consts<figure::BoneData>,
        lights: &Consts<Light>,
        shadows: &Consts<Shadow>,
    ) {
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
                locals: locals.buf.clone(),
                globals: globals.buf.clone(),
                bones: bones.buf.clone(),
                lights: lights.buf.clone(),
                shadows: shadows.buf.clone(),
                noise: (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
                tgt_color: self.tgt_color_view.clone(),
                tgt_depth: self.tgt_depth_view.clone(),
            },
        );
    }

    /// Queue the rendering of the provided terrain chunk model in the upcoming
    /// frame.
    pub fn render_terrain_chunk(
        &mut self,
        model: &Model<terrain::TerrainPipeline>,
        globals: &Consts<Globals>,
        locals: &Consts<terrain::Locals>,
        lights: &Consts<Light>,
        shadows: &Consts<Shadow>,
    ) {
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
                locals: locals.buf.clone(),
                globals: globals.buf.clone(),
                lights: lights.buf.clone(),
                shadows: shadows.buf.clone(),
                noise: (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
                tgt_color: self.tgt_color_view.clone(),
                tgt_depth: self.tgt_depth_view.clone(),
            },
        );
    }

    /// Queue the rendering of the provided terrain chunk model in the upcoming
    /// frame.
    pub fn render_fluid_chunk(
        &mut self,
        model: &Model<fluid::FluidPipeline>,
        globals: &Consts<Globals>,
        locals: &Consts<terrain::Locals>,
        lights: &Consts<Light>,
        shadows: &Consts<Shadow>,
        waves: &Texture,
    ) {
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
                globals: globals.buf.clone(),
                lights: lights.buf.clone(),
                shadows: shadows.buf.clone(),
                noise: (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
                waves: (waves.srv.clone(), waves.sampler.clone()),
                tgt_color: self.tgt_color_view.clone(),
                tgt_depth: self.tgt_depth_view.clone(),
            },
        );
    }

    /// Queue the rendering of the provided terrain chunk model in the upcoming
    /// frame.
    pub fn render_sprites(
        &mut self,
        model: &Model<sprite::SpritePipeline>,
        globals: &Consts<Globals>,
        instances: &Instances<sprite::Instance>,
        lights: &Consts<Light>,
        shadows: &Consts<Shadow>,
    ) {
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
                globals: globals.buf.clone(),
                lights: lights.buf.clone(),
                shadows: shadows.buf.clone(),
                noise: (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
                tgt_color: self.tgt_color_view.clone(),
                tgt_depth: self.tgt_depth_view.clone(),
            },
        );
    }

    /// Queue the rendering of the provided LoD terrain model in the upcoming frame.
    pub fn render_lod_terrain(
        &mut self,
        model: &Model<lod_terrain::LodTerrainPipeline>,
        globals: &Consts<Globals>,
        locals: &Consts<lod_terrain::Locals>,
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
                globals: globals.buf.clone(),
                noise: (self.noise_tex.srv.clone(), self.noise_tex.sampler.clone()),
                tgt_color: self.tgt_color_view.clone(),
                tgt_depth: self.tgt_depth_view.clone(),
            },
        );
    }

    /// Queue the rendering of the provided UI element in the upcoming frame.
    pub fn render_ui_element(
        &mut self,
        model: &Model<ui::UiPipeline>,
        tex: &Texture,
        scissor: Aabr<u16>,
        globals: &Consts<Globals>,
        locals: &Consts<ui::Locals>,
    ) {
        let Aabr { min, max } = scissor;
        self.encoder.draw(
            &gfx::Slice {
                start: model.vertex_range().start,
                end: model.vertex_range().end,
                base_vertex: 0,
                instances: None,
                buffer: gfx::IndexBuffer::Auto,
            },
            &self.ui_pipeline.pso,
            &ui::pipe::Data {
                vbuf: model.vbuf.clone(),
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

    pub fn render_post_process(
        &mut self,
        model: &Model<postprocess::PostProcessPipeline>,
        globals: &Consts<Globals>,
        locals: &Consts<postprocess::Locals>,
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
                src_sampler: (self.tgt_color_res.clone(), self.sampler.clone()),
                tgt_color: self.win_color_view.clone(),
                tgt_depth: self.win_depth_view.clone(),
            },
        )
    }
}

struct GfxPipeline<P: gfx::pso::PipelineInit> {
    pso: gfx::pso::PipelineState<gfx_backend::Resources, P::Meta>,
}

/// Creates all the pipelines used to render.
fn create_pipelines(
    factory: &mut gfx_backend::Factory,
    aa_mode: AaMode,
    cloud_mode: CloudMode,
    fluid_mode: FluidMode,
    shader_reload_indicator: &mut ReloadIndicator,
) -> Result<
    (
        GfxPipeline<skybox::pipe::Init<'static>>,
        GfxPipeline<figure::pipe::Init<'static>>,
        GfxPipeline<terrain::pipe::Init<'static>>,
        GfxPipeline<fluid::pipe::Init<'static>>,
        GfxPipeline<sprite::pipe::Init<'static>>,
        GfxPipeline<ui::pipe::Init<'static>>,
        GfxPipeline<lod_terrain::pipe::Init<'static>>,
        GfxPipeline<postprocess::pipe::Init<'static>>,
    ),
    RenderError,
> {
    let globals =
        assets::load_watched::<String>("voxygen.shaders.include.globals", shader_reload_indicator)
            .unwrap();
    let sky =
        assets::load_watched::<String>("voxygen.shaders.include.sky", shader_reload_indicator)
            .unwrap();
    let light =
        assets::load_watched::<String>("voxygen.shaders.include.light", shader_reload_indicator)
            .unwrap();
    let srgb =
        assets::load_watched::<String>("voxygen.shaders.include.srgb", shader_reload_indicator)
            .unwrap();
    let random =
        assets::load_watched::<String>("voxygen.shaders.include.random", shader_reload_indicator)
            .unwrap();
    let lod =
        assets::load_watched::<String>("voxygen.shaders.include.lod", shader_reload_indicator)
            .unwrap();

    let anti_alias = assets::load_watched::<String>(
        &["voxygen.shaders.antialias.", match aa_mode {
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

    let cloud = assets::load_watched::<String>(
        &["voxygen.shaders.include.cloud.", match cloud_mode {
            CloudMode::None => "none",
            CloudMode::Regular => "regular",
        }]
        .concat(),
        shader_reload_indicator,
    )
    .unwrap();

    let mut include_ctx = IncludeContext::new();
    include_ctx.include("globals.glsl", &globals);
    include_ctx.include("sky.glsl", &sky);
    include_ctx.include("light.glsl", &light);
    include_ctx.include("srgb.glsl", &srgb);
    include_ctx.include("random.glsl", &random);
    include_ctx.include("lod.glsl", &lod);
    include_ctx.include("anti-aliasing.glsl", &anti_alias);
    include_ctx.include("cloud.glsl", &cloud);

    // Construct a pipeline for rendering skyboxes
    let skybox_pipeline = create_pipeline(
        factory,
        skybox::pipe::new(),
        &assets::load_watched::<String>("voxygen.shaders.skybox-vert", shader_reload_indicator)
            .unwrap(),
        &assets::load_watched::<String>("voxygen.shaders.skybox-frag", shader_reload_indicator)
            .unwrap(),
        &include_ctx,
        gfx::state::CullFace::Back,
    )?;

    // Construct a pipeline for rendering figures
    let figure_pipeline = create_pipeline(
        factory,
        figure::pipe::new(),
        &assets::load_watched::<String>("voxygen.shaders.figure-vert", shader_reload_indicator)
            .unwrap(),
        &assets::load_watched::<String>("voxygen.shaders.figure-frag", shader_reload_indicator)
            .unwrap(),
        &include_ctx,
        gfx::state::CullFace::Back,
    )?;

    // Construct a pipeline for rendering terrain
    let terrain_pipeline = create_pipeline(
        factory,
        terrain::pipe::new(),
        &assets::load_watched::<String>("voxygen.shaders.terrain-vert", shader_reload_indicator)
            .unwrap(),
        &assets::load_watched::<String>("voxygen.shaders.terrain-frag", shader_reload_indicator)
            .unwrap(),
        &include_ctx,
        gfx::state::CullFace::Back,
    )?;

    // Construct a pipeline for rendering fluids
    let fluid_pipeline = create_pipeline(
        factory,
        fluid::pipe::new(),
        &assets::load_watched::<String>("voxygen.shaders.fluid-vert", shader_reload_indicator)
            .unwrap(),
        &assets::load_watched::<String>(
            &["voxygen.shaders.fluid-frag.", match fluid_mode {
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
        &assets::load_watched::<String>("voxygen.shaders.sprite-vert", shader_reload_indicator)
            .unwrap(),
        &assets::load_watched::<String>("voxygen.shaders.sprite-frag", shader_reload_indicator)
            .unwrap(),
        &include_ctx,
        gfx::state::CullFace::Back,
    )?;

    // Construct a pipeline for rendering UI elements
    let ui_pipeline = create_pipeline(
        factory,
        ui::pipe::new(),
        &assets::load_watched::<String>("voxygen.shaders.ui-vert", shader_reload_indicator)
            .unwrap(),
        &assets::load_watched::<String>("voxygen.shaders.ui-frag", shader_reload_indicator)
            .unwrap(),
        &include_ctx,
        gfx::state::CullFace::Back,
    )?;

    // Construct a pipeline for rendering terrain
    let lod_terrain_pipeline = create_pipeline(
        factory,
        lod_terrain::pipe::new(),
        &assets::load_watched::<String>(
            "voxygen.shaders.lod-terrain-vert",
            shader_reload_indicator,
        )
        .unwrap(),
        &assets::load_watched::<String>(
            "voxygen.shaders.lod-terrain-frag",
            shader_reload_indicator,
        )
        .unwrap(),
        &include_ctx,
        gfx::state::CullFace::Back,
    )?;

    // Construct a pipeline for rendering our post-processing
    let postprocess_pipeline = create_pipeline(
        factory,
        postprocess::pipe::new(),
        &assets::load_watched::<String>(
            "voxygen.shaders.postprocess-vert",
            shader_reload_indicator,
        )
        .unwrap(),
        &assets::load_watched::<String>(
            "voxygen.shaders.postprocess-frag",
            shader_reload_indicator,
        )
        .unwrap(),
        &include_ctx,
        gfx::state::CullFace::Back,
    )?;

    Ok((
        skybox_pipeline,
        figure_pipeline,
        terrain_pipeline,
        fluid_pipeline,
        sprite_pipeline,
        ui_pipeline,
        lod_terrain_pipeline,
        postprocess_pipeline,
    ))
}

/// Create a new pipeline from the provided vertex shader and fragment shader.
fn create_pipeline<'a, P: gfx::pso::PipelineInit>(
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
