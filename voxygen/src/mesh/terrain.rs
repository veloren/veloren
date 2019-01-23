// Library
use vek::*;

// Project
use common::{
    vol::{
        Vox,
        SizedVol,
        ReadVol,
    },
    volumes::dyna::Dyna,
    terrain::{Block, TerrainChunk},
};

// Crate
use crate::{
    mesh::Meshable,
    render::{
        self,
        Mesh,
        Quad,
        TerrainPipeline,
    },
};

type TerrainVertex = <TerrainPipeline as render::Pipeline>::Vertex;

// Utility function
// TODO: Evaluate how useful this is
fn create_quad(
    origin: Vec3<f32>,
    unit_x: Vec3<f32>,
    unit_y: Vec3<f32>,
    norm: Vec3<f32>,
    col: Rgb<f32>,
) -> Quad<TerrainPipeline> {
    Quad::new(
        TerrainVertex::new(origin, norm, col),
        TerrainVertex::new(origin + unit_x, norm, col),
        TerrainVertex::new(origin + unit_x + unit_y, norm, col),
        TerrainVertex::new(origin + unit_y, norm, col),
    )
}

impl<M> Meshable for Dyna<Block, M> {
    type Pipeline = TerrainPipeline;
    type Supplement = ();

    fn generate_mesh(&self, _: Self::Supplement) -> Mesh<Self::Pipeline> {
        let mut mesh = Mesh::new();

        for pos in self
            .iter_positions()
            .filter(|pos| pos.map(|e| e >= 1).reduce_and())
            .filter(|pos| pos.map2(self.get_size(), |e, sz| e < sz as i32 - 1).reduce_and())
        {
            if let Some(col) = self
                .get(pos)
                .ok()
                .and_then(|vox| vox.get_color())
            {
                let col = col.map(|e| e as f32 / 255.0);

                // -x
                if self.get(pos - Vec3::unit_x())
                    .map(|v| v.is_empty())
                    .unwrap_or(true)
                {
                    mesh.push_quad(create_quad(
                        Vec3::one() + pos.map(|e| e as f32) + Vec3::unit_y(),
                        -Vec3::unit_y(),
                        Vec3::unit_z(),
                        -Vec3::unit_x(),
                        col,
                    ));
                }
                // +x
                if self.get(pos + Vec3::unit_x())
                    .map(|v| v.is_empty())
                    .unwrap_or(true)
                {
                    mesh.push_quad(create_quad(
                        Vec3::one() + pos.map(|e| e as f32) + Vec3::unit_x(),
                        Vec3::unit_y(),
                        Vec3::unit_z(),
                        Vec3::unit_x(),
                        col,
                    ));
                }
                // -y
                if self.get(pos - Vec3::unit_y())
                    .map(|v| v.is_empty())
                    .unwrap_or(true)
                {
                    mesh.push_quad(create_quad(
                        Vec3::one() + pos.map(|e| e as f32),
                        Vec3::unit_x(),
                        Vec3::unit_z(),
                        -Vec3::unit_y(),
                        col,
                    ));
                }
                // +y
                if self.get(pos + Vec3::unit_y())
                    .map(|v| v.is_empty())
                    .unwrap_or(true)
                {
                    mesh.push_quad(create_quad(
                        Vec3::one() + pos.map(|e| e as f32) + Vec3::unit_y(),
                        Vec3::unit_z(),
                        Vec3::unit_x(),
                        Vec3::unit_y(),
                        col,
                    ));
                }
                // -z
                if self.get(pos - Vec3::unit_z())
                    .map(|v| v.is_empty())
                    .unwrap_or(true)
                {
                    mesh.push_quad(create_quad(
                        Vec3::one() + pos.map(|e| e as f32),
                        Vec3::unit_y(),
                        Vec3::unit_x(),
                        -Vec3::unit_z(),
                        col,
                    ));
                }
                // +z
                if self.get(pos + Vec3::unit_z())
                    .map(|v| v.is_empty())
                    .unwrap_or(true)
                {
                    mesh.push_quad(create_quad(
                        Vec3::one() + pos.map(|e| e as f32) + Vec3::unit_z(),
                        Vec3::unit_x(),
                        Vec3::unit_y(),
                        Vec3::unit_z(),
                        col,
                    ));
                }
            }
        }

        mesh
    }
}
