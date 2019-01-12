// Library
use vek::*;
use gfx::{
    self,
    traits::{Device, FactoryExt},
};

// Local
use super::{
    consts::Consts,
    model::Model,
    mesh::Mesh,
    Pipeline,
    RenderError,
    gfx_backend,
    pipelines::{
        Globals,
        character,
        skybox,
    },
};

/// Represents the format of the window's color target.
pub type TgtColorFmt = gfx::format::Srgba8;
/// Represents the format of the window's depth target.
pub type TgtDepthFmt = gfx::format::DepthStencil;

/// A handle to a window color target.
pub type TgtColorView = gfx::handle::RenderTargetView<gfx_backend::Resources, TgtColorFmt>;
/// A handle to a window depth target.
pub type TgtDepthView = gfx::handle::DepthStencilView<gfx_backend::Resources, TgtDepthFmt>;

/// A type that encapsulates rendering state. `Renderer` is central to Voxygen's rendering
/// subsystem and contains any state necessary to interact with the GPU, along with pipeline state
/// objects (PSOs) needed to renderer different kinds of model to the screen.
pub struct Renderer {
    device: gfx_backend::Device,
    encoder: gfx::Encoder<gfx_backend::Resources, gfx_backend::CommandBuffer>,
    factory: gfx_backend::Factory,

    tgt_color_view: TgtColorView,
    tgt_depth_view: TgtDepthView,

    skybox_pipeline: GfxPipeline<skybox::pipe::Init<'static>>,
    character_pipeline: GfxPipeline<character::pipe::Init<'static>>,
}

impl Renderer {
    /// Create a new `Renderer` from a variety of backend-specific components and the window
    /// targets.
    pub fn new(
        device: gfx_backend::Device,
        mut factory: gfx_backend::Factory,
        tgt_color_view: TgtColorView,
        tgt_depth_view: TgtDepthView,
    ) -> Result<Self, RenderError> {
        // Construct a pipeline for rendering skyboxes
        let skybox_pipeline = create_pipeline(
            &mut factory,
            skybox::pipe::new(),
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/skybox.vert")),
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/skybox.frag")),
        )?;

        // Construct a pipeline for rendering characters
        let character_pipeline = create_pipeline(
            &mut factory,
            character::pipe::new(),
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/character.vert")),
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/character.frag")),
        )?;

        Ok(Self {
            device,
            encoder: factory.create_command_buffer().into(),
            factory,

            tgt_color_view,
            tgt_depth_view,

            skybox_pipeline,
            character_pipeline,
        })
    }

    /// Queue the clearing of the color and depth targets ready for a new frame to be rendered.
    pub fn clear(&mut self, col: Rgba<f32>) {
        self.encoder.clear(&self.tgt_color_view, col.into_array());
        self.encoder.clear_depth(&self.tgt_depth_view, 1.0);
    }

    /// Perform all queued draw calls for this frame and clean up discarded items.
    pub fn flush(&mut self) {
        self.encoder.flush(&mut self.device);
        self.device.cleanup();
    }

    /// Create a new set of constants.
    pub fn create_consts<T: Copy + gfx::traits::Pod>(&mut self) -> Result<Consts<T>, RenderError> {
        Ok(Consts::new(&mut self.factory))
    }

    /// Create a new set of constants with a value.
    pub fn create_consts_with<T: Copy + gfx::traits::Pod>(
        &mut self,
        val: T
    ) -> Result<Consts<T>, RenderError> {
        let mut consts = self.create_consts()?;
        consts.update(&mut self.encoder, val)?;
        Ok(consts)
    }

    /// Update a set of constants with a new value.
    pub fn update_consts<T: Copy + gfx::traits::Pod>(
        &mut self,
        consts: &mut Consts<T>,
        val: T
    ) -> Result<(), RenderError> {
        consts.update(&mut self.encoder, val)
    }

    /// Create a new model from the provided mesh.
    pub fn create_model<P: Pipeline>(&mut self, mesh: &Mesh<P>) -> Result<Model<P>, RenderError> {
        Ok(Model::new(
            &mut self.factory,
            mesh,
        ))
    }

    /// Queue the rendering of the provided skybox model in the upcoming frame
    pub fn render_skybox(
        &mut self,
        model: &Model<skybox::SkyboxPipeline>,
        locals: &Consts<skybox::Locals>,
        globals: &Consts<Globals>,
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
}

struct GfxPipeline<P: gfx::pso::PipelineInit> {
    pso: gfx::pso::PipelineState<gfx_backend::Resources, P::Meta>,
}

/// Create a new pipeline from the provided vertex shader and fragment shader
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
        pso: factory.create_pipeline_from_program(
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
                .map_err(|err| RenderError::PipelineError(match err {
                    gfx::PipelineStateError::Program(err) =>
                        gfx::PipelineStateError::Program(err),
                    gfx::PipelineStateError::DescriptorInit(err) =>
                        gfx::PipelineStateError::DescriptorInit(err.into()),
                    gfx::PipelineStateError::DeviceCreate(err) =>
                        gfx::PipelineStateError::DeviceCreate(err),
                }))?,
    })
}
