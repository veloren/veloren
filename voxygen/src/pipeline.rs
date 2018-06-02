use gfx::{handle::Program, traits::FactoryExt, Primitive, state::Rasterizer};
use gfx::pso::{PipelineState, PipelineInit};
use gfx_device_gl;

pub struct Pipeline<P: PipelineInit> {
    program: Program<gfx_device_gl::Resources>,
    pso: PipelineState<gfx_device_gl::Resources, P::Meta>,
}

impl<P: PipelineInit> Pipeline<P> {
    pub fn new(factory: &mut gfx_device_gl::Factory, pipe: P, vs_code: &[u8], ps_code: &[u8]) -> Pipeline<P> {
        let program = factory.link_program(vs_code, ps_code).expect("Failed to compile shader program");
        Pipeline::<P> {
            pso: factory.create_pipeline_from_program(
                &program,
                Primitive::TriangleList,
                Rasterizer::new_fill().with_cull_back(),
                pipe,
            ).expect("Failed to create rendering pipeline"),
            program,
        }
    }

    pub fn pso(&self) -> &PipelineState<gfx_device_gl::Resources, P::Meta> {
        &self.pso
    }
}
