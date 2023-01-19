use crate::{
    mesh::{
        greedy::{self, GreedyConfig, GreedyMesh},
        MeshGen,
    },
    render::{Mesh, ParticleVertex, SpriteVertex, TerrainVertex},
    scene::math,
};
use common::{
    figure::Cell,
    vol::{BaseVol, ReadVol, SizedVol, Vox},
};
use core::convert::TryFrom;
use vek::*;

//    /// NOTE: bone_idx must be in [0, 15] (may be bumped to [0, 31] at some
//    /// point).
// TODO: this function name...
pub fn generate_mesh_base_vol_terrain<'a: 'b, 'b, V: 'a>(
    vol: V,
    (greedy, opaque_mesh, offs, scale, bone_idx): (
        &'b mut GreedyMesh<'a>,
        &'b mut Mesh<TerrainVertex>,
        Vec3<f32>,
        Vec3<f32>,
        u8,
    ),
) -> MeshGen<TerrainVertex, TerrainVertex, TerrainVertex, math::Aabb<f32>>
where
    V: BaseVol<Vox = Cell> + ReadVol + SizedVol,
{
    assert!(bone_idx <= 15, "Bone index for figures must be in [0, 15]");
    let max_size = greedy.max_size();
    // NOTE: Required because we steal two bits from the normal in the shadow uint
    // in order to store the bone index.  The two bits are instead taken out
    // of the atlas coordinates, which is why we "only" allow 1 << 15 per
    // coordinate instead of 1 << 16.
    assert!(max_size.reduce_max() < 1 << 15);

    let lower_bound = vol.lower_bound();
    let upper_bound = vol.upper_bound();
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
    let get_opacity = |vol: &mut V, pos: Vec3<i32>| vol.get(pos).map_or(true, |vox| vox.is_empty());
    let should_draw = |vol: &mut V, pos: Vec3<i32>, delta: Vec3<i32>, uv| {
        should_draw_greedy(pos, delta, uv, |vox| {
            vol.get(vox)
                .map(|vox| *vox)
                .unwrap_or_else(|_| Cell::empty())
        })
    };
    let create_opaque = |atlas_pos, pos, norm| {
        TerrainVertex::new_figure(atlas_pos, (pos + offs) * scale, norm, bone_idx)
    };

    greedy.push(GreedyConfig {
        data: vol,
        draw_delta,
        greedy_size,
        greedy_size_cross,
        get_ao: |_: &mut V, _: Vec3<i32>| 1.0,
        get_light,
        get_glow,
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
        make_face_texel: |vol: &mut V, pos, light, _, _| {
            let cell = vol.get(pos).ok();
            let (glowy, shiny) = cell
                .map(|c| (c.is_glowy(), c.is_shiny()))
                .unwrap_or_default();
            let col = cell
                .and_then(|vox| vox.get_color())
                .unwrap_or_else(Rgb::zero);
            TerrainVertex::make_col_light_figure(light, glowy, shiny, col)
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

pub fn generate_mesh_base_vol_sprite<'a: 'b, 'b, V: 'a>(
    vol: V,
    (greedy, opaque_mesh, vertical_stripes): (
        &'b mut GreedyMesh<'a, greedy::SpriteAtlasAllocator>,
        &'b mut Mesh<SpriteVertex>,
        bool,
    ),
    offset: Vec3<f32>,
) -> MeshGen<SpriteVertex, SpriteVertex, TerrainVertex, ()>
where
    V: BaseVol<Vox = Cell> + ReadVol + SizedVol,
{
    let max_size = greedy.max_size();
    // NOTE: Required because we steal two bits from the normal in the shadow uint
    // in order to store the bone index.  The two bits are instead taken out
    // of the atlas coordinates, which is why we "only" allow 1 << 15 per
    // coordinate instead of 1 << 16.
    assert!(u32::from(max_size.reduce_max()) < 1 << 16);

    let lower_bound = vol.lower_bound();
    let upper_bound = vol.upper_bound();
    assert!(
        lower_bound.x <= upper_bound.x
            && lower_bound.y <= upper_bound.y
            && lower_bound.z <= upper_bound.z
    );
    // Lower bound coordinates must fit in an i16 (which means upper bound
    // coordinates fit as integers in a f23).
    assert!(
        i16::try_from(lower_bound.x).is_ok()
            && i16::try_from(lower_bound.y).is_ok()
            && i16::try_from(lower_bound.z).is_ok(),
        "Sprite offsets should fit in i16",
    );
    let greedy_size = upper_bound - lower_bound + 1;
    // TODO: Should this be 16, 16, 64?
    assert!(
        greedy_size.x <= 32 && greedy_size.y <= 32 && greedy_size.z <= 64,
        "Sprite size out of bounds: {:?} ≤ (31, 31, 63)",
        greedy_size - 1
    );

    let (flat, flat_get) = {
        let (w, h, d) = (greedy_size + 2).into_tuple();
        let flat = {
            let mut flat = vec![Cell::empty(); (w * h * d) as usize];
            let mut i = 0;
            for x in -1..greedy_size.x + 1 {
                for y in -1..greedy_size.y + 1 {
                    for z in -1..greedy_size.z + 1 {
                        let wpos = lower_bound + Vec3::new(x, y, z);
                        let block = vol.get(wpos).map(|b| *b).unwrap_or_else(|_| Cell::empty());
                        flat[i] = block;
                        i += 1;
                    }
                }
            }
            flat
        };

        let flat_get = move |flat: &Vec<Cell>, Vec3 { x, y, z }| match flat
            .get((x * h * d + y * d + z) as usize)
            .copied()
        {
            Some(b) => b,
            None => panic!("x {} y {} z {} d {} h {}", x, y, z, d, h),
        };

        (flat, flat_get)
    };

    // NOTE: Cast to usize is safe because of previous check, since all values fit
    // into u16 which is safe to cast to usize.
    let greedy_size = greedy_size.as_::<usize>();

    let greedy_size_cross = greedy_size;
    let draw_delta = Vec3::new(1, 1, 1);

    let get_light = move |flat: &mut _, pos: Vec3<i32>| {
        if flat_get(flat, pos).is_empty() {
            1.0
        } else {
            0.0
        }
    };
    let get_glow = |_flat: &mut _, _pos: Vec3<i32>| 0.0;
    let get_color = move |flat: &mut _, pos: Vec3<i32>| {
        flat_get(flat, pos).get_color().unwrap_or_else(Rgb::zero)
    };
    let get_opacity = move |flat: &mut _, pos: Vec3<i32>| flat_get(flat, pos).is_empty();
    let should_draw = move |flat: &mut _, pos: Vec3<i32>, delta: Vec3<i32>, uv| {
        should_draw_greedy_ao(vertical_stripes, pos, delta, uv, |vox| flat_get(flat, vox))
    };
    // NOTE: Fits in i16 (much lower actually) so f32 is no problem (and the final
    // position, pos + mesh_delta, is guaranteed to fit in an f32).
    let mesh_delta = lower_bound.as_::<f32>();
    let create_opaque = |atlas_pos, pos: Vec3<f32>, norm, _meta| {
        SpriteVertex::new(atlas_pos, pos + offset + mesh_delta, norm)
    };

    greedy.push(GreedyConfig {
        data: flat,
        draw_delta,
        greedy_size,
        greedy_size_cross,
        get_ao: |_: &mut _, _: Vec3<i32>| 1.0,
        get_light,
        get_glow,
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
        make_face_texel: move |flat: &mut _, pos, light, _glow, _ao| {
            let cell = flat_get(flat, pos);
            let (glowy, shiny) = (cell.is_glowy(), cell.is_shiny());
            TerrainVertex::make_col_light_figure(light, glowy, shiny, get_color(flat, pos))
        },
    });

    (Mesh::new(), Mesh::new(), Mesh::new(), ())
}

pub fn generate_mesh_base_vol_particle<'a: 'b, 'b, V: 'a>(
    vol: V,
    greedy: &'b mut GreedyMesh<'a>,
) -> MeshGen<ParticleVertex, ParticleVertex, TerrainVertex, ()>
where
    V: BaseVol<Vox = Cell> + ReadVol + SizedVol,
{
    let max_size = greedy.max_size();
    // NOTE: Required because we steal two bits from the normal in the shadow uint
    // in order to store the bone index.  The two bits are instead taken out
    // of the atlas coordinates, which is why we "only" allow 1 << 15 per
    // coordinate instead of 1 << 16.
    assert!(u32::from(max_size.reduce_max()) < 1 << 16);

    let lower_bound = vol.lower_bound();
    let upper_bound = vol.upper_bound();
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
            .unwrap_or_else(Rgb::zero)
    };
    let get_opacity = |vol: &mut V, pos: Vec3<i32>| vol.get(pos).map_or(true, |vox| vox.is_empty());
    let should_draw = |vol: &mut V, pos: Vec3<i32>, delta: Vec3<i32>, uv| {
        should_draw_greedy(pos, delta, uv, |vox| {
            vol.get(vox)
                .map(|vox| *vox)
                .unwrap_or_else(|_| Cell::empty())
        })
    };
    let create_opaque = |_atlas_pos, pos: Vec3<f32>, norm| ParticleVertex::new(pos, norm);

    let mut opaque_mesh = Mesh::new();
    greedy.push(GreedyConfig {
        data: vol,
        draw_delta,
        greedy_size,
        greedy_size_cross,
        get_ao: |_: &mut V, _: Vec3<i32>| 1.0,
        get_light,
        get_glow,
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
        make_face_texel: move |vol: &mut V, pos, light, glow, ao| {
            TerrainVertex::make_col_light(light, glow, get_color(vol, pos), ao)
        },
    });

    (opaque_mesh, Mesh::new(), Mesh::new(), ())
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
