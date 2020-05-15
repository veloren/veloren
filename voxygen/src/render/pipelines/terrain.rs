use super::{
    super::{Pipeline, TgtColorFmt, TgtDepthStencilFmt},
    Globals, Light, Shadow,
};
use gfx::{
    self, gfx_constant_struct_meta, gfx_defines, gfx_impl_struct_meta, gfx_pipeline,
    gfx_pipeline_inner, gfx_vertex_struct_meta,
    state::{Comparison, Stencil, StencilOp},
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
        load_time: f32 = "load_time",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),

        locals: gfx::ConstantBuffer<Locals> = "u_locals",
        globals: gfx::ConstantBuffer<Globals> = "u_globals",
        lights: gfx::ConstantBuffer<Light> = "u_lights",
        shadows: gfx::ConstantBuffer<Shadow> = "u_shadows",

        shadow_maps: gfx::TextureSampler<f32> = "t_shadow_maps",

        map: gfx::TextureSampler<[f32; 4]> = "t_map",
        horizon: gfx::TextureSampler<[f32; 4]> = "t_horizon",

        noise: gfx::TextureSampler<f32> = "t_noise",

        tgt_color: gfx::RenderTarget<TgtColorFmt> = "tgt_color",
        tgt_depth_stencil: gfx::DepthStencilTarget<TgtDepthStencilFmt> = (gfx::preset::depth::LESS_EQUAL_WRITE,Stencil::new(Comparison::Always,0xff,(StencilOp::Keep,StencilOp::Keep,StencilOp::Keep))),
    }
}

impl Vertex {
    pub fn new(
        norm_bits: u32,
        light: u32,
        ao: u32,
        pos: Vec3<f32>,
        col: Rgb<f32>,
        meta: bool,
    ) -> Self {
        const EXTRA_NEG_Z: f32 = 32768.0;

        Self {
            pos_norm: 0
                | ((pos.x as u32) & 0x003F) << 0
                | ((pos.y as u32) & 0x003F) << 6
                | (((pos + EXTRA_NEG_Z).z.max(0.0).min((1 << 16) as f32) as u32) & 0xFFFF) << 12
                | if meta { 1 } else { 0 } << 28
                | (norm_bits & 0x7) << 29,
            col_light: 0
                | ((col.r.mul(255.0) as u32) & 0xFF) << 8
                | ((col.g.mul(255.0) as u32) & 0xFF) << 16
                | ((col.b.mul(255.0) as u32) & 0xFF) << 24
                | (ao >> 6) << 6
                | ((light >> 2) & 0x3F) << 0,
        }
    }
}

impl Locals {
    pub fn default() -> Self {
        Self {
            model_offs: [0.0; 3],
            load_time: 0.0,
        }
    }
}

pub struct TerrainPipeline;

impl Pipeline for TerrainPipeline {
    type Vertex = Vertex;
}
