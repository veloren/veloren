// Library
use gfx::{
    self,
    VertexBuffer,
    ConstantBuffer,
    RenderTarget,
    DepthTarget,
    preset::depth::LESS_EQUAL_WRITE,
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
    renderer::{TgtColorFmt, TgtDepthFmt},
};

gfx_defines! {
    vertex Vertex {
        pos: [f32; 3] = "v_pos",
        bone: u8 = "v_bone",
    }

    constant Locals {
        model: [[f32; 4]; 4] = "u_model",
    }

    pipeline pipe {
        vbuf: VertexBuffer<Vertex> = (),
        locals: ConstantBuffer<Locals> = "locals",
        tgt_color: RenderTarget<TgtColorFmt> = "tgt",
        tgt_depth: DepthTarget<TgtDepthFmt> = LESS_EQUAL_WRITE,
    }
}
