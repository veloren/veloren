use crate::{
    all::ForestKind,
    block::StructureMeta,
    generator::{Generator, SpawnRules, TownGen},
    sim::{
        local_cells, neighbors, uniform_idx_as_vec2, vec2_as_uniform_idx, LocationInfo, RiverKind,
        SimChunk, WorldSim, WORLD_SIZE,
    },
    util::{RandomPerm, Sampler, UnitChooser},
    CONFIG,
};
use common::{
    assets,
    terrain::{BlockKind, Structure, TerrainChunkSize},
    vol::RectVolSize,
};
use lazy_static::lazy_static;
use noise::NoiseFn;
use roots::{find_roots_cubic, Roots};
use std::{
    cmp::Reverse,
    f32,
    ops::{Add, Div, Mul, Neg, Sub},
    sync::Arc,
};
use vek::*;

pub struct ColumnGen<'a> {
    pub sim: &'a WorldSim,
}

static UNIT_CHOOSER: UnitChooser = UnitChooser::new(0x700F4EC7);
static DUNGEON_RAND: RandomPerm = RandomPerm::new(0x42782335);

lazy_static! {
    pub static ref DUNGEONS: Vec<Arc<Structure>> = vec![
        assets::load_map("world.structure.dungeon.ruins", |s: Structure| s
            .with_center(Vec3::new(57, 58, 61))
            .with_default_kind(BlockKind::Dense))
        .unwrap(),
        assets::load_map("world.structure.dungeon.ruins_2", |s: Structure| s
            .with_center(Vec3::new(53, 57, 60))
            .with_default_kind(BlockKind::Dense))
        .unwrap(),
        assets::load_map("world.structure.dungeon.ruins_3", |s: Structure| s
            .with_center(Vec3::new(58, 45, 72))
            .with_default_kind(BlockKind::Dense))
        .unwrap(),
        assets::load_map(
            "world.structure.dungeon.meso_sewer_temple",
            |s: Structure| s
                .with_center(Vec3::new(63, 62, 60))
                .with_default_kind(BlockKind::Dense)
        )
        .unwrap(),
        assets::load_map("world.structure.dungeon.ruins_maze", |s: Structure| s
            .with_center(Vec3::new(60, 60, 116))
            .with_default_kind(BlockKind::Dense))
        .unwrap(),
    ];
}

impl<'a> ColumnGen<'a> {
    pub fn new(sim: &'a WorldSim) -> Self {
        Self { sim }
    }

    fn get_local_structure(&self, wpos: Vec2<i32>) -> Option<StructureData> {
        let (pos, seed) = self
            .sim
            .gen_ctx
            .region_gen
            .get(wpos)
            .iter()
            .copied()
            .min_by_key(|(pos, _)| pos.distance_squared(wpos))
            .unwrap();

        let chunk_pos = pos.map2(TerrainChunkSize::RECT_SIZE, |e, sz: u32| e / sz as i32);
        let chunk = self.sim.get(chunk_pos)?;

        /* let sea_level = if alt_old < CONFIG.sea_level {
            CONFIG.sea_level
        } else {
            water_level
        }; */

        // let flux = chunk.flux;
        let water_factor = /*((WORLD_SIZE.x * WORLD_SIZE.y) / 1024) as f32;*/1.0 / (1024.0 * 1.0) as f32;
        let wdelta = /*flux * water_factor*/0.01f32;
        if seed % 5 == 2
            && chunk.temp > CONFIG.desert_temp
            // && chunk.alt_old > CONFIG.sea_level + 5.0
            && chunk.alt > chunk.water_alt + wdelta
            && chunk.chaos <= 0.35
        {
            /*Some(StructureData {
                pos,
                seed,
                meta: Some(StructureMeta::Pyramid { height: 140 }),
            })*/
            None
        } else if seed % 17 == 2 && chunk.chaos < 0.2 {
            Some(StructureData {
                pos,
                seed,
                meta: Some(StructureMeta::Volume {
                    units: UNIT_CHOOSER.get(seed),
                    volume: &DUNGEONS[DUNGEON_RAND.get(seed) as usize % DUNGEONS.len()],
                }),
            })
        } else {
            None
        }
    }

    fn gen_close_structures(&self, wpos: Vec2<i32>) -> [Option<StructureData>; 9] {
        let mut metas = [None; 9];
        self.sim
            .gen_ctx
            .structure_gen
            .get(wpos)
            .into_iter()
            .copied()
            .enumerate()
            .for_each(|(i, (pos, seed))| {
                metas[i] = self.get_local_structure(pos).or(Some(StructureData {
                    pos,
                    seed,
                    meta: None,
                }));
            });
        metas
    }
}

fn intersecting_rect(rect: Aabr<f32>, mut rot_vec: Vec2<f32>, vox: Vec2<f32>) -> Vec2<f32> {
    // Compute normalized points.
    rot_vec.normalize();
    let rot_mat = Mat2::<f32>::new(rot_vec.x, -rot_vec.y, rot_vec.y, rot_vec.x);
    // let rect_center = rect.center();
    let rect_center = Vec2::new(rect.min.x, (rect.max.y + rect.min.y) * 0.5);

    /* relx = x-cx
    rely = y-cy
    rotx = relx*cos(-theta) - rely*sin(-theta)
    roty = relx*sin(-theta) + rely*cos(-theta)
    dx = max(abs(rotx) - width / 2, 0);
    dy = max(abs(roty) - height / 2, 0);
    return dx * dx + dy * dy; */
    // The rectangle is at
    // (rect_start,
    //  rect_start + rot_mat.x * rect_dim,
    //  rect_start + rot_mat * rect_dim,
    //  rect_start + rot_mat.y).
    //
    // If we rotate in the opposite direction of rot_mat, we should get back to
    // (rect_start, rect_start + rect_dim.x, rect_start + rect_dim, rect_start + rect_dim.y)
    //
    // so we just rotate the point by the 180 degree rotation of rot_mat instead.
    // Instead of rotating the point to match the rectangle, we rotate the rectangle to match the
    // point.
    // Flip it, to rotate the point instead of the rectangle.
    //let rot_mat = rot_mat.mul(Mat2::<f32>::new(-1.0, 0.0, 0.0, -1.0));
    let rot_mat = rot_mat.mul(Mat2::<f32>::new(-1.0, 0.0, 0.0, 1.0));
    // Rotate the point around the origin.
    // let vox_rot : Vec2<f32> = vox.sub(Vec2::new(rect.min.x, (rect.max.y + rect.min.y) * 0.5));
    let vox_rot: Vec2<f32> = vox.sub(rect_center);
    let vox_rot: Vec2<f32> = vox_rot.mul(rot_mat);
    // let vox_rot = vox_rot.add(Vec2::new(rect.min.x, (rect.max.y + rect.min.y) * 0.5));
    let vox_rot = vox_rot.add(rect_center);
    // Check intersection.
    let half_size = rect.half_size();
    let center_rect = rect.center();
    let dx = ((vox_rot.x - center_rect.x).abs() - half_size.w).max(0.0);
    let dy = ((vox_rot.y - center_rect.y).abs() - half_size.h).max(0.0);
    // dy = max(abs(py - y) - height / 2, 0);
    // return dx * dx + dy * dy;
    // let distance = Vec2::new(dx, dy);
    // let distance = Aabr::new_empty(vox_rot).collision_vector_with_aabr(rect);
    /* if /*rect.contains_point(vox_rot)*/dx == 0.0 && dy == 0.0 {
    //if /*rect.contains_point(vox_rot)*/distance.x <= (rect.max.x - rect.min.x) * 0.5 && distance.y <= (rect.max.y - rect.min.y) * 0.5 {
        // println!("Contains point! {:?} {:?} {:?}", rect, rot_vec, vox);
        Ok(())
    } else {
        Err(Vec2::new(dx, dy))
    } */
    Vec2::new(dx, dy)

    /*let pts = [
        rect_start,
        Vec2::new(rect_dim.x, 0.0).mul(rect_rot).add(rect_start),
        Vec2::new(rect_dim.x, rect_dim.y).mul(rect_rot).add(rect_start),
        Vec2::new(0.0, rect_dim.y).mul(rect_rot).add(rect_start),
        rect_start,
    ];
    for (p1, p2) in pts.iter().windows(2) {
        let normal = Vec2::new(p2.y - p2.x, p1.x - p2.x);
        let projected = normal.x * vox_rot.x + normal.y * vox_rot.y;
        let mut min_b = None;
        let mut max_b = None;
        for p in p2.iter() {
            let projected = normal.x * p.x = normal.y * p.y;
            min_b = match min_b {
                None | Some(min_b) if projected < min_b => projected,
                _ => min_b
            };
            max_b = match max_b {
                None | Some(max_b) if projected > max_b => projected,
                _ => max_b
            };
        }
        if max_a.unwrap() < projected || projected < min_a.unwrap() {
            return false
        }
    }
    true
    /* foreach (var polygon in new[] { a, b })
    {
        for (int i1 = 0; i1 < polygon.Points.Count; i1++)
        {
            int i2 = (i1 + 1) % polygon.Points.Count;
            var p1 = polygon.Points[i1];
            var p2 = polygon.Points[i2];

            var normal = new Point(p2.Y - p1.Y, p1.X - p2.X);

            double? minA = null, maxA = null;
            foreach (var p in a.Points)
            {
                var projected = normal.X * p.X + normal.Y * p.Y;
                if (minA == null || projected < minA)
                    minA = projected;
                if (maxA == null || projected > maxA)
                    maxA = projected;
            }

            double? minB = null, maxB = null;
            foreach (var p in b.Points)
            {
                var projected = normal.X * p.X + normal.Y * p.Y;
                if (minB == null || projected < minB)
                    minB = projected;
                if (maxB == null || projected > maxB)
                    maxB = projected;
            }

            if (maxA < minB || maxB < minA)
                return false;
        }
    }
    return true; */*/
}

fn river_spline_coeffs(
    sim: &WorldSim,
    chunk_pos: Vec2<f32>,
    spline_derivative: Vec2<f32>,
    downhill_pos: Vec2<f32>,
) -> Vec3<Vec2<f32>> {
    let dxy = downhill_pos - chunk_pos;
    // Since all splines have been precomputed, we don't have to do that much work to evaluate the
    // spline.  The spline is just ax^2 + bx + c = 0, where
    //
    // a = dxy - chunk.river.spline_derivative
    // b = chunk.river.spline_derivative
    // c = chunk_pos
    Vec3::new(
        dxy - spline_derivative,
        spline_derivative,
        chunk_pos.map(|e| e as f32),
    )
    // let derivative_divisor = 1.0;
    /* //
    // a
    // Our goal is to find the momentum of all the rivers leading into this point.
    // Mass is approximated by cross-sectional area A so we can multiply A * v *
    // CONFIG.river_roughness to get the (sort of) momentum of uphill neighobrs.
    //
    // From there, we can just add together all of these values in order to get the linear
    // momentum.  This is what we will use in order to pick the "back" direction of this spline
    // segment.  The "forward" direction will be aimed in the direction of the river chunk's
    // velocity.
    //
    // (Actually, we don't currently need to do this at all, but it might be nice for computing
    // derivatives).
    //
    // The curve might be more "interesting" if we modified the slope continuously as we got closer
    // to the point, but we (currently?) choose not to do that because it would be nice to have a
    // closed form way of determining distance from any particular spline segment, which probably
    // doesn't work if we keep switching the derivative.
    let (uphill_momentum, uphill_mass) = uphill(chunk_idx)
        .map(|neighor_idx| {
            let neighobr_pos = uniform_idx_as_vec2(neighbor_idx);
            // Explicitly not getting interpolated version because we want the curve to be
            // predictible throughout the spline segment.
            let neighbor_chunk = sim.get(neighbor_pos);
            let neighbor_river = &neighor_chunk.river;
            // We can use this information even for incoming flux that doesn't visually show up as
            // a river; generally, rivers will have higher flux than non-rivers, so they will
            // impact the slope more.
            let area = cross_section.x * cross_section.y;
            (velocity.mul(area), area)
        })
        .sum();
    // Find the velocity of the center of mass, unless it's 0 in which case we can treat the
    // momentum as 0 as well.
    let uphill_velocity = if uphill_mass == 0.0 {
        Vec3::zero()
    } else {
        uphill_momentum / uphill_mass
    };
    // We could try to exploit the z component as well, but currently we opt not to.
    let uphill_velocity_2d = Vec2::new(uphill_velocity.x, uphill_velocity.y);
    let chunk_pos = uniform_idx_as_vec2(chunk_idx);
    // Implied "previous point" position is (conservatively) the inverse of the velocity, in the x
    // direction.
    // NOTE: the implied previous point position of the river is potentially extremely far away
    // (since the velocity could be *huge*, or really small).  For now, we let it be, but in the
    // future we'll probably want to tame it a bit.
    let prev_pos = chunk_pos - uphill_velocity_2d;
    // Find the "next point" position using a more straightforward technique--it's the point
    // downhill from this one.
    let next_pos = downhill_pos;
    // Quadratic spline equation: ax^2 + bx + c = 0.
    // Find the current chunk position and use this as the spline offset.
    let c = uniform_idx_as_vec2(chunk_idx);
    // Find a candidate first derivative using the computed previous offset.
    // dx_i(t)/dt / dy_i(t)/dt = dx_(i+1)/dt / dy_(i+1)/dt
    //
    // t = 1 -> b = 0
    bx = 0;
    ax = (next_pos - chunk_pos - bx) = (next_pos - chunk_pos).
    d_x = (2 * ax + bx) / deriv = 2 * (next_pos - chunk_pos) + 0;
    bx = d_x = 2 * (next_pos - chunk_pos) + 0;
    ax = (next_pos' - next_pos - bx) = (next_pos' - next_pos - (2 * (next_pos - chunk_pos)))
       = next_pos' - 3 * next_pos + 2 * chunk_pos;
    d_x = (2 * ax + bx) / deriv
        = 2 * (next_pos' - 3 * next_pos + 2 * chunk_pos) + 2 * (next_pos - chunk_pos)
        = 2 * next_pos' - 3 * next_pos + 4 * chunk_pos + 2 * next_pos - 2 * chunk_pos
        = 2 * next_pos' - next_pos + 2 * chunk_pos
    bx = d_x = 2 * next_pos' - next_pos + 2 * chunk_pos
    ax = (next_pos'' - next_pos' - bx)
       = (next_pos'' - next_pos' - (2 * next_pos' - next_pos + 2 * chunk_pos))
       = next_pos'' - 3 * next_pos' + next_pos - 2 * chunk_pos;

    let b = (2.0 * (next_pos - chunk_pos) + prev_pos); */
}

/// Find the nearest point from a quadratic spline to this point (in terms of t, the "distance along the curve"
/// by which our spline is parameterized).  Note that if t < 0.0 or t >= 1.0, we probably shouldn't
/// be considered "on the curve"... hopefully this works out okay and gives us what we want (a
/// river that extends outwards tangent to a quadratic curve, with width configured by distance
/// along the line).
fn quadratic_nearest_point(
    spline: &Vec3<Vec2<f32>>,
    point: Vec2<f32>,
) -> Option<(f32, Vec2<f32>, f32)> {
    let a = spline.z.x;
    let b = spline.y.x;
    let c = spline.x.x;
    let d = point.x;
    let e = spline.z.y;
    let f = spline.y.y;
    let g = spline.x.y;
    let h = point.y;
    // This is equivalent to solving the following cubic equation (derivation is a bit annoying):
    //
    // A = 2(c^2 + g^2)
    // B = 3(b * c + g * f)
    // C = ((a - d) * 2 * c + b^2 + (e - h) * 2 * g + f^2)
    // D = ((a - d) * b + (e - h) * f)
    //
    // Ax³ + Bx² + Cx + D = 0
    //
    // Once solved, this yield up to three possible values for t (reflecting minimal and maximal
    // values).  We should choose the minimal such real value with t between 0.0 and 1.0.  If we
    // fall outside those bounds, then we are outside the spline and return None.
    let a_ = (c * c + g * g) * 2.0;
    let b_ = (b * c + g * f) * 3.0;
    let a_d = a - d;
    let e_h = e - h;
    let c_ = a_d * c * 2.0 + b * b + e_h * g * 2.0 + f * f;
    let d_ = a_d * b + e_h * f;
    let roots = find_roots_cubic(a_, b_, c_, d_);
    let roots = roots.as_ref();

    let min_root = roots
        .into_iter()
        .copied()
        .filter_map(|root| {
            let river_point = spline.x * root * root + spline.y * root + spline.z;
            let river_zero = spline.z;
            let river_one = spline.x + spline.y + spline.z;
            if root >= 0.0 && root <= 1.0 {
                Some((root, river_point))
            } else if root < 0.0 && river_point.distance_squared(river_zero) < 0.0001 {
                Some((root, /*river_point*/ river_zero))
            } else if root > 1.0 && river_point.distance_squared(river_one) < 0.0001 {
                Some((root, /*river_point*/ river_one))
            } else {
                None
            }
            // point.map(|e| e as i32) == spline.z.map(|e| e as i32))
        })
        .map(|(root, river_point)| {
            // let root = root.max(0.0).min(1.0);
            // let river_point = spline.x * root * root + spline.y * root + spline.z;
            let river_distance = river_point.distance_squared(point);
            (root, river_point, river_distance)
        })
        // In the (unlikely?) case that distances are equal, prefer the earliest point along the
        // river.
        .min_by(|&(ap, _, a), &(bp, _, b)| {
            (a, ap < 0.0 || ap > 1.0, ap)
                .partial_cmp(&(b, bp < 0.0 || bp > 1.0, bp))
                .unwrap()
        });
    // let root_width = roots.len();
    min_root
}

impl<'a> Sampler<'a> for ColumnGen<'a> {
    type Index = Vec2<i32>;
    type Sample = Option<ColumnSample<'a>>;

    fn get(&self, wpos: Vec2<i32>) -> Option<ColumnSample<'a>> {
        let wposf = wpos.map(|e| e as f64);
        let chunk_pos = wpos.map2(TerrainChunkSize::RECT_SIZE, |e, sz: u32| e / sz as i32);
        /* let wpos_mid = chunk_pos.map2(Vec2::from(TerrainChunkSize::RECT_SIZE), |e, sz: u32| {
            (e as u32 * sz) as i32
        });
        let wposf_mid = wpos_mid.map(|e| e as f64); */

        let sim = &self.sim;

        let turb = Vec2::new(
            sim.gen_ctx.turb_x_nz.get((wposf.div(48.0)).into_array()) as f32,
            sim.gen_ctx.turb_y_nz.get((wposf.div(48.0)).into_array()) as f32,
        ) * 12.0;
        let wposf_turb = wposf + turb.map(|e| e as f64);
        /* let turb_mid = Vec2::new(
            sim.gen_ctx.turb_x_nz.get((wposf_mid.div(48.0)).into_array()) as f32,
            sim.gen_ctx.turb_y_nz.get((wposf_mid.div(48.0)).into_array()) as f32,
        );
        let wposf_turb_mid = wposf_mid + turb_mid.map(|e| e as f64); */

        // let alt_base = sim.get_interpolated(wpos, |chunk| chunk.alt_base)?;
        let chaos = sim.get_interpolated(wpos, |chunk| chunk.chaos)?;
        // let chaos_mid = sim.get_interpolated(wpos_mid, |chunk| chunk.chaos)?;
        let temp = sim.get_interpolated(wpos, |chunk| chunk.temp)?;
        /* let downhill_alt = sim.get_interpolated(wpos, |chunk| {
            sim.get(chunk.downhill).map(|chunk| chunk.alt).unwrap_or(CONFIG.sea_level)
        })?; */
        let humidity = sim.get_interpolated(wpos, |chunk| chunk.humidity)?;
        // let humidity_mid = sim.get_interpolated(wpos_mid, |chunk| chunk.humidity)?;
        let rockiness = sim.get_interpolated(wpos, |chunk| chunk.rockiness)?;
        let tree_density = sim.get_interpolated(wpos, |chunk| chunk.tree_density)?;
        let spawn_rate = sim.get_interpolated(wpos, |chunk| chunk.spawn_rate)?;

        let sim_chunk = sim.get(chunk_pos)?;
        let river_data = &sim_chunk.river;
        let neighbor_coef = Vec2::new(
            TerrainChunkSize::RECT_SIZE.x as f32,
            TerrainChunkSize::RECT_SIZE.y as f32,
        );
        let my_chunk_idx = vec2_as_uniform_idx(chunk_pos);
        let neighbor_river_data = local_cells(my_chunk_idx).filter_map(|neighbor_idx| {
            let neighbor_pos = uniform_idx_as_vec2(neighbor_idx);
            let neighbor_chunk = sim.get(neighbor_pos)?;
            Some((neighbor_pos, neighbor_chunk, &neighbor_chunk.river))
        });
        let neighbor_river_data =
            /* std::iter::once((chunk_pos, river_data))
            .chain(neighbor_river_data) */
            neighbor_river_data
            .map(|(posj, chunkj, river)| {
                let kind = match river.river_kind {
                    Some(kind) => kind,
                    None => {
                        return (posj, chunkj, river, None);
                    }
                };
                let downhill_pos = if let Some(pos) = chunkj.downhill {
                        pos
                    } else {
                        if kind.is_river() {
                            println!("What? River: {:?}, Pos: {:?}", river, posj);
                            panic!("How can a river have no downhill?");
                        }
                        return (posj, chunkj, river, None)
                    };
                let downhill_wpos = downhill_pos.map(|e| e as f32);// * neighbor_coef;
                let downhill_pos = downhill_pos.map2(TerrainChunkSize::RECT_SIZE, |e, sz: u32| e / sz as i32);
                let neighbor_pos = posj.map(|e| e as f32) * neighbor_coef;
                /* let downhill_pos = if downhill_idx <= -2 { return false } else {
                    uniform_idx_as_vec2(downhill_idx)
                }.map(|e| e as f32) * neighbor_coef; */
                let direction = neighbor_pos - downhill_wpos;
                /* let dxy = wposf - neighbor_pos;
                let neighbor_distance = dxy.magnitude(); */
                /* if river.cross_section.x > 0.5 {
                    println!("Pos: {:?}, Direction: {:?}, river: {:?}", wposf, direction, river.cross_section);
                } */
                /* let (min_y, max_y) = if direction.y < 0.0 {
                    (-direction.magnitude(), 0.0)
                } else {
                    (0.0, direction.magnitude())
                }; */
                // let (min_x, max_y) = (0.0, direction.magnitude());
                // let neighbor_distance = direction.magnitude();
                let river_width_min = if let RiverKind::River { cross_section } = kind {
                    cross_section.x/*.max(1.0)*///.max(if river.is_river { 2.0 } else { 0.0 });
                } else {
                    TerrainChunkSize::RECT_SIZE.x as f32
                    // neighbor_distance
                };
                let downhill_chunk = sim.get(downhill_pos)./*unwrap_or(chunkj)*/expect("How can this not work?");
                let my_pos = wposf.map(|e| e as f32);
                let coeffs = river_spline_coeffs(self.sim, neighbor_pos, chunkj.river.spline_derivative, downhill_wpos);
                let (direction, downhill_chunk, river_t, river_pos, river_dist) = match kind {
                    RiverKind::River { .. } => {
                        if let Some((t, pt, dist)) = quadratic_nearest_point(&coeffs, my_pos) {
                            (direction, downhill_chunk, t, pt, dist.sqrt())
                        } else {
                            /* if kind == RiverKind::River {
                                println!("...Wait... I think I'm starting to see.");
                            } */
                            return (posj, chunkj, river, None);
                        }
                    },
                    RiverKind::Lake { neighbor_pass_pos } => {
                        let pass_dist = neighbor_pass_pos
                                .map2(neighbor_pos.map2(TerrainChunkSize::RECT_SIZE, |f, g| (f as i32, g as i32)),
                                      |e, (f, g)| ((e - f) / g).abs())
                                .reduce_partial_max();
                        let mut spline_derivative = river.spline_derivative;
                        let neighbor_pass_pos = if pass_dist <= 1 {
                            neighbor_pass_pos
                        } else {
                            // return (posj, chunkj, river, None);
                            // spline_derivative = Vec2::zero();
                            downhill_wpos.map(|e| e as i32)
                            // neighbor_pass_pos
                        };
                        let pass_dist = neighbor_pass_pos
                                .map2(neighbor_pos.map2(TerrainChunkSize::RECT_SIZE, |f, g| (f as i32, g as i32)),
                                      |e, (f, g)| ((e - f) / g).abs())
                                .reduce_partial_max();
                        if pass_dist > 1 {
                            return (posj, chunkj, river, None);
                        }
                        let neighbor_pass_wpos = neighbor_pass_pos
                            .map(|e| e as f32);
                        /*    .map2(downhill_wpos.map2(TerrainChunkSize::RECT_SIZE, |f, g| (f, g as f32)),
                                  |e, (f, g)| if e == e.max(f - g).min(f + g) { e } else { f }); */// * neighbor_coef;
                        let neighbor_pass_pos = neighbor_pass_pos.map2(TerrainChunkSize::RECT_SIZE, |e, sz: u32| e / sz as i32);
                        /* let neighbor_pass_wpos =
                            neighbor_pass_pos.map2(TerrainChunkSize::RECT_SIZE, |e, sz: u32| e / sz as i32)
                            .map2(neighbor_pos,
                                  |e, f| e.max(f as i32 - 1).min(f as i32 + 1)); */
                        let coeffs = river_spline_coeffs(self.sim, neighbor_pos, spline_derivative, neighbor_pass_wpos);
                        let direction = neighbor_pos - neighbor_pass_wpos;
                        if let Some((t, pt, dist)) = quadratic_nearest_point(&coeffs, my_pos) {
                            // println!("Found lake {:?}: {:?} near wpos: {:?}, from {:?} to {:?}", (t, pt, dist.sqrt()), river, my_pos, neighbor_pos, neighbor_pass_wpos);
                            (direction,
                             sim.get(neighbor_pass_pos).expect("Must already work"),
                             t, pt, dist.sqrt())
                        } else {
                            /* (direction,
                             sim.get(neighbor_pass_pos).expect("Must already work"),
                             0.0, neighbor_pos, direction.magnitude())
                            /* if kind == RiverKind::River {
                                println!("...Wait... I think I'm starting to see.");
                            } */ */
                            return (posj, chunkj, river, None);
                        }
                        /* let dist = my_pos.distance(neighbor_pos);
                        (0.5, neighbor_pos, dist) */
                    },
                    RiverKind::Ocean => unreachable!(),
                };
                let river_width_max =
                    if let Some(RiverKind::River { cross_section }) = downhill_chunk.river.river_kind {
                        cross_section.x
                    } else {
                        TerrainChunkSize::RECT_SIZE.x as f32
                        // neighbor_distance
                    };
                /* if kind != RiverKind::River {
                    return (posj, chunkj, river, None)
                } */
                let river_width = Lerp::lerp(river_width_min, river_width_max, river_t);
                // To find the distance, we just evaluate the quadratic equation at river_t and see
                // if it's within width (but we should be able to use it for a lot more, and this
                // probably isn't the very best approach anyway since it will bleed out).
                // let river_pos = coeffs.x * river_t * river_t + coeffs.y * river_t + coeffs.z;
                let res = Vec2::new(0.0, (river_dist - (river_width * 0.5).max(1.0)).max(0.0));
                /* let res = intersecting_rect(Aabr {
                    min: Vec2::new(/*min_y*/0.0, -river_width.mul(0.5)).add(neighbor_pos),
                    max: Vec2::new(/*max_y*/direction.magnitude(), river_width.mul(0.5)).add(neighbor_pos),
                }, direction, wposf.map(|e| e as f32)); */
                (posj, chunkj, river, Some((direction, res, river_width, (river_t, (river_pos, coeffs), downhill_chunk))))
            });

        // Find the average distance to each neighboring body of water.
        let mut river_count = /*0.0f32*/0.0f32;
        let mut overlap_count = /*0.0f32*//*alt*/0.0f64;
        let mut river_distance_product = 1.0f32;
        let mut river_overlap_distance_product = /*1.0f32*/0.0f64;
        let mut min_river_distance = f32::INFINITY;
        let mut max_river = None;
        let mut max_key = None;
        let alt = sim.get_interpolated_monotone(wpos, |chunk| {
            /* match chunk.downhill.and_then(|pos| sim.get(pos)).and_then(|chunk| Some((chunk.river.river_kind?, chunk.alt))) {
                Some((RiverKind::River, downhill_alt)) => downhill_alt,
                _ => chunk.alt,
            } */
            //
            /*sim_chunk.alt*/
            chunk.alt
            // .min(chunk.alt)
            // .min(sim_chunk.water_alt/*.max(sim_chunk.alt)*/)
            // .max(sim_chunk.alt/* - wdelta*/)
            /* let new_alt = Lerp::lerp(chunk.alt - 5.0, chunk.alt, chunk.flux * water_factor);
            let new_water_alt = chunk.water_alt.max(new_alt);
            Lerp::lerp(
                chunk.alt - 5.0,
                new_alt,
                chunk.alt - new_water_alt,
            ) */
        })?;
        let alt_old = sim.get_interpolated_monotone(wpos, |chunk| chunk.alt_old)?;
        // For every neighbor at 0,0 or to the right or front of us, we need to compute its river
        // prism, and then figure out whether this column intersects its river line.  If it does,
        // we should set ourselves to the maximum of the lerps of all their fluxes as they converge
        // on this point (in theory they should augment each other, but we aren't going to worry
        // about that for now).
        //
        // IDEA:
        // For every "nearby" chunk, check whether it is a river.  If so, find the closest point on
        // the river segment to wposf (if two point are equidistant, choose the earlier one),
        // calling this point river_pos and the length (from 0 to 1) along the river segment for
        // the nearby chunk river_t.  Let river_dist be the distance from river_pos to wposf.
        //
        // Let river_alt be the interpolated river height at this point
        // (from the alt/water altitude at the river, to the alt/water_altitude of the downhill
        // river, increasing with river_t).
        //
        // Now, if river_dist is <= river_width * 0.5, then we don't care what altitude we use, and
        // mark that we are on a river (we decide what river to use using a heuristic, and set the
        // solely according to the computed river_alt for that point).
        //
        // Otherwise, we let dist = river_dist - river_width * 0.5.
        //
        // If dist >= TerrainChunkSize::RECT_SIZE.x, we don't include this river in the calculation
        // of the correct altitude for this point.
        //
        // Otherwise (i.e. dist < TerrainChunkSize::RECT_SIZE.x), we want to bias the altitude of
        // this point towards the altitude of the river.  Specifically, as the dist goes from
        // TerrainChunkSize::RECT_SIZE.x to 0, the weighted altitude of this point should go from
        // alt to river_alt.
        for (river_chunk_idx, river_chunk, river, dist) in neighbor_river_data.clone() {
            match river.river_kind {
                Some(kind) => {
                    if
                    /*kind != RiverKind::Ocean && */
                    kind.is_river() && !dist.is_some() {
                        // Ostensibly near a river segment, but not "usefully" so (there is no
                        // closest point between t = 0.0 and t = 1.0).
                        continue;
                    }
                    let river_dist = dist.map(
                        /*Vec2::magnitude*/
                        |(direction, dist, _, (river_t, _, downhill_river))| {
                            let downhill_height = Lerp::lerp(
                                river_chunk.alt.max(river_chunk.water_alt),
                                downhill_river.alt.max(downhill_river.water_alt),
                                river_t,
                            );
                            (
                                Reverse((/*river_t < 0.0 || river_t > 1.0,*/ dist.x, dist.y)),
                                /*Reverse*/ (downhill_height),
                            ) /*dist.magnitude()*/
                        },
                    );
                    let river_dist = river_dist.or_else(|| /*if kind != RiverKind::River */{
                        let neighbor_pos = river_chunk_idx.map(|e| e as f32) * neighbor_coef;
                        let dist = wposf.map(|e| e as f32) - neighbor_pos;
                        let dist = (dist.magnitude() - TerrainChunkSize::RECT_SIZE.x as f32 * 0.5).max(0.0);
                        Some((Reverse((0.0, dist)), /*Reverse*/(0.0))/*dist.magnitude()*/)
                    }/* else {
                        None
                    }*/);
                    let river_key = (
                        river_dist, //.map(Reverse),
                        Reverse(kind),
                        //Reverse(river_chunk.alt),
                        // river_chunk.alt,
                        // river.cross_section.x * river.cross_section.y
                    );
                    if max_key < Some(river_key) {
                        max_river = Some((river_chunk_idx, river_chunk, river, dist));
                        max_key = Some(river_key);
                    }

                    if kind == RiverKind::Ocean {
                        continue;
                    }
                    // NOTE: we scale by the distance to the river divided by the difference
                    // between the edge of the river that we intersect, and the remaining distance
                    // until the nearest point in "this" chunk (i.e. the one whose top-left corner
                    // is chunk_pos) that is at least 2 chunks away from the river source.
                    if let Some((
                        direction,
                        dist,
                        river_width,
                        (river_t, (river_pos, river_coeffs), downhill_river_chunk),
                    )) = dist
                    {
                        let max_distance = TerrainChunkSize::RECT_SIZE.x as f32; //river_width;//direction.magnitude();
                        let scale_factor =
                            1.0 * /*TerrainChunkSize::RECT_SIZE.x*/max_distance
                            /*- river_width * 0.5*/;
                        /* if kind != RiverKind::River {
                            continue;
                        } */
                        /* if dist.y < scale_factor * 1.0 && (river_t < 0.0 || river_t > 1.0) {
                            river_count += 1.0;// - (dist.y / scale_factor/* - 1.0*/).abs();
                        } */
                        let river_dist = if
                        /*dist.y > 0.0 && */
                        dist.x == 0.0 && dist.y < scale_factor
                        /*|| dist.x == 0.0 */
                        {
                            /* if dist.y == 0.0 {
                                // We are actually on a river, so compute an average.
                            } */
                            dist.y
                        } else {
                            // dist.x
                            continue;
                            // dist.x //dist.magnitude();
                        };
                        // let river_width = river.cross_section.x;
                        // We basically want to project outwards from river_pos, along the current
                        // tangent line, to chunks <= river_width * 1.0 away from this
                        // point.  We *don't* want to deal with closer chunks because they

                        // let river_dist = dist.y;
                        // NOTE: river_width <= 2 * max terrain chunk size width, so this should not
                        // lead to division by zero.
                        // NOTE: If distance = 0.0 this goes to zero, which is desired since it
                        // means points that actually intersect with rivers will not be interpolated
                        // with the "normal" height of this point.
                        // NOTE: We keep the maximum at 1.0 so we don't undo work from another river
                        // just by being far away.
                        let river_scale = (river_dist / scale_factor); //.min(1.0);
                                                                       // river_count += 1.0 - river_scale;
                                                                       // What we want: chunk that river point is in is at least 1 away from the
                                                                       // chunk that this column is in.
                                                                       // let river_chunk = river_pos.map2(TerrainChunkSize::RECT_SIZE, |e, sz: u32| e / sz as i32);
                                                                       // let chunk_dist = river_chunk.map(|e| e.abs()).reduce_partial_min();
                                                                       /* if (river_t < 0.0 || river_t > 1.0) {
                                                                           /*if river_dist < min_river_distance */{
                                                                               overlap_count += 1.0;
                                                                               river_overlap_distance_product *= river_scale;
                                                                           }
                                                                       } else if /*true*//*river_chunk >= 1.0 &&*/ /*river_scale < 1.0*/true
                                                                           /*river_dist*//*river_scale*//*river_dist < min_river_distance*/ {
                                                                           river_count += 1.0;
                                                                           // min_river_distance = /*river_dist*/river_scale;
                                                                           river_distance_product *= river_scale;
                                                                       } */
                        let river_alt = /*sim.get(/*river_pos*/max_border_river_pos)?*/
                            Lerp::lerp(
                                river_chunk.alt.max(river_chunk.water_alt),
                                downhill_river_chunk.alt.max(downhill_river_chunk.water_alt),
                                river_t,
                            );
                        let river_alt = if
                        /*kind.is_river()*/
                        true {
                            Lerp::lerp(river_alt, alt, river_scale)
                        } else {
                            alt.min(river_alt)
                        };
                        let river_alt_diff = river_alt - alt;
                        /* let river_alt_diff = if river_alt_diff >= 0.1 || river_alt_diff <= -0.1 {
                            river_alt_diff
                        } else if river_alt_diff < 0.0 {
                            -0.1
                        } else {
                            0.1
                        }; */
                        if
                        /*river_alt_diff.abs() >= 0.1*/
                        kind.is_river() {
                            let river_alt_inv = /*1.0 / (river_alt_diff as f64)*/river_alt_diff as f64;
                            river_overlap_distance_product +=
                                (1.0 - (river_scale as f64)) * river_alt_inv;
                            overlap_count += 1.0 - (river_scale as f64);
                        }
                        if
                        /*river_scale != 0.0*/
                        true {
                            river_count += 1.0;
                            river_distance_product *= /*1.0 / */river_scale;
                        }
                        /* if river_dist < min_river_distance {
                            min_river_distance = river_dist;
                        } */
                    }
                }
                None => {}
            }
        }
        // Harmonic mean.
        let river_scale_factor = /*if river_count == 0.0 {
            0.0
        } else */{
            let river_scale_factor = river_distance_product/* / river_count*/;
            /*if river_scale_factor == 0.0 {
                0.0
            } else */{
                /*1.0 / river_scale_factor*/river_scale_factor/* * river_scale_factor*/.powf(if river_count == 0.0 { 1.0 } else {1.0 / river_count })
            }
        };

        let alt = alt
            + if overlap_count == 0.0 {
                0.0
            } else {
                let weighted_alt = river_overlap_distance_product / overlap_count;
                let weighted_alt_ = if weighted_alt == 0.0 {
                    0.0
                } else {
                    /*1.0 / */
                    weighted_alt
                };
                /*println!("pos: {:?}, alt: {:?}, weighted alt: {:?}, weighted_alt_: {:?}, overlap_count: {:?}, product: {:?}, ", wposf, alt, weighted_alt, weighted_alt_, overlap_count, river_overlap_distance_product);*/
                weighted_alt_
            } as f32;
        /* // Geometric mean.
        let river_scale_factor = if river_distance_product == 0.0 || river_overlap_distance_product == 0.0 {
            0.0
        } else if river_count > 0.0 {
            river_distance_product
        } else {
            river_overlap_distance_product
        };// river_distance_product.min(river_overlap_distance_product); */
        /* let river_count = if river_count > 0.0 { 1.0 + overlap_count } else { 1.0 };
        let river_scale_factor = river_distance_product.powf(1.0 / /*(if river_count <= 0.0 { 1.0 } else { river_count })*/river_count); */

        /* // For every neighbor at 0,0 or to the right or front of us, we need to compute its river
        // prism, and then figure out whether this column intersects its river line.  If it does,
        // we should set ourselves to the maximum of the lerps of all their fluxes as they converge
        // on this point (in theory they should augment each other, but we aren't going to worry
        // about that for now).
        let /*max_neighbor_river*/(max_border_river_pos, max_border_river, max_border_river_dist) =
            neighbor_river_data
            .clone()
            /* .filter(|&(posj, ref river, distance)| {
                /* let downhill_pos = if let Some(pos) = sim.get(posj)
                    .and_then(|chunk| chunk.downhill) {
                        pos
                    } else {
                        return false
                    }.map(|e| e as f32);// * neighbor_coef;
                let neighbor_pos = posj.map(|e| e as f32) * neighbor_coef;
                /* let downhill_pos = if downhill_idx <= -2 { return false } else {
                    uniform_idx_as_vec2(downhill_idx)
                }.map(|e| e as f32) * neighbor_coef; */
                let direction = neighbor_pos - downhill_pos;
                /* let dxy = wposf - neighbor_pos;
                let neighbor_distance = dxy.magnitude(); */
                /* if river.cross_section.x > 0.5 {
                    println!("Pos: {:?}, Direction: {:?}, river: {:?}", wposf, direction, river.cross_section);
                } */
                /* let (min_y, max_y) = if direction.y < 0.0 {
                    (-direction.magnitude(), 0.0)
                } else {
                    (0.0, direction.magnitude())
                }; */
                // let (min_x, max_y) = (0.0, direction.magnitude());
                let river_width = river.cross_section.x;//.max(if river.is_river { 2.0 } else { 0.0 });
                let res = intersecting_rect(Aabr {
                    min: Vec2::new(/*min_y*/0.0, -river_width.mul(0.5)).add(neighbor_pos),
                    max: Vec2::new(/*max_y*/direction.magnitude(), river_width.mul(0.5)).add(neighbor_pos),
                }, direction, wposf.map(|e| e as f32));
                /*if res && river.is_river {
                    // println!("Pos: {:?}, Direction: {:?}, river: {:?}", wposf, direction, river.cross_section);
                }*/
                res
                // true */
                distance.map(|d| d.magnitude() == 0.0).unwrap_or(false)
            }) */
            .max_by(|(_, river1, dist1), (_, river2, dist2)|
                    /*(dist1 == &Some(Vec2::zero()) && river1.river_kind == Some(RiverKind::River),
                     river1.river_kind.map(Reverse), dist1.map(Vec2::magnitude).map(Reverse),
                     river1.cross_section.x * river1.cross_section.y)
                    .partial_cmp(&(dist2 == &Some(Vec2::zero()) && river2.river_kind == Some(RiverKind::River),
                                   river2.river_kind.map(Reverse),
                                   dist2.map(Vec2::magnitude).map(Reverse), river2.cross_section.x * river2.cross_section.y))*/
                    /*(river1.river_kind.map(Reverse), dist1.map(Vec2::magnitude).map(Reverse),
                     river1.cross_section.x * river1.cross_section.y)
                    .partial_cmp(&(river2.river_kind.map(Reverse), dist2.map(Vec2::magnitude).map(Reverse),
                                   river2.cross_section.x * river2.cross_section.y)) */
                    (dist1.map(Vec2::magnitude).map(Reverse), river1.river_kind.map(Reverse),
                     river1.cross_section.x * river1.cross_section.y)
                    .partial_cmp(&(dist2.map(Vec2::magnitude).map(Reverse), river2.river_kind.map(Reverse),
                                   river2.cross_section.x * river2.cross_section.y))
                    .unwrap())
            .unwrap();
        /* let (max_border_river_pos, max_border_river) =
            neighbor_river_data
            .max_by(|(_, river1, dist1), (_, river2, dist2)|
                    (river1.river_kind, river1.cross_section.x * river1.cross_section.y)
                    .partial_cmp(&(river2.river_kind,
                                   river2.cross_section.x * river2.cross_section.y))
                    .unwrap())
            .unwrap(); */ */

        /* let neighbor_dim = (river_pos.map(|e| e as f64).mul(neighbor_coef) - wposf);
        let neighbor_distance = neighbor_dim.magnitude(); */

        // we are intersecting its river line, and if so increase our flux.
        // let flux = sim_chunk.flux;

        // Never used
        //const RIVER_PROPORTION: f32 = 0.025;

        /*
        let river = dryness
            .abs()
            .neg()
            .add(RIVER_PROPORTION)
            .div(RIVER_PROPORTION)
            .max(0.0)
            .mul((1.0 - (chaos - 0.15) * 20.0).max(0.0).min(1.0));
        */
        let river = 0.0;

        let cliff_hill = (sim
            .gen_ctx
            .small_nz
            .get((wposf_turb.div(128.0)).into_array()) as f32)
            .mul(24.0);

        let riverless_alt_delta = (sim
            .gen_ctx
            .small_nz
            .get((wposf_turb.div(200.0)).into_array()) as f32)
            .abs()
            .mul(chaos.max(0.05))
            .mul(/*55.0*/ 11.0)
            + (sim
                .gen_ctx
                .small_nz
                .get((wposf_turb.div(400.0)).into_array()) as f32)
                .abs()
                .mul((1.0 - chaos).max(0.3))
                .mul(1.0 - humidity)
                .mul(/*65.0*/ 13.0);

        /* let riverless_alt_delta_mid =
        (sim
            .gen_ctx
            .small_nz
            .get((wposf_turb_mid.div(150.0)).into_array()) as f32)
            .abs()
            .mul(chaos_mid.max(0.025))
            .mul(64.0)
        + (sim
            .gen_ctx
            .small_nz
            .get((wposf_turb_mid.div(450.0)).into_array()) as f32)
            .abs()
            .mul(1.0 - chaos_mid)
            .mul(1.0 - humidity_mid)
            .mul(96.0); */

        let riverless_alt_delta = riverless_alt_delta
            - (1.0 - river)
                .mul(f32::consts::PI)
                .cos()
                .add(1.0)
                .mul(0.5)
                .mul(24.0);

        // let alt_orig = sim_chunk.alt;
        let downhill = sim_chunk.downhill;
        let downhill_pos = downhill.and_then(|downhill_pos| sim.get(downhill_pos));
        let downhill_alt = downhill_pos
            .map(|downhill_chunk| downhill_chunk.alt)
            .unwrap_or(CONFIG.sea_level);
        let wdelta = /*sim_chunk.flux * water_factor*//*0.01f32*/16.0f32;
        let water_alt = sim.get_interpolated_monotone(wpos, |chunk| {
            chunk.water_alt.max(
                /*Lerp::lerp(chunk.alt - 5.0, chunk.alt, chunk.flux * water_factor)*/
                chunk.alt,
            )
            /*    Lerp::lerp(
                    water_alt/*./*max(alt_orig).*/max(alt - 5.0)*/,
                    /*alt - 5.0*/
                    alt,
                    (flux/*(flux - 0.85) / (1.0 - 0.85)*/ * water_factor)
                );
            }*/
        })?; // + riverless_alt_delta;

        let downhill_water_alt = downhill_pos /*sim.get(max_border_river_pos)*/
            .map(|downhill_chunk| {
                /*if downhill_chunk.river.is_river {
                    water_alt
                } else {*/
                downhill_chunk
                    .water_alt
                    .min(sim_chunk.water_alt)
                    .max(sim_chunk.alt /* - wdelta*/)
                /*}*/
            })
            .unwrap_or(CONFIG.sea_level);
        /* let downhill_water_alt = downhill_pos
        .map(|downhill_chunk| downhill_chunk.water_alt
             .min(sim_chunk.water_alt)
             .max(sim_chunk.alt - wdelta))
        .unwrap_or(CONFIG.sea_level); */
        // let water_alt_orig = downhill_water_alt;
        let water_alt_orig = sim.get_interpolated_monotone(wpos, |chunk| {
            chunk.water_alt.sub(2.0).max(
                /*Lerp::lerp(chunk.alt - 5.0, chunk.alt, chunk.flux * water_factor)*/
                chunk.alt.sub(/*wdelta*/ 5.0),
            )
        })?;
        let flux = sim.get_interpolated(wpos, |chunk| chunk.flux)?;
        // let flux = sim_chunk.flux;
        let downhill_flux = downhill_pos
            .map(|downhill_chunk| downhill_chunk.flux)
            .unwrap_or(flux);
        /* let downhill_pos_x = sim.get_interpolated(wpos, |chunk| {
            chunk.downhill.map(|e|
                e.map2(Vec2::from(TerrainChunkSize::RECT_SIZE), |e, sz: u32| e as f32 / sz as f32)
            ).unwrap_or(wpos.map(|e| e as f32))
                .x
        })?;
        let downhill_pos_y = sim.get_interpolated(wpos, |chunk| {
            chunk.downhill.map(|e|
                e.map2(Vec2::from(TerrainChunkSize::RECT_SIZE), |e, sz: u32| e as f32 / sz as f32)
            ).unwrap_or(wpos.map(|e| e as f32))
                .y
        })?;
        let downhill_pos = Vec2::new(downhill_pos_x, downhill_pos_y).map(|e| e as i32);
        let flux = sim.get_interpolated(wpos, |chunk| chunk.flux)?;
        let downhill_flux = sim.get_interpolated(downhill_pos, |chunk| chunk.flux)?; */
        /* let downhill_flux = sim.get_interpolated(wpos, |chunk| {
            let downhill = chunk.downhill;
            let downhill_pos = downhill.and_then(|downhill_pos| sim.get(downhill_pos));
            let downhill_alt = downhill_pos.map(|downhill_chunk| downhill_chunk.alt)
                .unwrap_or(CONFIG.sea_level);
            // let flux = sim.get_interpolated(chunk., |chunk| chunk.flux)?;
            // let flux = chunk.flux;
            let downhill_flux = downhill_pos
                .map(|downhill_chunk| downhill_chunk.flux)
                .unwrap_or(flux);
            downhill_flux
            /* Lerp::lerp(
                downhill_flux,
                flux,
                (wpos - downhill.unwrap_or(wpos))
                    .map2(Vec2::from(TerrainChunkSize::RECT_SIZE), |e, sz: u32| {
                        e as f32 / sz as f32
                    })
                    // TODO: Make from 0 to 1 on diagonals.
                    .magnitude(),
            ) */
        })?; */
        /* let flux =
        Lerp::lerp(
            downhill_flux,
            flux,
            (wpos - downhill.unwrap_or(wpos)/*downhill_pos*/)
                .map2(Vec2::from(TerrainChunkSize::RECT_SIZE), |e, sz: u32| {
                    e as f32 / sz as f32
                })
                // TODO: Make from 0 to 1 on diagonals.
                .magnitude(),
        ); */

        /*Lerp::lerp((alt - 16.0).min(alt), alt.min(alt), (1.0 - flux/*(flux - 0.85) / (1.0 - 0.85)*/ * water_factor)),
        (flux * water_factor - /*0.33*/0.25) * 256.0,
            ),*/

        let water_factor = /*((WORLD_SIZE.x * WORLD_SIZE.y) / 1024) as f32*/1.0 / (1024.0 * 1.0) as f32;
        // let alt_mid = sim.get_interpolated(wpos_mid, |chunk| chunk.alt)?;
        //let water_alt_orig = sim.get_interpolated(wpos, |chunk| chunk.water_alt)?;
        // let water_alt_orig = sim_chunk.water_alt;// + riverless_alt_delta;
        /* let water_alt = sim.get_interpolated(wpos, |chunk| {
            chunk.water_alt
                .max(/*Lerp::lerp(chunk.alt - 5.0, chunk.alt, chunk.flux * water_factor)*/chunk.alt - wdelta)
        /*    Lerp::lerp(
                water_alt/*./*max(alt_orig).*/max(alt - 5.0)*/,
                /*alt - 5.0*/
                alt,
                (flux/*(flux - 0.85) / (1.0 - 0.85)*/ * water_factor)
            );
        }*/

        })?;// + riverless_alt_delta;*/
        /*let water_alt = sim.get_interpolated(wpos, |chunk| {
            chunk.water_alt
                .max(/*Lerp::lerp(chunk.alt - 5.0, chunk.alt, chunk.flux * water_factor)*/chunk.alt)
        /*    Lerp::lerp(
                water_alt/*./*max(alt_orig).*/max(alt - 5.0)*/,
                /*alt - 5.0*/
                alt,
                (flux/*(flux - 0.85) / (1.0 - 0.85)*/ * water_factor)
            );
        }*/

        })?;// + riverless_alt_delta;*/

        // let water_alt_orig = water_alt;
        // let water_alt = sim_chunk.water_alt;// + riverless_alt_delta;

        let is_cliffs = sim_chunk.is_cliffs;
        let near_cliffs = sim_chunk.near_cliffs;

        // Logistic regression.  Make sure x ∈ (0, 1).
        // let logit = |x: f32| x.ln() - x.neg().ln_1p();
        // 0.5 + 0.5 * tanh(ln(1 / (1 - 0.1) - 1) / (2 * (sqrt(3)/pi)))
        // let logistic_2_base = 3.0f32.sqrt().mul(f32::consts::FRAC_2_PI);
        // Assumes μ = 0, σ = 1
        // let logistic_cdf = |x: f32| x.div(logistic_2_base).tanh().mul(0.5).add(0.5);

        /* let water_level =
            riverless_alt - 4.0 /*- 5.0 * chaos */+ logistic_cdf(logit(flux)) * 8.0;
            /*if flux > 0.98 {
            riverless_alt
        } else {
            riverless_alt - 4.0 - 5.0 * chaos
        }; */ */
        // let water_level = riverless_alt - 4.0 - 5.0 * chaos;
        // let water_level = water_alt;

        let river_gouge = 0.5;
        let (alt_, water_level, warp_factor) = /*if /*water_alt_orig == alt_orig.max(0.0)*/water_alt_orig == CONFIG.sea_level {
            // This is flowing into the ocean.
            if alt_orig <= CONFIG.sea_level + 5.0 {
                (alt, CONFIG.sea_level)
            } else {
                (
                    Lerp::lerp(
                        alt,
                        Lerp::lerp((alt - 16.0).min(alt), alt.min(alt), (1.0 - flux/*(flux - 0.85) / (1.0 - 0.85)*/ * water_factor)),
                        (flux * water_factor - /*0.33*/0.25) * 256.0,
                    ),
                    /*Lerp::lerp(
                       alt,*/
                       // Lerp::lerp(alt.min(water_alt), water_alt, flux * water_factor * 16.0),
                       Lerp::lerp(alt - 16.0, alt, (/*(flux - 0.85) / (1.0 - 0.85)*/flux) * water_factor),
                       /*(flux - water_factor * 0.33) * 256.0,*/
                )
            }
        } else *//*if let Some(downhill) = downhill */
        if let Some((max_border_river_pos, river_chunk, max_border_river, max_border_river_dist)) = max_river {
            // This is flowing into a lake, or a lake, or is at least a non-ocean tile.
            //
            // If we are <= water_alt, we are in the lake; otherwise, we are flowing into it.
            /* if alt <= water_alt {
                (alt - 5.0, water_alt)
            } else *//*{*/
                /* // Compute the delta between alt and the "real" height of the chunk.
                let z_delta = alt - alt_orig;
                // Also find the horizontal delta.
                let xy_delta = wposf - wposf_mid; */
                // Find the slope, extend it to distance 5, and then project to the z axis.
                let (new_alt, new_water_alt, warp_factor) =
                    /*max_neighbor_river.and_then(|(river_pos, river)| {*/
                        /*river*/max_border_river.river_kind
                        .and_then(|river_kind| if let RiverKind::River { cross_section } = river_kind {
                            if max_border_river_dist.map(|(_, dist, _, _)| dist) != Some(Vec2::zero()) {
                                return None;
                            }
                            let (_, _, river_width, (river_t, (river_pos, _),
                                 downhill_river_chunk)) = max_border_river_dist.unwrap();
                            let river_alt = /*sim.get(/*river_pos*/max_border_river_pos)?*/
                                Lerp::lerp(
                                    river_chunk.alt.max(river_chunk.water_alt),
                                    downhill_river_chunk.alt.max(downhill_river_chunk.water_alt),
                                    river_t,
                                );
                            let new_alt = /*water_alt.min(river_alt)*/river_alt - river_gouge;//sim.get_interpolated(wpos, |chunk| chunk.alt./*min*/min(river_alt))?;
                            // println!("Pos: {:?}, river: {:?}", wposf, river.cross_section);
                            let river_dist = wposf.map(|e| e as f32).distance(river_pos);
                            let river_height_factor = river_dist / (river_width * 0.5);

                            Some((Lerp::lerp(
                                        new_alt - /*river*/cross_section.y.max(1.0),
                                        new_alt - 1.0,
                                        river_height_factor * river_height_factor),
                                  new_alt,
                                  0.0))
                        } else { None })
                    /*})*/
                    .unwrap_or_else(|| max_border_river.river_kind.and_then(|river_kind| {
                        // let river_chunk = sim.get(/*river_pos*/max_border_river_pos)?;
                        // let river_alt = if let Some((_, _, _, (river_t, downhill_chunk))) = river_chunk.alt;
                        // let new_alt = alt.min(river_alt)/*river_alt*/;//sim.get_interpolated(wpos, |chunk| chunk.alt./*min*/min(river_alt))?;
                        // let new_alt = alt./*min*/min(river_alt);
                        match river_kind {
                            RiverKind::Ocean => None,
                            RiverKind::Lake { .. } => {
                                let lake_dist = (max_border_river_pos.map(|e| e as f32) * neighbor_coef).distance(wposf.map(|e| e as f32));
                                let (_, dist, river_width, (river_t, (river_pos, _), downhill_river_chunk)) =
                                    if let Some(dist) = max_border_river_dist {
                                        dist
                                    } else {
                                        if lake_dist <= TerrainChunkSize::RECT_SIZE.x as f32 * 0.75 {
                                            let gouge_factor = if lake_dist <= TerrainChunkSize::RECT_SIZE.x as f32 * 0.5 + 2.0 { 1.0 } else { 0.0 };
                                            let lake_alt = alt - (1.0 + river_gouge) * gouge_factor;/*river_gouge -
                                                /*Lerp::lerp(
                                                    (river_chunk.alt.max(river_chunk.water_alt) - 1.0).min(alt - 1.0),
                                                    alt,
                                                    lake_dist / (TerrainChunkSize::RECT_SIZE.x as f32 * 0.5),
                                                )*//*if lake_dist <= TerrainChunkSize::RECT_SIZE.x as f32 * 0.5 + 2.0 { 1.0 } else { 0.0 }*/1.0*/;
                                            // /println!("Got it!");
                                            /* let lake_delta = (lake_dist - TerrainChunkSize::RECT_SIZE.x as f32 * 0.5).max(0.0);
                                            if lake_delta == 0.0 {
                                                return Some((alt - 1.0, /*river_chunk.water_alt.min(downhill_water_alt)*//*water_alt*//*downhill_water_alt*/
                                                         river_chunk.alt.max(river_chunk.water_alt),
                                                         river_scale_factor)) */
                                            return Some((if gouge_factor == 0.0 { lake_alt } else { lake_alt.min(river_chunk.alt.max(river_chunk.water_alt) - 1.0 - river_gouge * gouge_factor) }, (river_chunk.alt.max(river_chunk.water_alt) - river_gouge * gouge_factor).min(water_alt - river_gouge * gouge_factor), river_scale_factor * (1.0 - gouge_factor)));
                                        } else {
                                            // println!("Lake: {:?} Here: {:?}, Lake: {:?}", max_border_river, chunk_pos, max_border_river_pos.map(|e| e as f32) * neighbor_coef);
                                            return Some((alt, /*river_chunk.water_alt.min(downhill_water_alt)*//*water_alt*//*downhill_water_alt*/downhill_water_alt, river_scale_factor));
                                        }
                                    };

                                /* if let Some((_, dist, river_width,
                                                 (river_t, (river_pos, _), downhill_river_chunk))) = max_border_river_dist {
                                    (dist, river_pos, river_width, river_t, downhill_river_chunk)
                                } else {
                                    // println!("Lake: {:?} Here: {:?}, Lake: {:?}", max_border_river, chunk_pos, max_border_river_pos);
                                    return None;
                                }; */
                                let river_alt = /*sim.get(/*river_pos*/max_border_river_pos)?*/
                                    Lerp::lerp(
                                        river_chunk.alt.max(river_chunk.water_alt),
                                        downhill_river_chunk.alt.max(downhill_river_chunk.water_alt),
                                        river_t,
                                    )/*river_chunk.alt.max(river_chunk.water_alt)*/;
                                if dist == Vec2::zero() {
                                    let new_alt = /*water_alt.min(river_alt)*/river_alt - river_gouge/* - river_gouge*/;//sim.get_interpolated(wpos, |chunk| chunk.alt./*min*/min(river_alt))?;
                                    /* let (_, _, river_width, (river_t, (river_pos, _),
                                         downhill_river_chunk)) = max_border_river_dist.unwrap(); */
                                    // println!("Pos: {:?}, river: {:?}", wposf, river.cross_section);
                                    let river_dist = wposf.map(|e| e as f32).distance(river_pos);
                                    let river_height_factor = river_dist / (river_width * 0.5);

                                    return Some((/*Lerp::lerp(
                                                alt.min(new_alt - 1.0),
                                                new_alt - 1.0,
                                                river_height_factor * river_height_factor)*/(alt - river_gouge - 1.0).min(new_alt - 1.0),
                                          new_alt,
                                          0.0));
                                }
                                if lake_dist <= TerrainChunkSize::RECT_SIZE.x as f32 * 0.75 {
                                    let gouge_factor = if lake_dist <= TerrainChunkSize::RECT_SIZE.x as f32 * 0.5 + 2.0 { 1.0 } else { 0.0 };
                                    return Some((alt - (1.0 + river_gouge) * gouge_factor/*if lake_dist <= TerrainChunkSize::RECT_SIZE.x as f32 * 0.5 + 2.0 { 1.0 } else { 0.0 }*/, /*water_alt.min(downhill_river_chunk.water_alt), *//*river_chunk.water_alt.min(downhill_water_alt)*//*water_alt*//*downhill_water_alt*/

                                                 /*river_chunk.alt.max(river_chunk.water_alt)/* - river_gouge*/*//*new_alt*//*downhill_river_chunk.water_alt.max(downhill_river_chunk.alt)*/river_alt - river_gouge * gouge_factor,
                                                 /*downhill_water_alt,*/ river_scale_factor * (1.0 - gouge_factor)));
                                }
                                /* if dist.y >= TerrainChunkSize::RECT_SIZE.x as f32 {
                                } */
                                // let dist = max_border_river_dist?;
                                /* let downhill = sim_chunk.downhill;
                                let downhill_pos = downhill.and_then(|downhill_pos| sim.get(downhill_pos));
                                let downhill_alt = downhill_pos
                                    .map(|downhill_chunk| downhill_chunk.alt)
                                    .unwrap_or(CONFIG.sea_level); */
                                Some((/*alt.min(river_alt)*/
                                      // Lerp::lerp(alt.min(river_chunk.alt), downhill_river_chunk.alt, river_scale_factor)
                                     /*Lerp::lerp(river_alt/* - /*river*/max_border_river.cross_section.y.max(1.0)*/,
                                                alt - 1.0,
                                                /*dist.magnitude() / ((2 * TerrainChunkSize::RECT_SIZE.x) as f32 - river_width)*/
                                                river_scale_factor)*/alt,
                                      /*alt.min(Lerp::lerp(river_chunk.water_alt - 1.0, alt, river_scale_factor)),*/
                                      /*river_chunk.water_alt*/
                                      /*Lerp::lerp(water_alt, downhill_water_alt, dist.magnitude() / 16.0)),*/
                                      /*Lerp::lerp(water_alt, downhill_water_alt, dist.magnitude() / 16.0),
                                      dist.magnitude() / 16.0)*/
                                      /*downhill_water_alt*//*river_chunk.water_alt.min(sim_chunk.water_alt).max(sim_chunk.alt)*//*water_alt.min(river_chunk.water_alt*/
                                      /* downhill_chunk.water_alt.max(downhill_chunk.alt)
                                      .min(river_chunk.water_alt)
                                      .max(river_chunk.alt/* - wdelta*/) */

                                      /*river_chunk.water_alt*//*downhill_water_alt*//*water_alt.min(*//*water_alt.min(downhill_river_chunk.water_alt*/water_alt.min(downhill_river_chunk.water_alt.max(downhill_river_chunk.alt) - river_gouge)/*)*/,
                                      // water_alt.min(river_chunk.water_alt),
                                      river_scale_factor))
                            },
                            RiverKind::River { .. } => {
                                let (river_t, downhill_river_chunk) = if let Some((_, _, _, (river_t, _, downhill_river_chunk))) = max_border_river_dist {
                                    (river_t, downhill_river_chunk)
                                } else {
                                    println!("Lake: {:?} Here: {:?}, Lake: {:?}", max_border_river, chunk_pos, max_border_river_pos);
                                    panic!("Lakes should definitely have a downhill! ...Right?");
                                    // (0.0, river_chunk)
                                };

                                let river_alt = /*sim.get(/*river_pos*/max_border_river_pos)?*/
                                    Lerp::lerp(
                                        river_chunk.alt.max(river_chunk.water_alt),
                                        downhill_river_chunk.alt.max(downhill_river_chunk.water_alt),
                                        river_t,
                                    );
                                let new_alt = /*alt.min(river_alt)*/river_alt;//sim.get_interpolated(wpos, |chunk| chunk.alt./*min*/min(river_alt))?;
                                //
                                //
                                // let dist = max_border_river_dist?;
                                // let river_width = river_chunk.river.cross_section.x;
                                // let orig_alt = (new_alt - /*river*/max_border_river.cross_section.y.max(1.0);
                                Some(
                                    (/*Lerp::lerp(new_alt/* - /*river*/max_border_river.cross_section.y.max(1.0)*/,
                                                alt, dist.magnitude() / 16.0),*/
                                     /*Lerp::lerp(new_alt/* - /*river*/max_border_river.cross_section.y.max(1.0)*/,
                                                alt,
                                                /*dist.magnitude() / ((2 * TerrainChunkSize::RECT_SIZE.x) as f32 - river_width)*/
                                                river_scale_factor)/*alt*/*/alt,
                                     /*Lerp::lerp(new_alt/*water_alt*/, /*river_chunk.water_alt*/downhill_water_alt,
                                                dist.magnitude() / 16.0))*/
                                     /*Lerp::lerp(new_alt - /*river*/max_border_river.cross_section.y.max(1.0), downhill_water_alt,
                                                dist.magnitude() / 16.0),*/
                                     // Lerp::lerp(new_alt, downhill_water_alt, dist.magnitude() / 16.0),
                                     // FIXME: Make accurate.
                                     downhill_water_alt/*new_alt*/,
                                     /*dist.magnitude()*//*dist.magnitude() / *//*(max_border_river_pos - wposf).magnitude()*/
                                     /*wposf.map(|e| e as f32) - max_border_river_pos.map(|e| e as f32)
                                      .mul(neighbor_coef).mul(0.5)).magnitude()
                                     .max(1e-7)*//*16.0*//*((2 * TerrainChunkSize::RECT_SIZE.x) as f32 - river_width)*/
                                     river_scale_factor))
                            }
                        }
                        // Some((alt, water_alt))
                    })
                    .unwrap_or((alt, downhill_water_alt, /*1.0*/river_scale_factor)));
/*                        Some(RiverKind::Lake) | Some(RiverKind::Ocean) => (alt, water_alt),
                        Some(RiverKind::River) => {
                            // FIXME: Lerp.
                            (alt, water_alt)
                            max_border_river_dist.and_then(|dist|  {
                                // In which direction?
                                let neighbor_pos = if dist.x < 0.0 {
                                    if dist.y < 0.0 { }
                                    else if dist2 == 0.0 { }
                                    else { }
                                }
                                Some(
                                    (Lerp::lerp(new_alt  - /*river*/max_border_river.cross_section.y.max(1.0),
                                                alt, dist.magnitude() / 16.0),
                                     Lerp::lerp(new_alt/*water_alt*/, downhill_water_alt, dist.magnitude() / 16.0)))
                            }).unwrap_or((alt, downhill_water_alt))
                            // (alt, water_alt)
                        },
                        None => (alt, downhill_water_alt)
                    });*/
                /* // let (river_idx, max_neighbor_river) = max_neighbor_river;
                // let river_pos = uniform_idx_as_vec2(river_idx);
                if max_neighbor_river.is_river {
                    downhill_water_alt
                } else {
                    water_alt
                };
                let new_alt = if max_neighbor_river.is_river {
                    alt - max_neighbor_river.cross_section.y.max(1.0)
                } else {
                    alt
                }; */
                /*let new_water_alt =
                        Lerp::lerp(
                            downhill_water_alt/*./*max(alt_orig).*/max(alt - 5.0)*/,
                            /*alt - 5.0*/
                            water_alt,
                            neighbor_dim,
                            // flux/*(flux - 0.85) / (1.0 - 0.85)*/ * water_factor / wdelta
                        );*/
                /*let new_alt =
                    alt - flux * water_factor;*/
                    /* Lerp::lerp(
                        alt,
                        (alt - wdelta).min(flux * water_factor),
                        flux/*(flux - 0.85) / (1.0 - 0.85)*/ * water_factor
                    ); */
                (
                    /* Lerp::lerp(
                        alt - wdelta,
                        new_alt,
                        (alt - /*5.0 - */water_alt/*.max(alt_orig)*/)/*.div(CONFIG.mountain_scale * 0.25)*/ * 1.0,
                    ),
                    Lerp::lerp(
                        /*water_alt_orig*/water_alt/*.max(water_alt_orig)*//*./*max(alt_orig).*/max(alt - 5.0)*/,
                        new_water_alt,
                        (alt /*- 5.0 */- water_alt/*.max(alt_orig)*/) * 1.0,
                    ), */
                    new_alt,
                    new_water_alt,
                    warp_factor,
                )
                /*(
                    Lerp::lerp(
                        alt,
                        Lerp::lerp(water_alt, alt, (1.0 - flux/*(flux - 0.85) / (1.0 - 0.85)*/ * water_factor)),
                        (alt - water_alt) * 256.0 + (flux * water_factor - /*0.33*/0.25) * 256.0,
                    ),
                    /*Lerp::lerp(
                       alt,*/
                       // Lerp::lerp(alt.min(water_alt), water_alt, flux * water_factor * 16.0),
                       Lerp::lerp(water_alt, alt, (/*(flux - 0.85) / (1.0 - 0.85)*/flux) * water_factor),
                       /*(flux - water_factor * 0.33) * 256.0,*/
                )*/
        } else {
            (alt, downhill_water_alt, 1.0)
        }/* else {
            // Ocean tile, just use sea level.
            (alt, water_alt_orig)
        }*/;
        /* let (alt_, water_level, warp_factor) = if river_data.river_kind == Some(RiverKind::Lake) {
            println!("Okay pos: {:?} river: {:?} alt: {:}? water_alt: {:?} sim_chunk.water_alt: {:?}", wposf, river, alt, water_alt, sim_chunk.water_alt);
            (alt, water_alt.max(sim_chunk.water_alt), 0.0)
        } else {
            (alt_, water_level, warp_factor)
        }; */

        /* let alt_sub = alt.sub(water_level/* - 1.0*/).powi(2).max(1e-7)
        .min(wdelta.powi(2) - 1e-7).div(wdelta.powi(2))
        .mul(alt.sub(water_level/* - 1.0*/).signum()); */
        let warp_factor = 0.0;
        let riverless_alt_delta = Lerp::lerp(
            0.0,
            riverless_alt_delta,
            /*alt_sub.div(1.0.sub(alt_sub)).tanh()*/
            warp_factor,
        );
        /* let water_alt = if alt_ > water_alt {
            water_alt + riverless_alt_delta
        } else {
            water_alt
        }; */
        let alt = alt_ + riverless_alt_delta;
        let alt_old = alt_old + riverless_alt_delta;

        let rock = (sim.gen_ctx.small_nz.get(
            Vec3::new(wposf.x, wposf.y, alt as f64)
                .div(100.0)
                .into_array(),
        ) as f32)
            .mul(rockiness)
            .sub(0.4)
            .max(0.0)
            .mul(8.0);

        let wposf3d = Vec3::new(wposf.x, wposf.y, alt as f64);

        let marble_small = (sim.gen_ctx.hill_nz.get((wposf3d.div(3.0)).into_array()) as f32)
            .powf(3.0)
            .add(1.0)
            .mul(0.5);
        let marble = (sim.gen_ctx.hill_nz.get((wposf3d.div(48.0)).into_array()) as f32)
            .mul(0.75)
            .add(1.0)
            .mul(0.5)
            .add(marble_small.sub(0.5).mul(0.25));

        let temp = temp.add((marble - 0.5) * 0.25);
        let humidity = humidity.add((marble - 0.5) * 0.25);

        // Colours
        let cold_grass = Rgb::new(0.0, 0.5, 0.25);
        let warm_grass = Rgb::new(0.03, 0.8, 0.0);
        let dark_grass = Rgb::new(0.01, 0.3, 0.0);
        let wet_grass = Rgb::new(0.1, 0.8, 0.2);
        let cold_stone = Rgb::new(0.57, 0.67, 0.8);
        let warm_stone = Rgb::new(0.77, 0.77, 0.64);
        let beach_sand = Rgb::new(0.9, 0.82, 0.6);
        let desert_sand = Rgb::new(0.95, 0.75, 0.5);
        let snow = Rgb::new(0.8, 0.85, 1.0);

        let dirt = Lerp::lerp(
            Rgb::new(0.075, 0.07, 0.3),
            Rgb::new(0.75, 0.55, 0.1),
            marble,
        );
        let tundra = Lerp::lerp(snow, Rgb::new(0.01, 0.3, 0.0), 0.4 + marble * 0.6);
        let dead_tundra = Lerp::lerp(warm_stone, Rgb::new(0.3, 0.12, 0.2), marble);
        let cliff = Rgb::lerp(cold_stone, warm_stone, marble);

        let grass = Rgb::lerp(
            cold_grass,
            warm_grass,
            marble.sub(0.5).add(1.0.sub(humidity).mul(0.5)).powf(1.5),
        );
        let snow_moss = Rgb::lerp(snow, cold_grass, 0.4 + marble.powf(1.5) * 0.6);
        let moss = Rgb::lerp(dark_grass, cold_grass, marble.powf(1.5));
        let rainforest = Rgb::lerp(wet_grass, warm_grass, marble.powf(1.5));
        let sand = Rgb::lerp(beach_sand, desert_sand, marble);

        let tropical = Rgb::lerp(
            Rgb::lerp(
                grass,
                Rgb::new(0.15, 0.2, 0.15),
                marble_small
                    .sub(0.5)
                    .mul(0.2)
                    .add(0.75.mul(1.0.sub(humidity)))
                    .powf(0.667),
            ),
            Rgb::new(0.87, 0.62, 0.56),
            marble.powf(1.5).sub(0.5).mul(4.0),
        );

        // For below desert humidity, we are always sand or rock, depending on altitude and
        // temperature.
        let ground = Rgb::lerp(
            Rgb::lerp(
                dead_tundra,
                sand,
                temp.sub(CONFIG.snow_temp)
                    .div(CONFIG.desert_temp.sub(CONFIG.snow_temp))
                    .mul(0.5),
            ),
            cliff,
            alt.sub(CONFIG.mountain_scale * 0.25)
                .div(CONFIG.mountain_scale * 0.125),
        );
        // From desert to forest humidity, we go from tundra to dirt to grass to moss to sand,
        // depending on temperature.
        let ground = Rgb::lerp(
            ground,
            Rgb::lerp(
                Rgb::lerp(
                    Rgb::lerp(
                        Rgb::lerp(
                            tundra,
                            // snow_temp to 0
                            dirt,
                            temp.sub(CONFIG.snow_temp)
                                .div(CONFIG.snow_temp.neg())
                                /*.sub((marble - 0.5) * 0.05)
                                .mul(256.0)*/
                                .mul(1.0),
                        ),
                        // 0 to tropical_temp
                        grass,
                        temp.div(CONFIG.tropical_temp).mul(4.0),
                    ),
                    // tropical_temp to desert_temp
                    moss,
                    temp.sub(CONFIG.tropical_temp)
                        .div(CONFIG.desert_temp.sub(CONFIG.tropical_temp))
                        .mul(1.0),
                ),
                // above desert_temp
                sand,
                temp.sub(CONFIG.desert_temp)
                    .div(1.0 - CONFIG.desert_temp)
                    .mul(4.0),
            ),
            humidity
                .sub(CONFIG.desert_hum)
                .div(CONFIG.forest_hum.sub(CONFIG.desert_hum))
                .mul(1.0),
        );
        // From forest to jungle humidity, we go from snow to dark grass to grass to tropics to sand
        // depending on temperature.
        let ground = Rgb::lerp(
            ground,
            Rgb::lerp(
                Rgb::lerp(
                    Rgb::lerp(
                        snow_moss,
                        // 0 to tropical_temp
                        grass,
                        temp.div(CONFIG.tropical_temp).mul(4.0),
                    ),
                    // tropical_temp to desert_temp
                    tropical,
                    temp.sub(CONFIG.tropical_temp)
                        .div(CONFIG.desert_temp.sub(CONFIG.tropical_temp))
                        .mul(1.0),
                ),
                // above desert_temp
                sand,
                temp.sub(CONFIG.desert_temp)
                    .div(1.0 - CONFIG.desert_temp)
                    .mul(4.0),
            ),
            humidity
                .sub(CONFIG.forest_hum)
                .div(CONFIG.jungle_hum.sub(CONFIG.forest_hum))
                .mul(1.0),
        );
        // From jungle humidity upwards, we go from snow to grass to rainforest to tropics to sand.
        let ground = Rgb::lerp(
            ground,
            Rgb::lerp(
                Rgb::lerp(
                    Rgb::lerp(
                        snow_moss,
                        // 0 to tropical_temp
                        rainforest,
                        temp.div(CONFIG.tropical_temp).mul(4.0),
                    ),
                    // tropical_temp to desert_temp
                    tropical,
                    temp.sub(CONFIG.tropical_temp)
                        .div(CONFIG.desert_temp.sub(CONFIG.tropical_temp))
                        .mul(4.0),
                ),
                // above desert_temp
                sand,
                temp.sub(CONFIG.desert_temp)
                    .div(1.0 - CONFIG.desert_temp)
                    .mul(4.0),
            ),
            humidity.sub(CONFIG.jungle_hum).mul(1.0),
        );

        // Snow covering
        let ground = Rgb::lerp(
            snow,
            ground,
            temp.sub(CONFIG.snow_temp)
                .max(-humidity.sub(CONFIG.desert_hum))
                .mul(16.0)
                .add((marble_small - 0.5) * 0.5),
        );

        /*
        // Work out if we're on a path or near a town
        let dist_to_path = match &sim_chunk.location {
            Some(loc) => {
                let this_loc = &sim.locations[loc.loc_idx];
                this_loc
                    .neighbours
                    .iter()
                    .map(|j| {
                        let other_loc = &sim.locations[*j];

                        // Find the two location centers
                        let near_0 = this_loc.center.map(|e| e as f32);
                        let near_1 = other_loc.center.map(|e| e as f32);

                        // Calculate distance to path between them
                        (0.0 + (near_1.y - near_0.y) * wposf_turb.x as f32
                            - (near_1.x - near_0.x) * wposf_turb.y as f32
                            + near_1.x * near_0.y
                            - near_0.x * near_1.y)
                            .abs()
                            .div(near_0.distance(near_1))
                    })
                    .filter(|x| x.is_finite())
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap_or(f32::INFINITY)
            }
            None => f32::INFINITY,
        };

        let on_path = dist_to_path < 5.0 && !sim_chunk.near_cliffs; // || near_0.distance(wposf_turb.map(|e| e as f32)) < 150.0;

        let (alt, ground) = if on_path {
            (alt - 1.0, dirt)
        } else {
            (alt, ground)
        };
        */

        // Cities
        // TODO: In a later MR
        /*
        let building = match &sim_chunk.location {
            Some(loc) => {
                let loc = &sim.locations[loc.loc_idx];
                let rpos = wposf.map2(loc.center, |a, b| a as f32 - b as f32) / 256.0 + 0.5;

                if rpos.map(|e| e >= 0.0 && e < 1.0).reduce_and() {
                    (loc.settlement
                        .get_at(rpos)
                        .map(|b| b.seed % 20 + 10)
                        .unwrap_or(0)) as f32
                } else {
                    0.0
                }
            }
            None => 0.0,
        };

        let alt = alt + building;
        */

        // Caves
        let cave_at = |wposf: Vec2<f64>| {
            (sim.gen_ctx.cave_0_nz.get(
                Vec3::new(wposf.x, wposf.y, alt as f64 * 8.0)
                    .div(800.0)
                    .into_array(),
            ) as f32)
                .powf(2.0)
                .neg()
                .add(1.0)
                .mul((1.15 - chaos).min(1.0))
        };
        let cave_xy = cave_at(wposf);
        let cave_alt = alt - 24.0
            + (sim
                .gen_ctx
                .cave_1_nz
                .get(Vec2::new(wposf.x, wposf.y).div(48.0).into_array()) as f32)
                * 8.0
            + (sim
                .gen_ctx
                .cave_1_nz
                .get(Vec2::new(wposf.x, wposf.y).div(500.0).into_array()) as f32)
                .add(1.0)
                .mul(0.5)
                .powf(15.0)
                .mul(150.0);

        /* let sea_level = if alt_old < CONFIG.sea_level {
            CONFIG.sea_level
        } else {
            water_level
        }; */
        let near_ocean = max_river.and_then(|(_, _, river_data, _)| river_data.river_kind)
            == Some(RiverKind::Ocean);
        /* if river_data.river_kind == Some(RiverKind::Lake) && water_level < alt && water_alt >= alt {
            println!("Bug");
        } */

        let ocean_level = if near_ocean {
            alt - CONFIG.sea_level
        } else {
            5.0
        };

        let alt_min = max_river
            // .map(|(_, _, _, max_border_river_dist)| water_alt)
            .and_then(|(_, _, _, max_border_river_dist)| max_border_river_dist)
            .map(|(_, _, _, (_, _, downhill_river_chunk))| {
                alt.min(downhill_river_chunk.alt.max(downhill_river_chunk.water_alt))
            })
            .unwrap_or(alt);
        /*if water_level.max(water_alt) >= alt {
            alt
        } else {
            downhill_water_alt
        };*/

        Some(ColumnSample {
            alt,
            alt_min,
            alt_old,
            chaos,
            sea_level: if water_level.max(water_alt) >= alt
            /*_*/
            {
                alt
            } else {
                downhill_water_alt
            }, /*/*water_alt.sub(5.0),/**/ */water_alt_orig.sub(/*wdelta*/5.0)*//*downhill_water_alt*//*water_alt_orig.sub(5.0),*/
            water_level,
            flux,
            river,
            warp_factor,
            surface_color: Rgb::lerp(
                sand,
                // Land
                /*Rgb::lerp(
                    ground,
                    // Mountain
                    Rgb::lerp(
                        cliff,
                        snow,
                        (alt - CONFIG.sea_level
                            - 0.7 * CONFIG.mountain_scale
                            // - alt_base
                            - temp * 96.0
                            - marble * 24.0)
                            / 12.0,
                    ),
                    (alt - CONFIG.sea_level - 0.25 * CONFIG.mountain_scale + marble * 128.0)
                        / (0.25 * CONFIG.mountain_scale),
                ),*/
                ground,
                // Beach
                ((/*alt_old - CONFIG.sea_level/*alt - sea_level*/*/ocean_level - 1.0) / 2.0)
                    .min(1.0 - river * 2.0)
                    .max(0.0),
            ),
            sub_surface_color: dirt,
            tree_density,
            forest_kind: sim_chunk.forest_kind,
            close_structures: self.gen_close_structures(wpos),
            cave_xy,
            cave_alt,
            marble,
            marble_small,
            rock,
            is_cliffs,
            near_cliffs,
            cliff_hill,
            close_cliffs: sim.gen_ctx.cliff_gen.get(wpos),
            temp,
            spawn_rate,
            location: sim_chunk.location.as_ref(),

            chunk: sim_chunk,
            spawn_rules: sim_chunk
                .structures
                .town
                .as_ref()
                .map(|town| TownGen.spawn_rules(town, wpos))
                .unwrap_or(SpawnRules::default()),
        })
    }
}

#[derive(Clone)]
pub struct ColumnSample<'a> {
    pub alt: f32,
    pub alt_min: f32,
    pub alt_old: f32,
    pub chaos: f32,
    pub sea_level: f32,
    pub water_level: f32,
    pub flux: f32,
    pub river: f32,
    pub warp_factor: f32,
    pub surface_color: Rgb<f32>,
    pub sub_surface_color: Rgb<f32>,
    pub tree_density: f32,
    pub forest_kind: ForestKind,
    pub close_structures: [Option<StructureData>; 9],
    pub cave_xy: f32,
    pub cave_alt: f32,
    pub marble: f32,
    pub marble_small: f32,
    pub rock: f32,
    pub is_cliffs: bool,
    pub near_cliffs: bool,
    pub cliff_hill: f32,
    pub close_cliffs: [(Vec2<i32>, u32); 9],
    pub temp: f32,
    pub spawn_rate: f32,
    pub location: Option<&'a LocationInfo>,

    pub chunk: &'a SimChunk,
    pub spawn_rules: SpawnRules,
}

#[derive(Copy, Clone)]
pub struct StructureData {
    pub pos: Vec2<i32>,
    pub seed: u32,
    pub meta: Option<StructureMeta>,
}
