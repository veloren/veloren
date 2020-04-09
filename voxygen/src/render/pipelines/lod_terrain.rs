use super::{
    super::{Pipeline, TgtColorFmt, TgtDepthFmt},
    Globals,
};
use gfx::{
    self, gfx_constant_struct_meta, gfx_defines, gfx_impl_struct_meta, gfx_pipeline,
    gfx_pipeline_inner, gfx_vertex_struct_meta,
};
use vek::*;

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
        map: gfx::TextureSampler<[f32; 4]> = "t_map",
        horizon: gfx::TextureSampler<[f32; 4]> = "t_horizon",

        noise: gfx::TextureSampler<f32> = "t_noise",

        tgt_color: gfx::RenderTarget<TgtColorFmt> = "tgt_color",
        tgt_depth: gfx::DepthTarget<TgtDepthFmt> = gfx::preset::depth::LESS_EQUAL_WRITE,
    }
}

impl Vertex {
    pub fn new(pos: Vec2<f32>) -> Self {
        Self {
            pos: pos.into_array(),
        }
    }
}

impl Locals {
    pub fn default() -> Self { Self { nul: [0.0; 4] } }
}

pub struct LodTerrainPipeline;

impl Pipeline for LodTerrainPipeline {
    type Vertex = Vertex;
}
