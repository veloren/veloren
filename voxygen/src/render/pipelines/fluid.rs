use super::super::TerrainLocals;
use vek::*;
use zerocopy::AsBytes;

#[repr(C)]
#[derive(Copy, Clone, Debug, AsBytes)]
struct Vertex {
    pos_norm: u32,
}

// gfx_defines! {
//     vertex Vertex {
//         pos_norm: u32 = "v_pos_norm",
//     }

//     pipeline pipe {
//         vbuf: gfx::VertexBuffer<Vertex> = (),

//         locals: gfx::ConstantBuffer<TerrainLocals> = "u_locals",
//         globals: gfx::ConstantBuffer<Globals> = "u_globals",
//         lights: gfx::ConstantBuffer<Light> = "u_lights",
//         shadows: gfx::ConstantBuffer<Shadow> = "u_shadows",

//         point_shadow_maps: gfx::TextureSampler<f32> = "t_point_shadow_maps",
//         directed_shadow_maps: gfx::TextureSampler<f32> =
// "t_directed_shadow_maps",

//         alt: gfx::TextureSampler<[f32; 2]> = "t_alt",
//         horizon: gfx::TextureSampler<[f32; 4]> = "t_horizon",

//         noise: gfx::TextureSampler<f32> = "t_noise",
//         waves: gfx::TextureSampler<[f32; 4]> = "t_waves",

//         // Shadow stuff
//         light_shadows: gfx::ConstantBuffer<shadow::Locals> =
// "u_light_shadows",

//         tgt_color: gfx::BlendTarget<TgtColorFmt> = ("tgt_color",
// ColorMask::all(), gfx::preset::blend::ALPHA),         tgt_depth_stencil:
// gfx::DepthTarget<TgtDepthStencilFmt> = gfx::preset::depth::LESS_EQUAL_TEST,
//         // tgt_depth_stencil: gfx::DepthStencilTarget<TgtDepthStencilFmt> =
// (gfx::preset::depth::LESS_EQUAL_TEST,Stencil::new(Comparison::Always,0xff,
// (StencilOp::Keep,StencilOp::Keep,StencilOp::Keep))),     }
// }

impl Vertex {
    #[allow(clippy::identity_op)] // TODO: Pending review in #587
    #[allow(clippy::into_iter_on_ref)] // TODO: Pending review in #587
    pub fn new(pos: Vec3<f32>, norm: Vec3<f32>) -> Self {
        let (norm_axis, norm_dir) = norm
            .as_slice()
            .into_iter()
            .enumerate()
            .find(|(_i, e)| **e != 0.0)
            .unwrap_or((0, &1.0));
        let norm_bits = ((norm_axis << 1) | if *norm_dir > 0.0 { 1 } else { 0 }) as u32;

        const EXTRA_NEG_Z: f32 = 65536.0;

        Self {
            pos_norm: 0
                | ((pos.x as u32) & 0x003F) << 0
                | ((pos.y as u32) & 0x003F) << 6
                | (((pos.z + EXTRA_NEG_Z).max(0.0).min((1 << 17) as f32) as u32) & 0x1FFFF) << 12
                | (norm_bits & 0x7) << 29,
        }
    }
}
