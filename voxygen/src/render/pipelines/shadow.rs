use super::{
    super::{util::arr_to_mat, Pipeline, ShadowDepthStencilFmt, TerrainLocals},
    terrain::Vertex,
    Globals, Light, Shadow,
};
use gfx::{
    self, gfx_constant_struct_meta, gfx_defines, gfx_impl_struct_meta, gfx_pipeline,
    gfx_pipeline_inner,
};
use vek::*;

gfx_defines! {
    constant Locals {
        shadow_matrices: [[f32; 4]; 4] = "shadowMatrices",
    }

    pipeline pipe {
        // Terrain vertex stuff
        vbuf: gfx::VertexBuffer<Vertex> = (),

        locals: gfx::ConstantBuffer<TerrainLocals> = "u_locals",
        globals: gfx::ConstantBuffer<Globals> = "u_globals",
        lights: gfx::ConstantBuffer<Light> = "u_lights",
        shadows: gfx::ConstantBuffer<Shadow> = "u_shadows",

        map: gfx::TextureSampler<[f32; 4]> = "t_map",
        horizon: gfx::TextureSampler<[f32; 4]> = "t_horizon",

        noise: gfx::TextureSampler<f32> = "t_noise",

        // Shadow stuff
        light_shadows: gfx::ConstantBuffer<Locals> = "u_light_shadows",

        tgt_depth_stencil: gfx::DepthTarget<ShadowDepthStencilFmt> = gfx::preset::depth::LESS_EQUAL_WRITE,//,Stencil::new(Comparison::Always,0xff,(StencilOp::Keep,StencilOp::Keep,StencilOp::Keep))),
    }
}

impl Locals {
    pub fn new(shadow_mat: Mat4<f32>) -> Self {
        Self {
            shadow_matrices: arr_to_mat(shadow_mat.into_col_array()),
        }
    }

    pub fn default() -> Self { Self::new(Mat4::identity()) }
}

pub struct ShadowPipeline;

impl Pipeline for ShadowPipeline {
    type Vertex = Vertex;
}
