use super::{
    super::{
        ColLightFmt, ColLightInfo, Pipeline, RenderError, Renderer, ShadowDepthStencilFmt,
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
        pos_norm: u32 = "v_pos_norm",
    }

    constant Locals {
        shadow_matrices: [[f32; 4]; 4] = "shadowMatrices",
        texture_mats: [[f32; 4]; 4] = "texture_mat",
    }

    pipeline pipe {
        // Terrain vertex stuff
        vbuf: gfx::VertexBuffer<terrain::Vertex> = (),

        locals: gfx::ConstantBuffer<TerrainLocals> = "u_locals",
        globals: gfx::ConstantBuffer<Globals> = "u_globals",

        // Shadow stuff
        light_shadows: gfx::ConstantBuffer<Locals> = "u_light_shadows",

        tgt_depth_stencil: gfx::DepthTarget<ShadowDepthStencilFmt> = gfx::state::Depth {
            fun: gfx::state::Comparison::Less,
            write: true,
        },
    }

    pipeline figure_pipe {
        // Terrain vertex stuff
        vbuf: gfx::VertexBuffer<terrain::Vertex> = (),

        locals: gfx::ConstantBuffer<figure::Locals> = "u_locals",
        bones: gfx::ConstantBuffer<figure::BoneData> = "u_bones",
        globals: gfx::ConstantBuffer<Globals> = "u_globals",

        // Shadow stuff
        light_shadows: gfx::ConstantBuffer<Locals> = "u_light_shadows",

        tgt_depth_stencil: gfx::DepthTarget<ShadowDepthStencilFmt> = gfx::state::Depth {
            fun: gfx::state::Comparison::Less,
            write: true,
        },
    }
}

impl Vertex {
    pub fn new(pos: Vec3<f32>, norm: Vec3<f32>, meta: bool) -> Self {
        let norm_bits = if norm.x != 0.0 {
            if norm.x < 0.0 { 0 } else { 1 }
        } else if norm.y != 0.0 {
            if norm.y < 0.0 { 2 } else { 3 }
        } else if norm.z < 0.0 {
            4
        } else {
            5
        };

        const EXTRA_NEG_Z: f32 = 32768.0;

        Self {
            pos_norm: 0
                | ((pos.x as u32) & 0x003F) << 0
                | ((pos.y as u32) & 0x003F) << 6
                | (((pos + EXTRA_NEG_Z).z.max(0.0).min((1 << 16) as f32) as u32) & 0xFFFF) << 12
                | if meta { 1 } else { 0 } << 28
                | (norm_bits & 0x7) << 29,
        }
    }

    pub fn new_figure(pos: Vec3<f32>, norm: Vec3<f32>, bone_idx: u8) -> Self {
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
            &[&col_lights],
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
