use common::{
    figure::Segment,
    util::{linear_to_srgba, srgba_to_linear},
    vol::{IntoFullVolIterator, ReadVol, SizedVol, Vox},
};
use euc::{buffer::Buffer2d, rasterizer, Pipeline};
use image::{DynamicImage, RgbaImage};
use vek::*;

#[derive(Copy, Clone)]
pub enum SampleStrat {
    None,
    SuperSampling(u8),
    PixelCoverage,
}

#[derive(Clone)]
pub struct Transform {
    pub ori: Quaternion<f32>,
    pub offset: Vec3<f32>,
    pub zoom: f32,
    pub orth: bool,
    pub stretch: bool,
}
impl Default for Transform {
    fn default() -> Self {
        Self {
            ori: Quaternion::identity(),
            offset: Vec3::zero(),
            zoom: 1.0,
            orth: true,
            stretch: true,
        }
    }
}

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

#[derive(Clone, Copy)]
struct VsOut(Rgba<f32>);
impl euc::Interpolate for VsOut {
    #[inline(always)]
    fn lerp2(a: Self, b: Self, x: f32, y: f32) -> Self {
        //a * x + b * y
        Self(a.0.map2(b.0, |a, b| a.mul_add(x, b * y)))
    }

    #[inline(always)]
    #[allow(clippy::many_single_char_names)]
    fn lerp3(a: Self, b: Self, c: Self, x: f32, y: f32, z: f32) -> Self {
        //a * x + b * y + c * z
        Self(
            a.0.map2(b.0.map2(c.0, |b, c| b.mul_add(y, c * z)), |a, bc| {
                a.mul_add(x, bc)
            }),
        )
    }
}

impl<'a> Pipeline for Voxel {
    type Pixel = [u8; 4];
    type Vertex = Vert;
    type VsOut = VsOut;

    #[inline(always)]
    fn vert(
        &self,
        Vert {
            pos,
            col,
            //norm: _,
            ao_level,
        }: &Self::Vertex,
    ) -> ([f32; 4], Self::VsOut) {
        let light = Rgba::from_opaque(Rgb::from(*ao_level as f32 / 4.0 + 0.25));
        let color = light * srgba_to_linear(Rgba::from_opaque(*col));
        let position = (self.mvp * Vec4::from_point(*pos)).into_array();
        (position, VsOut(color))
    }

    #[inline(always)]
    fn frag(&self, color: &Self::VsOut) -> Self::Pixel {
        linear_to_srgba(color.0)
            .map(|e| (e * 255.0) as u8)
            .into_array()
    }
}

pub fn draw_vox(
    segment: &Segment,
    output_size: Vec2<u16>,
    transform: Transform,
    sample_strat: SampleStrat,
) -> RgbaImage {
    let output_size = output_size.map(|e| e as usize);
    debug_assert!(output_size.map(|e| e != 0).reduce_and());

    let ori_mat = Mat4::from(transform.ori);
    let rotated_segment_dims = (ori_mat * Vec4::from_direction(segment.size().map(|e| e as f32)))
        .xyz()
        .map(|e| e.abs());

    let dims = match sample_strat {
        SampleStrat::None => output_size,
        SampleStrat::SuperSampling(min_samples) => {
            output_size * (min_samples as f32).sqrt().ceil() as usize
        },
        // Assumes
        //  - rotations are multiples of 90 degrees
        //  - the projection is orthographic
        //  - no translation or zooming is performed
        //  - stretch is enabled
        SampleStrat::PixelCoverage => Vec2::new(
            rotated_segment_dims.x.round() as usize,
            rotated_segment_dims.y.round() as usize,
        ),
    }
    .into_array();

    debug_assert!(dims[0] != 0 && dims[1] != 0);

    // Rendering buffers
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
            rotated_segment_dims.map(|e| 2.0 / e)
        } else {
            let s = w.max(h).max(d);
            Vec3::from(2.0 / s)
        } * transform.zoom,
    ) * Mat4::translation_3d(transform.offset)
        * ori_mat
        * Mat4::translation_3d([-w / 2.0, -h / 2.0, -d / 2.0]);

    Voxel { mvp }.draw::<rasterizer::Triangles<_>, _>(
        &generate_mesh(segment, Vec3::from(0.0)),
        &mut color,
        Some(&mut depth),
    );

    let rgba_img = RgbaImage::from_vec(
        dims[0] as u32,
        dims[1] as u32,
        color
            .as_ref()
            .iter()
            .flatten()
            .copied()
            .collect::<Vec<u8>>(),
    )
    .unwrap();

    match sample_strat {
        SampleStrat::None => rgba_img,
        SampleStrat::SuperSampling(_) => DynamicImage::ImageRgba8(rgba_img)
            .resize_exact(
                output_size.x as u32,
                output_size.y as u32,
                image::FilterType::Triangle,
            )
            .to_rgba(),
        SampleStrat::PixelCoverage => super::pixel_art::resize_pixel_art(
            &rgba_img,
            output_size.x as u32,
            output_size.y as u32,
        ),
    }
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
