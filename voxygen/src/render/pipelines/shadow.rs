use super::super::{ColLightInfo, Renderer, Texture};
use vek::*;
use zerocopy::AsBytes;

// gfx_defines! {
//     constant Locals {
//         shadow_matrices: [[f32; 4]; 4] = "shadowMatrices",
//         texture_mats: [[f32; 4]; 4] = "texture_mat",
//     }

//     pipeline pipe {
//         // Terrain vertex stuff
//         vbuf: gfx::VertexBuffer<terrain::Vertex> = (),

//         locals: gfx::ConstantBuffer<TerrainLocals> = "u_locals",
//         globals: gfx::ConstantBuffer<Globals> = "u_globals",

//         // Shadow stuff
//         light_shadows: gfx::ConstantBuffer<Locals> = "u_light_shadows",

//         tgt_depth_stencil: gfx::DepthTarget<ShadowDepthStencilFmt> =
// gfx::state::Depth {             fun: gfx::state::Comparison::Less,
//             write: true,
//         },
//     }

//     pipeline figure_pipe {
//         // Terrain vertex stuff
//         vbuf: gfx::VertexBuffer<terrain::Vertex> = (),

//         locals: gfx::ConstantBuffer<figure::Locals> = "u_locals",
//         bones: gfx::ConstantBuffer<figure::BoneData> = "u_bones",
//         globals: gfx::ConstantBuffer<Globals> = "u_globals",

//         // Shadow stuff
//         light_shadows: gfx::ConstantBuffer<Locals> = "u_light_shadows",

//         tgt_depth_stencil: gfx::DepthTarget<ShadowDepthStencilFmt> =
// gfx::state::Depth {             fun: gfx::state::Comparison::Less,
//             write: true,
//         },
//     }
// }

#[repr(C)]
#[derive(Copy, Clone, Debug, AsBytes)]
pub struct Locals {
    shadow_matrices: [[f32; 4]; 4],
    texture_mats: [[f32; 4]; 4],
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

pub fn create_col_lights(
    renderer: &mut Renderer,
    (col_lights, col_lights_size): &ColLightInfo,
) -> Texture {
    let mut texture_info = wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: col_lights_size.x,
            height: col_lights_size.y,
            depth: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
    };

    let sampler_info = wgpu::SamplerDescriptor {
        label: None,
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        border_color: Some(wgpu::SamplerBorderColor::TransparentBlack),
        ..Default::default()
    };

    renderer.create_texture_with_data_raw(
        &texture_info,
        &sampler_info,
        col_lights_size.x * 4,
        [col_lights_size.x, col_lights_size.y],
        col_lights.as_bytes(),
    )
}
