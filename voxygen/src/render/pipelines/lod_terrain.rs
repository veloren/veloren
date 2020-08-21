use super::{
    super::{
        LodAltFmt, LodColorFmt, LodTextureFmt, Pipeline, Renderer, Texture, TgtColorFmt,
        TgtDepthStencilFmt,
    },
    Globals,
};
use vek::*;
use zerocopy::AsBytes;

#[repr(C)]
#[derive(Copy, Clone, Debug, AsBytes)]
struct Vertex {
    pos: [f32; 2],
}

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

impl Vertex {
    pub fn new(pos: Vec2<f32>) -> Self {
        Self {
            pos: pos.into_array(),
        }
    }
}

pub struct LodData {
    pub map: Texture<LodColorFmt>,
    pub alt: Texture<LodAltFmt>,
    pub horizon: Texture<LodTextureFmt>,
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
        border_color: gfx::texture::PackedColor,
    ) -> Self {
        let kind = gfx::texture::Kind::D2(map_size.x, map_size.y, gfx::texture::AaMode::Single);
        let info = gfx::texture::SamplerInfo::new(
            gfx::texture::FilterMethod::Bilinear,
            gfx::texture::WrapMode::Border,
        );
        Self {
            map: renderer
                .create_texture_immutable_raw(
                    kind,
                    gfx::texture::Mipmap::Provided,
                    &[gfx::memory::cast_slice(lod_base)],
                    SamplerInfo {
                        border: border_color,
                        ..info
                    },
                )
                .expect("Failed to generate map texture"),
            alt: renderer
                .create_texture_immutable_raw(
                    kind,
                    gfx::texture::Mipmap::Provided,
                    &[gfx::memory::cast_slice(lod_alt)],
                    SamplerInfo {
                        border: [0.0, 0.0, 0.0, 0.0].into(),
                        ..info
                    },
                )
                .expect("Failed to generate alt texture"),
            horizon: renderer
                .create_texture_immutable_raw(
                    kind,
                    gfx::texture::Mipmap::Provided,
                    &[gfx::memory::cast_slice(lod_horizon)],
                    SamplerInfo {
                        border: [1.0, 0.0, 1.0, 0.0].into(),
                        ..info
                    },
                )
                .expect("Failed to generate horizon texture"),
            tgt_detail,
        }
    }
}
