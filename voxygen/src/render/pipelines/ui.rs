// Library
use gfx::{
    self,
    // Macros
    gfx_defines,
    gfx_vertex_struct_meta,
    gfx_impl_struct_meta,
    gfx_pipeline,
    gfx_pipeline_inner,
};

// Local
use super::super::{
        Pipeline,
        TgtColorFmt,
        TgtDepthFmt,
        Mesh,
        Quad,
        Tri,
};

gfx_defines! {
    vertex Vertex {
        pos: [f32; 2] = "v_pos",
        uv: [f32; 2] = "v_uv",
        color: [f32; 4] = "v_color",
        mode: u32 = "v_mode",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),

        tex: gfx::TextureSampler<[f32; 4]> = "u_tex",

        scissor: gfx::Scissor = (),

        tgt_color: gfx::BlendTarget<TgtColorFmt> = ("tgt_color", gfx::state::ColorMask::all(), gfx::preset::blend::ALPHA),
        tgt_depth: gfx::DepthTarget<TgtDepthFmt> = gfx::preset::depth::PASS_TEST,
    }
}

pub struct UiPipeline;

impl Pipeline for UiPipeline {
    type Vertex = Vertex;
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

// TODO: don't use [f32; 4] for rectangle as the format (eg 2 points vs point + dims) is ambiguous
pub fn push_quad_to_mesh(mesh: &mut Mesh<UiPipeline>, rect: [f32; 4], uv_rect: [f32; 4], color: [f32; 4], mode: Mode) {
    let mode_val = mode.value();
    let v = |pos, uv| {
        Vertex {
            pos,
            uv,
            color,
            mode: mode_val,
        }
    };
    let (l, t, r, b) = (rect[0], rect[1], rect[2], rect[3]);
    let (uv_l, uv_t, uv_r, uv_b) = (uv_rect[0], uv_rect[1], uv_rect[2], uv_rect[3]);
    mesh.push_quad(Quad::new(
        v([r, t], [uv_r, uv_t]),
        v([l, t], [uv_l, uv_t]),
        v([l, b], [uv_l, uv_b]),
        v([r, b], [uv_r, uv_b]),
    ));
}

pub fn push_tri_to_mesh(mesh: &mut Mesh<UiPipeline>, tri: [[f32; 2]; 3], uv_tri: [[f32; 2]; 3], color: [f32; 4], mode: Mode) {
    let mode_val = mode.value();
    let v = |pos, uv| {
        Vertex {
            pos,
            uv,
            color,
            mode: mode_val,
        }
    };
    mesh.push_tri(Tri::new(
        v([tri[0][0], tri[0][1]], [uv_tri[0][0], uv_tri[0][1]]),
        v([tri[1][0], tri[1][1]], [uv_tri[1][0], uv_tri[1][1]]),
        v([tri[2][0], tri[2][1]], [uv_tri[2][0], uv_tri[2][1]]),
    ));
}
