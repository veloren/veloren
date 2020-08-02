use super::{
    super::{Pipeline, TgtColorFmt, TgtDepthStencilFmt},
    shadow, terrain, Globals, Light, Shadow,
};
use core::fmt;
use gfx::{
    self, gfx_constant_struct_meta, gfx_defines, gfx_impl_struct_meta, gfx_pipeline,
    gfx_pipeline_inner, gfx_vertex_struct_meta, state::ColorMask,
};
use vek::*;

gfx_defines! {
    vertex Vertex {
        pos: [f32; 3] = "v_pos",
        // pos_norm: u32 = "v_pos_norm",
        // Because we try to restrict terrain sprite data to a 128×128 block
        // we need an offset into the texture atlas.
        atlas_pos: u32 = "v_atlas_pos",
        // ____BBBBBBBBGGGGGGGGRRRRRRRR
        // col: u32 = "v_col",
        // ...AANNN
        // A = AO
        // N = Normal
        norm_ao: u32 = "v_norm_ao",
    }

    constant Locals {
        // Each matrix performs rotatation, translation, and scaling, relative to the sprite
        // origin, for all sprite instances.  The matrix will be in an array indexed by the
        // sprite instance's orientation (0 through 7).
        mat: [[f32; 4]; 4]  = "mat",
        wind_sway: [f32; 4] = "wind_sway",
        offs: [f32; 4] = "offs",
    }

    vertex/*constant*/ Instance {
        // Terrain block position and orientation
        pos_ori: u32 = "inst_pos_ori",
        inst_mat0: [f32; 4] = "inst_mat0",
        inst_mat1: [f32; 4] = "inst_mat1",
        inst_mat2: [f32; 4] = "inst_mat2",
        inst_mat3: [f32; 4] = "inst_mat3",
        // inst_mat: [[f32; 4]; 4] = "inst_mat",
        // inst_col: [f32; 3] = "inst_col",
        inst_wind_sway: f32 = "inst_wind_sway",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        ibuf: gfx::InstanceBuffer<Instance> = (),
        // ibuf: gfx::/*handle::RawBuffer*/ConstantBuffer<Instance> = "u_ibuf",
        col_lights: gfx::TextureSampler<[f32; 4]> = "t_col_light",

        locals: gfx::ConstantBuffer<Locals> = "u_locals",
        // A sprite instance is a cross between a sprite and a terrain chunk.
        terrain_locals: gfx::ConstantBuffer<terrain::Locals> = "u_terrain_locals",
        // locals: gfx::ConstantBuffer<terrain::Locals> = "u_locals",
        globals: gfx::ConstantBuffer<Globals> = "u_globals",
        lights: gfx::ConstantBuffer<Light> = "u_lights",
        shadows: gfx::ConstantBuffer<Shadow> = "u_shadows",

        point_shadow_maps: gfx::TextureSampler<f32> = "t_point_shadow_maps",
        directed_shadow_maps: gfx::TextureSampler<f32> = "t_directed_shadow_maps",

        alt: gfx::TextureSampler<[f32; 2]> = "t_alt",
        horizon: gfx::TextureSampler<[f32; 4]> = "t_horizon",

        noise: gfx::TextureSampler<f32> = "t_noise",

        // Shadow stuff
        light_shadows: gfx::ConstantBuffer<shadow::Locals> = "u_light_shadows",

        tgt_color: gfx::BlendTarget<TgtColorFmt> = ("tgt_color", ColorMask::all(), gfx::preset::blend::ALPHA),
        tgt_depth_stencil: gfx::DepthTarget<TgtDepthStencilFmt> = gfx::preset::depth::LESS_EQUAL_WRITE,
        // tgt_depth_stencil: gfx::DepthStencilTarget<TgtDepthStencilFmt> = (gfx::preset::depth::LESS_EQUAL_WRITE,Stencil::new(Comparison::Always,0xff,(StencilOp::Keep,StencilOp::Keep,StencilOp::Keep))),
    }
}

impl fmt::Display for Vertex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Vertex")
            .field("pos", &Vec3::<f32>::from(self.pos))
            .field(
                "atlas_pos",
                &Vec2::new(self.atlas_pos & 0xFFFF, (self.atlas_pos >> 16) & 0xFFFF),
            )
            .field("norm_ao", &self.norm_ao)
            .finish()
    }
}

impl Vertex {
    // NOTE: Limit to 16 (x) × 16 (y) × 32 (z).
    #[allow(clippy::collapsible_if)]
    pub fn new(
        atlas_pos: Vec2<u16>,
        pos: Vec3<f32>,
        norm: Vec3<f32>, /* , col: Rgb<f32>, ao: f32 */
    ) -> Self {
        let norm_bits = if norm.x != 0.0 {
            if norm.x < 0.0 { 0 } else { 1 }
        } else if norm.y != 0.0 {
            if norm.y < 0.0 { 2 } else { 3 }
        } else {
            if norm.z < 0.0 { 4 } else { 5 }
        };

        Self {
            // pos_norm: 0
            //     | ((pos.x as u32) & 0x003F) << 0
            //     | ((pos.y as u32) & 0x003F) << 6
            //     | (((pos + EXTRA_NEG_Z).z.max(0.0).min((1 << 16) as f32) as u32) & 0xFFFF) << 12
            //     | if meta { 1 } else { 0 } << 28
            //     | (norm_bits & 0x7) << 29,
            pos: pos.into_array(),
            /* col: col
            .map2(Rgb::new(0, 8, 16), |e, shift| ((e * 255.0) as u32) << shift)
            .reduce_bitor(), */
            atlas_pos: 0
                | ((atlas_pos.x as u32) & 0xFFFF) << 0
                | ((atlas_pos.y as u32) & 0xFFFF) << 16, /* | axis_bits & 3 */
            norm_ao: norm_bits, /* | (((ao * 3.9999) as u32) << 3) */
        }
    }
}

impl Instance {
    pub fn new(
        mat: Mat4<f32>,
        /* col: Rgb<f32>, */ wind_sway: f32,
        pos: Vec3<i32>,
        ori_bits: u8,
    ) -> Self {
        const EXTRA_NEG_Z: i32 = 32768;

        let mat_arr = mat.into_col_arrays();
        Self {
            pos_ori: 0
                | ((pos.x as u32) & 0x003F) << 0
                | ((pos.y as u32) & 0x003F) << 6
                | (((pos + EXTRA_NEG_Z).z.max(0).min(1 << 16/* as f32*/) as u32) & 0xFFFF) << 12
                // | if meta { 1 } else { 0 } << 28
                | (u32::from(ori_bits) & 0x7) << 29,
            inst_mat0: mat_arr[0],
            inst_mat1: mat_arr[1],
            inst_mat2: mat_arr[2],
            inst_mat3: mat_arr[3],
            // inst_mat: mat_arr,
            // inst_col: col.into_array(),
            inst_wind_sway: wind_sway,
        }
    }
}

impl Default for Instance {
    fn default() -> Self {
        Self::new(
            Mat4::identity(),
            /* Rgb::broadcast(1.0), */ 0.0,
            Vec3::zero(),
            0,
        )
    }
}

impl Default for Locals {
    fn default() -> Self { Self::new(Mat4::identity(), Vec3::one(), Vec3::zero(), 0.0) }
}

impl Locals {
    pub fn new(mat: Mat4<f32>, scale: Vec3<f32>, offs: Vec3<f32>, wind_sway: f32) -> Self {
        Self {
            mat: mat.into_col_arrays(),
            wind_sway: [scale.x, scale.y, scale.z, wind_sway],
            offs: [offs.x, offs.y, offs.z, 0.0],
        }
    }
}

pub struct SpritePipeline;

impl Pipeline for SpritePipeline {
    type Vertex = Vertex;
}
