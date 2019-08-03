use super::{
    consts::Consts,
    gfx_backend,
    mesh::Mesh,
    model::{DynamicModel, Model},
    pipelines::{figure, postprocess, skybox, terrain, ui, Globals, Light},
    texture::Texture,
    Pipeline, RenderError,
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
pub type TgtColorFmt = gfx::format::Rgba16F;
/// Represents the format of the pre-processed depth target.
pub type TgtDepthFmt = gfx::format::Depth;

/// Represents the format of the window's color target.
pub type WinColorFmt = gfx::format::Rgba8;
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

/// A type that encapsulates rendering state. `Renderer` is central to Voxygen's rendering
/// subsystem and contains any state necessary to interact with the GPU, along with pipeline state
/// objects (PSOs) needed to renderer different kinds of models to the screen.
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
    ui_pipeline: GfxPipeline<ui::pipe::Init<'static>>,
    postprocess_pipeline: GfxPipeline<postprocess::pipe::Init<'static>>,

    shader_reload_indicator: ReloadIndicator,
}

impl Renderer {
    /// Create a new `Renderer` from a variety of backend-specific components and the window targets.
    pub fn new(
        device: gfx_backend::Device,
        mut factory: gfx_backend::Factory,
        win_color_view: WinColorView,
        win_depth_view: WinDepthView,
    ) -> Result<Self, RenderError> {
        let mut shader_reload_indicator = ReloadIndicator::new();

        let (skybox_pipeline, figure_pipeline, terrain_pipeline, ui_pipeline, postprocess_pipeline) =
            create_pipelines(&mut factory, &mut shader_reload_indicator)?;

        let dims = win_color_view.get_dimensions();
        let (tgt_color_view, tgt_depth_view, tgt_color_res) =
            Self::create_rt_views(&mut factory, (dims.0, dims.1))?;

        let sampler = factory.create_sampler_linear();

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
            ui_pipeline,
            postprocess_pipeline,

            shader_reload_indicator,
        })
    }

    /// Get references to the internal render target views that get rendered to before post-processing.
    #[allow(dead_code)]
    pub fn tgt_views(&self) -> (&TgtColorView, &TgtDepthView) {
        (&self.tgt_color_view, &self.tgt_depth_view)
    }

    /// Get references to the internal render target views that get displayed directly by the window.
    #[allow(dead_code)]
    pub fn win_views(&self) -> (&WinColorView, &WinDepthView) {
        (&self.win_color_view, &self.win_depth_view)
    }

    /// Get mutable references to the internal render target views that get rendered to before post-processing.
    #[allow(dead_code)]
    pub fn tgt_views_mut(&mut self) -> (&mut TgtColorView, &mut TgtDepthView) {
        (&mut self.tgt_color_view, &mut self.tgt_depth_view)
    }

    /// Get mutable references to the internal render target views that get displayed directly by the window.
    #[allow(dead_code)]
    pub fn win_views_mut(&mut self) -> (&mut WinColorView, &mut WinDepthView) {
        (&mut self.win_color_view, &mut self.win_depth_view)
    }

    /// Resize internal render targets to match window render target dimensions.
    pub fn on_resize(&mut self) -> Result<(), RenderError> {
        let dims = self.win_color_view.get_dimensions();

        // Avoid panics when creating texture with w,h of 0,0.
        if dims.0 != 0 && dims.1 != 0 {
            let (tgt_color_view, tgt_depth_view, tgt_color_res) =
                Self::create_rt_views(&mut self.factory, (dims.0, dims.1))?;
            self.tgt_color_res = tgt_color_res;
            self.tgt_color_view = tgt_color_view;
            self.tgt_depth_view = tgt_depth_view;
        }

        Ok(())
    }

    fn create_rt_views(
        factory: &mut gfx_device_gl::Factory,
        size: (u16, u16),
    ) -> Result<(TgtColorView, TgtDepthView, TgtColorRes), RenderError> {
        let (_, tgt_color_res, tgt_color_view) = factory
            .create_render_target::<TgtColorFmt>(size.0, size.1)
            .map_err(RenderError::CombinedError)?;;
        let tgt_depth_view = factory
            .create_depth_stencil_view_only::<TgtDepthFmt>(size.0, size.1)
            .map_err(RenderError::CombinedError)?;;
        Ok((tgt_color_view, tgt_depth_view, tgt_color_res))
    }

    /// Get the resolution of the render target.
    pub fn get_resolution(&self) -> Vec2<u16> {
        Vec2::new(
            self.win_color_view.get_dimensions().0,
            self.win_color_view.get_dimensions().1,
        )
    }

    /// Queue the clearing of the color and depth targets ready for a new frame to be rendered.
    pub fn clear(&mut self) {
        self.encoder.clear_depth(&self.tgt_depth_view, 1.0);
        self.encoder.clear_depth(&self.win_depth_view, 1.0);
    }

    /// Perform all queued draw calls for this frame and clean up discarded items.
    pub fn flush(&mut self) {
        self.encoder.flush(&mut self.device);
        self.device.cleanup();

        // If the shaders files were changed attempt to recreate the shaders
        if self.shader_reload_indicator.reloaded() {
            match create_pipelines(&mut self.factory, &mut self.shader_reload_indicator) {
                Ok((
                    skybox_pipeline,
                    figure_pipeline,
                    terrain_pipline,
                    ui_pipeline,
                    postprocess_pipeline,
                )) => {
                    self.skybox_pipeline = skybox_pipeline;
                    self.figure_pipeline = figure_pipeline;
                    self.terrain_pipeline = terrain_pipline;
                    self.ui_pipeline = ui_pipeline;
                    self.postprocess_pipeline = postprocess_pipeline;
                }
                Err(e) => error!(
                    "Could not recreate shaders from assets due to an error: {:#?}",
                    e
                ),
            }
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
    pub fn max_texture_size(&self) -> usize {
        self.factory.get_capabilities().max_texture_size
    }

    /// Create a new texture from the provided image.
    pub fn create_texture<P: Pipeline>(
        &mut self,
        image: &image::DynamicImage,
    ) -> Result<Texture<P>, RenderError> {
        Texture::new(&mut self.factory, image)
    }

    /// Create a new dynamic texture (gfx::memory::Usage::Dynamic) with the specified dimensions.
    pub fn create_dynamic_texture<P: Pipeline>(
        &mut self,
        dims: Vec2<u16>,
    ) -> Result<Texture<P>, RenderError> {
        Texture::new_dynamic(&mut self.factory, dims.x, dims.y)
    }

    /// Update a texture with the provided offset, size, and data.
    pub fn update_texture<P: Pipeline>(
        &mut self,
        texture: &Texture<P>,
        offset: [u16; 2],
        size: [u16; 2],
        data: &[[u8; 4]],
    ) -> Result<(), RenderError> {
        texture.update(&mut self.encoder, offset, size, data)
    }

    /// Creates a download buffer, downloads the win_color_view, and converts to a image::DynamicImage.
    pub fn create_screenshot(&mut self) -> Result<image::DynamicImage, RenderError> {
        let (width, height) = self.get_resolution().into_tuple();
        use gfx::{
            format::{Formatted, SurfaceTyped},
            memory::Typed,
        };
        type WinSurfaceData = <<WinColorFmt as Formatted>::Surface as SurfaceTyped>::DataType;
        let download = self
            .factory
            .create_download_buffer::<WinSurfaceData>(width as usize * height as usize)
            .map_err(|err| RenderError::BufferCreationError(err))?;
        self.encoder
            .copy_texture_to_buffer_raw(
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
            )
            .map_err(|err| RenderError::CopyError(err))?;
        self.flush();

        // Assumes that the format is Rgba8.
        let raw_data = self
            .factory
            .read_mapping(&download)
            .map_err(|err| RenderError::MappingError(err))?
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
            &model.slice,
            &self.skybox_pipeline.pso,
            &skybox::pipe::Data {
                vbuf: model.vbuf.clone(),
                locals: locals.buf.clone(),
                globals: globals.buf.clone(),
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
    ) {
        self.encoder.draw(
            &model.slice,
            &self.figure_pipeline.pso,
            &figure::pipe::Data {
                vbuf: model.vbuf.clone(),
                locals: locals.buf.clone(),
                globals: globals.buf.clone(),
                bones: bones.buf.clone(),
                lights: lights.buf.clone(),
                tgt_color: self.tgt_color_view.clone(),
                tgt_depth: self.tgt_depth_view.clone(),
            },
        );
    }

    /// Queue the rendering of the provided terrain chunk model in the upcoming frame.
    pub fn render_terrain_chunk(
        &mut self,
        model: &Model<terrain::TerrainPipeline>,
        globals: &Consts<Globals>,
        locals: &Consts<terrain::Locals>,
        lights: &Consts<Light>,
    ) {
        self.encoder.draw(
            &model.slice,
            &self.terrain_pipeline.pso,
            &terrain::pipe::Data {
                vbuf: model.vbuf.clone(),
                locals: locals.buf.clone(),
                globals: globals.buf.clone(),
                lights: lights.buf.clone(),
                tgt_color: self.tgt_color_view.clone(),
                tgt_depth: self.tgt_depth_view.clone(),
            },
        );
    }

    /// Queue the rendering of the provided UI element in the upcoming frame.
    pub fn render_ui_element(
        &mut self,
        model: &Model<ui::UiPipeline>,
        tex: &Texture<ui::UiPipeline>,
        scissor: Aabr<u16>,
        globals: &Consts<Globals>,
        locals: &Consts<ui::Locals>,
    ) {
        let Aabr { min, max } = scissor;
        self.encoder.draw(
            &model.slice,
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
            &model.slice,
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

/// Create new the pipelines used by the renderer.
fn create_pipelines(
    factory: &mut gfx_backend::Factory,
    shader_reload_indicator: &mut ReloadIndicator,
) -> Result<
    (
        GfxPipeline<skybox::pipe::Init<'static>>,
        GfxPipeline<figure::pipe::Init<'static>>,
        GfxPipeline<terrain::pipe::Init<'static>>,
        GfxPipeline<ui::pipe::Init<'static>>,
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

    let mut include_ctx = IncludeContext::new();
    include_ctx.include("globals.glsl", &globals);
    include_ctx.include("sky.glsl", &sky);
    include_ctx.include("light.glsl", &light);

    // Construct a pipeline for rendering skyboxes
    let skybox_pipeline = create_pipeline(
        factory,
        skybox::pipe::new(),
        &assets::load_watched::<String>("voxygen.shaders.skybox.vert", shader_reload_indicator)
            .unwrap(),
        &assets::load_watched::<String>("voxygen.shaders.skybox.frag", shader_reload_indicator)
            .unwrap(),
        &include_ctx,
    )?;

    // Construct a pipeline for rendering figures
    let figure_pipeline = create_pipeline(
        factory,
        figure::pipe::new(),
        &assets::load_watched::<String>("voxygen.shaders.figure.vert", shader_reload_indicator)
            .unwrap(),
        &assets::load_watched::<String>("voxygen.shaders.figure.frag", shader_reload_indicator)
            .unwrap(),
        &include_ctx,
    )?;

    // Construct a pipeline for rendering terrain
    let terrain_pipeline = create_pipeline(
        factory,
        terrain::pipe::new(),
        &assets::load_watched::<String>("voxygen.shaders.terrain.vert", shader_reload_indicator)
            .unwrap(),
        &assets::load_watched::<String>("voxygen.shaders.terrain.frag", shader_reload_indicator)
            .unwrap(),
        &include_ctx,
    )?;

    // Construct a pipeline for rendering UI elements
    let ui_pipeline = create_pipeline(
        factory,
        ui::pipe::new(),
        &assets::load_watched::<String>("voxygen.shaders.ui.vert", shader_reload_indicator)
            .unwrap(),
        &assets::load_watched::<String>("voxygen.shaders.ui.frag", shader_reload_indicator)
            .unwrap(),
        &include_ctx,
    )?;

    // Construct a pipeline for rendering our post-processing
    let postprocess_pipeline = create_pipeline(
        factory,
        postprocess::pipe::new(),
        &assets::load_watched::<String>(
            "voxygen.shaders.postprocess.vert",
            shader_reload_indicator,
        )
        .unwrap(),
        &assets::load_watched::<String>(
            "voxygen.shaders.postprocess.frag",
            shader_reload_indicator,
        )
        .unwrap(),
        &include_ctx,
    )?;

    Ok((
        skybox_pipeline,
        figure_pipeline,
        terrain_pipeline,
        ui_pipeline,
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
) -> Result<GfxPipeline<P>, RenderError> {
    let vs = ctx.expand(vs).map_err(RenderError::IncludeError)?;
    let fs = ctx.expand(fs).map_err(RenderError::IncludeError)?;

    let program = factory
        .link_program(vs.as_bytes(), fs.as_bytes())
        .map_err(|err| RenderError::PipelineError(gfx::PipelineStateError::Program(err)))?;

    Ok(GfxPipeline {
        pso: factory
            .create_pipeline_from_program(
                &program,
                gfx::Primitive::TriangleList,
                gfx::state::Rasterizer {
                    front_face: gfx::state::FrontFace::CounterClockwise,
                    cull_face: gfx::state::CullFace::Back,
                    method: gfx::state::RasterMethod::Fill,
                    offset: None,
                    samples: Some(gfx::state::MultiSample),
                },
                pipe,
            )
            // Do some funky things to work around an oddity in gfx's error ownership rules.
            .map_err(|err| {
                RenderError::PipelineError(match err {
                    gfx::PipelineStateError::Program(err) => gfx::PipelineStateError::Program(err),
                    gfx::PipelineStateError::DescriptorInit(err) => {
                        gfx::PipelineStateError::DescriptorInit(err.into())
                    }
                    gfx::PipelineStateError::DeviceCreate(err) => {
                        gfx::PipelineStateError::DeviceCreate(err)
                    }
                })
            })?,
    })
}
