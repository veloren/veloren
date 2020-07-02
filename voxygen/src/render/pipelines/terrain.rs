use super::{
    super::{ColLightFmt, Pipeline, TgtColorFmt, TgtDepthStencilFmt},
    shadow, Globals, Light, Shadow,
};
use gfx::{
    self, gfx_constant_struct_meta, gfx_defines, gfx_impl_struct_meta, gfx_pipeline,
    gfx_pipeline_inner, gfx_vertex_struct_meta,
};
use vek::*;

gfx_defines! {
    vertex Vertex {
        // pos_norm: u32 = "v_pos_norm",
        // col_light: u32 = "v_col_light",
        pos_norm: u32 = "v_pos_norm",
        atlas_pos: u32 = "v_atlas_pos",
    }

    constant Locals {
        model_offs: [f32; 3] = "model_offs",
        load_time: f32 = "load_time",
        atlas_offs: [i32; 4] = "atlas_offs",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer</*shadow::Vertex*/Vertex> = (),
        // abuf: gfx::VertexBuffer<Vertex> = (),
        col_lights: gfx::TextureSampler<[f32; 4]> = "t_col_light",

        locals: gfx::ConstantBuffer<Locals> = "u_locals",
        globals: gfx::ConstantBuffer<Globals> = "u_globals",
        lights: gfx::ConstantBuffer<Light> = "u_lights",
        shadows: gfx::ConstantBuffer<Shadow> = "u_shadows",

        point_shadow_maps: gfx::TextureSampler<f32> = "t_point_shadow_maps",
        directed_shadow_maps: gfx::TextureSampler<f32> = "t_directed_shadow_maps",

        map: gfx::TextureSampler<[f32; 4]> = "t_map",
        horizon: gfx::TextureSampler<[f32; 4]> = "t_horizon",

        noise: gfx::TextureSampler<f32> = "t_noise",

        // Shadow stuff
        light_shadows: gfx::ConstantBuffer<shadow::Locals> = "u_light_shadows",

        tgt_color: gfx::RenderTarget<TgtColorFmt> = "tgt_color",
        tgt_depth_stencil: gfx::DepthTarget<TgtDepthStencilFmt> = gfx::preset::depth::LESS_EQUAL_WRITE,
        // tgt_depth_stencil: gfx::DepthStencilTarget<TgtDepthStencilFmt> = (gfx::preset::depth::LESS_EQUAL_WRITE,Stencil::new(Comparison::Always,0xff,(StencilOp::Keep,StencilOp::Keep,StencilOp::Keep))),
    }
}

impl Vertex {
    pub fn new(atlas_pos: Vec2<u16>, pos: Vec3<f32>, norm: Vec3<f32>, meta: bool) -> Self {
        const EXTRA_NEG_Z: f32 = 32768.0;

        let norm_bits = if norm.x != 0.0 {
            if norm.x < 0.0 { 0 } else { 1 }
        } else if norm.y != 0.0 {
            if norm.y < 0.0 { 2 } else { 3 }
        } else {
            if norm.z < 0.0 { 4 } else { 5 }
        };
        Self {
            pos_norm: 0
                | ((pos.x as u32) & 0x003F) << 0
                | ((pos.y as u32) & 0x003F) << 6
                | (((pos + EXTRA_NEG_Z).z.max(0.0).min((1 << 16) as f32) as u32) & 0xFFFF) << 12
                | if meta { 1 } else { 0 } << 28
                | (norm_bits & 0x7) << 29,
            atlas_pos: 0
                | ((atlas_pos.x as u32) & 0xFFFF) << 0
                | ((atlas_pos.y as u32) & 0xFFFF) << 16,
            /* col_light: 0
            | (((col.r * 255.0) as u32) & 0xFF) << 8
            | (((col.g * 255.0) as u32) & 0xFF) << 16
            | (((col.b * 255.0) as u32) & 0xFF) << 24
            | (ao >> 6) << 6
            | ((light >> 2) & 0x3F) << 0, */
        }
    }

    pub fn new_figure(
        // norm: Vec3<f32>,
        atlas_pos: Vec2<u16>,
        pos: Vec3<f32>,
        norm: Vec3<f32>,
        bone_idx: u8,
    ) -> Self {
        let norm_bits = if norm.x.min(norm.y).min(norm.z) < 0.0 {
            0
        } else {
            1
        };
        let axis_bits = if norm.x != 0.0 {
            0
        } else if norm.y != 0.0 {
            1
        } else {
            2
        };
        Self {
            pos_norm: pos
                .map2(Vec3::new(0, 9, 18), |e, shift| {
                    (((e * 2.0 + 256.0) as u32) & 0x1FF) << shift
                })
                .reduce_bitor()
                | (((bone_idx & 0xF) as u32) << 27)
                | (norm_bits << 31),
            atlas_pos: 0
                | ((atlas_pos.x as u32) & 0x7FFF) << 2
                | ((atlas_pos.y as u32) & 0x7FFF) << 17
                | axis_bits & 3,
            /* col_light: 0
            | (((col.r * 255.0) as u32) & 0xFF) << 8
            | (((col.g * 255.0) as u32) & 0xFF) << 16
            | (((col.b * 255.0) as u32) & 0xFF) << 24
            | (ao >> 6) << 6
            | ((light >> 2) & 0x3F) << 0, */
        }
    }

    /* pub fn new(
        norm_bits: u32,
        light: u32,
        ao: u32,
        pos: Vec3<f32>,
        col: Rgb<f32>,
        meta: bool,
    ) -> Self {
        const EXTRA_NEG_Z: f32 = 32768.0;

        Self {
            pos_norm: 0
                | ((pos.x as u32) & 0x003F) << 0
                | ((pos.y as u32) & 0x003F) << 6
                | (((pos + EXTRA_NEG_Z).z.max(0.0).min((1 << 16) as f32) as u32) & 0xFFFF) << 12
                | if meta { 1 } else { 0 } << 28
                | (norm_bits & 0x7) << 29,
            col_light: 0
                | ((col.r.mul(255.0) as u32) & 0xFF) << 8
                | ((col.g.mul(255.0) as u32) & 0xFF) << 16
                | ((col.b.mul(255.0) as u32) & 0xFF) << 24
                | (ao >> 6) << 6
                | ((light >> 2) & 0x3F) << 0,
        }
    } */

    pub fn make_col_light(
        light: /* u32 */ u8,
        // ao: u32,
        // col: Rgb<f32>,
        col: Rgb<u8>,
    ) -> <<ColLightFmt as gfx::format::Formatted>::Surface as gfx::format::SurfaceTyped>::DataType
    {
        [
            col.r, //.mul(255.0) as u8,
            col.g, //.mul(255.0) as u8,
            col.b, //.mul(255.0) as u8,
            light,
            /* | (ao as u8 >> 6) << 6
             * | //((light as u8 >> 2) & 0x3F) << 0,
             * | light */
        ]
    }

    pub fn with_bone_idx(self, bone_idx: u8) -> Self {
        Self {
            pos_norm: (self.pos_norm & !(0xF << 27)) | ((bone_idx as u32 & 0xF) << 27),
            ..self
        }
    }
}

impl Locals {
    pub fn default() -> Self {
        Self {
            model_offs: [0.0; 3],
            load_time: 0.0,
            atlas_offs: [0; 4],
        }
    }
}

pub struct TerrainPipeline;

impl Pipeline for TerrainPipeline {
    type Vertex = Vertex; //<<ColLightFmt as gfx::format::Formatted>::Surface as gfx::format::SurfaceTyped>::DataType;
}
