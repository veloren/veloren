use super::super::{Renderer, Texture};
use vek::*;
use zerocopy::AsBytes;

// gfx_defines! {
//     vertex Vertex {
//         pos: [f32; 2] = "v_pos",
//     }

//     constant Locals {
//         nul: [f32; 4] = "nul",
//     }

//     pipeline pipe {
//         vbuf: gfx::VertexBuffer<Vertex> = (),

//         locals: gfx::ConstantBuffer<Locals> = "u_locals",
//         globals: gfx::ConstantBuffer<Globals> = "u_globals",
//         map: gfx::TextureSampler<[f32; 4]> = "t_map",
//         alt: gfx::TextureSampler<[f32; 2]> = "t_alt",
//         horizon: gfx::TextureSampler<[f32; 4]> = "t_horizon",

//         noise: gfx::TextureSampler<f32> = "t_noise",

//         tgt_color: gfx::RenderTarget<TgtColorFmt> = "tgt_color",
//         tgt_depth_stencil: gfx::DepthTarget<TgtDepthStencilFmt> =
// gfx::preset::depth::LESS_EQUAL_WRITE,         // tgt_depth_stencil:
// gfx::DepthStencilTarget<TgtDepthStencilFmt> =
// (gfx::preset::depth::LESS_EQUAL_WRITE,Stencil::new(Comparison::Always,0xff,
// (StencilOp::Keep,StencilOp::Keep,StencilOp::Keep))),     }
// }

#[repr(C)]
#[derive(Copy, Clone, Debug, AsBytes)]
pub struct Vertex {
    pos: [f32; 2],
}

impl Vertex {
    pub fn new(pos: Vec2<f32>) -> Self {
        Self {
            pos: pos.into_array(),
        }
    }
}

pub struct LodData {
    pub map: Texture,
    pub alt: Texture,
    pub horizon: Texture,
    pub tgt_detail: u32,
}

impl LodData {
    pub fn new(
        renderer: &mut Renderer,
        map_size: Vec2<u16>,
        lod_base: &[u32],
        lod_alt: &[u32],
        lod_horizon: &[u32],
        tgt_detail: u32,
        //border_color: gfx::texture::PackedColor,
    ) -> Self {
        let mut texture_info = wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: map_size.x,
                height: map_size.y,
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

        let map = renderer.create_texture_with_data_raw(
            &texture_info,
            &sampler_info,
            map_size.x * 4,
            [map_size.x, map_size.y],
            lod_base.as_bytes(),
        );
        texture_info = wgpu::TextureFormat::Rg16Uint;
        let alt = renderer.create_texture_with_data_raw(
            &texture_info,
            &sampler_info,
            map_size.x * 4,
            [map_size.x, map_size.y],
            lod_base.as_bytes(),
        );
        texture_info = wgpu::TextureFormat::Rgba8Unorm;
        let horizon = renderer.create_texture_with_data_raw(
            &texture_info,
            &sampler_info,
            map_size.x * 4,
            [map_size.x, map_size.y],
            lod_base.as_bytes(),
        );

        Self {
            map,
            alt,
            horizon,
            tgt_detail,
        }

        // Self {
        //     map: renderer
        //         .create_texture_immutable_raw(
        //             kind,
        //             gfx::texture::Mipmap::Provided,
        //             &[gfx::memory::cast_slice(lod_base)],
        //             SamplerInfo {
        //                 border: border_color,
        //                 ..info
        //             },
        //         )
        //         .expect("Failed to generate map texture"),
        //     alt: renderer
        //         .create_texture_immutable_raw(
        //             kind,
        //             gfx::texture::Mipmap::Provided,
        //             &[gfx::memory::cast_slice(lod_alt)],
        //             SamplerInfo {
        //                 border: [0.0, 0.0, 0.0, 0.0].into(),
        //                 ..info
        //             },
        //         )
        //         .expect("Failed to generate alt texture"),
        //     horizon: renderer
        //         .create_texture_immutable_raw(
        //             kind,
        //             gfx::texture::Mipmap::Provided,
        //             &[gfx::memory::cast_slice(lod_horizon)],
        //             SamplerInfo {
        //                 border: [1.0, 0.0, 1.0, 0.0].into(),
        //                 ..info
        //             },
        //         )
        //         .expect("Failed to generate horizon texture"),
        //     tgt_detail,
        // }
    }
}
