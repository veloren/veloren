use super::{
    super::{Pipeline, TgtColorFmt, TgtDepthStencilFmt},
    Globals, Light, Shadow,
};
use common::comp::visual::ParticleEmitterMode;
use gfx::{
    self, gfx_defines, gfx_impl_struct_meta, gfx_pipeline, gfx_pipeline_inner,
    gfx_vertex_struct_meta,
    state::{ColorMask, Comparison, Stencil, StencilOp},
};
use vek::*;

gfx_defines! {
    vertex Vertex {
        pos: [f32; 3] = "v_pos",
        // ____BBBBBBBBGGGGGGGGRRRRRRRR
        col: u32 = "v_col",
        // ...AANNN
        // A = AO
        // N = Normal
        norm_ao: u32 = "v_norm_ao",
    }

    vertex Instance {
        inst_mat0: [f32; 4] = "inst_mat0",
        inst_mat1: [f32; 4] = "inst_mat1",
        inst_mat2: [f32; 4] = "inst_mat2",
        inst_mat3: [f32; 4] = "inst_mat3",
        inst_col: [f32; 3] = "inst_col",
        inst_vel: [f32; 3] = "inst_vel",
        inst_tick: [f32; 4] = "inst_tick",
        inst_wind_sway: f32 = "inst_wind_sway",
        mode: u8 = "mode",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        ibuf: gfx::InstanceBuffer<Instance> = (),

        globals: gfx::ConstantBuffer<Globals> = "u_globals",
        lights: gfx::ConstantBuffer<Light> = "u_lights",
        shadows: gfx::ConstantBuffer<Shadow> = "u_shadows",

        noise: gfx::TextureSampler<f32> = "t_noise",

        tgt_color: gfx::BlendTarget<TgtColorFmt> = ("tgt_color", ColorMask::all(), gfx::preset::blend::ALPHA),
        tgt_depth_stencil: gfx::DepthStencilTarget<TgtDepthStencilFmt> = (gfx::preset::depth::LESS_EQUAL_WRITE,Stencil::new(Comparison::Always,0xff,(StencilOp::Keep,StencilOp::Keep,StencilOp::Keep))),
    }
}

impl Vertex {
    #[allow(clippy::collapsible_if)]
    pub fn new(pos: Vec3<f32>, norm: Vec3<f32>, col: Rgb<f32>, ao: f32) -> Self {
        let norm_bits = if norm.x != 0.0 {
            if norm.x < 0.0 { 0 } else { 1 }
        } else if norm.y != 0.0 {
            if norm.y < 0.0 { 2 } else { 3 }
        } else {
            if norm.z < 0.0 { 4 } else { 5 }
        };

        Self {
            pos: pos.into_array(),
            col: col
                .map2(Rgb::new(0, 8, 16), |e, shift| ((e * 255.0) as u32) << shift)
                .reduce_bitor(),
            norm_ao: norm_bits | (((ao * 3.9999) as u32) << 3),
        }
    }
}

impl Instance {
    pub fn new(
        mat: Mat4<f32>,
        col: Rgb<f32>,
        vel: Vec3<f32>,
        tick: u64,
        wind_sway: f32,
        mode: ParticleEmitterMode,
    ) -> Self {
        let mat_arr = mat.into_col_arrays();
        Self {
            inst_mat0: mat_arr[0],
            inst_mat1: mat_arr[1],
            inst_mat2: mat_arr[2],
            inst_mat3: mat_arr[3],
            inst_col: col.into_array(),
            inst_vel: vel.into_array(),
            inst_tick: [tick as f32; 4],

            inst_wind_sway: wind_sway,

            mode: mode as u8,
        }
    }
}

impl Default for Instance {
    fn default() -> Self {
        Self::new(
            Mat4::identity(),
            Rgb::broadcast(1.0),
            Vec3::zero(),
            0,
            0.0,
            ParticleEmitterMode::Sprinkler,
        )
    }
}

pub struct ParticlePipeline;

impl Pipeline for ParticlePipeline {
    type Vertex = Vertex;
}
