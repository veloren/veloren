use crate::{
    mesh::{
        greedy::{self, GreedyConfig, GreedyMesh},
        MeshGen, Meshable,
    },
    render::{
        self, FigurePipeline, Mesh, ParticlePipeline, ShadowPipeline, SpritePipeline,
        TerrainPipeline,
    },
    scene::math,
};
use common::{
    figure::Cell,
    vol::{BaseVol, ReadVol, SizedVol, Vox},
};
use vek::*;

type SpriteVertex = <SpritePipeline as render::Pipeline>::Vertex;
type TerrainVertex = <TerrainPipeline as render::Pipeline>::Vertex;
type ParticleVertex = <ParticlePipeline as render::Pipeline>::Vertex;

impl<'a: 'b, 'b, V: 'a> Meshable<FigurePipeline, &'b mut GreedyMesh<'a>> for V
where
    V: BaseVol<Vox = Cell> + ReadVol + SizedVol,
    /* TODO: Use VolIterator instead of manually iterating
     * &'a V: IntoVolIterator<'a> + IntoFullVolIterator<'a>,
     * &'a V: BaseVol<Vox=Cell>, */
{
    type Pipeline = TerrainPipeline;
    type Result = math::Aabb<f32>;
    type ShadowPipeline = ShadowPipeline;
    /// NOTE: bone_idx must be in [0, 15] (may be bumped to [0, 31] at some
    /// point).
    type Supplement = (
        &'b mut GreedyMesh<'a>,
        &'b mut Mesh<Self::Pipeline>,
        Vec3<f32>,
        Vec3<f32>,
        u8,
    );
    type TranslucentPipeline = FigurePipeline;

    #[allow(clippy::or_fun_call)] // TODO: Pending review in #587
    fn generate_mesh(
        self,
        (greedy, opaque_mesh, offs, scale, bone_idx): Self::Supplement,
    ) -> MeshGen<FigurePipeline, &'b mut GreedyMesh<'a>, Self> {
        assert!(bone_idx <= 15, "Bone index for figures must be in [0, 15]");

        let max_size = greedy.max_size();
        // NOTE: Required because we steal two bits from the normal in the shadow uint
        // in order to store the bone index.  The two bits are instead taken out
        // of the atlas coordinates, which is why we "only" allow 1 << 15 per
        // coordinate instead of 1 << 16.
        assert!(max_size.width.max(max_size.height) < 1 << 15);

        let lower_bound = self.lower_bound();
        let upper_bound = self.upper_bound();
        assert!(
            lower_bound.x <= upper_bound.x
                && lower_bound.y <= upper_bound.y
                && lower_bound.z <= upper_bound.z
        );
        // NOTE: Figure sizes should be no more than 512 along each axis.
        let greedy_size = upper_bound - lower_bound + 1;
        assert!(greedy_size.x <= 512 && greedy_size.y <= 512 && greedy_size.z <= 512);
        // NOTE: Cast to usize is safe because of previous check, since all values fit
        // into u16 which is safe to cast to usize.
        let greedy_size = greedy_size.as_::<usize>();
        let greedy_size_cross = greedy_size;
        let draw_delta = lower_bound;

        let get_light = |vol: &mut V, pos: Vec3<i32>| {
            if vol.get(pos).map(|vox| vox.is_empty()).unwrap_or(true) {
                1.0
            } else {
                0.0
            }
        };
        let get_glow = |_vol: &mut V, _pos: Vec3<i32>| 0.0;
        let get_color = |vol: &mut V, pos: Vec3<i32>| {
            vol.get(pos)
                .ok()
                .and_then(|vox| vox.get_color())
                .unwrap_or(Rgb::zero())
        };
        let get_opacity =
            |vol: &mut V, pos: Vec3<i32>| vol.get(pos).map(|vox| vox.is_empty()).unwrap_or(true);
        let should_draw = |vol: &mut V, pos: Vec3<i32>, delta: Vec3<i32>, uv| {
            should_draw_greedy(pos, delta, uv, |vox| {
                vol.get(vox).map(|vox| *vox).unwrap_or(Vox::empty())
            })
        };
        let create_opaque = |atlas_pos, pos, norm| {
            TerrainVertex::new_figure(atlas_pos, (pos + offs) * scale, norm, bone_idx)
        };

        greedy.push(GreedyConfig {
            data: self,
            draw_delta,
            greedy_size,
            greedy_size_cross,
            get_light,
            get_glow,
            get_color,
            get_opacity,
            should_draw,
            push_quad: |atlas_origin, dim, origin, draw_dim, norm, meta: &()| {
                opaque_mesh.push_quad(greedy::create_quad(
                    atlas_origin,
                    dim,
                    origin,
                    draw_dim,
                    norm,
                    meta,
                    |atlas_pos, pos, norm, &_meta| create_opaque(atlas_pos, pos, norm),
                ));
            },
        });
        let bounds = math::Aabb {
            // NOTE: Casts are safe since lower_bound and upper_bound both fit in a i16.
            min: math::Vec3::from((lower_bound.as_::<f32>() + offs) * scale),
            max: math::Vec3::from((upper_bound.as_::<f32>() + offs) * scale),
        }
        .made_valid();

        (Mesh::new(), Mesh::new(), Mesh::new(), bounds)
    }
}

impl<'a: 'b, 'b, V: 'a> Meshable<SpritePipeline, &'b mut GreedyMesh<'a>> for V
where
    V: BaseVol<Vox = Cell> + ReadVol + SizedVol,
    /* TODO: Use VolIterator instead of manually iterating
     * &'a V: IntoVolIterator<'a> + IntoFullVolIterator<'a>,
     * &'a V: BaseVol<Vox=Cell>, */
{
    type Pipeline = SpritePipeline;
    type Result = ();
    type ShadowPipeline = ShadowPipeline;
    type Supplement = (&'b mut GreedyMesh<'a>, &'b mut Mesh<Self::Pipeline>, bool);
    type TranslucentPipeline = SpritePipeline;

    #[allow(clippy::or_fun_call)] // TODO: Pending review in #587
    fn generate_mesh(
        self,
        (greedy, opaque_mesh, vertical_stripes): Self::Supplement,
    ) -> MeshGen<SpritePipeline, &'b mut GreedyMesh<'a>, Self> {
        let max_size = greedy.max_size();
        // NOTE: Required because we steal two bits from the normal in the shadow uint
        // in order to store the bone index.  The two bits are instead taken out
        // of the atlas coordinates, which is why we "only" allow 1 << 15 per
        // coordinate instead of 1 << 16.
        assert!(max_size.width.max(max_size.height) < 1 << 16);

        let lower_bound = self.lower_bound();
        let upper_bound = self.upper_bound();
        assert!(
            lower_bound.x <= upper_bound.x
                && lower_bound.y <= upper_bound.y
                && lower_bound.z <= upper_bound.z
        );
        let greedy_size = upper_bound - lower_bound + 1;
        // TODO: Should this be 16, 16, 64?
        assert!(
            greedy_size.x <= 32 && greedy_size.y <= 32 && greedy_size.z <= 64,
            "Sprite size out of bounds: {:?} ≤ (31, 31, 63)",
            greedy_size - 1
        );
        // NOTE: Cast to usize is safe because of previous check, since all values fit
        // into u16 which is safe to cast to usize.
        let greedy_size = greedy_size.as_::<usize>();

        let greedy_size_cross = greedy_size;
        let draw_delta = lower_bound;

        let get_light = |vol: &mut V, pos: Vec3<i32>| {
            if vol.get(pos).map(|vox| vox.is_empty()).unwrap_or(true) {
                1.0
            } else {
                0.0
            }
        };
        let get_glow = |_vol: &mut V, _pos: Vec3<i32>| 0.0;
        let get_color = |vol: &mut V, pos: Vec3<i32>| {
            vol.get(pos)
                .ok()
                .and_then(|vox| vox.get_color())
                .unwrap_or(Rgb::zero())
        };
        let get_opacity =
            |vol: &mut V, pos: Vec3<i32>| vol.get(pos).map(|vox| vox.is_empty()).unwrap_or(true);
        let should_draw = |vol: &mut V, pos: Vec3<i32>, delta: Vec3<i32>, uv| {
            should_draw_greedy_ao(vertical_stripes, pos, delta, uv, |vox| {
                vol.get(vox).map(|vox| *vox).unwrap_or(Vox::empty())
            })
        };
        let create_opaque =
            |atlas_pos, pos: Vec3<f32>, norm, _meta| SpriteVertex::new(atlas_pos, pos, norm);

        greedy.push(GreedyConfig {
            data: self,
            draw_delta,
            greedy_size,
            greedy_size_cross,
            get_light,
            get_glow,
            get_color,
            get_opacity,
            should_draw,
            push_quad: |atlas_origin, dim, origin, draw_dim, norm, meta: &bool| {
                opaque_mesh.push_quad(greedy::create_quad(
                    atlas_origin,
                    dim,
                    origin,
                    draw_dim,
                    norm,
                    meta,
                    |atlas_pos, pos, norm, &meta| create_opaque(atlas_pos, pos, norm, meta),
                ));
            },
        });

        (Mesh::new(), Mesh::new(), Mesh::new(), ())
    }
}

impl<'a: 'b, 'b, V: 'a> Meshable<ParticlePipeline, &'b mut GreedyMesh<'a>> for V
where
    V: BaseVol<Vox = Cell> + ReadVol + SizedVol,
    /* TODO: Use VolIterator instead of manually iterating
     * &'a V: IntoVolIterator<'a> + IntoFullVolIterator<'a>,
     * &'a V: BaseVol<Vox=Cell>, */
{
    type Pipeline = ParticlePipeline;
    type Result = ();
    type ShadowPipeline = ShadowPipeline;
    type Supplement = &'b mut GreedyMesh<'a>;
    type TranslucentPipeline = ParticlePipeline;

    #[allow(clippy::or_fun_call)] // TODO: Pending review in #587
    fn generate_mesh(
        self,
        greedy: Self::Supplement,
    ) -> MeshGen<ParticlePipeline, &'b mut GreedyMesh<'a>, Self> {
        let max_size = greedy.max_size();
        // NOTE: Required because we steal two bits from the normal in the shadow uint
        // in order to store the bone index.  The two bits are instead taken out
        // of the atlas coordinates, which is why we "only" allow 1 << 15 per
        // coordinate instead of 1 << 16.
        assert!(max_size.width.max(max_size.height) < 1 << 16);

        let lower_bound = self.lower_bound();
        let upper_bound = self.upper_bound();
        assert!(
            lower_bound.x <= upper_bound.x
                && lower_bound.y <= upper_bound.y
                && lower_bound.z <= upper_bound.z
        );
        let greedy_size = upper_bound - lower_bound + 1;
        assert!(
            greedy_size.x <= 16 && greedy_size.y <= 16 && greedy_size.z <= 64,
            "Particle size out of bounds: {:?} ≤ (15, 15, 63)",
            greedy_size - 1
        );
        // NOTE: Cast to usize is safe because of previous check, since all values fit
        // into u16 which is safe to cast to usize.
        let greedy_size = greedy_size.as_::<usize>();

        let greedy_size_cross = greedy_size;
        let draw_delta = lower_bound;

        let get_light = |vol: &mut V, pos: Vec3<i32>| {
            if vol.get(pos).map(|vox| vox.is_empty()).unwrap_or(true) {
                1.0
            } else {
                0.0
            }
        };
        let get_glow = |_vol: &mut V, _pos: Vec3<i32>| 0.0;
        let get_color = |vol: &mut V, pos: Vec3<i32>| {
            vol.get(pos)
                .ok()
                .and_then(|vox| vox.get_color())
                .unwrap_or(Rgb::zero())
        };
        let get_opacity =
            |vol: &mut V, pos: Vec3<i32>| vol.get(pos).map(|vox| vox.is_empty()).unwrap_or(true);
        let should_draw = |vol: &mut V, pos: Vec3<i32>, delta: Vec3<i32>, uv| {
            should_draw_greedy(pos, delta, uv, |vox| {
                vol.get(vox).map(|vox| *vox).unwrap_or(Vox::empty())
            })
        };
        let create_opaque = |_atlas_pos, pos: Vec3<f32>, norm| ParticleVertex::new(pos, norm);

        let mut opaque_mesh = Mesh::new();
        greedy.push(GreedyConfig {
            data: self,
            draw_delta,
            greedy_size,
            greedy_size_cross,
            get_light,
            get_glow,
            get_color,
            get_opacity,
            should_draw,
            push_quad: |atlas_origin, dim, origin, draw_dim, norm, meta: &()| {
                opaque_mesh.push_quad(greedy::create_quad(
                    atlas_origin,
                    dim,
                    origin,
                    draw_dim,
                    norm,
                    meta,
                    |atlas_pos, pos, norm, &_meta| create_opaque(atlas_pos, pos, norm),
                ));
            },
        });

        (opaque_mesh, Mesh::new(), Mesh::new(), ())
    }
}

fn should_draw_greedy(
    pos: Vec3<i32>,
    delta: Vec3<i32>,
    _uv: Vec2<Vec3<i32>>,
    flat_get: impl Fn(Vec3<i32>) -> Cell,
) -> Option<(bool, /* u8 */ ())> {
    let from = flat_get(pos - delta);
    let to = flat_get(pos);
    let from_opaque = !from.is_empty();
    if from_opaque != to.is_empty() {
        None
    } else {
        // If going from transparent to opaque, backward facing; otherwise, forward
        // facing.
        Some((from_opaque, ()))
    }
}

fn should_draw_greedy_ao(
    vertical_stripes: bool,
    pos: Vec3<i32>,
    delta: Vec3<i32>,
    _uv: Vec2<Vec3<i32>>,
    flat_get: impl Fn(Vec3<i32>) -> Cell,
) -> Option<(bool, bool)> {
    let from = flat_get(pos - delta);
    let to = flat_get(pos);
    let from_opaque = !from.is_empty();
    if from_opaque != to.is_empty() {
        None
    } else {
        let faces_forward = from_opaque;
        let ao = !vertical_stripes || (pos.z & 1) != 0;
        // If going from transparent to opaque, backward facing; otherwise, forward
        // facing.
        Some((faces_forward, ao))
    }
}
