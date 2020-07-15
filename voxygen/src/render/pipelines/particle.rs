use super::{
    super::{Pipeline, TgtColorFmt, TgtDepthStencilFmt},
    Globals, Light, Shadow,
};
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
        // created_at time, so we can calculate time relativity, needed for relative animation.
        // can save 32 bits per instance, for particles that are not relatively animated.
        inst_time: f32 = "inst_time",

        // a seed value for randomness
        inst_entropy: f32 = "inst_entropy",

        // modes should probably be seperate shaders, as a part of scaling and optimisation efforts
        inst_mode: i32 = "inst_mode",

        // a triangle is:  f32 x 3 x 3 x 1  = 288 bits
        // a quad is:      f32 x 3 x 3 x 2  = 576 bits
        // a cube is:      f32 x 3 x 3 x 12 = 3456 bits
        // this matrix is: f32 x 4 x 4 x 1  = 512 bits (per instance!)
        // consider using vertex postion & entropy instead;
        // to determine initial offset, scale, orientation etc.
        inst_mat0: [f32; 4] = "inst_mat0",
        inst_mat1: [f32; 4] = "inst_mat1",
        inst_mat2: [f32; 4] = "inst_mat2",
        inst_mat3: [f32; 4] = "inst_mat3",
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

pub enum ParticleMode {
    CampfireSmoke,
    CampfireFire,
}

impl ParticleMode {
    pub fn into_uint(self) -> u32 { self as u32 }
}

impl Instance {
    pub fn new(
        inst_time: f64,
        inst_entropy: f32,
        inst_mode: ParticleMode,
        inst_mat: Mat4<f32>,
    ) -> Self {
        let inst_mat_col = inst_mat.into_col_arrays();
        Self {
            inst_time: inst_time as f32,
            inst_entropy,
            inst_mode: inst_mode as i32,

            inst_mat0: inst_mat_col[0],
            inst_mat1: inst_mat_col[1],
            inst_mat2: inst_mat_col[2],
            inst_mat3: inst_mat_col[3],
        }
    }
}

impl Default for Instance {
    fn default() -> Self { Self::new(0.0, 0.0, ParticleMode::CampfireSmoke, Mat4::identity()) }
}

pub struct ParticlePipeline;

impl Pipeline for ParticlePipeline {
    type Vertex = Vertex;
}
