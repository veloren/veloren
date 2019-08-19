use super::{
    super::{Pipeline, TgtColorFmt, TgtDepthFmt},
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
};
use vek::*;

gfx_defines! {
    vertex Vertex {
        pos: [f32; 3] = "v_pos",
        norm: [f32; 3] = "v_norm",
        col: [f32; 3] = "v_col",
    }

    vertex Instance {
        inst_pos: [f32; 3] = "inst_pos",
        inst_col: [f32; 3] = "inst_col",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        ibuf: gfx::InstanceBuffer<Instance> = (),

        globals: gfx::ConstantBuffer<Globals> = "u_globals",
        lights: gfx::ConstantBuffer<Light> = "u_lights",

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

impl Instance {
    pub fn new(inst_pos: Vec3<f32>, col: Rgb<f32>) -> Self {
        Self {
            inst_pos: inst_pos.into_array(),
            inst_col: col.into_array(),
        }
    }
}

impl Default for Instance {
    fn default() -> Self {
        Self::new(Vec3::zero(), Rgb::broadcast(1.0))
    }
}

pub struct SpritePipeline;

impl Pipeline for SpritePipeline {
    type Vertex = Vertex;
}
