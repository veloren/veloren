use gfx::{handle::Program, traits::FactoryExt, Primitive, state::Rasterizer};
use gfx::pso::PipelineState;
use gfx_device_gl;

use mesh::pipe;

pub struct Pipeline {
    program: Program<gfx_device_gl::Resources>,
    pso: PipelineState<gfx_device_gl::Resources, pipe::Meta>,
}

impl Pipeline {
    pub fn new(factory: &mut gfx_device_gl::Factory, vs_code: &[u8], ps_code: &[u8]) -> Pipeline {
        let program = factory.link_program(vs_code, ps_code).expect("Failed to compile shader program");
        Pipeline {
            pso: factory.create_pipeline_from_program(
                &program,
                Primitive::TriangleList,
                Rasterizer::new_fill().with_cull_back(),
                pipe::new(),
            ).expect("Failed to create rendering pipeline"),
            program,
        }
    }
}
