// Library
use gfx::{
    self,
    // Macros
    gfx_defines,
    gfx_vertex_struct_meta,
    gfx_constant_struct_meta,
    gfx_impl_struct_meta,
    gfx_pipeline,
    gfx_pipeline_inner,
};

// Local
use super::super::{
    Pipeline,
    TgtColorFmt,
    TgtDepthFmt,
    Mesh,
    Quad,
};

gfx_defines! {
    vertex Vertex {
        pos: [f32; 3] = "v_pos",
        uv: [f32; 2] = "v_uv",
    }

    constant Locals {
        bounds: [f32; 4] = "bounds",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),

        locals: gfx::ConstantBuffer<Locals> = "u_locals",
        tex: gfx::TextureSampler<[f32; 4]> = "u_tex",

        tgt_color: gfx::RenderTarget<TgtColorFmt> = "tgt_color",
        tgt_depth: gfx::DepthTarget<TgtDepthFmt> = gfx::preset::depth::PASS_TEST,
    }
}

impl Locals {
    pub fn default() -> Self {
        Self { bounds: [0.0, 0.0, 1.0, 1.0] }
    }

    pub fn new(bounds: [f32; 4]) -> Self {
        Self {
            bounds,
        }
    }
}

pub struct UiPipeline;

impl Pipeline for UiPipeline {
    type Vertex = Vertex;
}

pub fn create_quad_mesh() -> Mesh<UiPipeline> {
    let mut mesh = Mesh::new();

    #[rustfmt::skip]
    mesh.push_quad(Quad::new(
        Vertex { pos: [0.0, 0.0, 0.0], uv: [0.0, 0.0] },
        Vertex { pos: [0.0, 1.0, 0.0], uv: [0.0, 1.0] },
        Vertex { pos: [1.0, 1.0, 0.0], uv: [1.0, 1.0] },
        Vertex { pos: [1.0, 0.0, 0.0], uv: [1.0, 0.0] },
    ));

    mesh
}
