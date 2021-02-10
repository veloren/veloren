use crate::{
    mesh::{
        greedy::{self, GreedyConfig, GreedyMesh},
        MeshGen, Meshable,
    },
    render::{self, ColLightInfo, FluidPipeline, Mesh, ShadowPipeline, TerrainPipeline},
    scene::terrain::BlocksOfInterest,
};
use common::{
    span,
    terrain::Block,
    util::either_with,
    vol::{ReadVol, RectRasterableVol},
    volumes::vol_grid_2d::{CachedVolGrid2d, VolGrid2d},
};
use std::{collections::VecDeque, fmt::Debug};
use tracing::error;
use vek::*;

type TerrainVertex = <TerrainPipeline as render::Pipeline>::Vertex;
type FluidVertex = <FluidPipeline as render::Pipeline>::Vertex;

#[derive(Clone, Copy, PartialEq)]
enum FaceKind {
    /// Opaque face that is facing something non-opaque; either
    /// water (Opaque(true)) or something else (Opaque(false)).
    Opaque(bool),
    /// Fluid face that is facing something non-opaque, non-tangible,
    /// and non-fluid (most likely air).
    Fluid,
}

pub const SUNLIGHT: u8 = 24;
pub const MAX_LIGHT_DIST: i32 = SUNLIGHT as i32;

fn calc_light<V: RectRasterableVol<Vox = Block> + ReadVol + Debug>(
    is_sunlight: bool,
    // When above bounds
    default_light: u8,
    bounds: Aabb<i32>,
    vol: &VolGrid2d<V>,
    lit_blocks: impl Iterator<Item = (Vec3<i32>, u8)>,
) -> impl Fn(Vec3<i32>) -> f32 + 'static + Send + Sync {
    span!(_guard, "calc_light");
    const UNKNOWN: u8 = 255;
    const OPAQUE: u8 = 254;

    let outer = Aabb {
        min: bounds.min - Vec3::new(SUNLIGHT as i32, SUNLIGHT as i32, 1),
        max: bounds.max + Vec3::new(SUNLIGHT as i32, SUNLIGHT as i32, 1),
    };

    let mut vol_cached = vol.cached();

    let mut light_map = vec![UNKNOWN; outer.size().product() as usize];
    let lm_idx = {
        let (w, h, _) = outer.clone().size().into_tuple();
        move |x, y, z| (w * h * z + h * x + y) as usize
    };
    // Light propagation queue
    let mut prop_que = lit_blocks
        .map(|(pos, light)| {
            let rpos = pos - outer.min;
            light_map[lm_idx(rpos.x, rpos.y, rpos.z)] = light.min(SUNLIGHT); // Brightest light
            (rpos.x as u8, rpos.y as u8, rpos.z as u16)
        })
        .collect::<VecDeque<_>>();
    // Start sun rays
    if is_sunlight {
        for x in 0..outer.size().w {
            for y in 0..outer.size().h {
                let mut light = SUNLIGHT;
                for z in (0..outer.size().d).rev() {
                    match vol_cached
                        .get(outer.min + Vec3::new(x, y, z))
                        .map_or(None, |b| b.get_max_sunlight())
                    {
                        None => {},
                        Some(0) => {
                            light_map[lm_idx(x, y, z)] = 0;
                            break;
                        },
                        Some(max_sunlight) => light = light.min(max_sunlight),
                    }

                    light_map[lm_idx(x, y, z)] = light;
                    prop_que.push_back((x as u8, y as u8, z as u16));
                }
            }
        }
    }

    // Determines light propagation
    let propagate = |src: u8,
                     dest: &mut u8,
                     pos: Vec3<i32>,
                     prop_que: &mut VecDeque<_>,
                     vol: &mut CachedVolGrid2d<V>| {
        if *dest != OPAQUE {
            if *dest == UNKNOWN {
                if vol
                    .get(outer.min + pos)
                    .ok()
                    .map_or(false, |b| b.is_fluid())
                {
                    *dest = src - 1;
                    // Can't propagate further
                    if *dest > 1 {
                        prop_que.push_back((pos.x as u8, pos.y as u8, pos.z as u16));
                    }
                } else {
                    *dest = OPAQUE;
                }
            } else if *dest < src - 1 {
                *dest = src - 1;
                // Can't propagate further
                if *dest > 1 {
                    prop_que.push_back((pos.x as u8, pos.y as u8, pos.z as u16));
                }
            }
        }
    };

    // Propagate light
    while let Some(pos) = prop_que.pop_front() {
        let pos = Vec3::new(pos.0 as i32, pos.1 as i32, pos.2 as i32);
        let light = light_map[lm_idx(pos.x, pos.y, pos.z)];

        // If ray propagate downwards at full strength
        if is_sunlight && light == SUNLIGHT && false {
            // Down is special cased and we know up is a ray
            // Special cased ray propagation
            let pos = Vec3::new(pos.x, pos.y, pos.z - 1);
            let (is_air, is_liquid) = vol_cached
                .get(outer.min + pos)
                .ok()
                .map_or((false, false), |b| (b.is_air(), b.is_liquid()));
            light_map[lm_idx(pos.x, pos.y, pos.z)] = if is_air {
                prop_que.push_back((pos.x as u8, pos.y as u8, pos.z as u16));
                SUNLIGHT
            } else if is_liquid {
                prop_que.push_back((pos.x as u8, pos.y as u8, pos.z as u16));
                SUNLIGHT - 1
            } else {
                OPAQUE
            }
        } else {
            // Up
            // Bounds checking
            if pos.z + 1 < outer.size().d {
                propagate(
                    light,
                    light_map.get_mut(lm_idx(pos.x, pos.y, pos.z + 1)).unwrap(),
                    Vec3::new(pos.x, pos.y, pos.z + 1),
                    &mut prop_que,
                    &mut vol_cached,
                )
            }
            // Down
            if pos.z > 0 {
                propagate(
                    light,
                    light_map.get_mut(lm_idx(pos.x, pos.y, pos.z - 1)).unwrap(),
                    Vec3::new(pos.x, pos.y, pos.z - 1),
                    &mut prop_que,
                    &mut vol_cached,
                )
            }
        }
        // The XY directions
        if pos.y + 1 < outer.size().h {
            propagate(
                light,
                light_map.get_mut(lm_idx(pos.x, pos.y + 1, pos.z)).unwrap(),
                Vec3::new(pos.x, pos.y + 1, pos.z),
                &mut prop_que,
                &mut vol_cached,
            )
        }
        if pos.y > 0 {
            propagate(
                light,
                light_map.get_mut(lm_idx(pos.x, pos.y - 1, pos.z)).unwrap(),
                Vec3::new(pos.x, pos.y - 1, pos.z),
                &mut prop_que,
                &mut vol_cached,
            )
        }
        if pos.x + 1 < outer.size().w {
            propagate(
                light,
                light_map.get_mut(lm_idx(pos.x + 1, pos.y, pos.z)).unwrap(),
                Vec3::new(pos.x + 1, pos.y, pos.z),
                &mut prop_que,
                &mut vol_cached,
            )
        }
        if pos.x > 0 {
            propagate(
                light,
                light_map.get_mut(lm_idx(pos.x - 1, pos.y, pos.z)).unwrap(),
                Vec3::new(pos.x - 1, pos.y, pos.z),
                &mut prop_que,
                &mut vol_cached,
            )
        }
    }

    let min_bounds = Aabb {
        min: bounds.min - 1,
        max: bounds.max + 1,
    };

    // Minimise light map to reduce duplication. We can now discard light info
    // for blocks outside of the chunk borders.
    let mut light_map2 = vec![UNKNOWN; min_bounds.size().product() as usize];
    let lm_idx2 = {
        let (w, h, _) = min_bounds.clone().size().into_tuple();
        move |x, y, z| (w * h * z + h * x + y) as usize
    };
    for x in 0..min_bounds.size().w {
        for y in 0..min_bounds.size().h {
            for z in 0..min_bounds.size().d {
                let off = min_bounds.min - outer.min;
                light_map2[lm_idx2(x, y, z)] = light_map[lm_idx(x + off.x, y + off.y, z + off.z)];
            }
        }
    }

    drop(light_map);

    move |wpos| {
        let pos = wpos - min_bounds.min;
        let l = light_map2
            .get(lm_idx2(pos.x, pos.y, pos.z))
            .copied()
            .unwrap_or(default_light);

        if l != OPAQUE && l != UNKNOWN {
            l as f32 / SUNLIGHT as f32
        } else {
            0.0
        }
    }
}

impl<'a, V: RectRasterableVol<Vox = Block> + ReadVol + Debug + 'static>
    Meshable<TerrainPipeline, FluidPipeline> for &'a VolGrid2d<V>
{
    type Pipeline = TerrainPipeline;
    #[allow(clippy::type_complexity)]
    type Result = (
        Aabb<f32>,
        ColLightInfo,
        Box<dyn Fn(Vec3<i32>) -> f32 + Send + Sync>,
        Box<dyn Fn(Vec3<i32>) -> f32 + Send + Sync>,
    );
    type ShadowPipeline = ShadowPipeline;
    type Supplement = (Aabb<i32>, Vec2<u16>, &'a BlocksOfInterest);
    type TranslucentPipeline = FluidPipeline;

    #[allow(clippy::collapsible_if)]
    #[allow(clippy::many_single_char_names)]
    #[allow(clippy::needless_range_loop)] // TODO: Pending review in #587
    #[allow(clippy::or_fun_call)] // TODO: Pending review in #587
    fn generate_mesh(
        self,
        (range, max_texture_size, _boi): Self::Supplement,
    ) -> MeshGen<TerrainPipeline, FluidPipeline, Self> {
        span!(
            _guard,
            "generate_mesh",
            "<&VolGrid2d as Meshable<_, _>>::generate_mesh"
        );

        // Find blocks that should glow
        // TODO: Search neighbouring chunks too!
        // let glow_blocks = boi.lights
        //     .iter()
        //     .map(|(pos, glow)| (*pos + range.min.xy(), *glow));
        /*  DefaultVolIterator::new(self, range.min - MAX_LIGHT_DIST, range.max + MAX_LIGHT_DIST)
        .filter_map(|(pos, block)| block.get_glow().map(|glow| (pos, glow))); */

        let mut glow_blocks = Vec::new();

        // TODO: This expensive, use BlocksOfInterest instead
        let mut volume = self.cached();
        for x in -MAX_LIGHT_DIST..range.size().w + MAX_LIGHT_DIST {
            for y in -MAX_LIGHT_DIST..range.size().h + MAX_LIGHT_DIST {
                for z in -1..range.size().d + 1 {
                    let wpos = range.min + Vec3::new(x, y, z);
                    volume
                        .get(wpos)
                        .ok()
                        .and_then(|b| b.get_glow())
                        .map(|glow| glow_blocks.push((wpos, glow)));
                }
            }
        }

        // Calculate chunk lighting (sunlight defaults to 1.0, glow to 0.0)
        let light = calc_light(true, SUNLIGHT, range, self, core::iter::empty());
        let glow = calc_light(false, 0, range, self, glow_blocks.into_iter());

        let mut opaque_limits = None::<Limits>;
        let mut fluid_limits = None::<Limits>;
        let mut air_limits = None::<Limits>;
        let flat_get = {
            span!(_guard, "copy to flat array");
            let (w, h, d) = range.size().into_tuple();
            // z can range from -1..range.size().d + 1
            let d = d + 2;
            let flat = {
                let mut volume = self.cached();

                const AIR: Block = Block::air(common::terrain::sprite::SpriteKind::Empty);

                // TODO: Once we can manage it sensibly, consider using something like
                // Option<Block> instead of just assuming air.
                let mut flat = vec![AIR; (w * h * d) as usize];
                let mut i = 0;
                for x in 0..range.size().w {
                    for y in 0..range.size().h {
                        for z in -1..range.size().d + 1 {
                            let wpos = range.min + Vec3::new(x, y, z);
                            let block = volume
                                .get(wpos)
                                .map(|b| *b)
                                // TODO: Replace with None or some other more reasonable value,
                                // since it's not clear this will work properly with liquid.
                                .unwrap_or(AIR);
                            if block.is_opaque() {
                                opaque_limits = opaque_limits
                                    .map(|l| l.including(z))
                                    .or_else(|| Some(Limits::from_value(z)));
                            } else if block.is_liquid() {
                                fluid_limits = fluid_limits
                                    .map(|l| l.including(z))
                                    .or_else(|| Some(Limits::from_value(z)));
                            } else {
                                // Assume air
                                air_limits = air_limits
                                    .map(|l| l.including(z))
                                    .or_else(|| Some(Limits::from_value(z)));
                            };
                            flat[i] = block;
                            i += 1;
                        }
                    }
                }
                flat
            };

            move |Vec3 { x, y, z }| {
                // z can range from -1..range.size().d + 1
                let z = z + 1;
                match flat.get((x * h * d + y * d + z) as usize).copied() {
                    Some(b) => b,
                    None => panic!("x {} y {} z {} d {} h {}", x, y, z, d, h),
                }
            }
        };

        // Constrain iterated area
        let (z_start, z_end) = match (air_limits, fluid_limits, opaque_limits) {
            (Some(air), Some(fluid), Some(opaque)) => air.three_way_intersection(fluid, opaque),
            (Some(air), Some(fluid), None) => air.intersection(fluid),
            (Some(air), None, Some(opaque)) => air.intersection(opaque),
            (None, Some(fluid), Some(opaque)) => fluid.intersection(opaque),
            // No interfaces (Note: if there are multiple fluid types this could change)
            (Some(_), None, None) | (None, Some(_), None) | (None, None, Some(_)) => None,
            (None, None, None) => {
                error!("Impossible unless given an input AABB that has a height of zero");
                None
            },
        }
        .map_or((0, 0), |limits| {
            let (start, end) = limits.into_tuple();
            let start = start.max(0);
            let end = end.min(range.size().d - 1).max(start);
            (start, end)
        });

        let max_size =
            guillotiere::Size::new(i32::from(max_texture_size.x), i32::from(max_texture_size.y));
        assert!(z_end >= z_start);
        let greedy_size = Vec3::new(range.size().w - 2, range.size().h - 2, z_end - z_start + 1);
        // NOTE: Terrain sizes are limited to 32 x 32 x 16384 (to fit in 24 bits: 5 + 5
        // + 14). FIXME: Make this function fallible, since the terrain
        // information might be dynamically generated which would make this hard
        // to enforce.
        assert!(greedy_size.x <= 32 && greedy_size.y <= 32 && greedy_size.z <= 16384);
        // NOTE: Cast is safe by prior assertion on greedy_size; it fits into a u16,
        // which always fits into a f32.
        let max_bounds: Vec3<f32> = greedy_size.as_::<f32>();
        // NOTE: Cast is safe by prior assertion on greedy_size; it fits into a u16,
        // which always fits into a usize.
        let greedy_size = greedy_size.as_::<usize>();
        let greedy_size_cross = Vec3::new(greedy_size.x - 1, greedy_size.y - 1, greedy_size.z);
        let draw_delta = Vec3::new(1, 1, z_start);

        let get_light = |_: &mut (), pos: Vec3<i32>| volume
            .get(range.min + pos)
            .map_or(1.0, |b| if b.is_opaque() { 0.0 } else { light(pos + range.min) });
        let get_glow = |_: &mut (), pos: Vec3<i32>| glow(pos + range.min);
        let get_color =
            |_: &mut (), pos: Vec3<i32>| flat_get(pos).get_color().unwrap_or(Rgb::zero());
        let get_opacity = |_: &mut (), pos: Vec3<i32>| !flat_get(pos).is_opaque();
        let flat_get = |pos| flat_get(pos);
        let should_draw = |_: &mut (), pos: Vec3<i32>, delta: Vec3<i32>, _uv| {
            should_draw_greedy(pos, delta, flat_get)
        };
        // NOTE: Conversion to f32 is fine since this i32 is actually in bounds for u16.
        let mesh_delta = Vec3::new(0.0, 0.0, (z_start + range.min.z) as f32);
        let create_opaque = |atlas_pos, pos, norm, meta| {
            TerrainVertex::new(atlas_pos, pos + mesh_delta, norm, meta)
        };
        let create_transparent = |_atlas_pos, pos, norm| FluidVertex::new(pos + mesh_delta, norm);

        let mut greedy = GreedyMesh::new(max_size);
        let mut opaque_mesh = Mesh::new();
        let mut fluid_mesh = Mesh::new();
        greedy.push(GreedyConfig {
            data: (),
            draw_delta,
            greedy_size,
            greedy_size_cross,
            get_light,
            get_glow,
            get_color,
            get_opacity,
            should_draw,
            push_quad: |atlas_origin, dim, origin, draw_dim, norm, meta: &FaceKind| match meta {
                FaceKind::Opaque(meta) => {
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
                FaceKind::Fluid => {
                    fluid_mesh.push_quad(greedy::create_quad(
                        atlas_origin,
                        dim,
                        origin,
                        draw_dim,
                        norm,
                        &(),
                        |atlas_pos, pos, norm, &_meta| create_transparent(atlas_pos, pos, norm),
                    ));
                },
            },
        });

        let min_bounds = mesh_delta;
        let bounds = Aabb {
            min: min_bounds,
            max: max_bounds + min_bounds,
        };
        let (col_lights, col_lights_size) = greedy.finalize();

        (
            opaque_mesh,
            fluid_mesh,
            Mesh::new(),
            (
                bounds,
                (col_lights, col_lights_size),
                Box::new(light),
                Box::new(glow),
            ),
        )
    }
}

fn should_draw_greedy(
    pos: Vec3<i32>,
    delta: Vec3<i32>,
    flat_get: impl Fn(Vec3<i32>) -> Block,
) -> Option<(bool, FaceKind)> {
    let from = flat_get(pos - delta);
    let to = flat_get(pos);
    let from_opaque = from.is_opaque();
    if from_opaque == to.is_opaque() {
        // Check the interface of liquid and non-tangible non-liquid (e.g. air).
        let from_liquid = from.is_liquid();
        if from_liquid == to.is_liquid() || from.is_opaque() || to.is_opaque() {
            None
        } else {
            // While liquid is not culled, we still try to keep a consistent orientation as
            // we do for land; if going from liquid to non-liquid,
            // forwards-facing; otherwise, backwards-facing.
            Some((from_liquid, FaceKind::Fluid))
        }
    } else {
        // If going from transparent to opaque, backward facing; otherwise, forward
        // facing.  Also, if either from or to is fluid, set the meta accordingly.
        Some((
            from_opaque,
            FaceKind::Opaque(if from_opaque {
                to.is_liquid()
            } else {
                from.is_liquid()
            }),
        ))
    }
}

/// 1D Aabr
#[derive(Copy, Clone, Debug)]
struct Limits {
    min: i32,
    max: i32,
}

impl Limits {
    fn from_value(v: i32) -> Self { Self { min: v, max: v } }

    fn including(mut self, v: i32) -> Self {
        if v < self.min {
            self.min = v
        } else if v > self.max {
            self.max = v
        }
        self
    }

    fn union(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    // Find limits that include the overlap of the two
    fn intersection(self, other: Self) -> Option<Self> {
        // Expands intersection by 1 since that fits our use-case
        // (we need to get blocks on either side of the interface)
        let min = self.min.max(other.min) - 1;
        let max = self.max.min(other.max) + 1;

        (min < max).then_some(Self { min, max })
    }

    // Find limits that include any areas of overlap between two of the three
    fn three_way_intersection(self, two: Self, three: Self) -> Option<Self> {
        let intersection = self.intersection(two);
        let intersection = either_with(self.intersection(three), intersection, Limits::union);
        either_with(two.intersection(three), intersection, Limits::union)
    }

    fn into_tuple(self) -> (i32, i32) { (self.min, self.max) }
}
