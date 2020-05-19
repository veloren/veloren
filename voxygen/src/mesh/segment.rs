use crate::{
    mesh::{vol, Meshable},
    render::{self, FigurePipeline, Mesh, SpritePipeline},
};
use common::{
    figure::Cell,
    util::{linear_to_srgb, srgb_to_linear},
    vol::{BaseVol, ReadVol, SizedVol, Vox},
};
use vek::*;

type FigureVertex = <FigurePipeline as render::Pipeline>::Vertex;
type SpriteVertex = <SpritePipeline as render::Pipeline>::Vertex;

impl<'a, V: 'a> Meshable<'a, FigurePipeline, FigurePipeline> for V
where
    V: BaseVol<Vox = Cell> + ReadVol + SizedVol,
    /* TODO: Use VolIterator instead of manually iterating
     * &'a V: IntoVolIterator<'a> + IntoFullVolIterator<'a>,
     * &'a V: BaseVol<Vox=Cell>, */
{
    type Pipeline = FigurePipeline;
    type ShadowPipeline = FigurePipeline;
    type Supplement = (Vec3<f32>, Vec3<f32>);
    type TranslucentPipeline = FigurePipeline;

    // TODO: Make sprites cast shadows?

    fn generate_mesh(
        &'a self,
        (offs, scale): Self::Supplement,
    ) -> (
        Mesh<Self::Pipeline>,
        Mesh<Self::TranslucentPipeline>,
        Mesh<Self::ShadowPipeline>,
    ) {
        let mut mesh = Mesh::new();

        let vol_iter = (self.lower_bound().x..self.upper_bound().x)
            .map(|i| {
                (self.lower_bound().y..self.upper_bound().y).map(move |j| {
                    (self.lower_bound().z..self.upper_bound().z).map(move |k| Vec3::new(i, j, k))
                })
            })
            .flatten()
            .flatten()
            .map(|pos| (pos, self.get(pos).map(|x| *x).unwrap_or(Vox::empty())));

        for (pos, vox) in vol_iter {
            if let Some(col) = vox.get_color() {
                vol::push_vox_verts(
                    &mut mesh,
                    faces_to_make(self, pos, true, |vox| vox.is_empty()),
                    offs + pos.map(|e| e as f32),
                    &[[[Rgba::from_opaque(col); 3]; 3]; 3],
                    |origin, norm, col, light, ao, _meta| {
                        FigureVertex::new(
                            origin * scale,
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
                                    ls[z][y][x] = self
                                        .get(pos + Vec3::new(x as i32, y as i32, z as i32) - 1)
                                        .map(|v| v.is_empty())
                                        .unwrap_or(true)
                                        .then_some(1.0);
                                }
                            }
                        }
                        ls
                    },
                );
            }
        }

        (mesh, Mesh::new(), Mesh::new())
    }
}

impl<'a, V: 'a> Meshable<'a, SpritePipeline, SpritePipeline> for V
where
    V: BaseVol<Vox = Cell> + ReadVol + SizedVol,
    /* TODO: Use VolIterator instead of manually iterating
     * &'a V: IntoVolIterator<'a> + IntoFullVolIterator<'a>,
     * &'a V: BaseVol<Vox=Cell>, */
{
    type Pipeline = SpritePipeline;
    type ShadowPipeline = SpritePipeline;
    type Supplement = (Vec3<f32>, Vec3<f32>);
    type TranslucentPipeline = SpritePipeline;

    // TODO: Make sprites cast shadows?

    fn generate_mesh(
        &'a self,
        (offs, scale): Self::Supplement,
    ) -> (
        Mesh<Self::Pipeline>,
        Mesh<Self::TranslucentPipeline>,
        Mesh<Self::ShadowPipeline>,
    ) {
        let mut mesh = Mesh::new();

        let vol_iter = (self.lower_bound().x..self.upper_bound().x)
            .map(|i| {
                (self.lower_bound().y..self.upper_bound().y).map(move |j| {
                    (self.lower_bound().z..self.upper_bound().z).map(move |k| Vec3::new(i, j, k))
                })
            })
            .flatten()
            .flatten()
            .map(|pos| (pos, self.get(pos).map(|x| *x).unwrap_or(Vox::empty())));

        for (pos, vox) in vol_iter {
            if let Some(col) = vox.get_color() {
                vol::push_vox_verts(
                    &mut mesh,
                    faces_to_make(self, pos, true, |vox| vox.is_empty()),
                    offs + pos.map(|e| e as f32),
                    &[[[Rgba::from_opaque(col); 3]; 3]; 3],
                    |origin, norm, col, light, ao, _meta| {
                        SpriteVertex::new(
                            origin * scale,
                            norm,
                            linear_to_srgb(srgb_to_linear(col) * light),
                            ao,
                        )
                    },
                    &{
                        let mut ls = [[[None; 3]; 3]; 3];
                        for x in 0..3 {
                            for y in 0..3 {
                                for z in 0..3 {
                                    ls[z][y][x] = self
                                        .get(pos + Vec3::new(x as i32, y as i32, z as i32) - 1)
                                        .map(|v| v.is_empty())
                                        .unwrap_or(true)
                                        .then_some(1.0);
                                }
                            }
                        }
                        ls
                    },
                );
            }
        }

        (mesh, Mesh::new(), Mesh::new())
    }
}

/// Use the 6 voxels/blocks surrounding the one at the specified position
/// to detemine which faces should be drawn
fn faces_to_make<V: ReadVol>(
    seg: &V,
    pos: Vec3<i32>,
    error_makes_face: bool,
    should_add: impl Fn(&V::Vox) -> bool,
) -> [Option<()>; 6] {
    let (x, y, z) = (Vec3::unit_x(), Vec3::unit_y(), Vec3::unit_z());
    let make_face = |offset| {
        let res = seg
            .get(pos + offset)
            .map(|v| should_add(v))
            .unwrap_or(error_makes_face);
        if res { Some(()) } else { None }
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
