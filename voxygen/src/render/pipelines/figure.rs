use super::{
    super::{util::arr_to_mat, Pipeline, TgtColorFmt, TgtDepthFmt},
    Globals, Light, Shadow,
};
use gfx::{
    self, gfx_constant_struct_meta, gfx_defines, gfx_impl_struct_meta, gfx_pipeline,
    gfx_pipeline_inner, gfx_vertex_struct_meta,
};
use vek::*;

gfx_defines! {
    vertex Vertex {
        pos: [f32; 3] = "v_pos",
        norm: [f32; 3] = "v_norm",
        col: [f32; 3] = "v_col",
        bone_idx: u8 = "v_bone_idx",
    }

    constant Locals {
        model_mat: [[f32; 4]; 4] = "model_mat",
        model_col: [f32; 4] = "model_col",
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

        map: gfx::TextureSampler<[f32; 4]> = "t_map",
        horizon: gfx::TextureSampler<[f32; 4]> = "t_horizon",

        noise: gfx::TextureSampler<f32> = "t_noise",

        tgt_color: gfx::RenderTarget<TgtColorFmt> = "tgt_color",
        tgt_depth: gfx::DepthTarget<TgtDepthFmt> = gfx::preset::depth::LESS_EQUAL_WRITE,
    }
}

impl Vertex {
    pub fn new(pos: Vec3<f32>, norm: Vec3<f32>, col: Rgb<f32>, bone_idx: u8) -> Self {
        Self {
            pos: pos.into_array(),
            col: col.into_array(),
            norm: norm.into_array(),
            bone_idx,
        }
    }

    pub fn with_bone_idx(mut self, bone_idx: u8) -> Self {
        self.bone_idx = bone_idx;
        self
    }
}

impl Locals {
    pub fn new(model_mat: Mat4<f32>, col: Rgba<f32>) -> Self {
        Self {
            model_mat: arr_to_mat(model_mat.into_col_array()),
            model_col: col.into_array(),
        }
    }
}

impl Default for Locals {
    fn default() -> Self { Self::new(Mat4::identity(), Rgba::broadcast(1.0)) }
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
