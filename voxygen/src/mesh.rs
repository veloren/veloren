use gfx;

use renderer::ColorFormat;

gfx_defines! {
    vertex Vertex {
        pos: [f32; 3] = "vert_pos",
        norm: [f32; 3] = "vert_norm",
        col: [f32; 3] = "vert_col",
    }

    constant Transform {
        trans: [[f32; 4]; 4] = "uni_trans",
    }

    pipeline pipe {
        vert_buf: gfx::VertexBuffer<Vertex> = (),
        trans: gfx::ConstantBuffer<Transform> = "trans",
        out: gfx::RenderTarget<ColorFormat> = "tgt0",
    }
}

pub struct Mesh {
    vertices: Vec<Vertex>,
}

impl Mesh {
    pub fn new() -> Mesh {
        Mesh {
            vertices: Vec::new(),
        }
    }
}
