use vek::*;

use crate::render::{
    mesh::{Mesh, Quad},
    Pipeline,
};

/// Given volume, position, and cardinal directions, compute each vertex's AO
/// value. `dirs` should be a slice of length 5 so that the sliding window of
/// size 2 over the slice yields each vertex' adjacent positions.
#[allow(unsafe_code)]
fn get_ao_quad(
    shift: Vec3<i32>,
    dirs: &[Vec3<i32>],
    darknesses: &[[[Option<f32>; 3]; 3]; 3],
) -> Vec4<(f32, f32)> {
    dirs.windows(2)
        .map(|offs| {
            let vox_opaque = |pos: Vec3<i32>| {
                let pos = (pos + 1).map(|e| e as usize);
                unsafe {
                    darknesses
                        .get_unchecked(pos.z)
                        .get_unchecked(pos.y)
                        .get_unchecked(pos.x)
                        .is_none()
                }
            };

            let (s1, s2) = (
                vox_opaque(shift + offs[0]),
                vox_opaque(shift + offs[1]),
                /*
                vol.get(pos + shift + offs[0])
                    .map(&is_opaque)
                    .unwrap_or(false),
                vol.get(pos + shift + offs[1])
                    .map(&is_opaque)
                    .unwrap_or(false),
                */
            );

            let mut darkness = 0.0;
            let mut total = 0.0f32;
            for x in 0..2 {
                for y in 0..2 {
                    let dark_pos = shift + offs[0] * x + offs[1] * y + 1;
                    if let Some(dark) = unsafe {
                        darknesses
                            .get_unchecked(dark_pos.z as usize)
                            .get_unchecked(dark_pos.y as usize)
                            .get_unchecked(dark_pos.x as usize)
                    } {
                        darkness += dark;
                        total += 1.0;
                    }
                }
            }
            let darkness = darkness / total.max(1.0);

            (
                darkness,
                if s1 && s2 {
                    0.0
                } else {
                    let corner = vox_opaque(shift + offs[0] + offs[1]);
                    // Map both 1 and 2 neighbors to 0.5 occlusion.
                    if s1 || s2 || corner { 0.4 } else { 1.0 }
                },
            )
        })
        .collect::<Vec4<(f32, f32)>>()
}

#[allow(unsafe_code)]
fn get_col_quad(dirs: &[Vec3<i32>], cols: &[[[Rgba<u8>; 3]; 3]; 3]) -> Vec4<Rgb<f32>> {
    dirs.windows(2)
        .map(|offs| {
            let primary_col = Rgb::from(cols[1][1][1]).map(|e: u8| e as f32);
            let mut color = Rgb::zero();
            let mut total = 0.0;
            for x in 0..2 {
                for y in 0..2 {
                    let col_pos = offs[0] * x + offs[1] * y + 1;
                    let col = unsafe {
                        cols.get_unchecked(col_pos.z as usize)
                            .get_unchecked(col_pos.y as usize)
                            .get_unchecked(col_pos.x as usize)
                    };
                    if col.a > 0 {
                        let col = Rgb::new(col.r, col.g, col.b).map(|e| e as f32);
                        if Vec3::<f32>::from(primary_col).distance_squared(Vec3::from(col))
                            < (0.025f32 * 256.0).powf(2.0)
                        {
                            color += col;
                            total += 256.0;
                        }
                    }
                }
            }

            color / total
        })
        .collect()
}

// Utility function
fn create_quad<P: Pipeline, F: Fn(Vec3<f32>, Vec3<f32>, Rgb<f32>, f32, f32) -> P::Vertex>(
    origin: Vec3<f32>,
    unit_x: Vec3<f32>,
    unit_y: Vec3<f32>,
    norm: Vec3<f32>,
    cols: Vec4<Rgb<f32>>,
    darkness_ao: Vec4<(f32, f32)>,
    vcons: &F,
) -> Quad<P> {
    let darkness = darkness_ao.map(|e| e.0);
    let ao = darkness_ao.map(|e| e.1);

    let ao_map = ao;

    if ao[0].min(ao[2]) < ao[1].min(ao[3]) {
        Quad::new(
            vcons(origin + unit_y, norm, cols[3], darkness[3], ao_map[3]),
            vcons(origin, norm, cols[0], darkness[0], ao_map[0]),
            vcons(origin + unit_x, norm, cols[1], darkness[1], ao_map[1]),
            vcons(
                origin + unit_x + unit_y,
                norm,
                cols[2],
                darkness[2],
                ao_map[2],
            ),
        )
    } else {
        Quad::new(
            vcons(origin, norm, cols[0], darkness[0], ao_map[0]),
            vcons(origin + unit_x, norm, cols[1], darkness[1], ao_map[1]),
            vcons(
                origin + unit_x + unit_y,
                norm,
                cols[2],
                darkness[2],
                ao_map[2],
            ),
            vcons(origin + unit_y, norm, cols[3], darkness[3], ao_map[3]),
        )
    }
}

pub fn push_vox_verts<P: Pipeline>(
    mesh: &mut Mesh<P>,
    faces: [bool; 6],
    offs: Vec3<f32>,
    cols: &[[[Rgba<u8>; 3]; 3]; 3],
    vcons: impl Fn(Vec3<f32>, Vec3<f32>, Rgb<f32>, f32, f32) -> P::Vertex,
    darknesses: &[[[Option<f32>; 3]; 3]; 3],
) {
    let (x, y, z) = (Vec3::unit_x(), Vec3::unit_y(), Vec3::unit_z());

    // -x
    if faces[0] {
        mesh.push_quad(create_quad(
            offs,
            Vec3::unit_z(),
            Vec3::unit_y(),
            -Vec3::unit_x(),
            get_col_quad(&[-z, -y, z, y, -z], cols),
            get_ao_quad(-Vec3::unit_x(), &[-z, -y, z, y, -z], darknesses),
            &vcons,
        ));
    }
    // +x
    if faces[1] {
        mesh.push_quad(create_quad(
            offs + Vec3::unit_x(),
            Vec3::unit_y(),
            Vec3::unit_z(),
            Vec3::unit_x(),
            get_col_quad(&[-y, -z, y, z, -y], cols),
            get_ao_quad(Vec3::unit_x(), &[-y, -z, y, z, -y], darknesses),
            &vcons,
        ));
    }
    // -y
    if faces[2] {
        mesh.push_quad(create_quad(
            offs,
            Vec3::unit_x(),
            Vec3::unit_z(),
            -Vec3::unit_y(),
            get_col_quad(&[-x, -z, x, z, -x], cols),
            get_ao_quad(-Vec3::unit_y(), &[-x, -z, x, z, -x], darknesses),
            &vcons,
        ));
    }
    // +y
    if faces[3] {
        mesh.push_quad(create_quad(
            offs + Vec3::unit_y(),
            Vec3::unit_z(),
            Vec3::unit_x(),
            Vec3::unit_y(),
            get_col_quad(&[-z, -x, z, x, -z], cols),
            get_ao_quad(Vec3::unit_y(), &[-z, -x, z, x, -z], darknesses),
            &vcons,
        ));
    }
    // -z
    if faces[4] {
        mesh.push_quad(create_quad(
            offs,
            Vec3::unit_y(),
            Vec3::unit_x(),
            -Vec3::unit_z(),
            get_col_quad(&[-y, -x, y, x, -y], cols),
            get_ao_quad(-Vec3::unit_z(), &[-y, -x, y, x, -y], darknesses),
            &vcons,
        ));
    }
    // +z
    if faces[5] {
        mesh.push_quad(create_quad(
            offs + Vec3::unit_z(),
            Vec3::unit_x(),
            Vec3::unit_y(),
            Vec3::unit_z(),
            get_col_quad(&[-x, -y, x, y, -x], cols),
            get_ao_quad(Vec3::unit_z(), &[-x, -y, x, y, -x], darknesses),
            &vcons,
        ));
    }
}
