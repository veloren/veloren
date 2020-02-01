use super::{
    super::{Mesh, Pipeline, Quad, TgtColorFmt, TgtDepthFmt},
    Globals,
};
use gfx::{
    self, gfx_constant_struct_meta, gfx_defines, gfx_impl_struct_meta, gfx_pipeline,
    gfx_pipeline_inner, gfx_vertex_struct_meta,
};

gfx_defines! {
    vertex Vertex {
        pos: [f32; 3] = "v_pos",
    }

    constant Locals {
        nul: [f32; 4] = "nul",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),

        locals: gfx::ConstantBuffer<Locals> = "u_locals",
        globals: gfx::ConstantBuffer<Globals> = "u_globals",

        noise: gfx::TextureSampler<f32> = "t_noise",

        tgt_color: gfx::RenderTarget<TgtColorFmt> = "tgt_color",
        tgt_depth: gfx::DepthTarget<TgtDepthFmt> = gfx::preset::depth::LESS_EQUAL_WRITE,
    }
}

impl Locals {
    pub fn default() -> Self { Self { nul: [0.0; 4] } }
}

pub struct SkyboxPipeline;

impl Pipeline for SkyboxPipeline {
    type Vertex = Vertex;
}

pub fn create_mesh() -> Mesh<SkyboxPipeline> {
    let mut mesh = Mesh::new();

    // -x
    #[rustfmt::skip]
    mesh.push_quad(Quad::new(
        Vertex { pos: [-1.0, -1.0, -1.0] },
        Vertex { pos: [-1.0,  1.0, -1.0] },
        Vertex { pos: [-1.0,  1.0,  1.0] },
        Vertex { pos: [-1.0, -1.0,  1.0] },
    ));
    // +x
    #[rustfmt::skip]
    mesh.push_quad(Quad::new(
        Vertex { pos: [ 1.0, -1.0,  1.0] },
        Vertex { pos: [ 1.0,  1.0,  1.0] },
        Vertex { pos: [ 1.0,  1.0, -1.0] },
        Vertex { pos: [ 1.0, -1.0, -1.0] },
    ));
    // -y
    #[rustfmt::skip]
    mesh.push_quad(Quad::new(
        Vertex { pos: [ 1.0, -1.0, -1.0] },
        Vertex { pos: [-1.0, -1.0, -1.0] },
        Vertex { pos: [-1.0, -1.0,  1.0] },
        Vertex { pos: [ 1.0, -1.0,  1.0] },
    ));
    // +y
    #[rustfmt::skip]
    mesh.push_quad(Quad::new(
        Vertex { pos: [ 1.0,  1.0,  1.0] },
        Vertex { pos: [-1.0,  1.0,  1.0] },
        Vertex { pos: [-1.0,  1.0, -1.0] },
        Vertex { pos: [ 1.0,  1.0, -1.0] },
    ));
    // -z
    #[rustfmt::skip]
    mesh.push_quad(Quad::new(
        Vertex { pos: [-1.0, -1.0, -1.0] },
        Vertex { pos: [ 1.0, -1.0, -1.0] },
        Vertex { pos: [ 1.0,  1.0, -1.0] },
        Vertex { pos: [-1.0,  1.0, -1.0] },
    ));
    // +z
    #[rustfmt::skip]
    mesh.push_quad(Quad::new(
        Vertex { pos: [-1.0,  1.0,  1.0] },
        Vertex { pos: [ 1.0,  1.0,  1.0] },
        Vertex { pos: [ 1.0, -1.0,  1.0] },
        Vertex { pos: [-1.0, -1.0,  1.0] },
    ));

    mesh
}
