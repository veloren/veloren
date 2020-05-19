use super::{
    super::{util::arr_to_mat, Pipeline, ShadowDepthStencilFmt, TerrainLocals},
    Globals, Light, Shadow,
};
use gfx::{
    self, gfx_constant_struct_meta, gfx_defines, gfx_impl_struct_meta, gfx_pipeline,
    gfx_pipeline_inner, gfx_vertex_struct_meta,
};
use vek::*;

gfx_defines! {
    vertex Vertex {
        // pos: [f32; 4] = "v_pos",
        pos_norm: u32 = "v_pos_norm",
        // col_light: u32 = "v_col_light",
    }

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

impl Vertex {
    pub fn new(pos: Vec3<f32>, norm: Vec3<f32>) -> Self {
        let norm_bits = if norm.x != 0.0 {
            if norm.x < 0.0 { 0 } else { 1 }
        } else if norm.y != 0.0 {
            if norm.y < 0.0 { 2 } else { 3 }
        } else {
            if norm.z < 0.0 { 4 } else { 5 }
        };
        // let ao = 0xFFu32;
        // let light = 0xFFu32;
        // let col = Rgb::new(1.0f32, 0.0, 0.0);
        let meta = true;

        const EXTRA_NEG_Z: f32 = 32768.0;

        Self {
            pos_norm: 0
                | ((pos.x as u32) & 0x003F) << 0
                | ((pos.y as u32) & 0x003F) << 6
                | (((pos + EXTRA_NEG_Z).z.max(0.0).min((1 << 16) as f32) as u32) & 0xFFFF) << 12
                | if meta { 1 } else { 0 } << 28
                | (norm_bits & 0x7) << 29,
            /* col_light: 0
            | (((col.r * 255.0) as u32) & 0xFF) << 8
            | (((col.g * 255.0) as u32) & 0xFF) << 16
            | (((col.b * 255.0) as u32) & 0xFF) << 24
            | (ao >> 6) << 6
            | ((light >> 2) & 0x3F) << 0, */
        }
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
