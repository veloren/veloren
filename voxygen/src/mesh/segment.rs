use crate::{
    mesh::{vol, Meshable},
    render::{self, FigurePipeline, Mesh, SpritePipeline},
};
use common::{
    figure::Segment,
    util::{linear_to_srgb, srgb_to_linear},
    vol::{IntoFullVolIterator, ReadVol, Vox},
};
use vek::*;

type FigureVertex = <FigurePipeline as render::Pipeline>::Vertex;
type SpriteVertex = <SpritePipeline as render::Pipeline>::Vertex;

impl Meshable<FigurePipeline, FigurePipeline> for Segment {
    type Pipeline = FigurePipeline;
    type TranslucentPipeline = FigurePipeline;
    type Supplement = Vec3<f32>;

    fn generate_mesh(
        &self,
        offs: Self::Supplement,
    ) -> (Mesh<Self::Pipeline>, Mesh<Self::TranslucentPipeline>) {
        let mut mesh = Mesh::new();

        for (pos, vox) in self.full_vol_iter() {
            if let Some(col) = vox.get_color() {
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
                    &{
                        let mut ls = [[[0.0; 3]; 3]; 3];
                        for x in 0..3 {
                            for y in 0..3 {
                                for z in 0..3 {
                                    ls[x][y][z] = if self
                                        .get(pos + Vec3::new(x as i32, y as i32, z as i32) - 1)
                                        .map(|v| v.is_empty())
                                        .unwrap_or(true)
                                    {
                                        1.0
                                    } else {
                                        0.0
                                    };
                                }
                            }
                        }
                        ls
                    },
                    |vox| vox.is_empty(),
                    |vox| !vox.is_empty(),
                );
            }
        }

        (mesh, Mesh::new())
    }
}

impl Meshable<SpritePipeline, SpritePipeline> for Segment {
    type Pipeline = SpritePipeline;
    type TranslucentPipeline = SpritePipeline;
    type Supplement = Vec3<f32>;

    fn generate_mesh(
        &self,
        offs: Self::Supplement,
    ) -> (Mesh<Self::Pipeline>, Mesh<Self::TranslucentPipeline>) {
        let mut mesh = Mesh::new();

        for (pos, vox) in self.full_vol_iter() {
            if let Some(col) = vox.get_color() {
                let col = col.map(|e| e as f32 / 255.0);

                vol::push_vox_verts(
                    &mut mesh,
                    self,
                    pos,
                    offs + pos.map(|e| e as f32),
                    col,
                    |origin, norm, col, ao, light| {
                        SpriteVertex::new(
                            origin,
                            norm,
                            linear_to_srgb(srgb_to_linear(col) * ao * light),
                        )
                    },
                    true,
                    &[[[1.0; 3]; 3]; 3],
                    |vox| vox.is_empty(),
                    |vox| !vox.is_empty(),
                );
            }
        }

        (mesh, Mesh::new())
    }
}
