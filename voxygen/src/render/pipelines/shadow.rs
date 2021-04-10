use super::{
    super::{
        ColLightFmt, ColLightInfo, Pipeline, RenderError, Renderer, ShadowDepthStencilFmt,
        TerrainLocals, Texture,
    },
    figure, terrain, Globals,
};
use gfx::{
    self, gfx_constant_struct_meta, gfx_defines, gfx_impl_struct_meta, gfx_pipeline,
    gfx_pipeline_inner,
};
use vek::*;

gfx_defines! {
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
        (col_lights, col_lights_size): &ColLightInfo,
    ) -> Result<Texture<ColLightFmt>, RenderError> {
        renderer.create_texture_immutable_raw(
            gfx::texture::Kind::D2(
                col_lights_size.x,
                col_lights_size.y,
                gfx::texture::AaMode::Single,
            ),
            gfx::texture::Mipmap::Provided,
            &[col_lights],
            gfx::texture::SamplerInfo::new(
                gfx::texture::FilterMethod::Bilinear,
                gfx::texture::WrapMode::Clamp,
            ),
        )
    }
}

impl Pipeline for ShadowPipeline {
    type Vertex = terrain::Vertex;
}
