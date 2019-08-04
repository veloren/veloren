use crate::{
    mesh::{vol, Meshable},
    render::{self, FigurePipeline, Mesh},
};
use common::{
    figure::Segment,
    util::{linear_to_srgb, srgb_to_linear},
    vol::{ReadVol, SizedVol},
};
use vek::*;

type FigureVertex = <FigurePipeline as render::Pipeline>::Vertex;

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
                    |origin, norm, col, ao, light| {
                        FigureVertex::new(
                            origin,
                            norm,
                            linear_to_srgb(srgb_to_linear(col) * ao * light),
                            0,
                        )
                    },
                    true,
                    &[[[1.0; 3]; 3]; 3],
                );
            }
        }

        mesh
    }
}
