// Project
use common::figure::Segment;

// Library
use vek::*;

// Project
use common::vol::{
    SizedVol,
    ReadVol,
};

// Crate
use crate::{
    mesh::Meshable,
    render::{
        self,
        Mesh,
        Quad,
        FigurePipeline,
    },
};

type FigureVertex = <FigurePipeline as render::Pipeline>::Vertex;

// Utility function
// TODO: Evaluate how useful this is
fn create_quad(
    origin: Vec3<f32>,
    unit_x: Vec3<f32>,
    unit_y: Vec3<f32>,
    norm: Vec3<f32>,
    col: Rgb<f32>,
    bone: u8,
) -> Quad<FigurePipeline> {
    Quad::new(
        FigureVertex::new(origin, norm, col, bone),
        FigureVertex::new(origin + unit_x, norm, col, bone),
        FigureVertex::new(origin + unit_x + unit_y, norm, col, bone),
        FigureVertex::new(origin + unit_y, norm, col, bone),
    )
}

impl Meshable for Segment {
    type Pipeline = FigurePipeline;

    fn generate_mesh_with_offset(&self, offs: Vec3<f32>) -> Mesh<FigurePipeline> {
        let mut mesh = Mesh::new();

        for pos in self.iter_positions() {
            if let Some(col) = self
                .get(pos)
                .ok()
                .and_then(|vox| vox.get_color())
            {
                let col = col.map(|e| e as f32 / 255.0);

                // TODO: Face occlusion

                // -x
                mesh.push_quad(create_quad(
                    offs + pos.map(|e| e as f32) + Vec3::unit_y(),
                    -Vec3::unit_y(),
                    Vec3::unit_z(),
                    -Vec3::unit_x(),
                    col,
                    0,
                ));
                // +x
                mesh.push_quad(create_quad(
                    offs + pos.map(|e| e as f32) + Vec3::unit_x(),
                    Vec3::unit_y(),
                    Vec3::unit_z(),
                    Vec3::unit_x(),
                    col,
                    0,
                ));
                // -y
                mesh.push_quad(create_quad(
                    offs + pos.map(|e| e as f32),
                    Vec3::unit_x(),
                    Vec3::unit_z(),
                    -Vec3::unit_y(),
                    col,
                    0,
                ));
                // +y
                mesh.push_quad(create_quad(
                    offs + pos.map(|e| e as f32) + Vec3::unit_y(),
                    Vec3::unit_z(),
                    Vec3::unit_x(),
                    Vec3::unit_y(),
                    col,
                    0,
                ));
                // -z
                mesh.push_quad(create_quad(
                    offs + pos.map(|e| e as f32),
                    Vec3::unit_y(),
                    Vec3::unit_x(),
                    -Vec3::unit_z(),
                    col,
                    0,
                ));
                // +z
                mesh.push_quad(create_quad(
                    offs + pos.map(|e| e as f32) + Vec3::unit_z(),
                    Vec3::unit_x(),
                    Vec3::unit_y(),
                    Vec3::unit_z(),
                    col,
                    0,
                ));
            }
        }

        mesh
    }
}
