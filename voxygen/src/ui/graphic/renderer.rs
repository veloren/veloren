use super::Transform;
use common::{
    figure::Segment,
    util::{linear_to_srgba, srgba_to_linear},
    vol::{IntoFullVolIterator, ReadVol, SizedVol, Vox},
};
use euc::{buffer::Buffer2d, rasterizer, Pipeline};
use image::{DynamicImage, RgbaImage};
use vek::*;

struct Voxel {
    mvp: Mat4<f32>,
}

// TODO: use norm or remove it
#[derive(Copy, Clone)]
struct Vert {
    pos: Vec3<f32>,
    col: Rgb<f32>,
    //norm: Vec3<f32>,
    ao_level: u8,
}
impl Vert {
    fn new(pos: Vec3<f32>, col: Rgb<f32>, _norm: Vec3<f32>, ao_level: u8) -> Self {
        Vert {
            pos,
            col,
            //norm,
            ao_level,
        }
    }
}

impl<'a> Pipeline for Voxel {
    type Vertex = Vert;
    type VsOut = Rgba<f32>;
    type Pixel = [u8; 4];

    #[inline(always)]
    fn vert(
        &self,
        Vert {
            pos,
            col,
            //norm: _,
            ao_level,
        }: &Self::Vertex,
    ) -> ([f32; 3], Self::VsOut) {
        let light = Rgba::from_opaque(Rgb::from(*ao_level as f32 / 4.0 + 0.25));
        let color = light * srgba_to_linear(Rgba::from_opaque(*col));
        let position = Vec3::from(self.mvp * Vec4::from_point(*pos)).into_array();
        (position, color)
    }
    #[inline(always)]
    fn frag(&self, color: &Self::VsOut) -> Self::Pixel {
        linear_to_srgba(*color)
            .map(|e| (e * 255.0) as u8)
            .into_array()
    }
}

pub fn draw_vox(
    segment: &Segment,
    output_size: Vec2<u16>,
    transform: Transform,
    min_samples: Option<u8>,
) -> RgbaImage {
    let scale = min_samples.map_or(1.0, |s| s as f32).sqrt().ceil() as usize;
    let dims = output_size.map(|e| e as usize * scale).into_array();
    let mut color = Buffer2d::new(dims, [0; 4]);
    let mut depth = Buffer2d::new(dims, 1.0);

    let (w, h, d) = segment.size().map(|e| e as f32).into_tuple();

    let mvp = if transform.orth {
        Mat4::<f32>::orthographic_rh_no(FrustumPlanes {
            left: -1.0,
            right: 1.0,
            bottom: -1.0,
            top: 1.0,
            near: 0.0,
            far: 1.0,
        })
    } else {
        Mat4::<f32>::perspective_fov_rh_no(
            1.1,            // fov
            dims[0] as f32, // width
            dims[1] as f32, // height
            0.0,
            1.0,
        )
    } * Mat4::scaling_3d(
        // TODO replace with camera-like parameters?
        if transform.stretch {
            Vec3::new(2.0 / w, 2.0 / d, 2.0 / h) // Only works with flipped models :(
        } else {
            let s = w.max(h).max(d);
            Vec3::new(2.0 / s, 2.0 / s, 2.0 / s)
        } * transform.zoom,
    ) * Mat4::translation_3d(transform.offset)
        * Mat4::from(transform.ori)
        * Mat4::translation_3d([-w / 2.0, -h / 2.0, -d / 2.0]);

    Voxel { mvp }.draw::<rasterizer::Triangles<_>, _>(
        &generate_mesh(segment, Vec3::from(0.0)),
        &mut color,
        &mut depth,
    );

    let image = DynamicImage::ImageRgba8(
        RgbaImage::from_vec(
            dims[0] as u32,
            dims[1] as u32,
            color
                .as_ref()
                .iter()
                .flatten()
                .cloned()
                .collect::<Vec<u8>>(),
        )
        .unwrap(),
    );
    if scale > 1 {
        image.resize_exact(
            output_size.x as u32,
            output_size.y as u32,
            image::FilterType::Triangle,
        )
    } else {
        image
    }
    .to_rgba()
}

fn ao_level(side1: bool, corner: bool, side2: bool) -> u8 {
    if side1 && side2 {
        0
    } else {
        3 - [side1, corner, side2].iter().filter(|e| **e).count() as u8
    }
}
// TODO: Generalize meshing code.
fn create_quad(
    origin: Vec3<f32>,
    unit_x: Vec3<f32>,
    unit_y: Vec3<f32>,
    norm: Vec3<f32>,
    col: Rgb<f32>,
    occluders: [bool; 8],
) -> [Vert; 6] {
    let a_ao = ao_level(occluders[0], occluders[1], occluders[2]);
    let b_ao = ao_level(occluders[2], occluders[3], occluders[4]);
    let c_ao = ao_level(occluders[4], occluders[5], occluders[6]);
    let d_ao = ao_level(occluders[6], occluders[7], occluders[0]);

    let a = Vert::new(origin, col, norm, a_ao);
    let b = Vert::new(origin + unit_x, col, norm, b_ao);
    let c = Vert::new(origin + unit_x + unit_y, col, norm, c_ao);
    let d = Vert::new(origin + unit_y, col, norm, d_ao);

    // Flip to fix anisotropy.
    let (a, b, c, d) = if a_ao + c_ao > b_ao + d_ao {
        (d, a, b, c)
    } else {
        (a, b, c, d)
    };

    [
        a, b, c, // Tri 1
        c, d, a, // Tri 2
    ]
}

fn generate_mesh(segment: &Segment, offs: Vec3<f32>) -> Vec<Vert> {
    let mut vertices = Vec::new();

    for (pos, vox) in segment.full_vol_iter() {
        if let Some(col) = vox.get_color() {
            let col = col.map(|e| e as f32 / 255.0);

            let is_empty = |pos| segment.get(pos).map(|v| v.is_empty()).unwrap_or(true);

            let occluders = |unit_x, unit_y, dir| {
                // Would be nice to generate unit_x and unit_y from a given direction.
                [
                    !is_empty(pos + dir - unit_x),
                    !is_empty(pos + dir - unit_x - unit_y),
                    !is_empty(pos + dir - unit_y),
                    !is_empty(pos + dir + unit_x - unit_y),
                    !is_empty(pos + dir + unit_x),
                    !is_empty(pos + dir + unit_x + unit_y),
                    !is_empty(pos + dir + unit_y),
                    !is_empty(pos + dir - unit_x + unit_y),
                ]
            };

            // -x
            if is_empty(pos - Vec3::unit_x()) {
                vertices.extend_from_slice(&create_quad(
                    offs + pos.map(|e| e as f32) + Vec3::unit_y(),
                    -Vec3::unit_y(),
                    Vec3::unit_z(),
                    -Vec3::unit_x(),
                    col,
                    occluders(-Vec3::unit_y(), Vec3::unit_z(), -Vec3::unit_x()),
                ));
            }
            // +x
            if is_empty(pos + Vec3::unit_x()) {
                vertices.extend_from_slice(&create_quad(
                    offs + pos.map(|e| e as f32) + Vec3::unit_x(),
                    Vec3::unit_y(),
                    Vec3::unit_z(),
                    Vec3::unit_x(),
                    col,
                    occluders(Vec3::unit_y(), Vec3::unit_z(), Vec3::unit_x()),
                ));
            }
            // -y
            if is_empty(pos - Vec3::unit_y()) {
                vertices.extend_from_slice(&create_quad(
                    offs + pos.map(|e| e as f32),
                    Vec3::unit_x(),
                    Vec3::unit_z(),
                    -Vec3::unit_y(),
                    col,
                    occluders(Vec3::unit_x(), Vec3::unit_z(), -Vec3::unit_y()),
                ));
            }
            // +y
            if is_empty(pos + Vec3::unit_y()) {
                vertices.extend_from_slice(&create_quad(
                    offs + pos.map(|e| e as f32) + Vec3::unit_y(),
                    Vec3::unit_z(),
                    Vec3::unit_x(),
                    Vec3::unit_y(),
                    col,
                    occluders(Vec3::unit_z(), Vec3::unit_x(), Vec3::unit_y()),
                ));
            }
            // -z
            if is_empty(pos - Vec3::unit_z()) {
                vertices.extend_from_slice(&create_quad(
                    offs + pos.map(|e| e as f32),
                    Vec3::unit_y(),
                    Vec3::unit_x(),
                    -Vec3::unit_z(),
                    col,
                    occluders(Vec3::unit_y(), Vec3::unit_x(), -Vec3::unit_z()),
                ));
            }
            // +z
            if is_empty(pos + Vec3::unit_z()) {
                vertices.extend_from_slice(&create_quad(
                    offs + pos.map(|e| e as f32) + Vec3::unit_z(),
                    Vec3::unit_x(),
                    Vec3::unit_y(),
                    Vec3::unit_z(),
                    col,
                    occluders(Vec3::unit_x(), Vec3::unit_y(), Vec3::unit_z()),
                ));
            }
        }
    }

    vertices
}
