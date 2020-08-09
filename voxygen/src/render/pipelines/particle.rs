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
        // can save 32 bits per instance, for particles that don't need randomness/uniqueness.
        inst_entropy: f32 = "inst_entropy",

        // modes should probably be seperate shaders, as a part of scaling and optimisation efforts.
        // can save 32 bits per instance, and have cleaner tailor made code.
        inst_mode: i32 = "inst_mode",

        // a triangle is: f32 x 3 x 3 x 1  = 288 bits
        // a quad is:     f32 x 3 x 3 x 2  = 576 bits
        // a cube is:     f32 x 3 x 3 x 12 = 3456 bits
        // this vec is:   f32 x 3 x 1 x 1  = 96 bits (per instance!)
        // consider using a throw-away mesh and
        // positioning the vertex verticies instead,
        // if we have:
        // - a triangle mesh, and 3 or more instances.
        // - a quad mesh, and 6 or more instances.
        // - a cube mesh, and 36 or more instances.
        inst_pos: [f32; 3] = "inst_pos",
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
    CampfireSmoke = 0,
    CampfireFire = 1,
    GunPowderSpark = 2,
    Shrapnel = 3,
}

impl ParticleMode {
    pub fn into_uint(self) -> u32 { self as u32 }
}

impl Instance {
    pub fn new(inst_time: f64, inst_mode: ParticleMode, inst_pos: Vec3<f32>) -> Self {
        use rand::Rng;
        Self {
            inst_time: inst_time as f32,
            inst_entropy: rand::thread_rng().gen(),
            inst_mode: inst_mode as i32,
            inst_pos: inst_pos.into_array(),
        }
    }
}

impl Default for Instance {
    fn default() -> Self { Self::new(0.0, ParticleMode::CampfireSmoke, Vec3::zero()) }
}

pub struct ParticlePipeline;

impl Pipeline for ParticlePipeline {
    type Vertex = Vertex;
}
