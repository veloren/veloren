use super::super::{Globals, Pipeline, Quad, Tri, WinColorFmt, WinDepthFmt};
use gfx::{
    self,
    gfx_constant_struct_meta,
    // Macros
    gfx_defines,
    gfx_impl_struct_meta,
    gfx_pipeline,
    gfx_pipeline_inner,
    gfx_vertex_struct_meta,
};
use vek::*;

gfx_defines! {
    vertex Vertex {
        pos: [f32; 2] = "v_pos",
        uv: [f32; 2] = "v_uv",
        color: [f32; 4] = "v_color",
        mode: u32 = "v_mode",
    }

    constant Locals {
        pos: [f32; 4] = "w_pos",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),

        locals: gfx::ConstantBuffer<Locals> = "u_locals",
        globals: gfx::ConstantBuffer<Globals> = "u_globals",
        tex: gfx::TextureSampler<[f32; 4]> = "u_tex",

        scissor: gfx::Scissor = (),

        tgt_color: gfx::BlendTarget<WinColorFmt> = ("tgt_color", gfx::state::ColorMask::all(), gfx::preset::blend::ALPHA),
        tgt_depth: gfx::DepthTarget<WinDepthFmt> = gfx::preset::depth::LESS_EQUAL_TEST,
    }
}

pub struct UiPipeline;

impl Pipeline for UiPipeline {
    type Vertex = Vertex;
}

impl From<Vec4<f32>> for Locals {
    fn from(pos: Vec4<f32>) -> Self {
        Self {
            pos: pos.into_array(),
        }
    }
}

impl Default for Locals {
    fn default() -> Self {
        Self { pos: [0.0; 4] }
    }
}

/// Draw text from the text cache texture `tex` in the fragment shader.
pub const MODE_TEXT: u32 = 0;
/// Draw an image from the texture at `tex` in the fragment shader.
pub const MODE_IMAGE: u32 = 1;
/// Ignore `tex` and draw simple, colored 2D geometry.
pub const MODE_GEOMETRY: u32 = 2;

pub enum Mode {
    Text,
    Image,
    Geometry,
}

impl Mode {
    fn value(self) -> u32 {
        match self {
            Mode::Text => MODE_TEXT,
            Mode::Image => MODE_IMAGE,
            Mode::Geometry => MODE_GEOMETRY,
        }
    }
}

pub fn create_quad(
    rect: Aabr<f32>,
    uv_rect: Aabr<f32>,
    color: Rgba<f32>,
    mode: Mode,
) -> Quad<UiPipeline> {
    let mode_val = mode.value();
    let v = |pos, uv| Vertex {
        pos,
        uv,
        color: color.into_array(),
        mode: mode_val,
    };
    let aabr_to_lbrt = |aabr: Aabr<f32>| (aabr.min.x, aabr.min.y, aabr.max.x, aabr.max.y);

    let (l, b, r, t) = aabr_to_lbrt(rect);
    let (uv_l, uv_b, uv_r, uv_t) = aabr_to_lbrt(uv_rect);

    match (uv_b > uv_t, uv_l > uv_r) {
        (true, true) => Quad::new(
            v([r, t], [uv_l, uv_b]),
            v([l, t], [uv_l, uv_t]),
            v([l, b], [uv_r, uv_t]),
            v([r, b], [uv_r, uv_b]),
        ),
        (false, false) => Quad::new(
            v([r, t], [uv_l, uv_b]),
            v([l, t], [uv_l, uv_t]),
            v([l, b], [uv_r, uv_t]),
            v([r, b], [uv_r, uv_b]),
        ),
        _ => Quad::new(
            v([r, t], [uv_r, uv_t]),
            v([l, t], [uv_l, uv_t]),
            v([l, b], [uv_l, uv_b]),
            v([r, b], [uv_r, uv_b]),
        ),
    }
}

pub fn create_tri(
    tri: [[f32; 2]; 3],
    uv_tri: [[f32; 2]; 3],
    color: Rgba<f32>,
    mode: Mode,
) -> Tri<UiPipeline> {
    let mode_val = mode.value();
    let v = |pos, uv| Vertex {
        pos,
        uv,
        color: color.into_array(),
        mode: mode_val,
    };
    Tri::new(
        v([tri[0][0], tri[0][1]], [uv_tri[0][0], uv_tri[0][1]]),
        v([tri[1][0], tri[1][1]], [uv_tri[1][0], uv_tri[1][1]]),
        v([tri[2][0], tri[2][1]], [uv_tri[2][0], uv_tri[2][1]]),
    )
}
