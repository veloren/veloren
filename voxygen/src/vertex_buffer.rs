use gfx::{handle::Buffer, traits::FactoryExt};
use gfx::pso::PipelineState;
use gfx_device_gl;

use renderer::ColorView;
use mesh::{Mesh, Vertex, pipe};

pub struct VertexBuffer {
    data: pipe::Data<gfx_device_gl::Resources>,
}

impl VertexBuffer {
    pub fn new(factory: &mut gfx_device_gl::Factory, color_view: &ColorView, mesh: &Mesh) -> VertexBuffer {
        VertexBuffer {
            data: pipe::Data {
                vbuf: factory.create_vertex_buffer(mesh.vertices()),
                uniforms: factory.create_constant_buffer(1),
                out: color_view.clone(),
            },
        }
    }
}
