use super::{
    super::{Mesh, Pipeline, Tri, WinColorFmt, WinDepthFmt},
    Globals,
};
use gfx::{
    self, gfx_constant_struct_meta, gfx_defines, gfx_impl_struct_meta, gfx_pipeline,
    gfx_pipeline_inner, gfx_vertex_struct_meta,
};

gfx_defines! {
    vertex Vertex {
        pos: [f32; 2] = "v_pos",
    }

    constant Locals {
        nul: [f32; 4] = "nul",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),

        locals: gfx::ConstantBuffer<Locals> = "u_locals",
        globals: gfx::ConstantBuffer<Globals> = "u_globals",

        src_sampler: gfx::TextureSampler<<WinColorFmt as gfx::format::Formatted>::View> = "src_color",

        tgt_color: gfx::RenderTarget<WinColorFmt> = "tgt_color",
        tgt_depth: gfx::DepthTarget<WinDepthFmt> = gfx::preset::depth::PASS_TEST,
    }
}

impl Locals {
    pub fn default() -> Self { Self { nul: [0.0; 4] } }
}

pub struct PostProcessPipeline;

impl Pipeline for PostProcessPipeline {
    type Vertex = Vertex;
}

pub fn create_mesh() -> Mesh<PostProcessPipeline> {
    let mut mesh = Mesh::new();

    #[rustfmt::skip]
    mesh.push_tri(Tri::new(
        Vertex { pos: [ 1.0, -1.0] },
        Vertex { pos: [-1.0,  1.0] },
        Vertex { pos: [-1.0, -1.0] },
    ));

    #[rustfmt::skip]
    mesh.push_tri(Tri::new(
        Vertex { pos: [1.0, -1.0] },
        Vertex { pos: [1.0,  1.0] },
        Vertex { pos: [-1.0, 1.0] },
    ));

    mesh
}
