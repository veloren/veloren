use super::super::{Mesh, Tri};
use vek::*;
use zerocopy::AsBytes;

// gfx_defines! {
//     vertex Vertex {
//         pos: [f32; 2] = "v_pos",
//     }
//
//     constant Locals {
//         proj_mat_inv: [[f32; 4]; 4] = "proj_mat_inv",
//         view_mat_inv: [[f32; 4]; 4] = "view_mat_inv",
//     }
//
//     pipeline pipe {
//         vbuf: gfx::VertexBuffer<Vertex> = (),
//
//         locals: gfx::ConstantBuffer<Locals> = "u_locals",
//         globals: gfx::ConstantBuffer<Globals> = "u_globals",
//
//         map: gfx::TextureSampler<[f32; 4]> = "t_map",
//         alt: gfx::TextureSampler<[f32; 2]> = "t_alt",
//         horizon: gfx::TextureSampler<[f32; 4]> = "t_horizon",
//
//         color_sampler: gfx::TextureSampler<<TgtColorFmt as
// gfx::format::Formatted>::View> = "src_color",         depth_sampler:
// gfx::TextureSampler<<TgtDepthStencilFmt as gfx::format::Formatted>::View> =
// "src_depth",
//
//         noise: gfx::TextureSampler<f32> = "t_noise",
//
//         tgt_color: gfx::RenderTarget<WinColorFmt> = "tgt_color",
//     }
// }

#[repr(C)]
#[derive(Copy, Clone, Debug, AsBytes)]
pub struct Locals {
    proj_mat_inv: [[f32; 4]; 4],
    view_mat_inv: [[f32; 4]; 4],
}

impl Default for Locals {
    fn default() -> Self { Self::new(Mat4::identity(), Mat4::identity()) }
}

impl Locals {
    pub fn new(proj_mat_inv: Mat4<f32>, view_mat_inv: Mat4<f32>) -> Self {
        Self {
            proj_mat_inv: proj_mat_inv.into_col_arrays(),
            view_mat_inv: view_mat_inv.into_col_arrays(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, AsBytes)]
pub struct Vertex {
    pub pos: [f32; 2],
}

pub fn create_mesh() -> Mesh<Vertex> {
    let mut mesh = Mesh::new();

    #[rustfmt::skip]
    mesh.push_tri(Tri::new(
        Vertex { pos: [ 1.0, -1.0] },
        Vertex { pos: [-1.0,  1.0] },
        Vertex { pos: [-1.0, -1.0] },
    ));

    #[rustfmt::skip]
    mesh.push_tri(Tri::new(
        Vertex { pos: [1.0, -1.0] },
        Vertex { pos: [1.0,  1.0] },
        Vertex { pos: [-1.0, 1.0] },
    ));

    mesh
}
