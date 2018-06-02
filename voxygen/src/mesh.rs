use gfx;

use renderer::ColorFormat;

gfx_defines! {
    vertex Vertex {
        pos: [f32; 3] = "vert_pos",
        norm: [f32; 3] = "vert_norm",
        col: [f32; 3] = "vert_col",
    }

    constant Uniforms {
        trans: [[f32; 4]; 4] = "uni_trans",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        uniforms: gfx::ConstantBuffer<Uniforms> = "uniform",
        out: gfx::RenderTarget<ColorFormat> = "target",
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

    pub fn vertices<'a>(&'a self) -> &'a Vec<Vertex> {
        &self.vertices
    }
}
