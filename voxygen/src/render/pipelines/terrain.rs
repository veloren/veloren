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
        pos_norm: u32 = "v_pos_norm",
        atlas_pos: u32 = "v_atlas_pos",
    }

    constant Locals {
        model_offs: [f32; 3] = "model_offs",
        load_time: f32 = "load_time",
        atlas_offs: [i32; 4] = "atlas_offs",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        col_lights: gfx::TextureSampler<[f32; 4]> = "t_col_light",

        locals: gfx::ConstantBuffer<Locals> = "u_locals",
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

        tgt_color: gfx::RenderTarget<TgtColorFmt> = "tgt_color",
        tgt_depth_stencil: gfx::DepthTarget<TgtDepthStencilFmt> = gfx::preset::depth::LESS_EQUAL_WRITE,
        // tgt_depth_stencil: gfx::DepthStencilTarget<TgtDepthStencilFmt> = (gfx::preset::depth::LESS_EQUAL_WRITE,Stencil::new(Comparison::Always,0xff,(StencilOp::Keep,StencilOp::Keep,StencilOp::Keep))),
    }
}

impl Vertex {
    #[allow(clippy::identity_op)] // TODO: Pending review in #587
    /// NOTE: meta is true when the terrain vertex is touching water.
    pub fn new(atlas_pos: Vec2<u16>, pos: Vec3<f32>, norm: Vec3<f32>, meta: bool) -> Self {
        const EXTRA_NEG_Z: f32 = 32768.0;

        let norm_bits = if norm.x != 0.0 {
            if norm.x < 0.0 { 0 } else { 1 }
        } else if norm.y != 0.0 {
            if norm.y < 0.0 { 2 } else { 3 }
        } else if norm.z < 0.0 {
            4
        } else {
            5
        };
        Self {
            pos_norm: ((pos.x as u32) & 0x003F) << 0
                | ((pos.y as u32) & 0x003F) << 6
                | (((pos + EXTRA_NEG_Z).z.max(0.0).min((1 << 16) as f32) as u32) & 0xFFFF) << 12
                | if meta { 1 } else { 0 } << 28
                | (norm_bits & 0x7) << 29,
            atlas_pos: ((atlas_pos.x as u32) & 0xFFFF) << 0 | ((atlas_pos.y as u32) & 0xFFFF) << 16,
        }
    }

    pub fn new_figure(atlas_pos: Vec2<u16>, pos: Vec3<f32>, norm: Vec3<f32>, bone_idx: u8) -> Self {
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
            atlas_pos: ((atlas_pos.x as u32) & 0x7FFF) << 2
                | ((atlas_pos.y as u32) & 0x7FFF) << 17
                | axis_bits & 3,
        }
    }

    pub fn make_col_light(
        // 0 to 31
        light: u8,
        // 0 to 31
        glow: u8,
        col: Rgb<u8>,
    ) -> <<ColLightFmt as gfx::format::Formatted>::Surface as gfx::format::SurfaceTyped>::DataType
    {
        //[col.r, col.g, col.b, light]
        // It would be nice for this to be cleaner, but we want to squeeze 5 fields into
        // 4. We can do this because both `light` and `glow` go from 0 to 31,
        // meaning that they can both fit into 5 bits. If we steal a bit from
        // red and blue each (not green, human eyes are more sensitive to
        // changes in green) then we get just enough to expand the nibbles of
        // the alpha field enough to fit both `light` and `glow`.
        //
        // However, we now have a problem. In the shader code with use hardware
        // filtering to get at the `light` and `glow` attributes (but not
        // colour, that remains constant across a block). How do we resolve this
        // if we're twiddling bits? The answer is to very carefully manipulate
        // the bit pattern such that the fields we want to filter (`light` and
        // `glow`) always sit as the higher bits of the fields. Then, we can do
        // some modulation magic to extract them from the filtering unharmed and use
        // unfiltered texture access (i.e: `texelFetch`) to access the colours, plus a
        // little bit-fiddling.
        //
        // TODO: This isn't currently working (no idea why). See `srgb.glsl` for current
        // impl that intead does manual bit-twiddling and filtering.
        [
            (light.min(31) << 3) | ((col.r >> 1) & 0b111),
            (glow.min(31) << 3) | ((col.b >> 1) & 0b111),
            (col.r & 0b11110000) | (col.b >> 4),
            col.g, // Green is lucky, it remains unscathed
        ]
    }

    pub fn make_col_light_figure(
        // 0 to 31
        light: u8,
        glowy: bool,
        shiny: bool,
        col: Rgb<u8>,
    ) -> <<ColLightFmt as gfx::format::Formatted>::Surface as gfx::format::SurfaceTyped>::DataType
    {
        let attr = 0 | ((glowy as u8) << 0) | ((shiny as u8) << 1);
        [
            (light.min(31) << 3) | ((col.r >> 1) & 0b111),
            (attr.min(31) << 3) | ((col.b >> 1) & 0b111),
            (col.r & 0b11110000) | (col.b >> 4),
            col.g, // Green is lucky, it remains unscathed
        ]
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
    type Vertex = Vertex;
}
