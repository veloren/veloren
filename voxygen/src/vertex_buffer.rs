use gfx;
use gfx::{traits::FactoryExt, Slice, IndexBuffer};
use gfx_device_gl;

use mesh::{Mesh, Vertex};
use renderer::{Renderer, ColorFormat};

type Data = pipe::Data<gfx_device_gl::Resources>;

gfx_defines! {
    constant Constants {
        trans: [[f32; 4]; 4] = "uni_trans",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        constants: gfx::ConstantBuffer<Constants> = "constants",
        out: gfx::RenderTarget<ColorFormat> = "target",
    }
}

pub struct VertexBuffer {
    data: Data,
    len: u32,
}

impl VertexBuffer {
    pub fn new(renderer: &mut Renderer, mesh: &Mesh) -> VertexBuffer {
        VertexBuffer {
            data: Data {
                vbuf: renderer.factory_mut().create_vertex_buffer(mesh.vertices()),
                constants: renderer.factory_mut().create_constant_buffer(1),
                out: renderer.color_view().clone(),
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
