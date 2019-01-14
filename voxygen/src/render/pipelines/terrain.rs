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
use vek::*;

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
        norm: [f32; 3] = "v_norm",
        col: [f32; 3] = "v_col",
    }

    constant Locals {
        model_offs: [f32; 3] = "model_offs",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),

        locals: gfx::ConstantBuffer<Locals> = "u_locals",
        globals: gfx::ConstantBuffer<Globals> = "u_globals",

        tgt_color: gfx::RenderTarget<TgtColorFmt> = "tgt_color",
        tgt_depth: gfx::DepthTarget<TgtDepthFmt> = gfx::preset::depth::LESS_EQUAL_WRITE,
    }
}

impl Vertex {
    pub fn new(pos: Vec3<f32>, norm: Vec3<f32>, col: Rgb<f32>) -> Self {
        Self {
            pos: pos.into_array(),
            col: col.into_array(),
            norm: norm.into_array(),
        }
    }
}

impl Locals {
    pub fn default() -> Self {
        Self {
            model_offs: [0.0; 3],
        }
    }
}

pub struct TerrainPipeline;

impl Pipeline for TerrainPipeline {
    type Vertex = Vertex;
}
