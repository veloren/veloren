// Library
use vek::*;

// Project
use common::{
    figure::Segment,
    vol::{ReadVol, SizedVol},
};

// Crate
use crate::{
    mesh::{vol, Meshable},
    render::{self, FigurePipeline, Mesh},
};

type FigureVertex = <FigurePipeline as render::Pipeline>::Vertex;

fn create_vertex(origin: Vec3<f32>, norm: Vec3<f32>, col: Rgb<f32>) -> FigureVertex {
    FigureVertex::new(origin, norm, col, 0)
}

impl Meshable for Segment {
    type Pipeline = FigurePipeline;
    type Supplement = Vec3<f32>;

    fn generate_mesh(&self, offs: Self::Supplement) -> Mesh<Self::Pipeline> {
        let mut mesh = Mesh::new();

        for pos in self.iter_positions() {
            if let Some(col) = self.get(pos).ok().and_then(|vox| vox.get_color()) {
                let col = col.map(|e| e as f32 / 255.0);

                vol::push_vox_verts(
                    &mut mesh,
                    self,
                    pos,
                    offs + pos.map(|e| e as f32),
                    col,
                    create_vertex,
                    true,
                );
            }
        }

        mesh
    }
}
