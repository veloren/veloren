// Library
use vek::*;

// Project
use common::{
    terrain::Block,
    vol::{ReadVol, SizedVol, Vox},
    volumes::dyna::Dyna,
};

// Crate
use crate::{
    mesh::Meshable,
    render::{self, Mesh, Quad, TerrainPipeline},
};

type TerrainVertex = <TerrainPipeline as render::Pipeline>::Vertex;

// Utility function
fn create_quad(
    origin: Vec3<f32>,
    unit_x: Vec3<f32>,
    unit_y: Vec3<f32>,
    norm: Vec3<f32>,
    col: Rgb<f32>,
    ao: Vec4<f32>,
) -> Quad<TerrainPipeline> {
    let ao_scale = 1.0;
    let dark = col * (1.0 - ao_scale);

    if ao[0] + ao[2] < ao[1] + ao[3] {
        Quad::new(
            TerrainVertex::new(origin + unit_y, norm, Rgb::lerp(dark, col, ao[3])),
            TerrainVertex::new(origin, norm, Rgb::lerp(dark, col, ao[0])),
            TerrainVertex::new(origin + unit_x, norm, Rgb::lerp(dark, col, ao[1])),
            TerrainVertex::new(origin + unit_x + unit_y, norm, Rgb::lerp(dark, col, ao[2])),
        )
    } else {
        Quad::new(
            TerrainVertex::new(origin, norm, Rgb::lerp(dark, col, ao[0])),
            TerrainVertex::new(origin + unit_x, norm, Rgb::lerp(dark, col, ao[1])),
            TerrainVertex::new(origin + unit_x + unit_y, norm, Rgb::lerp(dark, col, ao[2])),
            TerrainVertex::new(origin + unit_y, norm, Rgb::lerp(dark, col, ao[3])),
        )
    }
}

impl<M> Meshable for Dyna<Block, M> {
    type Pipeline = TerrainPipeline;
    type Supplement = ();

    fn generate_mesh(&self, _: Self::Supplement) -> Mesh<Self::Pipeline> {
        let mut mesh = Mesh::new();

        for pos in self
            .iter_positions()
            .filter(|pos| pos.map(|e| e >= 1).reduce_and())
            .filter(|pos| {
                pos.map2(self.get_size(), |e, sz| e < sz as i32 - 1)
                    .reduce_and()
            })
        {
            let offs = pos.map(|e| e as f32 - 1.0);

            if let Some(col) = self.get(pos).ok().and_then(|vox| vox.get_color()) {
                let col = col.map(|e| e as f32 / 255.0);

                let (x, y, z) = (Vec3::unit_x(), Vec3::unit_y(), Vec3::unit_z());

                fn get_ao<M>(
                    dyna: &Dyna<Block, M>,
                    pos: Vec3<i32>,
                    dirs: &[Vec3<i32>],
                ) -> Vec4<f32> {
                    dirs.windows(2)
                        .map(|offs| {
                            let (s1, s2) = (
                                dyna.get(pos + offs[0])
                                    .map(|v| v.is_empty() as i32)
                                    .unwrap_or(1),
                                dyna.get(pos + offs[1])
                                    .map(|v| v.is_empty() as i32)
                                    .unwrap_or(1),
                            );

                            if s1 == 0 && s2 == 0 {
                                0
                            } else {
                                let corner = dyna
                                    .get(pos + offs[0] + offs[1])
                                    .map(|v| v.is_empty() as i32)
                                    .unwrap_or(1);
                                s1 + s2 + corner
                            }
                        })
                        .map(|i| i as f32 / 3.0)
                        .collect::<Vec4<f32>>()
                }

                // -x
                if self
                    .get(pos - Vec3::unit_x())
                    .map(|v| v.is_empty())
                    .unwrap_or(true)
                {
                    mesh.push_quad(create_quad(
                        offs,
                        Vec3::unit_z(),
                        Vec3::unit_y(),
                        -Vec3::unit_x(),
                        col,
                        get_ao(self, pos - Vec3::unit_x(), &[-z, -y, z, y, -z]),
                    ));
                }
                // +x
                if self
                    .get(pos + Vec3::unit_x())
                    .map(|v| v.is_empty())
                    .unwrap_or(true)
                {
                    mesh.push_quad(create_quad(
                        offs + Vec3::unit_x(),
                        Vec3::unit_y(),
                        Vec3::unit_z(),
                        Vec3::unit_x(),
                        col,
                        get_ao(self, pos + Vec3::unit_x(), &[-y, -z, y, z, -y]),
                    ));
                }
                // -y
                if self
                    .get(pos - Vec3::unit_y())
                    .map(|v| v.is_empty())
                    .unwrap_or(true)
                {
                    mesh.push_quad(create_quad(
                        offs,
                        Vec3::unit_x(),
                        Vec3::unit_z(),
                        -Vec3::unit_y(),
                        col,
                        get_ao(self, pos - Vec3::unit_y(), &[-x, -z, x, z, -x]),
                    ));
                }
                // +y
                if self
                    .get(pos + Vec3::unit_y())
                    .map(|v| v.is_empty())
                    .unwrap_or(true)
                {
                    mesh.push_quad(create_quad(
                        offs + Vec3::unit_y(),
                        Vec3::unit_z(),
                        Vec3::unit_x(),
                        Vec3::unit_y(),
                        col,
                        get_ao(self, pos + Vec3::unit_y(), &[-z, -x, z, x, -z]),
                    ));
                }
                // -z
                if self
                    .get(pos - Vec3::unit_z())
                    .map(|v| v.is_empty())
                    .unwrap_or(true)
                {
                    mesh.push_quad(create_quad(
                        offs,
                        Vec3::unit_y(),
                        Vec3::unit_x(),
                        -Vec3::unit_z(),
                        col,
                        get_ao(self, pos - Vec3::unit_z(), &[-y, -x, y, x, -y]),
                    ));
                }
                // +z
                if self
                    .get(pos + Vec3::unit_z())
                    .map(|v| v.is_empty())
                    .unwrap_or(true)
                {
                    mesh.push_quad(create_quad(
                        offs + Vec3::unit_z(),
                        Vec3::unit_x(),
                        Vec3::unit_y(),
                        Vec3::unit_z(),
                        col,
                        get_ao(self, pos + Vec3::unit_z(), &[-x, -y, x, y, -x]),
                    ));
                }
            }
        }

        mesh
    }
}
