use super::{
    super::{Pipeline, TerrainLocals, TgtColorFmt, TgtDepthFmt},
    Globals, Light,
};
use gfx::{
    self,
    // Macros
    gfx_defines,
    gfx_impl_struct_meta,
    gfx_pipeline,
    gfx_pipeline_inner,
    gfx_vertex_struct_meta,
    state::ColorMask,
};
use std::ops::Mul;
use vek::*;

gfx_defines! {
    vertex Vertex {
        pos_norm: u32 = "v_pos_norm",
        col_light: u32 = "v_col_light",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),

        locals: gfx::ConstantBuffer<TerrainLocals> = "u_locals",
        globals: gfx::ConstantBuffer<Globals> = "u_globals",
        lights: gfx::ConstantBuffer<Light> = "u_lights",

        tgt_color: gfx::BlendTarget<TgtColorFmt> = ("tgt_color", ColorMask::all(), gfx::preset::blend::ALPHA),
        tgt_depth: gfx::DepthTarget<TgtDepthFmt> = gfx::preset::depth::LESS_EQUAL_TEST,
    }
}

impl Vertex {
    pub fn new(pos: Vec3<f32>, norm: Vec3<f32>, col: Rgb<f32>, light: f32, _opac: f32) -> Self {
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
                | ((col.r.mul(200.0) as u32) & 0xFF) << 8
                | ((col.g.mul(200.0) as u32) & 0xFF) << 16
                | ((col.b.mul(200.0) as u32) & 0xFF) << 24
                | ((light.mul(255.0) as u32) & 0xFF) << 0,
            //| ((opac.mul(0.4) as u32) & 0xFF) << 0,
        }
    }
}

pub struct FluidPipeline;

impl Pipeline for FluidPipeline {
    type Vertex = Vertex;
}
