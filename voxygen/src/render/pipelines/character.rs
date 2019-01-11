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
use super::{
    Globals,
    super::{
        Pipeline,
        TgtColorFmt,
        TgtDepthFmt,
    },
};

gfx_defines! {
    vertex Vertex {
        pos: [f32; 3] = "v_pos",
        bone: u8 = "v_bone",
    }

    constant Locals {
        model_mat: [[f32; 4]; 4] = "model_mat",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        locals: gfx::ConstantBuffer<Locals> = "u_locals",
        globals: gfx::ConstantBuffer<Globals> = "u_globals",
        tgt_color: gfx::RenderTarget<TgtColorFmt> = "tgt_color",
        tgt_depth: gfx::DepthTarget<TgtDepthFmt> = gfx::preset::depth::LESS_EQUAL_WRITE,
    }
}

pub struct CharacterPipeline;

impl Pipeline for CharacterPipeline {
    type Vertex = Vertex;
}
