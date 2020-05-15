use super::{
    super::{util::arr_to_mat, Pipeline, TgtColorFmt, TgtDepthStencilFmt},
    Globals, Light, Shadow,
};
use gfx::{
    self, gfx_constant_struct_meta, gfx_defines, gfx_impl_struct_meta, gfx_pipeline,
    gfx_pipeline_inner, gfx_vertex_struct_meta,
    state::{ColorMask, Comparison, Stencil, StencilOp},
};
use vek::*;

gfx_defines! {
    vertex Vertex {
        pos_norm: u32 = "v_pos_norm",
        col: u32 = "v_col",
        // BBBBBBAA
        // B = Bone
        // A = AO
        ao_bone: u8 = "v_ao_bone",
    }

    constant Locals {
        model_mat: [[f32; 4]; 4] = "model_mat",
        model_col: [f32; 4] = "model_col",
        flags: u32 = "flags",
    }

    constant BoneData {
        bone_mat: [[f32; 4]; 4] = "bone_mat",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),

        locals: gfx::ConstantBuffer<Locals> = "u_locals",
        globals: gfx::ConstantBuffer<Globals> = "u_globals",
        bones: gfx::ConstantBuffer<BoneData> = "u_bones",
        lights: gfx::ConstantBuffer<Light> = "u_lights",
        shadows: gfx::ConstantBuffer<Shadow> = "u_shadows",

        shadow_maps: gfx::TextureSampler<f32> = "t_shadow_maps",

        map: gfx::TextureSampler<[f32; 4]> = "t_map",
        horizon: gfx::TextureSampler<[f32; 4]> = "t_horizon",

        noise: gfx::TextureSampler<f32> = "t_noise",

        tgt_color: gfx::BlendTarget<TgtColorFmt> = ("tgt_color", ColorMask::all(), gfx::preset::blend::ALPHA),
        tgt_depth_stencil: gfx::DepthStencilTarget<TgtDepthStencilFmt> = (gfx::preset::depth::LESS_EQUAL_WRITE,Stencil::new(Comparison::Always,0xff,(StencilOp::Keep,StencilOp::Keep,StencilOp::Replace))),
    }
}

impl Vertex {
    pub fn new(pos: Vec3<f32>, norm: Vec3<f32>, col: Rgb<f32>, ao: f32, bone_idx: u8) -> Self {
        let norm_bits = if norm.x != 0.0 {
            if norm.x < 0.0 { 0 } else { 1 }
        } else if norm.y != 0.0 {
            if norm.y < 0.0 { 2 } else { 3 }
        } else {
            if norm.z < 0.0 { 4 } else { 5 }
        };
        Self {
            pos_norm: pos
                .map2(Vec3::new(0, 9, 18), |e, shift| {
                    (((e * 2.0 + 256.0) as u32) & 0x3FF) << shift
                })
                .reduce_bitor()
                | (norm_bits << 29),
            col: col
                .map2(Rgb::new(0, 8, 16), |e, shift| ((e * 255.0) as u32) << shift)
                .reduce_bitor(),
            ao_bone: (bone_idx << 2) | ((ao * 3.9999) as u8),
        }
    }

    pub fn with_bone_idx(mut self, bone_idx: u8) -> Self {
        self.ao_bone = (self.ao_bone & 0b11) | (bone_idx << 2);
        self
    }
}

impl Locals {
    pub fn new(model_mat: Mat4<f32>, col: Rgba<f32>, is_player: bool) -> Self {
        let mut flags = 0;
        flags |= is_player as u32;

        Self {
            model_mat: arr_to_mat(model_mat.into_col_array()),
            model_col: col.into_array(),
            flags,
        }
    }
}

impl Default for Locals {
    fn default() -> Self { Self::new(Mat4::identity(), Rgba::broadcast(1.0), false) }
}

impl BoneData {
    pub fn new(bone_mat: Mat4<f32>) -> Self {
        Self {
            bone_mat: arr_to_mat(bone_mat.into_col_array()),
        }
    }

    pub fn default() -> Self { Self::new(Mat4::identity()) }
}

pub struct FigurePipeline;

impl Pipeline for FigurePipeline {
    type Vertex = Vertex;
}
