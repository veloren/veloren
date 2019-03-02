// Library
use gfx::{
    self,
    // Macros
    gfx_defines,
    gfx_vertex_struct_meta,
    gfx_constant_struct_meta,
    gfx_impl_struct_meta,
    gfx_pipeline,
    gfx_pipeline_inner,
};
use vek::*;

// Local
use super::{
    Globals,
    super::{
        Pipeline,
        TgtColorFmt,
        TgtDepthFmt,
        util::arr_to_mat,
    },
};

gfx_defines! {
    vertex Vertex {
        pos: [f32; 3] = "v_pos",
        norm: [f32; 3] = "v_norm",
        col: [f32; 3] = "v_col",
        bone_idx: u8 = "v_bone_idx",
    }

    constant Locals {
        model_mat: [[f32; 4]; 4] = "model_mat",
    }

    constant BoneData {
        bone_mat: [[f32; 4]; 4] = "bone_mat",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),

        locals: gfx::ConstantBuffer<Locals> = "u_locals",
        globals: gfx::ConstantBuffer<Globals> = "u_globals",
        bones: gfx::ConstantBuffer<BoneData> = "u_bones",

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
    pub fn new(model_mat: Mat4<f32>) -> Self {
        Self {
            model_mat: arr_to_mat(model_mat.into_col_array()),
        }
    }
    pub fn default() -> Self {
        Self::new(Mat4::identity())
    }
}

impl BoneData {
    pub fn new(bone_mat: Mat4<f32>) -> Self {
        Self {
            bone_mat: arr_to_mat(bone_mat.into_col_array()),
        }
    }

    pub fn default() -> Self {
        Self::new(Mat4::identity())
    }
}

pub struct FigurePipeline;

impl Pipeline for FigurePipeline {
    type Vertex = Vertex;
}
