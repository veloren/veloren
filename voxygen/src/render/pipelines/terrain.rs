use super::{
    super::{Pipeline, TgtColorFmt, TgtDepthFmt},
    Globals, Light,
};
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
use std::ops::Mul;
use vek::*;

gfx_defines! {
    vertex Vertex {
        pos_norm: u32 = "v_pos_norm",
        col_light: u32 = "v_col_light",
    }

    constant Locals {
        model_offs: [f32; 3] = "model_offs",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),

        locals: gfx::ConstantBuffer<Locals> = "u_locals",
        globals: gfx::ConstantBuffer<Globals> = "u_globals",
        lights: gfx::ConstantBuffer<Light> = "u_lights",

        tgt_color: gfx::RenderTarget<TgtColorFmt> = "tgt_color",
        tgt_depth: gfx::DepthTarget<TgtDepthFmt> = gfx::preset::depth::LESS_EQUAL_WRITE,
    }
}

impl Vertex {
    pub fn new(pos: Vec3<f32>, norm: Vec3<f32>, col: Rgb<f32>, light: f32) -> Self {
        let (norm_axis, norm_dir) = norm
            .as_slice()
            .into_iter()
            .enumerate()
            .find(|(_i, e)| **e != 0.0)
            .unwrap_or((0, &1.0));
        let norm_bits = (norm_axis << 1) | if *norm_dir > 0.0 { 1 } else { 0 };

        Self {
            pos_norm: 0
                | ((pos.x as u32) & 0x00FF) << 0
                | ((pos.y as u32) & 0x00FF) << 8
                | ((pos.z.max(0.0).min((1 << 13) as f32) as u32) & 0x1FFF) << 16
                | ((norm_bits as u32) & 0x7) << 29,
            col_light: 0
                | ((col.r.mul(255.0) as u32) & 0xFF) << 8
                | ((col.g.mul(255.0) as u32) & 0xFF) << 16
                | ((col.b.mul(255.0) as u32) & 0xFF) << 24
                | ((light.mul(255.0) as u32) & 0xFF) << 0,
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
