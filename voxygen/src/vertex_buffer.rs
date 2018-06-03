use gfx::{traits::FactoryExt, Slice, IndexBuffer};
use gfx_device_gl;

use renderer::ColorView;
use mesh::{Mesh, pipe};

type Data = pipe::Data<gfx_device_gl::Resources>;

pub struct VertexBuffer {
    data: Data,
    len: u32,
}

impl VertexBuffer {
    pub fn new(factory: &mut gfx_device_gl::Factory, color_view: &ColorView, mesh: &Mesh) -> VertexBuffer {
        VertexBuffer {
            data: Data {
                vbuf: factory.create_vertex_buffer(mesh.vertices()),
                constants: factory.create_constant_buffer(1),
                out: color_view.clone(),
            },
            len: mesh.vert_count(),
        }
    }

    pub fn data<'a>(&'a self) -> &'a Data {
        &self.data
    }

    pub fn slice(&self) -> Slice<gfx_device_gl::Resources> {
        Slice::<gfx_device_gl::Resources> {
            start: 0,
            end: self.len,
            base_vertex: 0,
            instances: None,
            buffer: IndexBuffer::Auto,
        }
    }
}
