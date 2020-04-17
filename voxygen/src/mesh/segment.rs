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
    type Supplement = Vec3<f32>;
    type TranslucentPipeline = FigurePipeline;

    fn generate_mesh(
        &self,
        offs: Self::Supplement,
    ) -> (Mesh<Self::Pipeline>, Mesh<Self::TranslucentPipeline>) {
        let mut mesh = Mesh::new();

        for (pos, vox) in self.full_vol_iter() {
            if let Some(col) = vox.get_color() {
                vol::push_vox_verts(
                    &mut mesh,
                    faces_to_make(self, pos, true, |vox| vox.is_empty()),
                    offs + pos.map(|e| e as f32),
                    &[[[Rgba::from_opaque(col); 3]; 3]; 3],
                    |origin, norm, col, light, ao| {
                        FigureVertex::new(
                            origin,
                            norm,
                            linear_to_srgb(srgb_to_linear(col) * light),
                            ao,
                            0,
                        )
                    },
                    &{
                        let mut ls = [[[None; 3]; 3]; 3];
                        for x in 0..3 {
                            for y in 0..3 {
                                for z in 0..3 {
                                    ls[z][y][x] = if self
                                        .get(pos + Vec3::new(x as i32, y as i32, z as i32) - 1)
                                        .map(|v| v.is_empty())
                                        .unwrap_or(true)
                                    {
                                        Some(1.0)
                                    } else {
                                        None
                                    };
                                }
                            }
                        }
                        ls
                    },
                );
            }
        }

        (mesh, Mesh::new())
    }
}

impl Meshable<SpritePipeline, SpritePipeline> for Segment {
    type Pipeline = SpritePipeline;
    type Supplement = Vec3<f32>;
    type TranslucentPipeline = SpritePipeline;

    fn generate_mesh(
        &self,
        offs: Self::Supplement,
    ) -> (Mesh<Self::Pipeline>, Mesh<Self::TranslucentPipeline>) {
        let mut mesh = Mesh::new();

        for (pos, vox) in self.full_vol_iter() {
            if let Some(col) = vox.get_color() {
                vol::push_vox_verts(
                    &mut mesh,
                    faces_to_make(self, pos, true, |vox| vox.is_empty()),
                    offs + pos.map(|e| e as f32),
                    &[[[Rgba::from_opaque(col); 3]; 3]; 3],
                    |origin, norm, col, light, ao| {
                        SpriteVertex::new(
                            origin,
                            norm,
                            linear_to_srgb(srgb_to_linear(col) * light.min(ao.powf(0.5) * 0.75 + 0.25)),
                        )
                    },
                    &{
                        let mut ls = [[[None; 3]; 3]; 3];
                        for x in 0..3 {
                            for y in 0..3 {
                                for z in 0..3 {
                                    ls[z][y][x] = if self
                                        .get(pos + Vec3::new(x as i32, y as i32, z as i32) - 1)
                                        .map(|v| v.is_empty())
                                        .unwrap_or(true)
                                    {
                                        Some(1.0)
                                    } else {
                                        None
                                    };
                                }
                            }
                        }
                        ls
                    },
                );
            }
        }

        (mesh, Mesh::new())
    }
}

/// Use the 6 voxels/blocks surrounding the one at the specified position
/// to detemine which faces should be drawn
fn faces_to_make<V: ReadVol>(
    seg: &V,
    pos: Vec3<i32>,
    error_makes_face: bool,
    should_add: impl Fn(&V::Vox) -> bool,
) -> [bool; 6] {
    let (x, y, z) = (Vec3::unit_x(), Vec3::unit_y(), Vec3::unit_z());
    let make_face = |offset| {
        seg.get(pos + offset)
            .map(|v| should_add(v))
            .unwrap_or(error_makes_face)
    };
    [
        make_face(-x),
        make_face(x),
        make_face(-y),
        make_face(y),
        make_face(-z),
        make_face(z),
    ]
}
