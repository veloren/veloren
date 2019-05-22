use std::ops::{Add, Mul, Div};
use gfx::{
    self,
    gfx_constant_struct_meta,
    // Macros
    gfx_defines,
    gfx_impl_struct_meta,
    gfx_pipeline,
    gfx_pipeline_inner,
    gfx_vertex_struct_meta,
};
use vek::*;
use super::{
    super::{Pipeline, TgtColorFmt, TgtDepthFmt},
    Globals,
};

gfx_defines! {
    vertex Vertex {
        pos: u32 = "v_pos",
        col_norm: u32 = "v_col_norm",
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
            pos: 0
                | ((pos.x as u32) & 0x00FF) << 0
                | ((pos.y as u32) & 0x00FF) << 8
                | ((pos.z as u32) & 0xFFFF) << 16,
            col_norm: 0
                | ((col.r.mul(255.0) as u32) & 0xFF) << 8
                | ((col.g.mul(255.0) as u32) & 0xFF) << 16
                | ((col.b.mul(255.0) as u32) & 0xFF) << 24
                | ((norm.x.add(1.0) as u32) & 0x3) << 0
                | ((norm.y.add(1.0) as u32) & 0x3) << 2
                | ((norm.z.add(1.0) as u32) & 0x3) << 4,
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
