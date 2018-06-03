use gfx;
use gfx::{traits::FactoryExt, Slice, IndexBuffer};
use gfx_device_gl;
use nalgebra::Matrix4;

use mesh::{Mesh, Vertex};
use renderer::{Renderer, ColorFormat, DepthFormat};

type Data = pipe::Data<gfx_device_gl::Resources>;

gfx_defines! {
    constant Constants {
        camera_mat: [[f32; 4]; 4] = "camera_mat",
        model_mat: [[f32; 4]; 4] = "model_mat",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        constants: gfx::ConstantBuffer<Constants> = "constants",
        out_color: gfx::RenderTarget<ColorFormat> = "target",
        out_depth: gfx::DepthTarget<DepthFormat> = gfx::preset::depth::LESS_EQUAL_WRITE,
    }
}

fn mat4_to_array(mat: &Matrix4<f32>) -> [[f32; 4]; 4] {
    let s = mat.as_slice();
    [
        [s[0],  s[1],  s[2],  s[3]],
        [s[4],  s[5],  s[6],  s[7]],
        [s[8],  s[9],  s[10], s[11]],
        [s[12], s[13], s[14], s[15]],
    ]
}

impl Constants {
    pub fn new(camera_mat: &Matrix4<f32>, model_mat: &Matrix4<f32>) -> Constants {
        Constants {
            camera_mat: mat4_to_array(&camera_mat),
            model_mat: mat4_to_array(&camera_mat),
        }
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
                out_color: renderer.color_view().clone(),
                out_depth: renderer.depth_view().clone(),
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
