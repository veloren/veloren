use euc::{
    Pipeline,
    rasterizer,
    buffer::Buffer2d,
    Interpolate,
};
use common::{
    figure::Segment,
    vol::{
        Vox,
        SizedVol,
        ReadVol,
    },
};
use vek::*;


trait Shader {
    type VertExtra;
    type VsOut: Clone + Interpolate;
    fn vert(&self, v_color: Rgba<f32>, v_pos: Vec3<f32>, vert_extra: &Self::VertExtra) -> (Vec3<f32>, Self::VsOut);
    fn frag(&self, vs_out: &Self::VsOut) -> Rgba<f32>;
}

struct Voxel<S> where S: Shader {
    mvp: Mat4<f32>,
    shader: S,
}

struct SimpleShader;
impl Shader for SimpleShader {
    type VertExtra = ();
    type VsOut = Rgba<f32>;
    fn vert(&self, v_color: Rgba<f32>, v_pos: Vec3<f32>, _: &Self::VertExtra) -> (Vec3<f32>, Self::VsOut) {
        (
            v_pos,
            v_color,
        )
    }
    fn frag(&self, vs_out: &Self::VsOut) -> Rgba<f32> {
        *vs_out
    }
}

impl<'a, S> Pipeline for Voxel<S> where S: Shader {
    type Vertex = (Vec3<f32>, Rgb<f32>, S::VertExtra);
    type VsOut = S::VsOut;
    type Pixel = [u8; 4];

    #[inline(always)]
    fn vert(&self, (v_pos, v_color, v_extra): &Self::Vertex) -> ([f32; 3], Self::VsOut) {
        let (pos, out) = self.shader.vert(
            srgb_to_linear(Rgba::from_opaque(*v_color)),
            Vec3::from(self.mvp * Vec4::from_point(*v_pos)),
            v_extra,
        );
        (
            pos.into_array(),
            out,
        )
    }
    #[inline(always)]
    fn frag(&self, vs_out: &Self::VsOut) -> Self::Pixel {
        let color = self.shader.frag(vs_out);
        linear_to_srgb(color).map(|e| (e * 255.0) as u8).into_array()
    }
}

pub fn draw_vox(segment: &Segment, output_size: Vec2<u16>) -> Vec<[u8; 4]> {
    let dims = output_size.map(|e| e as usize).into_array();
    let mut color = Buffer2d::new(dims, [50; 4]);
    let mut depth = Buffer2d::new(dims, 1.0);

    let mvp =
        Mat4::rotation_y(0.6) *
        Mat4::<f32>::scaling_3d(1.0 / 14.0) *
        Mat4::translation_2d([-14.0, -14.0]) * 
        Mat4::rotation_x(-std::f32::consts::PI / 2.0 );
    Voxel {
        mvp,
        shader: SimpleShader,
    }
    .draw::<rasterizer::Triangles<_>, _>(
        &generate_mesh(segment, Vec3::from(0.0)),
        &mut color,
        &mut depth,
    );

    // TODO: remove this clone
    color.as_ref().to_vec()
}

type Vert = <Voxel<SimpleShader> as Pipeline>::Vertex;

// TODO: generalise meshing code
fn create_quad(
    origin: Vec3<f32>,
    unit_x: Vec3<f32>,
    unit_y: Vec3<f32>,
    //norm: Vec3<f32>,
    col: Rgb<f32>,
) -> [Vert; 6] {
    let a = (origin, col, ());
    let b = (origin + unit_x, col, ());
    let c = (origin + unit_x + unit_y, col, ());
    let d = (origin + unit_y, col, ());
    [
        a, b, c, // Tri 1
        c, d, a, // Tri 2
    ]
}
fn generate_mesh(segment: &Segment, offs: Vec3<f32>) -> Vec<Vert> {
    let mut vertices = Vec::new();

    for pos in segment.iter_positions() {
        if let Some(col) = segment
            .get(pos)
            .ok()
            .and_then(|vox| vox.get_color())
        {
            let col = col.map(|e| e as f32 / 255.0);

            // -x
            if segment.get(pos - Vec3::unit_x())
                .map(|v| v.is_empty())
                .unwrap_or(true)
            {
                vertices.extend_from_slice(&create_quad(
                    offs + pos.map(|e| e as f32) + Vec3::unit_y(),
                    -Vec3::unit_y(),
                    Vec3::unit_z(),
                    //-Vec3::unit_x(),
                    col,
                ));
            }
            // +x
            if segment.get(pos + Vec3::unit_x())
                .map(|v| v.is_empty())
                .unwrap_or(true)
            {
                 vertices.extend_from_slice(&create_quad(
                    offs + pos.map(|e| e as f32) + Vec3::unit_x(),
                    Vec3::unit_y(),
                    Vec3::unit_z(),
                    //Vec3::unit_x(),
                    col,
                ));
            }
            // -y
            if segment.get(pos - Vec3::unit_y())
                .map(|v| v.is_empty())
                .unwrap_or(true)
            {
                 vertices.extend_from_slice(&create_quad(
                    offs + pos.map(|e| e as f32),
                    Vec3::unit_x(),
                    Vec3::unit_z(),
                    //-Vec3::unit_y(),
                    col,
                ));
            }
            // +y
            if segment.get(pos + Vec3::unit_y())
                .map(|v| v.is_empty())
                .unwrap_or(true)
            {
                 vertices.extend_from_slice(&create_quad(
                    offs + pos.map(|e| e as f32) + Vec3::unit_y(),
                    Vec3::unit_z(),
                    Vec3::unit_x(),
                    //Vec3::unit_y(),
                    col,
                ));
            }
            // -z
            if segment.get(pos - Vec3::unit_z())
                .map(|v| v.is_empty())
                .unwrap_or(true)
            {
                 vertices.extend_from_slice(&create_quad(
                    offs + pos.map(|e| e as f32),
                    Vec3::unit_y(),
                    Vec3::unit_x(),
                    //-Vec3::unit_z(),
                    col,
                ));
            }
            // +z
            if segment.get(pos + Vec3::unit_z())
                .map(|v| v.is_empty())
                .unwrap_or(true)
            {
                 vertices.extend_from_slice(&create_quad(
                    offs + pos.map(|e| e as f32) + Vec3::unit_z(),
                    Vec3::unit_x(),
                    Vec3::unit_y(),
                    //Vec3::unit_z(),
                    col,
                ));
            }
        }
    }

    vertices
}

// TODO: put these in utility a module
#[inline(always)]
fn to_linear(x: f32) -> f32 {
    if x <= 0.04045 {
        x / 12.92
    } else {
        ((x + 0.055) / 1.055).powf(2.4)
    }
}
#[inline(always)]
fn to_srgb(x: f32) -> f32 {
    if x <= 0.0031308 {
        x * 12.92
    } else {
        x.powf(1.0 / 2.4) * 1.055 - 0.055
    }
}
#[inline(always)]
fn srgb_to_linear(c: Rgba<f32>) -> Rgba<f32> {
    Rgba {
        r: to_linear(c.r),
        g: to_linear(c.g),
        b: to_linear(c.b),
        a: c.a,
    }
}
#[inline(always)]
fn linear_to_srgb(c: Rgba<f32>) -> Rgba<f32> {
    Rgba {
        r: to_srgb(c.r),
        g: to_srgb(c.g),
        b: to_srgb(c.b),
        a: c.a,
    }
}