use super::{
    super::{
        ColLightFmt, ColLightInfo, Light, Pipeline, RenderError, Renderer, ShadowDepthStencilFmt,
        TerrainLocals, Texture,
    },
    figure, terrain, Globals,
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
        // atlas_pos: u32 = "v_atlas_pos",
    }

    constant Locals {
        shadow_matrices: [[f32; 4]; 4] = "shadowMatrices",
        texture_mats: [[f32; 4]; 4] = "texture_mat",
    }

    pipeline pipe {
        // Terrain vertex stuff
        vbuf: gfx::VertexBuffer</*Vertex*/terrain::Vertex> = (),

        locals: gfx::ConstantBuffer<TerrainLocals> = "u_locals",
        globals: gfx::ConstantBuffer<Globals> = "u_globals",
        lights: gfx::ConstantBuffer<Light> = "u_lights",
        // shadows: gfx::ConstantBuffer<Shadow> = "u_shadows",

        // map: gfx::TextureSampler<[f32; 4]> = "t_map",
        // horizon: gfx::TextureSampler<[f32; 4]> = "t_horizon",

        // noise: gfx::TextureSampler<f32> = "t_noise",

        // Shadow stuff
        light_shadows: gfx::ConstantBuffer<Locals> = "u_light_shadows",

        tgt_depth_stencil: gfx::DepthTarget<ShadowDepthStencilFmt> = gfx::state::Depth {
            fun: gfx::state::Comparison::LessEqual,
            write: true,
        },
        // tgt_depth_stencil: gfx::DepthTarget<ShadowDepthStencilFmt> = gfx::preset::depth::LESS_EQUAL_WRITE,//,Stencil::new(Comparison::Always,0xff,(StencilOp::Keep,StencilOp::Keep,StencilOp::Keep))),
    }

    pipeline figure_pipe {
        // Terrain vertex stuff
        vbuf: gfx::VertexBuffer</*Vertex*/terrain::Vertex> = (),

        locals: gfx::ConstantBuffer<figure::Locals> = "u_locals",
        bones: gfx::ConstantBuffer<figure::BoneData> = "u_bones",
        globals: gfx::ConstantBuffer<Globals> = "u_globals",
        // lights: gfx::ConstantBuffer<Light> = "u_lights",
        // shadows: gfx::ConstantBuffer<Shadow> = "u_shadows",

        // map: gfx::TextureSampler<[f32; 4]> = "t_map",
        // horizon: gfx::TextureSampler<[f32; 4]> = "t_horizon",

        // noise: gfx::TextureSampler<f32> = "t_noise",

        // Shadow stuff
        light_shadows: gfx::ConstantBuffer<Locals> = "u_light_shadows",

        tgt_depth_stencil: gfx::DepthTarget<ShadowDepthStencilFmt> = gfx::state::Depth {
            fun: gfx::state::Comparison::LessEqual,
            write: true,
        },
        // tgt_depth_stencil: gfx::DepthTarget<ShadowDepthStencilFmt> = gfx::preset::depth::LESS_WRITE,//,Stencil::new(Comparison::Always,0xff,(StencilOp::Keep,StencilOp::Keep,StencilOp::Keep))),
    }
}

impl Vertex {
    pub fn new(
        pos: Vec3<f32>,
        norm: Vec3<f32>,
        meta: bool, /* , atlas_pos: Vec2<u16> */
    ) -> Self {
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
        // let meta = true;

        const EXTRA_NEG_Z: f32 = 32768.0;

        Self {
            pos_norm: 0
                | ((pos.x as u32) & 0x003F) << 0
                | ((pos.y as u32) & 0x003F) << 6
                | (((pos + EXTRA_NEG_Z).z.max(0.0).min((1 << 16) as f32) as u32) & 0xFFFF) << 12
                | if meta { 1 } else { 0 } << 28
                | (norm_bits & 0x7) << 29,
            /* atlas_pos: 0
                | ((atlas_pos.x as u32) & 0xFFFF) << 0
                | ((atlas_pos.y as u32) & 0xFFFF) << 16, */
            /* col_light: 0
            | (((col.r * 255.0) as u32) & 0xFF) << 8
            | (((col.g * 255.0) as u32) & 0xFF) << 16
            | (((col.b * 255.0) as u32) & 0xFF) << 24
            | (ao >> 6) << 6
            | ((light >> 2) & 0x3F) << 0, */
        }
    }

    pub fn new_figure(
        pos: Vec3<f32>,
        norm: Vec3<f32>,
        /* col: Rgb<f32>, ao: f32, */ bone_idx: u8,
    ) -> Self {
        let norm_bits = if norm.x.min(norm.y).min(norm.z) < 0.0 {
            0
        } else {
            1
        };
        Self {
            pos_norm: pos
                .map2(Vec3::new(0, 9, 18), |e, shift| {
                    (((e * 2.0 + 256.0) as u32) & 0x1FF) << shift
                })
                .reduce_bitor()
                | (((bone_idx & 0xF) as u32) << 27)
                | (norm_bits << 31),
            // col: col
            //     .map2(Rgb::new(0, 8, 16), |e, shift| ((e * 255.0) as u32) << shift)
            //     .reduce_bitor(),
            // ao_bone: (bone_idx << 2) | ((ao * 3.9999) as u8),
        }
    }

    pub fn with_bone_idx(self, bone_idx: u8) -> Self {
        Self {
            pos_norm: (self.pos_norm & !(0xF << 27)) | ((bone_idx as u32 & 0xF) << 27),
        }
    }
}

impl Locals {
    pub fn new(shadow_mat: Mat4<f32>, texture_mat: Mat4<f32>) -> Self {
        Self {
            shadow_matrices: shadow_mat.into_col_arrays(),
            texture_mats: texture_mat.into_col_arrays(),
        }
    }

    pub fn default() -> Self { Self::new(Mat4::identity(), Mat4::identity()) }
}

pub struct ShadowPipeline;

impl ShadowPipeline {
    pub fn create_col_lights(
        renderer: &mut Renderer,
        (col_lights, col_lights_size): ColLightInfo,
    ) -> Result<Texture<ColLightFmt>, RenderError> {
        renderer.create_texture_immutable_raw(
            gfx::texture::Kind::D2(
                col_lights_size.x,
                col_lights_size.y,
                gfx::texture::AaMode::Single,
            ),
            gfx::texture::Mipmap::Provided,
            &[&col_lights /* .raw_pixels() */],
            gfx::texture::SamplerInfo::new(
                gfx::texture::FilterMethod::Bilinear,
                gfx::texture::WrapMode::Clamp,
            ),
        )
    }
}

impl Pipeline for ShadowPipeline {
    type Vertex = Vertex;
}
