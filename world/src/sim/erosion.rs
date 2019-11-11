use super::{downhill, neighbors, uniform_idx_as_vec2, uphill, WORLD_SIZE};
use crate::{config::CONFIG, util::RandomField};
use common::{terrain::TerrainChunkSize, vol::RectVolSize};
use noise::{NoiseFn, Point3};
use num::Float;
use ordered_float::NotNan;
use rayon::prelude::*;
use std::{
    cmp::{Ordering, Reverse},
    collections::BinaryHeap,
    f32, f64, mem, u32,
};
use vek::*;

/// Compute the water flux at all chunks, given a list of chunk indices sorted by increasing
/// height.
pub fn get_drainage(newh: &[u32], downhill: &[isize], _boundary_len: usize) -> Box<[f32]> {
    // FIXME: Make the below work.  For now, we just use constant flux.
    // Initially, flux is determined by rainfall.  We currently treat this as the same as humidity,
    // so we just use humidity as a proxy.  The total flux across the whole map is normalize to
    // 1.0, and we expect the average flux to be 0.5.  To figure out how far from normal any given
    // chunk is, we use its logit.
    // NOTE: If there are no non-boundary chunks, we just set base_flux to 1.0; this should still
    // work fine because in that case there's no erosion anyway.
    // let base_flux = 1.0 / ((WORLD_SIZE.x * WORLD_SIZE.y) as f32);
    let base_flux = 1.0;
    let mut flux = vec![base_flux; WORLD_SIZE.x * WORLD_SIZE.y].into_boxed_slice();
    for &chunk_idx in newh.into_iter().rev() {
        let chunk_idx = chunk_idx as usize;
        let downhill_idx = downhill[chunk_idx];
        if downhill_idx >= 0 {
            flux[downhill_idx as usize] += flux[chunk_idx];
        }
    }
    flux
}

/// Kind of water on this tile.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RiverKind {
    Ocean,
    Lake {
        /// In addition to a downhill node (pointing to, eventually, the bottom of the lake), each
        /// lake also has a "pass" that identifies the direction out of which water should flow
        /// from this lake if it is minimally flooded.  While some lakes may be too full for this
        /// to be the actual pass used by their enclosing lake, we still use this as a way to make
        /// sure that lake connections to rivers flow in the correct direction.
        neighbor_pass_pos: Vec2<i32>,
    },
    /// River should be maximal.
    River {
        /// Dimensions of the river's cross-sectional area, as m × m (rivers are approximated
        /// as an open rectangular prism in the direction of the velocity vector).
        cross_section: Vec2<f32>,
    },
}

impl RiverKind {
    pub fn is_river(&self) -> bool {
        if let RiverKind::River { .. } = *self {
            true
        } else {
            false
        }
    }

    pub fn is_lake(&self) -> bool {
        if let RiverKind::Lake { .. } = *self {
            true
        } else {
            false
        }
    }
}

impl PartialOrd for RiverKind {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (*self, *other) {
            (RiverKind::Ocean, RiverKind::Ocean) => Some(Ordering::Equal),
            (RiverKind::Ocean, _) => Some(Ordering::Less),
            (_, RiverKind::Ocean) => Some(Ordering::Greater),
            (RiverKind::Lake { .. }, RiverKind::Lake { .. }) => None,
            (RiverKind::Lake { .. }, _) => Some(Ordering::Less),
            (_, RiverKind::Lake { .. }) => Some(Ordering::Greater),
            (RiverKind::River { .. }, RiverKind::River { .. }) => None,
        }
    }
}

/// From velocity and cross_section we can calculate the volumetric flow rate, as the
/// cross-sectional area times the velocity.
///
/// TODO: we might choose to include a curve for the river, as long as it didn't allow it to
/// cross more than one neighboring chunk away.  For now we defer this to rendering time.
///
/// NOTE: This structure is 57 (or more likely 64) bytes, which is kind of big.
#[derive(Clone, Debug, Default)]
pub struct RiverData {
    /// A velocity vector (in m / minute, i.e. voxels / second from a game perspective).
    ///
    /// TODO: To represent this in a better-packed way, use u8s instead (as "f8s").
    pub(crate) velocity: Vec3<f32>,
    /// The computed derivative for the segment of river starting at this chunk (and flowing
    /// downhill).  Should be 0 at endpoints.  For rivers with more than one incoming segment, we
    /// weight the derivatives by flux (cross-sectional area times velocity) which is correlated
    /// with mass / second; treating the derivative as "velocity" with respect to length along the
    /// river, we treat the weighted sum of incoming splines as the "momentum", and can divide it
    /// by the total incoming mass as a way to find the velocity of the center of mass.  We can
    /// then use this derivative to find a "tangent" for the incoming river segment at this point,
    /// and as the linear part of the interpolating spline at this point.
    ///
    /// Note that we aren't going to have completely smooth curves here anyway, so we will probably
    /// end up applying a dampening factor as well (maybe based on the length?) to prevent
    /// extremely wild oscillations.
    pub(crate) spline_derivative: Vec2<f32>,
    /// If this chunk is part of a river, this should be true.  We can't just compute this from the
    /// cross section because once a river becomes visible, we want it to stay visible until it
    /// reaches its sink.
    pub river_kind: Option<RiverKind>,
    /// We also have a second record for recording any rivers in nearby chunks that manage to
    /// intersect this chunk, though this is unlikely to happen in current gameplay.  This is
    /// because river areas are allowed to cross arbitrarily many chunk boundaries, if they are
    /// wide enough.  In such cases we may choose to render the rivers as particularly deep in
    /// those places.
    pub(crate) neighbor_rivers: Vec<u32>,
}

impl RiverData {
    pub fn is_river(&self) -> bool {
        self.river_kind
            .as_ref()
            .map(RiverKind::is_river)
            .unwrap_or(false)
    }

    pub fn is_lake(&self) -> bool {
        self.river_kind
            .as_ref()
            .map(RiverKind::is_lake)
            .unwrap_or(false)
    }
}

/// Draw rivers and assign them heights, widths, and velocities.  Take some liberties with the
/// constant factors etc. in order to make it more likely that we draw rivers at all.
pub fn get_rivers(
    newh: &[u32],
    water_alt: &[f32],
    downhill: &[isize],
    indirection: &[i32],
    drainage: &[f32],
) -> Box<[RiverData]> {
    // For continuity-preserving quadratic spline interpolation, we (appear to) need to build
    // up the derivatives from the top down.  Fortunately this computation seems tractable.

    let mut rivers = vec![RiverData::default(); WORLD_SIZE.x * WORLD_SIZE.y].into_boxed_slice();
    let neighbor_coef = TerrainChunkSize::RECT_SIZE.map(|e| e as f64);
    // NOTE: This technically makes us discontinuous, so we should be cautious about using this.
    let derivative_divisor = 1.0;
    for &chunk_idx in newh.into_iter().rev() {
        let chunk_idx = chunk_idx as usize;
        let downhill_idx = downhill[chunk_idx];
        if downhill_idx < 0 {
            // We are in the ocean.
            debug_assert!(downhill_idx == -2);
            rivers[chunk_idx].river_kind = Some(RiverKind::Ocean);
            continue;
        }
        let downhill_idx = downhill_idx as usize;
        let downhill_pos = uniform_idx_as_vec2(downhill_idx);
        let dxy = (downhill_pos - uniform_idx_as_vec2(chunk_idx)).map(|e| e as f64);
        let neighbor_dim = neighbor_coef * dxy;
        // First, we calculate the river's volumetric flow rate.
        let chunk_drainage = drainage[chunk_idx] as f64;
        // Volumetric flow rate is just the total drainage area to this chunk, times rainfall
        // height per chunk per minute (needed in order to use this as a m³ volume).
        // TODO: consider having different rainfall rates (and including this information in the
        // computation of drainage).
        let volumetric_flow_rate = chunk_drainage * CONFIG.rainfall_chunk_rate as f64;
        let downhill_drainage = drainage[downhill_idx] as f64;

        // We know the drainage to the downhill node is just chunk_drainage - 1.0 (the amount of
        // rainfall this chunk is said to get), so we don't need to explicitly remember the
        // incoming mass.  How do we find a slope for endpoints where there is no incoming data?
        // Currently, we just assume it's set to 0.0.
        // TODO: Fix this when we add differing amounts of rainfall.
        let incoming_drainage = downhill_drainage - 1.0;
        let get_river_spline_derivative =
            |neighbor_dim: Vec2<f64>, spline_derivative: Vec2<f32>| {
                /*if incoming_drainage == 0.0 {
                    Vec2::zero()
                } else */
                {
                    // "Velocity of center of mass" of splines of incoming flows.
                    let river_prev_slope = spline_derivative.map(|e| e as f64)/* / incoming_drainage*/;
                    // NOTE: We need to make sure the slope doesn't get *too* crazy.
                    // ((dpx - cx) - 4 * MAX).abs() = bx
                    // NOTE: This will fail if the distance between chunks in any direction
                    // is exactly TerrainChunkSize::RECT * 4.0, but hopefully this should not be possible.
                    // NOTE: This isn't measuring actual distance, you can go farther on diagonals.
                    // let max_deriv = neighbor_dim - neighbor_coef * 4.0;
                    let max_deriv = neighbor_dim - neighbor_coef * 2.0 * 2.0.sqrt();
                    let extra_divisor = river_prev_slope
                        .map2(max_deriv, |e, f| (e / f).abs())
                        .reduce_partial_max();
                    // Set up the river's spline derivative.  For each incoming river at pos with
                    // river_spline_derivative bx, we can compute our interpolated slope as:
                    //   d_x = 2 * (chunk_pos - pos - bx) + bx
                    //       = 2 * (chunk_pos - pos) - bx
                    //
                    // which is exactly twice what was weighted by uphill nodes to get our
                    // river_spline_derivative in the first place.
                    //
                    // NOTE: this probably implies that the distance shouldn't be normalized, since the
                    // distances aren't actually equal between x and y... we'll see what happens.
                    (if extra_divisor > 1.0 {
                        river_prev_slope / extra_divisor
                    } else {
                        river_prev_slope
                    })
                    .map(|e| e as f32)
                }
            };

        let river = &rivers[chunk_idx];
        let river_spline_derivative =
            get_river_spline_derivative(neighbor_dim, river.spline_derivative);

        let indirection_idx = indirection[chunk_idx];
        // Find the lake we are flowing into.
        let lake_idx = if indirection_idx < 0 {
            // If we're a lake bottom, our own indirection is negative.
            let mut lake = &mut rivers[chunk_idx];
            let neighbor_pass_idx = downhill_idx;
            // Mass flow from this lake is treated as a weighting factor (this is currently
            // considered proportional to drainage, but in the direction of "lake side of pass to
            // pass.").
            let neighbor_pass_pos = downhill_pos;
            lake.river_kind = Some(RiverKind::Lake {
                neighbor_pass_pos: neighbor_pass_pos
                    * TerrainChunkSize::RECT_SIZE.map(|e| e as i32),
            });
            lake.spline_derivative = Vec2::zero()/*river_spline_derivative*/;
            let pass_idx = (-indirection_idx) as usize;
            let pass_pos = uniform_idx_as_vec2(pass_idx);
            let lake_direction = neighbor_coef * (neighbor_pass_pos - pass_pos).map(|e| e as f64);
            // Our side of the pass must have already been traversed (even if our side of the pass
            // is the lake bottom), so we acquire its computed river_spline_derivative.
            let pass = &rivers[pass_idx];
            debug_assert!(pass.is_lake());
            let pass_spline_derivative = pass.spline_derivative.map(|e| e as f64)/*Vec2::zero()*/;
            // Normally we want to not normalize, but for the lake we don't want to generate a
            // super long edge since it could lead to a lot of oscillation... this is another
            // reason why we shouldn't use the lake bottom.
            // lake_direction.normalize();
            // We want to assign the drained node from any lake to be a river.
            let lake_drainage = /*drainage[chunk_idx]*/chunk_drainage;
            let lake_neighbor_pass_incoming_drainage = incoming_drainage;
            let weighted_flow = (lake_direction * 2.0 - pass_spline_derivative)
                / derivative_divisor
                * lake_drainage
                / lake_neighbor_pass_incoming_drainage;
            let mut lake_neighbor_pass = &mut rivers[neighbor_pass_idx];
            // We definitely shouldn't have encountered this yet!
            debug_assert!(lake_neighbor_pass.velocity == Vec3::zero());
            // TODO: Rethink making the lake neighbor pass always a river or lake, no matter how
            // much incoming water there is?  Sometimes it looks weird having a river emerge from a
            // tiny pool.
            lake_neighbor_pass.river_kind = Some(RiverKind::River {
                cross_section: Vec2::default(),
            });
            // We also want to add to the out-flow side of the pass a (flux-weighted)
            // derivative coming from the lake center.
            //
            // NOTE: Maybe consider utilizing 3D component of spline somehow?  Currently this is
            // basically a flat vector, but that might be okay from lake to other side of pass.
            lake_neighbor_pass.spline_derivative += /*Vec2::new(weighted_flow.x, weighted_flow.y)*/
                weighted_flow.map(|e| e as f32);
            continue;
        } else {
            indirection_idx as usize
        };
        // Add our spline derivative to the downhill river (weighted by the chunk's drainage).
        //
        // TODO: consider utilizing height difference component of flux as well; currently we
        // just discard it in figuring out the spline's slope.
        let downhill_river = &mut rivers[downhill_idx];
        let weighted_flow = (neighbor_dim * 2.0 - river_spline_derivative.map(|e| e as f64))
            / derivative_divisor
            * chunk_drainage
            / incoming_drainage;
        downhill_river.spline_derivative += weighted_flow.map(|e| e as f32);

        // Find the pass this lake is flowing into (i.e. water at the lake bottom gets
        // pushed towards the point identified by pass_idx).
        let neighbor_pass_idx = downhill[lake_idx];
        // Find our own water height.
        let chunk_water_alt = water_alt[chunk_idx];
        if neighbor_pass_idx >= 0 {
            // We may be a river.  But we're not sure yet, since we still could be
            // underwater.  Check the lake height and see if our own water height is within ε of
            // it.
            let pass_idx = (-indirection[lake_idx]) as usize;
            let lake_water_alt = water_alt[lake_idx];
            if chunk_water_alt == lake_water_alt {
                // Not a river.
                // Check whether we we are the lake side of the pass.
                // NOTE: Safe because this is a lake.
                let (neighbor_pass_pos, river_spline_derivative) = if pass_idx == chunk_idx
                /*true*/
                {
                    // This is a pass, so set our flow direction to point to the neighbor pass
                    // rather than downhill.
                    // NOTE: Safe because neighbor_pass_idx >= 0.
                    (
                        uniform_idx_as_vec2(neighbor_pass_idx as usize),
                        river_spline_derivative,
                    )
                } else {
                    // Try pointing towards the lake side of the pass.
                    (uniform_idx_as_vec2(pass_idx), river_spline_derivative)
                };
                let mut lake = &mut rivers[chunk_idx];
                lake.spline_derivative = river_spline_derivative;
                lake.river_kind = Some(RiverKind::Lake {
                    neighbor_pass_pos: neighbor_pass_pos
                        * TerrainChunkSize::RECT_SIZE.map(|e| e as i32),
                });
                continue;
            }
        // Otherwise, we must be a river.
        } else {
            // We are flowing into the ocean.
            debug_assert!(neighbor_pass_idx == -2);
            // But we are not the ocean, so we must be a river.
        }
        // Now, we know we are a river *candidate*.  We still don't know whether we are actually a
        // river, though.  There are two ways for that to happen:
        // (i) We are already a river.
        // (ii) Using the Gauckler–Manning–Strickler formula for cross-sectional average velocity
        //      of water, we establish that the river can be "big enough" to appear on the Veloren
        //      map.
        //
        // This is very imprecise, of course, and (ii) may (and almost certainly will) change over
        // time.
        //
        // In both cases, we preemptively set our child to be a river, to make sure we have an
        // unbroken stream.  Also in both cases, we go to the effort of computing an effective
        // water velocity vector and cross-sectional dimensions, as well as figuring out the
        // derivative of our interpolating spline (since this percolates through the whole river
        // network).
        let downhill_water_alt = water_alt[downhill_idx];
        let neighbor_distance = neighbor_dim.magnitude();
        let dz = (downhill_water_alt - chunk_water_alt) * CONFIG.mountain_scale;
        let slope = dz.abs() as f64 / neighbor_distance;
        if slope == 0.0 {
            // This is not a river--how did this even happen?
            let pass_idx = (-indirection_idx) as usize;
            log::error!("Our chunk (and downhill, lake, pass, neighbor_pass): {:?} (to {:?}, in {:?} via {:?} to {:?}), chunk water alt: {:?}, lake water alt: {:?}",
                uniform_idx_as_vec2(chunk_idx),
                uniform_idx_as_vec2(downhill_idx),
                uniform_idx_as_vec2(lake_idx),
                uniform_idx_as_vec2(pass_idx),
                if neighbor_pass_idx >= 0 { Some(uniform_idx_as_vec2(neighbor_pass_idx as usize)) } else { None },
                water_alt[chunk_idx],
                water_alt[lake_idx]);
            panic!("Should this happen at all?");
        }
        let slope_sqrt = slope.sqrt();
        // Now, we compute a quantity that is proportional to the velocity of the chunk, derived
        // from the Manning formula, equal to
        // volumetric_flow_rate / slope_sqrt * CONFIG.river_roughness.
        let almost_velocity = volumetric_flow_rate / slope_sqrt * CONFIG.river_roughness as f64;
        // From this, we can figure out the width of the chunk if we know the height.  For now, we
        // hardcode the height to 0.5, but it should almost certainly be much more complicated than
        // this.
        // let mut height = 0.5f32;
        // We approximate the river as a rectangular prism.  Theoretically, we need to solve the
        // following quintic equation to determine its width from its height:
        //
        // h^5 * w^5 = almost_velocity^3 * (w + 2 * h)^2.
        //
        // This is because one of the quantities in the Manning formula (the unknown) is R_h =
        // (area of cross-section / h)^(2/3).
        //
        // Unfortunately, quintic equations do not in general have algebraic solutions, and it's
        // not clear (to me anyway) that this one does in all cases.
        //
        // In practice, for high ratios of width to height, we can approximate the rectangular
        // prism's perimeter as equal to its width, so R_h as equal to height.  This greatly
        // simplifies the calculation.  For simplicity, we do this even for low ratios of width to
        // height--I found that for most real rivers, at least big ones, this approximation is
        // "good enough."  We don't need to be *that* realistic :P
        //
        // NOTE: Derived from a paper on estimating river width.
        let mut width = 5.0
            * (CONFIG.river_width_to_depth as f64
                * (CONFIG.river_width_to_depth as f64 + 2.0).powf(2.0 / 3.0))
            .powf(3.0 / 8.0)
            * volumetric_flow_rate.powf(3.0 / 8.0)
            * slope.powf(-3.0 / 16.0)
            * (CONFIG.river_roughness as f64).powf(3.0 / 8.0);
        width = width.max(0.0);

        let mut height = if width == 0.0 {
            CONFIG.river_min_height as f64
        } else {
            (almost_velocity / width).powf(3.0 / 5.0)
        };

        // We can now weight the river's drainage by its direction, which we use to help improve
        // the slope of the downhill node.
        let river_direction = Vec3::new(
            neighbor_dim.x,
            neighbor_dim.y,
            (dz as f64).signum() * (dz as f64),
        );

        // Now, we can check whether this is "really" a river.
        // Currently, we just check that width and height are at least 0.5 and
        // CONFIG.river_min_height.
        let river = &rivers[chunk_idx];
        let is_river = river.is_river() || width >= 0.5 && height >= CONFIG.river_min_height as f64;
        let mut downhill_river = &mut rivers[downhill_idx];

        if is_river {
            // Provisionally make the downhill chunk a river as well.
            downhill_river.river_kind = Some(RiverKind::River {
                cross_section: Vec2::default(),
            });

            // Additionally, if the cross-sectional area for this river exceeds the max river
            // width, the river is overflowing the two chunks adjacent to it, which we'd prefer to
            // avoid since only its two immediate neighbors (orthogonal to the downhill direction)
            // are guaranteed uphill of it.
            // Solving this properly most likely requires modifying the erosion model to
            // take channel width into account, which is a formidable task that likely requires
            // rethinking the current grid-based erosion model (or at least, requires tracking some
            // edges that aren't implied by the grid graph).  For now, we will solve this problem
            // by making the river deeper when it hits the max width, until it consumes all the
            // available energy in this part of the river.
            let max_width = TerrainChunkSize::RECT_SIZE.x as f64 * CONFIG.river_max_width as f64;
            if width > max_width {
                width = max_width;
                height = (almost_velocity / width).powf(3.0 / 5.0);
            }
        }
        // Now we can compute the river's approximate velocity magnitude as well, as
        let velocity_magnitude =
            1.0 / CONFIG.river_roughness as f64 * height.powf(2.0 / 3.0) * slope_sqrt;

        // Set up the river's cross-sectional area.
        let cross_section = Vec2::new(width as f32, height as f32);
        // Set up the river's velocity vector.
        let mut velocity = river_direction;
        velocity.normalize();
        velocity *= velocity_magnitude;

        let mut river = &mut rivers[chunk_idx];
        // NOTE: Not trying to do this more cleverly because we want to keep the river's neighbors.
        // TODO: Actually put something in the neighbors.
        river.velocity = velocity.map(|e| e as f32);
        river.spline_derivative = river_spline_derivative;
        river.river_kind = if is_river {
            Some(RiverKind::River { cross_section })
        } else {
            None
        };
    }
    rivers
}

/// Precompute the maximum slope at all points.
///
/// TODO: See if allocating in advance is worthwhile.
fn get_max_slope(h: &[f32], rock_strength_nz: &(impl NoiseFn<Point3<f64>> + Sync)) -> Box<[f32]> {
    const MIN_MAX_ANGLE: f64 = 6.0 / 360.0 * 2.0 * f64::consts::PI;
    const MAX_MAX_ANGLE: f64 = 54.0 / 360.0 * 2.0 * f64::consts::PI;
    const MAX_ANGLE_RANGE: f64 = MAX_MAX_ANGLE - MIN_MAX_ANGLE;
    h.par_iter()
        .enumerate()
        .map(|(posi, &z)| {
            let wposf = (uniform_idx_as_vec2(posi)).map(|e| e as f64);
            let wposz = z as f64 * CONFIG.mountain_scale as f64;
            // Normalized to be between 6 and and 54 degrees.
            let div_factor = 32.0;
            let rock_strength = rock_strength_nz
                .get([
                    wposf.x, /* / div_factor*/
                    wposf.y, /* / div_factor*/
                    wposz,
                ])
                .max(-1.0)
                .min(1.0)
                * 0.5
                + 0.5;
            // Powering rock_strength^((1.25 - z)^6) means the maximum angle increases with z, but
            // not too fast.  At z = 0.25 the angle is not affected at all, below it the angle is
            // lower, and above it the angle is higher.
            //
            // Logistic regression.  Make sure x ∈ (0, 1).
            let logit = |x: f64| x.ln() - (-x).ln_1p();
            // 0.5 + 0.5 * tanh(ln(1 / (1 - 0.1) - 1) / (2 * (sqrt(3)/pi)))
            let logistic_2_base = 3.0f64.sqrt() * f64::consts::FRAC_2_PI;
            // Assumes μ = 0, σ = 1
            let logistic_cdf = |x: f64| (x / logistic_2_base).tanh() * 0.5 + 0.5;

            // We do log-odds against center, so that our log odds are 0 when x = 0.25, lower when x is
            // lower, and higher when x is higher.
            //
            // TODO: Make all this stuff configurable... but honestly, it's so complicated that I'm not
            // sure anyone would be able to usefully tweak them on a per-map basis?  Plus it's just a
            // hacky heuristic anyway.
            let center = 0.25;
            let delta = 0.05;
            let dmin = center - delta;
            let dmax = center + delta;
            let log_odds = |x: f64| logit(x) - logit(center);
            let rock_strength = logistic_cdf(
                1.0 * logit(rock_strength.min(1.0f64 - 1e-7).max(1e-7))
                    + 1.0 * log_odds((z as f64).min(dmax).max(dmin)),
            );
            let max_slope = (rock_strength * MAX_ANGLE_RANGE + MIN_MAX_ANGLE).tan();
            max_slope as f32
        })
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

/// Erode all chunks by amount.
///
/// Our equation is:
///
///   dh(p) / dt = uplift(p)−k * A(p)^m * slope(p)^n
///
///   where A(p) is the drainage area at p, m and n are constants
///   (we choose m = 0.5 and n = 1), and k is a constant.  We choose
///
///   k = 2.244 * uplift.max() / (desired_max_height)
///
///   since this tends to produce mountains of max height desired_max_height; and we set
///   desired_max_height = 1.0 to reflect limitations of mountain scale.
///
/// This algorithm does this in four steps:
///
/// 1. Sort the nodes in h by height (so the lowest node by altitude is first in the
///    list, and the highest node by altitude is last).
/// 2. Iterate through the list in *reverse.*  For each node, we compute its drainage area as
///    the sum of the drainage areas of its "children" nodes (i.e. the nodes with directed edges to
///    this node).  To do this efficiently, we start with the "leaves" (the highest nodes), which
///    have no neighbors higher than them, hence no directed edges to them.  We add their area to
///    themselves, and then to all neighbors that they flow into (their "ancestors" in the flow
///    graph); currently, this just means the node immediately downhill of this node.
///    As we go lower, we know that all our "children" already had their areas computed, which
///    means that we can repeat the process in order to derive all the final areas.
/// 3. Now, iterate through the list in *order.*  Whether we used the filling method to compute a
///    "filled" version of each depression, or used the lake connection algoirthm described in [1],
///    each node is guaranteed to have zero or one drainage edges out, representing the direction
///    of water flow for that node.  For nodes i with zero drainage edges out (boundary nodes and
///    lake bottoms) we set the slope to 0 (so the change in altitude is uplift(i))
///    For nodes with at least one drainage edge out, we take advantage of the fact that we are
///    computing new heights in order and rewrite our equation as (letting j = downhill[i], A[i]
///    be the computed area of point i, p(i) be the x-y position of point i,
///    flux(i) = k * A[i]^m / ((p(i) - p(j)).magnitude()), and δt = 1):
///
///    h[i](t + dt) = h[i](t) + δt * (uplift[i] + flux(i) * h[j](t + δt)) / (1 + flux(i) * δt).
///
///    Since we compute heights in ascending order by height, and j is downhill from i, h[j] will
///    always be the *new* h[j](t + δt), while h[i] will still not have been computed yet, so we
///    only need to visit each node once.
///
/// [1] Guillaume Cordonnier, Jean Braun, Marie-Paule Cani, Bedrich Benes, Eric Galin, et al..
///     Large Scale Terrain Generation from Tectonic Uplift and Fluvial Erosion.
///     Computer Graphics Forum, Wiley, 2016, Proc. EUROGRAPHICS 2016, 35 (2), pp.165-175.
///     ⟨10.1111/cgf.12820⟩. ⟨hal-01262376⟩
///
fn erode(
    h: &mut [f32],
    erosion_base: f32,
    max_uplift: f32,
    _seed: &RandomField,
    rock_strength_nz: &(impl NoiseFn<Point3<f64>> + Sync),
    uplift: impl Fn(usize) -> f32,
    is_ocean: impl Fn(usize) -> bool + Sync,
) {
    log::debug!("Done draining...");
    let mmaxh = 1.0;
    // Landslide constant: ideally scaled to 10 m^-2 / y^-1
    let l = 200.0 * max_uplift as f64;
    // Erosion constant.
    let k = erosion_base as f64 + 2.244 / mmaxh as f64 * max_uplift as f64;
    let ((dh, indirection, newh, area), max_slope) = rayon::join(
        || {
            let mut dh = downhill(h, |posi| is_ocean(posi));
            log::debug!("Computed downhill...");
            let (boundary_len, indirection, newh) = get_lakes(&h, &mut dh);
            log::debug!("Got lakes...");
            let area = get_drainage(&newh, &dh, boundary_len);
            log::debug!("Got flux...");
            (dh, indirection, newh, area)
        },
        || {
            let max_slope = get_max_slope(h, rock_strength_nz);
            log::debug!("Got max slopes...");
            max_slope
        },
    );

    assert!(h.len() == dh.len() && dh.len() == area.len());
    // max angle of slope depends on rock strength, which is computed from noise function.
    let neighbor_coef = TerrainChunkSize::RECT_SIZE.map(|e| e as f64);
    let chunk_area = neighbor_coef.x * neighbor_coef.y;
    // Iterate in ascending height order.
    let mut maxh = 0.0;
    let mut nland = 0usize;
    let mut sums = 0.0;
    let mut sumh = 0.0;
    for &posi in &*newh {
        let posi = posi as usize;
        let posj = dh[posi];
        if posj < 0 {
            if posj == -1 {
                panic!("Disconnected lake!");
            }
        // Egress with no outgoing flows.
        } else {
            nland += 1;
            let posj = posj as usize;
            let dxy = (uniform_idx_as_vec2(posi) - uniform_idx_as_vec2(posj)).map(|e| e as f64);

            // Has an outgoing flow edge (posi, posj).
            // flux(i) = k * A[i]^m / ((p(i) - p(j)).magnitude()), and δt = 1
            let neighbor_distance = (neighbor_coef * dxy).magnitude();
            // Since the area is in meters^(2m) and neighbor_distance is in m, so long as m = 0.5,
            // we have meters^(1) / meters^(1), so they should cancel out.  Otherwise, we would
            // want to divide neighbor_distance by CONFIG.mountain_scale and area[posi] by
            // CONFIG.mountain_scale^2, to make sure we were working in the correct units for dz
            // (which has 1.0 height unit = CONFIG.mountain_scale meters).
            let flux = k * (chunk_area * area[posi] as f64).sqrt() / neighbor_distance;
            // h[i](t + dt) = (h[i](t) + δt * (uplift[i] + flux(i) * h[j](t + δt))) / (1 + flux(i) * δt).
            // NOTE: posj has already been computed since it's downhill from us.
            let h_j = h[posj] as f64;
            let old_h_i = h[posi] as f64;
            let mut new_h_i = (old_h_i + (uplift(posi) as f64 + flux * h_j)) / (1.0 + flux);
            // Find out if this is a lake bottom.
            let indirection_idx = indirection[posi];
            let is_lake_bottom = indirection_idx < 0;
            // Test the slope.
            let max_slope = max_slope[posi] as f64;
            let dz = (new_h_i - h_j).max(0.0) * CONFIG.mountain_scale as f64;
            let mag_slope = dz/*.abs()*/ / neighbor_distance;
            let _fake_neighbor = is_lake_bottom && dxy.x.abs() > 1.0 && dxy.y.abs() > 1.0;
            // If you're on the lake bottom and not right next to your neighbor, don't compute a
            // slope.
            if
            /* !is_lake_bottom */ /* !fake_neighbor */
            true {
                if
                /* !is_lake_bottom && */
                mag_slope > max_slope {
                    // println!("old slope: {:?}, new slope: {:?}, dz: {:?}, h_j: {:?}, new_h_i: {:?}", mag_slope, max_slope, dz, h_j, new_h_i);
                    // Thermal erosion says this can't happen, so we reduce dh_i to make the slope
                    // exactly max_slope.
                    // max_slope = (old_h_i + dh - h_j) * CONFIG.mountain_scale / NEIGHBOR_DISTANCE
                    // dh = max_slope * NEIGHBOR_DISTANCE / CONFIG.mountain_scale + h_j - old_h_i.
                    let dh = max_slope * neighbor_distance / CONFIG.mountain_scale as f64;
                    new_h_i = (h_j + dh).max(new_h_i - l * (mag_slope - max_slope));
                    let dz = (new_h_i - h_j).max(0.0) * CONFIG.mountain_scale as f64;
                    let slope = dz/*.abs()*/ / neighbor_distance;
                    sums += slope;
                // let slope = dz.signum() * max_slope;
                // new_h_i = slope * neighbor_distance / CONFIG.mountain_scale as f64 + h_j;
                // sums += max_slope;
                } else {
                    sums += mag_slope;
                    // Just use the computed rate.
                }
                h[posi] = new_h_i as f32;
                sumh += new_h_i;
            }
        }
        maxh = h[posi].max(maxh);
    }
    log::debug!(
        "Done eroding (max height: {:?}) (avg height: {:?}) (avg slope: {:?})",
        maxh,
        if nland == 0 {
            f64::INFINITY
        } else {
            sumh / nland as f64
        },
        if nland == 0 {
            f64::INFINITY
        } else {
            sums / nland as f64
        },
    );
}

/// The Planchon-Darboux algorithm for extracting drainage networks.
///
/// http://horizon.documentation.ird.fr/exl-doc/pleins_textes/pleins_textes_7/sous_copyright/010031925.pdf
///
/// See https://github.com/mewo2/terrain/blob/master/terrain.js
pub fn fill_sinks(
    h: impl Fn(usize) -> f32 + Sync,
    is_ocean: impl Fn(usize) -> bool + Sync,
) -> Box<[f32]> {
    // NOTE: We are using the "exact" version of depression-filling, which is slower but doesn't
    // change altitudes.
    let epsilon = /*1.0 / (1 << 7) as f32 / CONFIG.mountain_scale*/0.0;
    let infinity = f32::INFINITY;
    let range = 0..WORLD_SIZE.x * WORLD_SIZE.y;
    let oldh = range
        .into_par_iter()
        .map(|posi| h(posi))
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let mut newh = oldh
        .par_iter()
        .enumerate()
        .map(|(posi, &h)| {
            let is_near_edge = is_ocean(posi);
            if is_near_edge {
                h
            } else {
                infinity
            }
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();

    loop {
        let mut changed = false;
        for posi in 0..newh.len() {
            let nh = newh[posi];
            let oh = oldh[posi];
            if nh == oh {
                continue;
            }
            for nposi in neighbors(posi) {
                let onbh = newh[nposi];
                let nbh = onbh + epsilon;
                // If there is even one path downhill from this node's original height, fix
                // the node's new height to be equal to its original height, and break out of the
                // loop.
                if oh >= nbh {
                    newh[posi] = oh;
                    changed = true;
                    break;
                }
                // Otherwise, we know this node's original height is below the new height of all of
                // its neighbors.  Then, we try to choose the minimum new height among all this
                // node's neighbors that is (plus a constant epislon) below this node's new height.
                //
                // (If there is no such node, then the node's new height must be (minus a constant
                // epsilon) lower than the new height of every neighbor, but above its original
                // height.  But this can't be true for *all* nodes, because if this is true for any
                // node, it is not true of any of its neighbors.  So all neighbors must either be
                // their original heights, or we will have another iteration of the loop (one of
                // its neighbors was changed to its minimum neighbor).  In the second case, in the
                // next round, all neighbor heights will be at most nh + epsilon).
                if nh > nbh && nbh > oh {
                    newh[posi] = nbh;
                    changed = true;
                }
            }
        }
        if !changed {
            return newh;
        }
    }
}

/// Computes which tiles are ocean tiles by

/// Algorithm for finding and connecting lakes.  Assumes newh and downhill have already
/// been computed.  When a lake's value is negative, it is its own lake root, and when it is 0, it
/// is on the boundary of Ω.
///
/// Returns a 4-tuple containing:
/// - The first indirection vector (associating chunk indices with their lake's root node).
/// - A list of chunks on the boundary (non-lake egress points).
/// - The second indirection vector (associating chunk indices with their lake's adjacency list).
/// - The adjacency list (stored in a single vector), indexed by the second indirection vector.
pub fn get_lakes(h: &[f32], downhill: &mut [isize]) -> (usize, Box<[i32]>, Box<[u32]>) {
    // Associates each lake index with its root node (the deepest one in the lake), and a list of
    // adjacent lakes.  The list of adjacent lakes includes the lake index of the adjacent lake,
    // and a node index in the adjacent lake which has a neighbor in this lake.  The particular
    // neighbor should be the one that generates the minimum "pass height" encountered so far,
    // i.e. the chosen pair should minimize the maximum of the heights of the nodes in the pair.

    // We start by taking steps to allocate an indirection vector to use for storing lake indices.
    // Initially, each entry in this vector will contain 0.  We iterate in ascending order through
    // the sorted newh.  If the node has downhill == -2, it is a boundary node Ω and we store it in
    // the boundary vector.  If the node has downhill == -1, it is a fresh lake, and we store 0 in
    // it.  If the node has non-negative downhill, we use the downhill index to find the next node
    // down; if the downhill node has a lake entry < 0, then downhill is a lake and its entry
    // can be negated to find an (over)estimate of the number of entries it needs.  If the downhill
    // node has a non-negative entry, then its entry is the lake index for this node, so we should
    // access that entry and increment it, then set our own entry to it.
    let mut boundary = Vec::with_capacity(downhill.len());
    let mut indirection = vec![/*-1i32*/0i32; WORLD_SIZE.x * WORLD_SIZE.y].into_boxed_slice();

    let mut newh = Vec::with_capacity(downhill.len());

    // Now, we know that the sum of all the indirection nodes will be the same as the number of
    // nodes.  We can allocate a *single* vector with 8 * nodes entries, to be used as storage
    // space, and augment our indirection vector with the starting index, resulting in a vector of
    // slices.  As we go, we replace each lake entry with its index in the new indirection buffer,
    // allowing
    let mut lakes = vec![(-1, 0); /*(indirection.len() - boundary.len())*/indirection.len() * 8];
    let mut indirection_ = vec![0u32; indirection.len()];
    // First, find all the lakes.
    let mut lake_roots = Vec::with_capacity(downhill.len()); // Test
    for (chunk_idx, &dh) in (&*downhill)
        .into_iter()
        .enumerate()
        .filter(|(_, &dh_idx)| dh_idx < 0)
    {
        if dh == -2 {
            // On the boundary, add to the boundary vector.
            boundary.push(chunk_idx);
        // Still considered a lake root, though.
        } else if dh == -1 {
            lake_roots.push(chunk_idx);
        } else {
            panic!("Impossible.");
        }
        // Find all the nodes uphill from this lake.  Since there is only one outgoing edge
        // in the "downhill" graph, this is guaranteed never to visit a node more than
        // once.
        let start = newh.len();
        let indirection_idx = (start * 8) as u32;
        // New lake root
        newh.push(chunk_idx as u32);
        let mut cur = start;
        while cur < newh.len() {
            let node = newh[cur as usize];
            for child in uphill(downhill, node as usize) {
                // lake_idx is the index of our lake root.
                indirection[child] = chunk_idx as i32;
                indirection_[child] = indirection_idx;
                newh.push(child as u32);
            }
            cur += 1;
        }
        // Find the number of elements pushed.
        let length = (cur - start) * 8;
        // New lake root (lakes have indirection set to -length).
        indirection[chunk_idx] = -(length as i32);
        indirection_[chunk_idx] = indirection_idx;
    }
    assert_eq!(newh.len(), downhill.len());

    log::debug!("Old lake roots: {:?}", lake_roots.len());

    let newh = newh.into_boxed_slice();
    // Now, we know that the sum of all the indirection nodes will be the same as the number of
    // nodes.  We can allocate a *single* vector with 8 * nodes entries, to be used as storage
    // space, and augment our indirection vector with the starting index, resulting in a vector of
    // slices.  As we go, we replace each lake entry with its index in the new indirection buffer,
    // allowing
    for &chunk_idx_ in newh.into_iter() {
        let chunk_idx = chunk_idx_ as usize;
        let lake_idx_ = indirection_[chunk_idx];
        let lake_idx = lake_idx_ as usize;
        let height = h[chunk_idx_ as usize];
        // For every neighbor, check to see whether it is already set; if the neighbor is set,
        // its height is ≤ our height.  We should search through the edge list for the
        // neighbor's lake to see if there's an entry; if not, we insert, and otherwise we
        // get its height.  We do the same thing in our own lake's entry list.  If the maximum
        // of the heights we get out from this process is greater than the maximum of this
        // chunk and its neighbor chunk, we switch to this new edge.
        for neighbor_idx in neighbors(chunk_idx) {
            let neighbor_height = h[neighbor_idx];
            let neighbor_lake_idx_ = indirection_[neighbor_idx];
            let neighbor_lake_idx = neighbor_lake_idx_ as usize;
            if neighbor_lake_idx_ < lake_idx_ {
                // We found an adjacent node that is not on the boundary and has already
                // been processed, and also has a non-matching lake.  Therefore we can use
                // split_at_mut to get disjoint slices.
                let (lake, neighbor_lake) = {
                    // println!("Okay, {:?} < {:?}", neighbor_lake_idx, lake_idx);
                    let (neighbor_lake, lake) = lakes.split_at_mut(lake_idx);
                    (lake, &mut neighbor_lake[neighbor_lake_idx..])
                };

                // We don't actually need to know the real length here, because we've reserved
                // enough spaces that we should always either find a -1 (available slot) or an
                // entry for this chunk.
                'outer: for pass in lake.iter_mut() {
                    if pass.0 == -1 {
                        // println!("One time, in my mind, one time... (neighbor lake={:?} lake={:?})", neighbor_lake_idx, lake_idx_);
                        *pass = (chunk_idx_ as i32, neighbor_idx as u32);
                        // Should never run out of -1s in the neighbor lake if we didn't find
                        // the neighbor lake in our lake.
                        *neighbor_lake
                            .iter_mut()
                            .filter(|neighbor_pass| neighbor_pass.0 == -1)
                            .next()
                            .unwrap() = (neighbor_idx as i32, chunk_idx_);
                        // panic!("Should never happen; maybe didn't reserve enough space in lakes?")
                        break;
                    } else if indirection_[pass.1 as usize] == neighbor_lake_idx_ {
                        for neighbor_pass in neighbor_lake.iter_mut() {
                            // Should never run into -1 while looping here, since (i, j)
                            // and (j, i) should be added together.
                            if indirection_[neighbor_pass.1 as usize] == lake_idx_ {
                                let pass_height = h[neighbor_pass.1 as usize];
                                let neighbor_pass_height = h[pass.1 as usize];
                                if height.max(neighbor_height)
                                    < pass_height.max(neighbor_pass_height)
                                {
                                    *pass = (chunk_idx_ as i32, neighbor_idx as u32);
                                    *neighbor_pass = (neighbor_idx as i32, chunk_idx_);
                                }
                                break 'outer;
                            }
                        }
                        // Should always find a corresponding match in the neighbor lake if
                        // we found the neighbor lake in our lake.
                        let indirection_idx = indirection[chunk_idx];
                        let lake_chunk_idx = if indirection_idx >= 0 {
                            indirection_idx as usize
                        } else {
                            chunk_idx as usize
                        };
                        let indirection_idx = indirection[neighbor_idx];
                        let neighbor_lake_chunk_idx = if indirection_idx >= 0 {
                            indirection_idx as usize
                        } else {
                            neighbor_idx as usize
                        };
                        panic!(
                            "For edge {:?} between lakes {:?}, couldn't find partner \
                             for pass {:?}. \
                             Should never happen; maybe forgot to set both edges?",
                            (
                                (chunk_idx, uniform_idx_as_vec2(chunk_idx as usize)),
                                (neighbor_idx, uniform_idx_as_vec2(neighbor_idx as usize))
                            ),
                            (
                                (
                                    lake_chunk_idx,
                                    uniform_idx_as_vec2(lake_chunk_idx as usize),
                                    lake_idx_
                                ),
                                (
                                    neighbor_lake_chunk_idx,
                                    uniform_idx_as_vec2(neighbor_lake_chunk_idx as usize),
                                    neighbor_lake_idx_
                                )
                            ),
                            (
                                (pass.0, uniform_idx_as_vec2(pass.0 as usize)),
                                (pass.1, uniform_idx_as_vec2(pass.1 as usize))
                            ),
                        );
                    }
                }
            }
        }
    }

    // Now it's time to calculate the lake connections graph T_L covering G_L.
    let mut candidates = BinaryHeap::with_capacity(indirection.len());
    // let mut pass_flows : Vec<i32> = vec![-1; indirection.len()];

    // We start by going through each pass, deleting the ones that point out of boundary nodes and
    // adding ones that point into boundary nodes from non-boundary nodes.
    for edge in &mut lakes {
        let edge: &mut (i32, u32) = edge;
        // Only consider valid elements.
        if edge.0 == -1 {
            continue;
        }
        // Check to see if this edge points out *from* a boundary node.
        // Delete it if so.
        let from = edge.0 as usize;
        let indirection_idx = indirection[from];
        let lake_idx = if indirection_idx < 0 {
            from
        } else {
            indirection_idx as usize
        };
        if downhill[lake_idx] == -2 {
            edge.0 = -1;
            continue;
        }
        // This edge is not pointing out from a boundary node.
        // Check to see if this edge points *to* a boundary node.
        // Add it to the candidate set if so.
        let to = edge.1 as usize;
        let indirection_idx = indirection[to];
        let lake_idx = if indirection_idx < 0 {
            to
        } else {
            indirection_idx as usize
        };
        if downhill[lake_idx] == -2 {
            // Find the pass height
            let pass = h[from].max(h[to]);
            candidates.push(Reverse((
                NotNan::new(pass).unwrap(),
                (edge.0 as u32, edge.1),
            )));
        }
    }

    let mut pass_flows_sorted: Vec<usize> = Vec::with_capacity(indirection.len());

    // Now all passes pointing to the boundary are in candidates.
    // As long as there are still candidates, we continue...
    // NOTE: After a lake is added to the stream tree, the lake bottom's indirection entry no
    // longer negates its maximum number of passes, but the lake side of the chosen pass.  As such,
    // we should make sure not to rely on using it this way afterwards.
    // provides information about the number of candidate passes in a lake.
    while let Some(Reverse((_, (chunk_idx, neighbor_idx)))) = candidates.pop() {
        // We have the smallest candidate.
        let lake_idx = indirection_[chunk_idx as usize] as usize;
        let indirection_idx = indirection[chunk_idx as usize];
        let lake_chunk_idx = if indirection_idx >= 0 {
            indirection_idx as usize
        } else {
            chunk_idx as usize
        };
        if downhill[lake_chunk_idx] >= 0 {
            // Candidate lake has already been connected.
            continue;
        }
        // println!("Got here...");
        assert_eq!(downhill[lake_chunk_idx], -1);
        // Candidate lake has not yet been connected, and is the lowest candidate.
        // Delete all other outgoing edges.
        let max_len = -if indirection_idx < 0 {
            indirection_idx
        } else {
            indirection[indirection_idx as usize]
        } as usize;
        // Add this chunk to the tree.
        downhill[lake_chunk_idx] = neighbor_idx as isize;
        // Also set the indirection of the lake bottom to the negation of the
        // lake side of the chosen pass (chunk_idx).
        // NOTE: This can't overflow i32 because WORLD_SIZE.x * WORLD_SIZE.y should fit in an i32.
        indirection[lake_chunk_idx] = -(chunk_idx as i32);
        // Add this edge to the sorted list.
        pass_flows_sorted.push(lake_chunk_idx);
        // pass_flows_sorted.push((chunk_idx as u32, neighbor_idx as u32));
        for edge in &mut lakes[lake_idx..lake_idx + max_len] {
            if *edge == (chunk_idx as i32, neighbor_idx as u32) {
                // Skip deleting this edge.
                continue;
            }
            // Delete the old edge, and remember it.
            let edge = mem::replace(edge, (-1, 0));
            if edge.0 == -1 {
                // Don't fall off the end of the list.
                break;
            }
            // Don't add incoming pointers from already-handled lakes or boundary nodes.
            let indirection_idx = indirection[edge.1 as usize];
            let neighbor_lake_idx = if indirection_idx < 0 {
                edge.1 as usize
            } else {
                indirection_idx as usize
            };
            if downhill[neighbor_lake_idx] != -1 {
                continue;
            }
            // Find the pass height
            let pass = h[edge.0 as usize].max(h[edge.1 as usize]);
            // Put the reverse edge in candidates, sorted by height, then chunk idx, and finally
            // neighbor idx.
            candidates.push(Reverse((
                NotNan::new(pass).unwrap(),
                (edge.1, edge.0 as u32),
            )));
        }
        // println!("I am a pass: {:?}", (uniform_idx_as_vec2(chunk_idx as usize), uniform_idx_as_vec2(neighbor_idx as usize)));
    }
    log::debug!("Total lakes: {:?}", pass_flows_sorted.len());

    // Perform the bfs once again.
    let mut newh = Vec::with_capacity(downhill.len());
    (&*boundary)
        .iter()
        .chain(pass_flows_sorted.iter())
        .for_each(|&chunk_idx| {
            // Find all the nodes uphill from this lake.  Since there is only one outgoing edge
            // in the "downhill" graph, this is guaranteed never to visit a node more than
            // once.
            let start = newh.len();
            // New lake root
            newh.push(chunk_idx as u32);
            let mut cur = start;
            while cur < newh.len() {
                let node = newh[cur as usize];

                for child in uphill(downhill, node as usize) {
                    // lake_idx is the index of our lake root.
                    // Check to make sure child (flowing into us) isn't a lake.
                    if indirection[child] >= 0
                    /* Note: equal to chunk_idx should be same */
                    {
                        assert!(h[child] >= h[node as usize]);
                        newh.push(child as u32);
                    }
                }
                cur += 1;
            }
        });
    assert_eq!(newh.len(), downhill.len());
    (boundary.len(), indirection, newh.into_boxed_slice())
}

/// Perform erosion n times.
pub fn do_erosion(
    erosion_base: f32,
    _max_uplift: f32,
    n: usize,
    seed: &RandomField,
    rock_strength_nz: &(impl NoiseFn<Point3<f64>> + Sync),
    oldh: impl Fn(usize) -> f32 + Sync,
    is_ocean: impl Fn(usize) -> bool + Sync,
    uplift: impl Fn(usize) -> f32 + Sync,
) -> Box<[f32]> {
    let oldh_ = (0..WORLD_SIZE.x * WORLD_SIZE.y)
        .into_par_iter()
        .map(|posi| oldh(posi))
        .collect::<Vec<_>>()
        .into_boxed_slice();
    // TODO: Don't do this, maybe?
    // (To elaborate, maybe we should have varying uplift or compute it some other way).
    let uplift = (0..oldh_.len())
        .into_par_iter()
        .map(|posi| uplift(posi))
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let sum_uplift = uplift
        .into_par_iter()
        .cloned()
        .map(|e| e as f64)
        .sum::<f64>();
    log::debug!("Sum uplifts: {:?}", sum_uplift);

    let max_uplift = uplift
        .into_par_iter()
        .cloned()
        .max_by(|a, b| a.partial_cmp(&b).unwrap())
        .unwrap();
    log::debug!("Max uplift: {:?}", max_uplift);
    let mut h = oldh_;
    for i in 0..n {
        log::debug!("Erosion iteration #{:?}", i);
        erode(
            &mut h,
            erosion_base,
            max_uplift,
            seed,
            rock_strength_nz,
            |posi| uplift[posi],
            |posi| is_ocean(posi),
        );
    }
    h
}
