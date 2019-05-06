use super::{
    consts::Consts,
    gfx_backend,
    mesh::Mesh,
    model::Model,
    pipelines::{figure, postprocess, skybox, terrain, ui, Globals},
    texture::Texture,
    Pipeline, RenderError,
};
use gfx::{
    self,
    handle::Sampler,
    traits::{Device, Factory, FactoryExt},
};
use image;
use vek::*;

/// Represents the format of the window's color target.
pub type TgtColorFmt = gfx::format::Rgba8;
/// Represents the format of the window's depth target.
pub type TgtDepthFmt = gfx::format::DepthStencil;

/// A handle to a window color target.
pub type TgtColorView = gfx::handle::RenderTargetView<gfx_backend::Resources, TgtColorFmt>;
/// A handle to a window depth target.
pub type TgtDepthView = gfx::handle::DepthStencilView<gfx_backend::Resources, TgtDepthFmt>;

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

    win_color_view: TgtColorView,
    win_depth_view: TgtDepthView,

    tgt_color_view: TgtColorView,
    tgt_depth_view: TgtDepthView,

    tgt_color_res: TgtColorRes,

    sampler: Sampler<gfx_backend::Resources>,

    skybox_pipeline: GfxPipeline<skybox::pipe::Init<'static>>,
    figure_pipeline: GfxPipeline<figure::pipe::Init<'static>>,
    terrain_pipeline: GfxPipeline<terrain::pipe::Init<'static>>,
    ui_pipeline: GfxPipeline<ui::pipe::Init<'static>>,
    postprocess_pipeline: GfxPipeline<postprocess::pipe::Init<'static>>,
}

impl Renderer {
    /// Create a new `Renderer` from a variety of backend-specific components and the window
    /// targets.
    pub fn new(
        device: gfx_backend::Device,
        mut factory: gfx_backend::Factory,
        win_color_view: TgtColorView,
        win_depth_view: TgtDepthView,
    ) -> Result<Self, RenderError> {
        // Construct a pipeline for rendering skyboxes
        let skybox_pipeline = create_pipeline(
            &mut factory,
            skybox::pipe::new(),
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/skybox.vert")),
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/skybox.frag")),
        )?;

        // Construct a pipeline for rendering figures
        let figure_pipeline = create_pipeline(
            &mut factory,
            figure::pipe::new(),
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/figure.vert")),
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/figure.frag")),
        )?;

        // Construct a pipeline for rendering terrain
        let terrain_pipeline = create_pipeline(
            &mut factory,
            terrain::pipe::new(),
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/terrain.vert")),
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/terrain.frag")),
        )?;

        // Construct a pipeline for rendering UI elements
        let ui_pipeline = create_pipeline(
            &mut factory,
            ui::pipe::new(),
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/ui.vert")),
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/ui.frag")),
        )?;

        // Construct a pipeline for rendering our post-processing
        let postprocess_pipeline = create_pipeline(
            &mut factory,
            postprocess::pipe::new(),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/shaders/postprocess.vert"
            )),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/shaders/postprocess.frag"
            )),
        )?;

        let dims = win_color_view.get_dimensions();
        let d_dims = win_depth_view.get_dimensions();

        let (_, tgt_color_res, tgt_color_view) = factory
            .create_render_target::<TgtColorFmt>(dims.0, dims.1)
            .map_err(RenderError::CombinedError)?;
        let (_, _, tgt_depth_view) = factory
            .create_depth_stencil(d_dims.0, d_dims.1)
            .map_err(RenderError::CombinedError)?;

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
        })
    }

    /// Get references to the internal render target views that get displayed directly by the window.
    pub fn target_views(&self) -> (&TgtColorView, &TgtDepthView) {
        (&self.win_color_view, &self.win_depth_view)
    }

    /// Get mutable references to the internal render target views that get displayed directly by the window.
    pub fn target_views_mut(&mut self) -> (&mut TgtColorView, &mut TgtDepthView) {
        (&mut self.win_color_view, &mut self.win_depth_view)
    }

    pub fn on_resize(&mut self) -> Result<(), RenderError> {
        let dims = self.win_color_view.get_dimensions();
        let d_dims = self.win_depth_view.get_dimensions();

        if dims.0 > 0 && dims.1 > 0 {
            let (_, tgt_color_res, tgt_color_view) = self
                .factory
                .create_render_target::<TgtColorFmt>(dims.0, dims.1)
                .map_err(RenderError::CombinedError)?;
            self.tgt_color_res = tgt_color_res;
            self.tgt_color_view = tgt_color_view;
        }

        if d_dims.0 > 0 && d_dims.1 > 0 {
            let (_, _, tgt_depth_view) = self
                .factory
                .create_depth_stencil(d_dims.0, d_dims.1)
                .map_err(RenderError::CombinedError)?;
            self.tgt_depth_view = tgt_depth_view;
        }

        Ok(())
    }

    /// Get the resolution of the render target.
    pub fn get_resolution(&self) -> Vec2<u16> {
        Vec2::new(
            self.tgt_color_view.get_dimensions().0,
            self.tgt_color_view.get_dimensions().1,
        )
    }

    /// Queue the clearing of the color and depth targets ready for a new frame to be rendered.
    /// TODO: Make a version of this that doesn't clear the colour target for speed
    pub fn clear(&mut self, col: Rgba<f32>) {
        self.encoder.clear(&self.tgt_color_view, col.into_array());
        self.encoder.clear_depth(&self.tgt_depth_view, 1.0);
        self.encoder.clear(&self.win_color_view, col.into_array());
        self.encoder.clear_depth(&self.win_depth_view, 1.0);
    }

    /// Perform all queued draw calls for this frame and clean up discarded items.
    pub fn flush(&mut self) {
        self.encoder.flush(&mut self.device);
        self.device.cleanup();
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

    /// Create a new texture from the provided image.
    pub fn create_texture<P: Pipeline>(
        &mut self,
        image: &image::DynamicImage,
    ) -> Result<Texture<P>, RenderError> {
        Texture::new(&mut self.factory, image)
    }

    /// Create a new dynamic texture (gfx::memory::Usage::Dynamic) with the specified dimensions
    pub fn create_dynamic_texture<P: Pipeline>(
        &mut self,
        dims: Vec2<u16>,
    ) -> Result<Texture<P>, RenderError> {
        Texture::new_dynamic(&mut self.factory, dims.x, dims.y)
    }

    /// Update a texture with the provided offset, size, and data
    pub fn update_texture<P: Pipeline>(
        &mut self,
        texture: &Texture<P>,
        offset: [u16; 2],
        size: [u16; 2],
        data: &[[u8; 4]],
    ) -> Result<(), RenderError> {
        texture.update(&mut self.encoder, offset, size, data)
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
    ) {
        self.encoder.draw(
            &model.slice,
            &self.figure_pipeline.pso,
            &figure::pipe::Data {
                vbuf: model.vbuf.clone(),
                locals: locals.buf.clone(),
                globals: globals.buf.clone(),
                bones: bones.buf.clone(),
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
    ) {
        self.encoder.draw(
            &model.slice,
            &self.terrain_pipeline.pso,
            &terrain::pipe::Data {
                vbuf: model.vbuf.clone(),
                locals: locals.buf.clone(),
                globals: globals.buf.clone(),
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

/// Create a new pipeline from the provided vertex shader and fragment shader.
fn create_pipeline<'a, P: gfx::pso::PipelineInit>(
    factory: &mut gfx_backend::Factory,
    pipe: P,
    vs: &[u8],
    fs: &[u8],
) -> Result<GfxPipeline<P>, RenderError> {
    let program = factory
        .link_program(vs, fs)
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
            // Do some funky things to work around an oddity in gfx's error ownership rules
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
