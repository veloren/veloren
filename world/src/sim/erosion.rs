use super::{
    diffusion, downhill, neighbors, uniform_idx_as_vec2, uphill, vec2_as_uniform_idx,
    NEIGHBOR_DELTA, WORLD_SIZE,
};
use crate::{config::CONFIG, util::RandomField};
use common::{terrain::TerrainChunkSize, vol::RectVolSize};
// use faster::*;
use itertools::izip;
use noise::{NoiseFn, Point3};
use num::{Float, Zero};
use ordered_float::NotNan;
use packed_simd::m32;
use rayon::prelude::*;
use std::{
    cmp::{Ordering, Reverse},
    collections::BinaryHeap,
    f32, f64, fmt, mem,
    time::Instant,
    u32,
};
use vek::*;

pub type Alt = f64;
pub type Compute = f64;
pub type Computex8 = [Compute; 8];

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
    newh.into_iter().rev().for_each(|&chunk_idx| {
        let chunk_idx = chunk_idx as usize;
        let downhill_idx = downhill[chunk_idx];
        if downhill_idx >= 0 {
            flux[downhill_idx as usize] += flux[chunk_idx];
        }
    });
    flux
}

/// Compute the water flux at all chunks for multiple receivers, given a list of chunk indices
/// sorted by increasing height and weights for each receiver.
pub fn get_multi_drainage(
    mstack: &[u32],
    mrec: &[u8],
    mwrec: &[Computex8],
    _boundary_len: usize,
) -> Box<[Compute]> {
    // FIXME: Make the below work.  For now, we just use constant flux.
    // Initially, flux is determined by rainfall.  We currently treat this as the same as humidity,
    // so we just use humidity as a proxy.  The total flux across the whole map is normalize to
    // 1.0, and we expect the average flux to be 0.5.  To figure out how far from normal any given
    // chunk is, we use its logit.
    // NOTE: If there are no non-boundary chunks, we just set base_flux to 1.0; this should still
    // work fine because in that case there's no erosion anyway.
    let base_area = 1.0;
    let mut area = vec![base_area; WORLD_SIZE.x * WORLD_SIZE.y].into_boxed_slice();
    mstack.into_iter().for_each(|&ij| {
        let ij = ij as usize;
        let wrec_ij = &mwrec[ij];
        let area_ij = area[ij];
        mrec_downhill(mrec, ij).for_each(|(k, ijr)| {
            area[ijr] += area_ij * wrec_ij[k];
        });
    });
    area
    /*
      a=dx*dy*precip
      do ij=1,nn
        ijk=mstack(ij)
        do k =1,mnrec(ijk)
          a(mrec(k,ijk))=a(mrec(k,ijk))+a(ijk)*mwrec(k,ijk)
        enddo
      enddo
    */
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
pub fn get_rivers<F: fmt::Debug + Float + Into<f64>, G: Float + Into<f64>>(
    newh: &[u32],
    water_alt: &[F],
    downhill: &[isize],
    indirection: &[i32],
    drainage: &[G],
) -> Box<[RiverData]> {
    // For continuity-preserving quadratic spline interpolation, we (appear to) need to build
    // up the derivatives from the top down.  Fortunately this computation seems tractable.

    let mut rivers = vec![RiverData::default(); WORLD_SIZE.x * WORLD_SIZE.y].into_boxed_slice();
    let neighbor_coef = TerrainChunkSize::RECT_SIZE.map(|e| e as f64);
    // (Roughly) area of a chunk, times minutes per second.
    // NOTE: Clearly, this should "actually" be 1/60 (or maybe 1/64, if you want to retain powers
    // of 2).
    //
    // But since we want rivers to form more often than they do in real life, we use this as a
    // way to control the frequency of river formation.  As grid_scale increases, mins_per_sec
    // should decrease, until it hits 1 / 60 or 1/ 64.  For example, if grid_scale is multiplied
    // by 4, mins_per_sec should be multiplied by 1 / (4.0 * 4.0).
    let mins_per_sec = 1.0/*1.0 / 16.0*//*1.0 / 64.0*/;
    let chunk_area_factor = neighbor_coef.x * neighbor_coef.y * mins_per_sec;
    // NOTE: This technically makes us discontinuous, so we should be cautious about using this.
    let derivative_divisor = 1.0;
    newh.into_iter().rev().for_each(|&chunk_idx| {
        let chunk_idx = chunk_idx as usize;
        let downhill_idx = downhill[chunk_idx];
        if downhill_idx < 0 {
            // We are in the ocean.
            debug_assert!(downhill_idx == -2);
            rivers[chunk_idx].river_kind = Some(RiverKind::Ocean);
            return;
        }
        let downhill_idx = downhill_idx as usize;
        let downhill_pos = uniform_idx_as_vec2(downhill_idx);
        let dxy = (downhill_pos - uniform_idx_as_vec2(chunk_idx)).map(|e| e as f64);
        let neighbor_dim = neighbor_coef * dxy;
        // First, we calculate the river's volumetric flow rate.
        let chunk_drainage = drainage[chunk_idx].into();
        // Volumetric flow rate is just the total drainage area to this chunk, times rainfall
        // height per chunk per minute, times minutes per second
        // (needed in order to use this as a m³ volume).
        // TODO: consider having different rainfall rates (and including this information in the
        // computation of drainage).
        let volumetric_flow_rate =
            chunk_drainage * chunk_area_factor * CONFIG.rainfall_chunk_rate as f64;
        let downhill_drainage = drainage[downhill_idx].into();

        // We know the drainage to the downhill node is just chunk_drainage - 1.0 (the amount of
        // rainfall this chunk is said to get), so we don't need to explicitly remember the
        // incoming mass.  How do we find a slope for endpoints where there is no incoming data?
        // Currently, we just assume it's set to 0.0.
        // TODO: Fix this when we add differing amounts of rainfall.
        let incoming_drainage = downhill_drainage - 1.0;
        let get_river_spline_derivative =
            |neighbor_dim: Vec2<f64>, spline_derivative: Vec2<f32>| {
                // "Velocity of center of mass" of splines of incoming flows.
                let river_prev_slope = spline_derivative.map(|e| e as f64);
                // NOTE: We need to make sure the slope doesn't get *too* crazy.
                // ((dpx - cx) - 4 * MAX).abs() = bx
                // NOTE: This will fail if the distance between chunks in any direction
                // is exactly TerrainChunkSize::RECT * 4.0, but hopefully this should not be possible.
                // NOTE: This isn't measuring actual distance, you can go farther on diagonals.
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
            };

        let river = &rivers[chunk_idx];
        let river_spline_derivative =
            get_river_spline_derivative(neighbor_dim, river.spline_derivative);

        let indirection_idx = indirection[chunk_idx];
        // Find the lake we are flowing into.
        let lake_idx = if indirection_idx < 0 {
            // If we're a lake bottom, our own indirection is negative.
            let pass_idx = (-indirection_idx) as usize;
            // NOTE: Must exist since this lake had a downhill in the first place.
            let neighbor_pass_idx = downhill[pass_idx] as usize/*downhill_idx*/;
            let mut lake_neighbor_pass = &mut rivers[neighbor_pass_idx];
            // We definitely shouldn't have encountered this yet!
            debug_assert!(lake_neighbor_pass.velocity == Vec3::zero());
            // TODO: Rethink making the lake neighbor pass always a river or lake, no matter how
            // much incoming water there is?  Sometimes it looks weird having a river emerge from a
            // tiny pool.
            lake_neighbor_pass.river_kind = Some(RiverKind::River {
                cross_section: Vec2::default(),
            });
            chunk_idx
        } else {
            indirection_idx as usize
        };

        // Find the pass this lake is flowing into (i.e. water at the lake bottom gets
        // pushed towards the point identified by pass_idx).
        let pass_idx = if downhill[lake_idx] < 0 {
            // Flows into nothing, so this lake is its own pass.
            lake_idx
        } else {
            (-indirection[lake_idx]) as usize
        };

        // Add our spline derivative to the downhill river (weighted by the chunk's drainage).
        // NOTE: Don't add the spline derivative to the lake side of the pass for our own lake,
        // because we don't want to preserve weird curvature from before we hit the lake in the
        // outflowing river (this will not apply to one-chunk lakes, which are their own pass).
        if pass_idx != downhill_idx {
            // TODO: consider utilizing height difference component of flux as well; currently we
            // just discard it in figuring out the spline's slope.
            let downhill_river = &mut rivers[downhill_idx];
            let weighted_flow = (neighbor_dim * 2.0 - river_spline_derivative.map(|e| e as f64))
                / derivative_divisor
                * chunk_drainage
                / incoming_drainage;
            downhill_river.spline_derivative += weighted_flow.map(|e| e as f32);
        }

        let neighbor_pass_idx = downhill[pass_idx];
        // Find our own water height.
        let chunk_water_alt = water_alt[chunk_idx];
        if neighbor_pass_idx >= 0 {
            // We may be a river.  But we're not sure yet, since we still could be
            // underwater.  Check the lake height and see if our own water height is within ε of
            // it.
            let lake_water_alt = water_alt[lake_idx];
            if chunk_water_alt == lake_water_alt {
                // Not a river.
                // Check whether we we are the lake side of the pass.
                // NOTE: Safe because this is a lake.
                let (neighbor_pass_pos, river_spline_derivative) = if pass_idx == chunk_idx
                {
                    // This is a pass, so set our flow direction to point to the neighbor pass
                    // rather than downhill.
                    // NOTE: Safe because neighbor_pass_idx >= 0.
                    (
                        uniform_idx_as_vec2(downhill_idx),
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
                return;
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
        // (ii) Using the Gauckler–Manning–Strickler formula for cross-sectional
        //      average velocity of water, we establish that the river can be
        //      "big enough" to appear on the Veloren map.
        //
        // This is very imprecise, of course, and (ii) may (and almost certainly will)
        // change over time.
        //
        // In both cases, we preemptively set our child to be a river, to make sure we have an
        // unbroken stream.  Also in both cases, we go to the effort of computing an effective
        // water velocity vector and cross-sectional dimensions, as well as figuring out the
        // derivative of our interpolating spline (since this percolates through the whole river
        // network).
        let downhill_water_alt = water_alt[downhill_idx];
        let neighbor_distance = neighbor_dim.magnitude();
        let dz = (downhill_water_alt - chunk_water_alt).into();
        let slope = dz.abs() / neighbor_distance;
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
        let river_direction = Vec3::new(neighbor_dim.x, neighbor_dim.y, dz.signum() * dz);

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
    });
    rivers
}

/// Precompute the maximum slope at all points.
///
/// TODO: See if allocating in advance is worthwhile.
fn get_max_slope(
    h: &[Alt],
    rock_strength_nz: &(impl NoiseFn<Point3<f64>> + Sync),
    height_scale: impl Fn(usize) -> Alt + Sync,
) -> Box<[f64]> {
    let min_max_angle = (15.0 / 360.0 * 2.0 * f64::consts::PI).tan();
    let max_max_angle = (60.0 / 360.0 * 2.0 * f64::consts::PI).tan();
    let max_angle_range = max_max_angle - min_max_angle;
    h.par_iter()
        .enumerate()
        .map(|(posi, &z)| {
            let wposf = uniform_idx_as_vec2(posi).map(|e| e as f64)
                * TerrainChunkSize::RECT_SIZE.map(|e| e as f64);
            let height_scale = height_scale(posi);
            let wposz = z as f64 / height_scale as f64;
            // Normalized to be between 6 and and 54 degrees.
            let div_factor = (2.0 * TerrainChunkSize::RECT_SIZE.x as f64) / 8.0;
            let rock_strength = rock_strength_nz.get([wposf.x, wposf.y, wposz * div_factor]);
            let rock_strength = rock_strength.max(-1.0).min(1.0) * 0.5 + 0.5;
            // Logistic regression.  Make sure x ∈ (0, 1).
            let logit = |x: f64| x.ln() - (-x).ln_1p();
            // 0.5 + 0.5 * tanh(ln(1 / (1 - 0.1) - 1) / (2 * (sqrt(3)/pi)))
            let logistic_2_base = 3.0f64.sqrt() * f64::consts::FRAC_2_PI;
            // Assumes μ = 0, σ = 1
            let logistic_cdf = |x: f64| (x / logistic_2_base).tanh() * 0.5 + 0.5;

            // We do log-odds against center, so that our log odds are 0 when x = 0.25, lower when x is
            // lower, and higher when x is higher.
            //
            // (NOTE: below sea level, we invert it).
            //
            // TODO: Make all this stuff configurable... but honestly, it's so complicated that I'm not
            // sure anyone would be able to usefully tweak them on a per-map basis?  Plus it's just a
            // hacky heuristic anyway.
            let center = 0.4;
            let dmin = center - 0.05;
            let dmax = center + 0.05;
            let log_odds = |x: f64| logit(x) - logit(center);
            let rock_strength = logistic_cdf(
                1.0 * logit(rock_strength.min(1.0f64 - 1e-7).max(1e-7))
                    + 1.0
                        * log_odds(
                            (wposz / CONFIG.mountain_scale as f64)
                                .abs()
                                .min(dmax)
                                .max(dmin),
                        ),
            );
            let max_slope = rock_strength * max_angle_range + min_max_angle;
            // NOTE: If you want to disable varying rock strength entirely, uncomment  this line.
            // let max_slope = 3.0.sqrt() / 3.0;
            max_slope
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
///   (we choose m = 0.4 and n = 1), and k is a constant.  We choose
///
///   k = 2.244 * uplift.max() / (desired_max_height)
///
///   since this tends to produce mountains of max height desired_max_height; and we set
///   desired_max_height = 1.0 to reflect limitations of mountain scale.
///
/// This algorithm does this in four steps:
///
/// 1. Sort the nodes in h by height (so the lowest node by altitude is first
///    in the list, and the highest node by altitude is last).
/// 2. Iterate through the list in *reverse.*  For each node, we compute its
///    drainage area as the sum of the drainage areas of its "children" nodes
///    (i.e. the nodes with directed edges to this node).  To do this
///    efficiently, we start with the "leaves" (the highest nodes), which
///    have no neighbors higher than them, hence no directed edges to them.
///    We add their area to themselves, and then to all neighbors that they
///    flow into (their "ancestors" in the flow graph); currently, this just
///    means the node immediately downhill of this node. As we go lower, we
///    know that all our "children" already had their areas computed, which
///    means that we can repeat the process in order to derive all the final
///    areas.
/// 3. Now, iterate through the list in *order.*  Whether we used the filling
///    method to compute a "filled" version of each depression, or used the lake
///    connection algoirthm described in [1], each node is guaranteed to have
///    zero or one drainage edges out, representing the direction of water flow
///    for that node. For nodes i with zero drainage edges out (boundary nodes
///    and lake bottoms) we set the slope to 0 (so the change in altitude is
///    uplift(i))
///    For nodes with at least one drainage edge out, we take advantage of the
///    fact that we are computing new heights in order and rewrite our equation
///    as (letting j = downhill[i], A[i] be the computed area of point i,
///    p(i) be the x-y position of point i,
///    flux(i) = k * A[i]^m / ((p(i) - p(j)).magnitude()), and δt = 1):
///
///    h[i](t + dt) = h[i](t) + δt * (uplift[i] + flux(i) * h[j](t + δt)) / (1 + flux(i) * δt).
///
///    Since we compute heights in ascending order by height, and j is downhill from i, h[j] will
///    always be the *new* h[j](t + δt), while h[i] will still not have been computed yet, so we
///    only need to visit each node once.
///
/// Afterwards, we also apply a hillslope diffusion process using an ADI (alternating direction
/// implicit) method:
///
/// https://github.com/fastscape-lem/fastscapelib-fortran/blob/master/src/Diffusion.f90
///
/// We also borrow the implementation for sediment transport from
///
/// https://github.com/fastscape-lem/fastscapelib-fortran/blob/master/src/StreamPowerLaw.f90
///
/// The  approximate equation for soil production function (predictng the rate at which bedrock
/// turns into soil, increasing the distance between the basement and altitude) is taken from
/// equation (11) from [2].  This (among numerous other sources) also includes at least one
/// prediction that hillslope diffusion should be nonlinear, which we sort of attempt to
/// approximate.
///
/// [1] Guillaume Cordonnier, Jean Braun, Marie-Paule Cani,
///     Bedrich Benes, Eric Galin, et al..
///     Large Scale Terrain Generation from Tectonic Uplift and Fluvial Erosion.
///     Computer Graphics Forum, Wiley, 2016, Proc. EUROGRAPHICS 2016, 35 (2), pp.165-175.
///     ⟨10.1111/cgf.12820⟩. ⟨hal-01262376⟩
///
/// [2] William E. Dietrich, Dino G. Bellugi, Leonard S. Sklar,
///     Jonathan D. Stock
///     Geomorphic Transport Laws for Predicting Landscape Form and Dynamics.
///     Prediction in Geomorphology, Geophysical Monograph 135.
///     Copyright 2003 by the American Geophysical Union
///     10.1029/135GM09
fn erode(
    // Height above sea level of topsoil
    h: &mut [Alt],
    // Height above sea level of bedrock
    b: &mut [Alt],
    // Height above sea level of water
    wh: &mut [Alt],
    max_uplift: f32,
    max_g: f32,
    kdsed: f64,
    _seed: &RandomField,
    rock_strength_nz: &(impl NoiseFn<Point3<f64>> + Sync),
    uplift: impl Fn(usize) -> f32 + Sync,
    n_f: impl Fn(usize) -> f32 + Sync,
    m_f: impl Fn(usize) -> f32 + Sync,
    kf: impl Fn(usize) -> f64 + Sync,
    kd: impl Fn(usize) -> f64,
    g: impl Fn(usize) -> f32 + Sync,
    epsilon_0: impl Fn(usize) -> f32 + Sync,
    alpha: impl Fn(usize) -> f32 + Sync,
    is_ocean: impl Fn(usize) -> bool + Sync,
    // scaling factors
    height_scale: impl Fn(f32) -> Alt + Sync,
    k_da_scale: impl Fn(f64) -> f64,
) {
    let compute_stats = true;
    log::debug!("Done draining...");
    // NOTE: To experimentally allow erosion to proceed below sea level, replace 0.0 with
    // -<Alt as Float>::infinity().
    let min_erosion_height = 0.0; // -<Alt as Float>::infinity();

    // NOTE: The value being divided by here sets the effective maximum uplift rate, as everything
    // is scaled to it!
    let dt = max_uplift as f64 / 1e-3;
    log::debug!("dt={:?}", dt);
    // Minimum sediment thickness before we treat erosion as sediment based.
    let sediment_thickness = |_n| /*6.25e-5*/1.0e-4 * dt;
    let neighbor_coef = TerrainChunkSize::RECT_SIZE.map(|e| e as f64);
    let chunk_area = neighbor_coef.x * neighbor_coef.y;
    let min_length = neighbor_coef.reduce_partial_min();
    let max_stable = min_length * min_length / dt;

    // Debris flow area coefficient (m^(-2q)).
    let q = 0.2;
    // NOTE: Set to 1.0 to make (assuming n = 1) the erosion equation linear during each stream
    // power iteration.  This will result in significant speedups, at the cost of less interesting
    // erosion behavior (linear vs. nonlinear).
    let q_ = 1.5;
    let k_da = 2.5 * k_da_scale(q);
    let nx = WORLD_SIZE.x;
    let ny = WORLD_SIZE.y;
    let dx = TerrainChunkSize::RECT_SIZE.x as f64;
    let dy = TerrainChunkSize::RECT_SIZE.y as f64;

    #[rustfmt::skip]
    // ε₀ and α are part of the soil production approximate
    // equation:
    //
    // -∂z_b / ∂t = ε₀ * e^(-αH)
    //
    // where
    //    z_b is the elevation of the soil-bedrock interface (i.e. the basement),
    //    ε₀ is the production rate of exposed bedrock (H = 0),
    //    H is the soil thickness normal to the ground surface,
    //    and α is a parameter (units of 1 / length).
    //
    // Note that normal depth at i, for us, will be interpreted as the soil depth vector,
    //   sed_i = ((0, 0), h_i - b_i),
    // projected onto the ground surface slope vector,
    //   ground_surface_i = ((p_i - p_j), h_i - h_j),
    // yielding the soil depth vector
    //   H_i = sed_i - sed_i ⋅ ground_surface_i / (ground_surface_i ⋅ ground_surface_i) * ground_surface_i
    //
    //       = ((0, 0), h_i - b_i) -
    //         (0 * ||p_i - p_j|| + (h_i - b_i) * (h_i - h_j)) / (||p_i - p_j||^2 + (h_i - h_j)^2)
    //         * (p_i - p_j, h_i - h_j)
    //       = ((0, 0), h_i - b_i) -
    //         ((h_i - b_i) * (h_i - h_j)) / (||p_i - p_j||^2 + (h_i - h_j)^2)
    //         * (p_i - p_j, h_i - h_j)
    //       = (h_i - b_i) *
    //         (((0, 0), 1) - (h_i - h_j) / (||p_i - p_j||^2 + (h_i - h_j)^2) * (p_i - p_j, h_i - h_j))
    //   H_i_fact = (h_i - h_j) / (||p_i - p_j||^2 + (h_i - h_j)^2)
    //   H_i = (h_i - b_i) * ((((0, 0), 1) - H_i_fact * (p_i - p_j, h_i - h_j)))
    //       = (h_i - b_i) * (-H_i_fact * (p_i - p_j), 1 - H_i_fact * (h_i - h_j))
    //   ||H_i|| = (h_i - b_i) * √(H_i_fact^2 * ||p_i - p_j||^2 + (1 - H_i_fact * (h_i - h_j))^2)
    //           = (h_i - b_i) * √(H_i_fact^2 * ||p_i - p_j||^2 + 1 - 2 * H_i_fact * (h_i - h_j) +
    //                             H_i_fact^2 * (h_i - h_j)^2)
    //           = (h_i - b_i) * √(H_i_fact^2 * (||p_i - p_j||^2 + (h_i - h_j)^2) +
    //                             1 - 2 * H_i_fact * (h_i - h_j))
    //           = (h_i - b_i) * √((h_i - h_j)^2 / (||p_i - p_j||^2 + (h_i - h_j)^2) * (||p_i - p_j||^2 + (h_i - h_j)^2) +
    //                             1 - 2 * (h_i - h_j)^2 / (||p_i - p_j||^2 + (h_i - h_j)^2))
    //           = (h_i - b_i) * √((h_i - h_j)^2 - 2(h_i - h_j)^2 / (||p_i - p_j||^2 + (h_i - h_j)^2) + 1)
    //
    // where j is i's receiver and ||p_i - p_j|| is the horizontal displacement between them.  The
    // idea here is that we first compute the hypotenuse between the horizontal and vertical
    // displacement of ground (getting the horizontal component of the triangle), and then this is
    // taken as one of the non-hypotenuse sides of the triangle whose other non-hypotenuse side is
    // the normal height H_i, while their square adds up to the vertical displacement (h_i - b_i).
    // If h and b have different slopes, this may not work completely correctly, but this is
    // probably fine as an approximation.

    // Spatio-temporal variation in net precipitation rate ((m / year) / (m / year))  (ratio of
    // precipitation rate at chunk relative to mean precipitation rate p₀).
    let p = 1.0;
    // Dimensionless multiplier for stream power erosion constant when land becomes sediment.
    let k_fs_mult_sed = 4.0;
    // Dimensionless multiplier for G when land becomes sediment.
    let g_fs_mult_sed = 1.0;
    let ((dh, newh, maxh, mrec, mstack, mwrec, area), (mut max_slopes, h_t)) = rayon::join(
        || {
            let mut dh = downhill(|posi| h[posi], |posi| is_ocean(posi) && h[posi] <= 0.0);
            log::debug!("Computed downhill...");
            let (boundary_len, _indirection, newh, maxh) = get_lakes(|posi| h[posi], &mut dh);
            log::debug!("Got lakes...");
            let (mrec, mstack, mwrec) = get_multi_rec(
                |posi| h[posi],
                &dh,
                &newh,
                wh,
                nx,
                ny,
                dx as Compute,
                dy as Compute,
                maxh,
            );
            log::debug!("Got multiple receivers...");
            // TODO: Figure out how to switch between single-receiver and multi-receiver drainage,
            // as the former is much less computationally costly.
            // let area = get_drainage(&newh, &dh, boundary_len);
            let area = get_multi_drainage(&mstack, &mrec, &*mwrec, boundary_len);
            log::debug!("Got flux...");
            (dh, newh, maxh, mrec, mstack, mwrec, area)
        },
        || {
            rayon::join(
                || {
                    let max_slope =
                        get_max_slope(h, rock_strength_nz, |posi| height_scale(n_f(posi)));
                    log::debug!("Got max slopes...");
                    max_slope
                },
                || h.to_vec().into_boxed_slice(),
            )
        },
    );

    assert!(h.len() == dh.len() && dh.len() == area.len());

    // max angle of slope depends on rock strength, which is computed from noise function.
    // TODO: Make more principled.
    let mid_slope = (30.0 / 360.0 * 2.0 * f64::consts::PI).tan();

    type SimdType = f32;
    type MaskType = m32;

    // Precompute factors for Stream Power Law.
    let czero = <SimdType as Zero>::zero();
    let (k_fs_fact, k_df_fact) = rayon::join(
        || {
            dh.par_iter()
                .enumerate()
                .map(|(posi, &posj)| {
                    let mut k_tot = [czero; 8];
                    if posj < 0 {
                        // Egress with no outgoing flows, no stream power.
                        k_tot
                    } else {
                        let old_b_i = b[posi];
                        let sed = (h_t[posi] - old_b_i) as f64;
                        let n = n_f(posi);
                        // Higher rock strength tends to lead to higher erodibility?
                        let kd_factor = 1.0;
                        let k_fs = kf(posi) / kd_factor;

                        let k = if sed > sediment_thickness(n) {
                            // Sediment
                            k_fs_mult_sed * k_fs
                        } else {
                            // Bedrock
                            k_fs
                        } * dt;
                        let n = n as f64;
                        let m = m_f(posi) as f64;

                        let mwrec_i = &mwrec[posi];
                        mrec_downhill(&mrec, posi).for_each(|(kk, posj)| {
                            let dxy = (uniform_idx_as_vec2(posi) - uniform_idx_as_vec2(posj))
                                .map(|e| e as f64);
                            let neighbor_distance = (neighbor_coef * dxy).magnitude();
                            let knew = (k
                                * (p as f64
                                    * chunk_area
                                    * (area[posi] as f64 * mwrec_i[kk] as f64))
                                    .powf(m)
                                / neighbor_distance.powf(n))
                                as SimdType;
                            k_tot[kk] = knew;
                        });
                        k_tot
                    }
                })
                .collect::<Vec<[SimdType; 8]>>()
        },
        || {
            dh.par_iter()
                .enumerate()
                .map(|(posi, &posj)| {
                    let mut k_tot = [czero; 8];
                    let uplift_i = uplift(posi) as f64;
                    debug_assert!(uplift_i.is_normal() && uplift_i > 0.0 || uplift_i == 0.0);
                    if posj < 0 {
                        // Egress with no outgoing flows, no stream power.
                        k_tot
                    } else {
                        let area_i = area[posi] as f64;
                        let max_slope = max_slopes[posi];
                        let chunk_area_pow = chunk_area.powf(q);

                        let old_b_i = b[posi];
                        let sed = (h_t[posi] - old_b_i) as f64;
                        let n = n_f(posi);
                        let g_i = if sed > sediment_thickness(n) {
                            // Sediment
                            (g_fs_mult_sed * g(posi)) as f64
                        } else {
                            // Bedrock
                            g(posi) as f64
                        };

                        // Higher rock strength tends to lead to higher curvature?
                        let kd_factor = (max_slope / mid_slope).powf(2.0);
                        let k_da = k_da * kd_factor;

                        let mwrec_i = &mwrec[posi];
                        mrec_downhill(&mrec, posi).for_each(|(kk, posj)| {
                            let mwrec_kk = mwrec_i[kk] as f64;

                            #[rustfmt::skip]
                            // Working equation:
                            //   U = uplift per time
                            //   D = sediment deposition per time
                            //   E = fluvial erosion per time
                            //   0 = U + D - E - k_df * (1 + k_da * (mrec_kk * A)^q) * (∂B/∂p)^(q_)
                            //
                            //   k_df = (U + D - E) / (1 + k_da * (mrec_kk * A)^q) / (∂B/∂p)^(q_)
                            //
                            // Want: ∂B/∂p = max slope at steady state, i.e.
                            //     ∂B/∂p = max_slope
                            // Then:
                            //   k_df = (U + D - E) / (1 + k_da * (mrec_kk * A)^q) / max_slope^(q_)
                            // Letting
                            //   k = k_df * Δt
                            // we get:
                            //     k = (U + D - E)Δt / (1 + k_da * (mrec_kk * A)^q) / (ΔB)^(q_)
                            //
                            // Now ∂B/∂t under constant uplift, without debris flow (U + D - E), is
                            //   U + D - E = U - E + G/(p̃A) * ∫_A((U - ∂h/∂t) * dA)
                            //
                            // Observing that at steady state ∂h/∂t should theoretically
                            // be 0, we can simplify to:
                            //   U + D = U + G/(p̃A) * ∫_A(U * dA)
                            //
                            // Upper bounding this at uplift = max_uplift/∂t for the whole prior
                            // drainage area, and assuming we account for just mrec_kk of
                            // the combined uplift and deposition, we get:
                            //
                            //   U + D ≤ mrec_kk * U + G/p̃ * max_uplift/∂t
                            //   (U + D - E)Δt ≤ (mrec_kk * uplift_i + G/p̃ * mrec_kk * max_uplift - EΔt)
                            //
                            // therefore
                            //   k * (1 + k_da * (mrec_kk * A)^q) * max_slope^(q_) ≤ (mrec_kk * (uplift_i + G/p̃ * max_uplift) - EΔt)
                            // i.e.
                            //   k ≤ (mrec_kk * (uplift_i + G/p̃ * max_uplift) - EΔt) / (1 + k_da * (mrec_kk * A)^q) / max_slope^q_
                            //
                            // (eliminating EΔt maintains the sign, but it's somewhat imprecise;
                            //  we can address this later, e.g. by assigning a debris flow / fluvial erosion ratio).
                            let chunk_neutral_area = 0.1e6; // 1 km^2 * (1000 m / km)^2 = 1e6 m^2
                            let k = (mwrec_kk * (uplift_i + max_uplift as f64 * g_i / p as f64))
                                / (1.0 + k_da * (mwrec_kk * chunk_neutral_area).powf(q))
                                / max_slope.powf(q_);
                            let dxy = (uniform_idx_as_vec2(posi) - uniform_idx_as_vec2(posj))
                                .map(|e| e as f64);
                            let neighbor_distance = (neighbor_coef * dxy).magnitude();

                            let knew = (k
                                * (1.0 + k_da * chunk_area_pow * (area_i * mwrec_kk).powf(q))
                                / neighbor_distance.powf(q_))
                                as SimdType;
                            k_tot[kk] = knew;
                        });
                        k_tot
                    }
                })
                .collect::<Vec<[SimdType; 8]>>()
        },
    );

    log::debug!("Computed stream power factors...");

    let mut lake_water_volume =
        vec![0.0 as Compute; WORLD_SIZE.x * WORLD_SIZE.y].into_boxed_slice();
    let mut elev = vec![0.0 as Compute; WORLD_SIZE.x * WORLD_SIZE.y].into_boxed_slice();
    let mut h_p = vec![0.0 as Compute; WORLD_SIZE.x * WORLD_SIZE.y].into_boxed_slice();
    let mut deltah = vec![0.0 as Compute; WORLD_SIZE.x * WORLD_SIZE.y].into_boxed_slice();

    // calculate the elevation / SPL, including sediment flux
    let tol1 = 1.0e-4 as Compute * (maxh as Compute + 1.0);
    let tol2 = 1.0e-3 as Compute * (max_uplift as Compute + 1.0);
    let tol = tol1.max(tol2);
    let mut err = 2.0 * tol;

    // Some variables for tracking statistics, currently only for debugging purposes.
    let mut minh = <Alt as Float>::infinity();
    let mut maxh = 0.0;
    let mut nland = 0usize;
    let mut ncorr = 0usize;
    let mut sums = 0.0;
    let mut sumh = 0.0;
    let mut sumsed = 0.0;
    let mut sumsed_land = 0.0;
    let mut ntherm = 0usize;
    // ln of product of actual slopes (only where actual is above critical).
    let mut prods_therm = 0.0;
    // ln of product of critical slopes (only where actual is above critical).
    let mut prodscrit_therm = 0.0;
    let avgz = |x, y: usize| if y == 0 { f64::INFINITY } else { x / y as f64 };
    let geomz = |x: f64, y: usize| {
        if y == 0 {
            f64::INFINITY
        } else {
            (x / y as f64).exp()
        }
    };

    // Gauss-Seidel iteration

    let mut lake_silt = vec![0.0 as Compute; WORLD_SIZE.x * WORLD_SIZE.y].into_boxed_slice();
    let mut lake_sill = vec![-1isize; WORLD_SIZE.x * WORLD_SIZE.y].into_boxed_slice();

    let mut n_gs_stream_power_law = 0;
    // NOTE: Increasing this can theoretically sometimes be necessary for convergence, but in
    // practice it is fairly unlikely that you should need to do this (especially if you stick to
    // g ∈ [0, 1]).
    let max_n_gs_stream_power_law = 99;
    let mut mstack_inv = vec![0usize; dh.len()];
    let mut h_t_stack = vec![Zero::zero(); dh.len()];
    let mut dh_stack = vec![0isize; dh.len()];
    let mut h_stack = vec![Zero::zero(); dh.len()];
    let mut b_stack = vec![Zero::zero(); dh.len()];
    let mut area_stack = vec![Zero::zero(); dh.len()];
    assert!(mstack.len() == dh.len());
    assert!(b.len() == dh.len());
    assert!(h_t.len() == dh.len());
    let mstack_inv = &mut *mstack_inv;
    mstack.iter().enumerate().for_each(|(stacki, &posi)| {
        let posi = posi as usize;
        mstack_inv[posi] = stacki;
        dh_stack[stacki] = dh[posi];
        h_t_stack[stacki] = h_t[posi];
        h_stack[stacki] = h[posi];
        b_stack[stacki] = b[posi];
        area_stack[stacki] = area[posi];
    });

    while err > tol && n_gs_stream_power_law < max_n_gs_stream_power_law {
        log::debug!("Stream power iteration #{:?}", n_gs_stream_power_law);

        // Reset statistics in each loop.
        maxh = 0.0;
        minh = <Alt as Float>::infinity();
        nland = 0usize;
        ncorr = 0usize;
        sums = 0.0;
        sumh = 0.0;
        sumsed = 0.0;
        sumsed_land = 0.0;
        ntherm = 0usize;
        prods_therm = 0.0;
        prodscrit_therm = 0.0;

        let start_time = Instant::now();
        // Keep track of how many iterations we've gone to test for convergence.
        n_gs_stream_power_law += 1;

        rayon::join(
            || {
                // guess/update the elevation at t+Δt (k)
                (&mut *h_p, &*h_stack)
                    .into_par_iter()
                    .for_each(|(h_p, h_)| {
                        *h_p = (*h_) as Compute;
                    });
            },
            || {
                // calculate erosion/deposition of sediment at each node
                (&*mstack, &mut *deltah, &*h_t_stack, &*h_stack)
                    .into_par_iter()
                    .for_each(|(&posi, deltah, &h_t_i, &h_i)| {
                        let posi = posi as usize;
                        let uplift_i = uplift(posi) as Alt;
                        let delta = (h_t_i + uplift_i - h_i) as Compute;
                        *deltah = delta;
                    });
            },
        );
        log::debug!(
            "(Done precomputation, time={:?}ms).",
            start_time.elapsed().as_millis()
        );
        #[rustfmt::skip]
        // sum the erosion in stack order
        //
        // After:
        // deltah_i = Σ{j ∈ {i} ∪ upstream_i(t)}(h_j(t, FINAL) + U_j * Δt - h_i(t + Δt, k))
        let start_time = Instant::now();
        izip!(&*mstack, &*dh_stack, &h_t_stack, &*h_p)
            .enumerate()
            .for_each(|(stacki, (&posi, &posj, &h_t_i, &h_p_i))| {
                let posi = posi as usize;
                let deltah_i = deltah[stacki];
                if posj < 0 {
                    lake_silt[stacki] = deltah_i;
                } else {
                    let uplift_i = uplift(posi) as Alt;
                    let uphill_deltah_i = deltah_i - ((h_t_i + uplift_i) as Compute - h_p_i);
                    let lposi = lake_sill[stacki];
                    if lposi == stacki as isize {
                        if uphill_deltah_i <= 0.0 {
                            lake_silt[stacki] = 0.0;
                        } else {
                            lake_silt[stacki] = uphill_deltah_i;
                        }
                    }
                    let mwrec_i = &mwrec[posi];
                    mrec_downhill(&mrec, posi).for_each(|(k, posj)| {
                        let stack_posj = mstack_inv[posj];
                        deltah[stack_posj] += deltah_i * mwrec_i[k];
                    });
                }
            });
        log::debug!(
            "(Done sediment transport computation, time={:?}ms).",
            start_time.elapsed().as_millis()
        );
        #[rustfmt::skip]
        // do ij=nn,1,-1
        //   ijk=stack(ij)
        //   ijr=rec(ijk)
        //   if (ijr.ne.ijk) then
        //     dh(ijk)=dh(ijk)-(ht(ijk)-hp(ijk))
        //     if (lake_sill(ijk).eq.ijk) then
        //       if (dh(ijk).le.0.d0) then
        //         lake_sediment(ijk)=0.d0
        //       else
        //         lake_sediment(ijk)=dh(ijk)
        //       endif
        //     endif
        //     dh(ijk)=dh(ijk)+(ht(ijk)-hp(ijk))
        //     dh(ijr)=dh(ijr)+dh(ijk)
        //   else
        //     lake_sediment(ijk)=dh(ijk)
        //   endif
        // enddo

        let start_time = Instant::now();
        (
            &*mstack,
            &mut *elev,
            &*dh_stack,
            &*h_t_stack,
            &*area_stack,
            &*deltah,
            &*h_p,
            &*b_stack,
        )
            .into_par_iter()
            .for_each(
                |(&posi, elev, &dh_i, &h_t_i, &area_i, &deltah_i, &h_p_i, &b_i)| {
                    let posi = posi as usize;

                    let uplift_i = uplift(posi) as Alt;
                    if dh_i < 0 {
                        *elev = (h_t_i + uplift_i) as Compute;
                    } else {
                        let old_h_after_uplift_i = (h_t_i + uplift_i) as Compute;
                        let area_i = area_i as Compute;
                        let uphill_silt_i = deltah_i - (old_h_after_uplift_i - h_p_i);
                        let old_b_i = b_i;
                        let sed = (h_t_i - old_b_i) as f64;
                        let n = n_f(posi);
                        let g_i = if sed > sediment_thickness(n) {
                            (g_fs_mult_sed * g(posi)) as Compute
                        } else {
                            g(posi) as Compute
                        };
                        // Make sure deposition coefficient doesn't result in more deposition than there
                        // actually was material to deposit.  The current assumption is that as long as we
                        // are storing at most as much sediment as there actually was along the river, we
                        // are in the clear.
                        let g_i_ratio = g_i / (p * area_i);
                        // One side of nonlinear equation (23):
                        //
                        // h_i(t) + U_i * Δt + G / (p̃ * Ã_i) * Σ
                        // {j ∈ upstream_i(t)}(h_j(t, FINAL)
                        // + U_j * Δt - h_j(t + Δt, k))
                        //
                        // where
                        //
                        // Ã_i = A_i / (∆x∆y) = N_i,
                        // number of cells upstream of cell i.
                        *elev = old_h_after_uplift_i + uphill_silt_i * g_i_ratio;
                    }
                },
            );
        log::debug!(
            "(Done elevation estimation, time={:?}ms).",
            start_time.elapsed().as_millis()
        );

        let start_time = Instant::now();
        // TODO: Consider taking advantage of multi-receiver flow here.
        // Iterate in ascending height order.
        let mut sum_err = 0.0 as Compute;
        itertools::izip!(&*mstack, &*elev, &*b_stack, &*h_t_stack, &*dh_stack, &*h_p)
            .enumerate()
            .rev()
            .for_each(|(stacki, (&posi, &elev_i, &b_i, &h_t_i, &dh_i, &h_p_i))| {
                let iteration_error = 0.0;
                let posi = posi as usize;
                let old_elev_i = elev_i as f64;
                let old_b_i = b_i;
                let old_ht_i = h_t_i;
                let sed = (old_ht_i - old_b_i) as f64;

                let posj = dh_i;
                if posj < 0 {
                    if posj == -1 {
                        panic!("Disconnected lake!");
                    }
                    if h_t_i > 0.0 {
                        log::warn!("Ocean above zero?");
                    }
                    // Egress with no outgoing flows.
                    // wh for oceans is always at least min_erosion_height.
                    let uplift_i = uplift(posi) as Alt;
                    wh[posi] = min_erosion_height.max(h_t_i + uplift_i);
                    lake_sill[stacki] = posi as isize;
                    lake_water_volume[stacki] = 0.0;
                } else {
                    let posj = posj as usize;

                    // Has an outgoing flow edge (posi, posj).
                    // flux(i) = k * A[i]^m / ((p(i) - p(j)).magnitude()), and δt = 1
                    // h[i](t + dt) = (h[i](t) + δt * (uplift[i] + flux(i) * h[j](t + δt))) / (1 + flux(i) * δt).
                    // NOTE: posj has already been computed since it's downhill from us.
                    // Therefore, we can rely on wh being set to the water height for that node.
                    // let h_j = h[posj_stack] as f64;
                    let wh_j = wh[posj] as f64;
                    let old_h_i = h_stack[stacki] as f64;
                    let mut new_h_i = old_h_i;

                    // Only perform erosion if we are above the water level of the previous node.
                    // NOTE: Can replace wh_j with h_j here (and a few other places) to allow erosion underwater,
                    // producing very different looking maps!
                    if old_elev_i > wh_j
                    /*h_j*/
                    {
                        let dtherm = 0.0f64;
                        let h0 = old_elev_i + dtherm;

                        // hi(t + ∂t) = (hi(t) + ∂t(ui + kp^mAi^m(hj(t + ∂t)/||pi - pj||))) / (1 + ∂t * kp^mAi^m / ||pi - pj||)
                        let n = n_f(posi) as f64;

                        // Fluvial erosion.
                        let k_df_fact = &k_df_fact[posi];
                        let k_fs_fact = &k_fs_fact[posi];
                        if (n - 1.0).abs() <= 1.0e-3 && (q_ - 1.0).abs() <= 1.0e-3 {
                            let mut f = h0;
                            let mut df = 1.0;
                            mrec_downhill(&mrec, posi).for_each(|(kk, posj)| {
                                let posj_stack = mstack_inv[posj];
                                let h_j = h_stack[posj_stack] as f64;
                                // This can happen in cases where receiver kk is neither uphill of
                                // nor downhill from posi's direct receiver.
                                // NOTE: Fastscape does h_t[posi] + uplift(posi) as f64 >= h_t[posj] + uplift(posj) as f64
                                // NOTE: We also considered using old_elev_i > wh[posj] here.
                                if old_elev_i > h_j {
                                    let elev_j = h_j;
                                    let fact = k_fs_fact[kk] as f64 + k_df_fact[kk] as f64;
                                    f += fact * elev_j;
                                    df += fact;
                                }
                            });
                            new_h_i = f / df;
                        } else {
                            // Local Newton-Raphson
                            // TODO: Work out how (if possible) to make this converge for tiny n.
                            let omega1 = 0.875f64 * n;
                            let omega2 = 0.875f64 / q_;
                            let omega = omega1.max(omega2);
                            let tolp = 1.0e-3;
                            let mut errp = 2.0 * tolp;
                            let mut rec_heights = [0.0; 8];
                            let mut mask = [MaskType::new(false); 8];
                            mrec_downhill(&mrec, posi).for_each(|(kk, posj)| {
                                let posj_stack = mstack_inv[posj];
                                let h_j = h_stack[posj_stack];
                                // NOTE: Fastscape does h_t[posi] + uplift(posi) as f64 >= h_t[posj] + uplift(posj) as f64
                                // NOTE: We also considered using old_elev_i > wh[posj] here.
                                if old_elev_i > h_j as f64 {
                                    mask[kk] = MaskType::new(true);
                                    rec_heights[kk] = h_j as SimdType;
                                }
                            });
                            while errp > tolp {
                                let mut f = new_h_i - h0;
                                let mut df = 1.0;
                                izip!(&mask, &rec_heights, k_fs_fact, k_df_fact).for_each(
                                    |(&mask_kk, &rec_heights_kk, &k_fs_fact_kk, &k_df_fact_kk)| {
                                        if mask_kk.test() {
                                            let h_j = rec_heights_kk;
                                            let elev_j = h_j;
                                            let dh = 0.0.max(new_h_i as SimdType - elev_j);
                                            let powf = |a: SimdType, b| a.powf(b);
                                            let dh_fs_sample = k_fs_fact_kk as SimdType
                                                * powf(dh, n as SimdType - 1.0);
                                            let dh_df_sample = k_df_fact_kk as SimdType
                                                * powf(dh, q_ as SimdType - 1.0);
                                            // Want: h_i(t+Δt) = h0 - fact * (h_i(t+Δt) - h_j(t+Δt))^n
                                            // Goal: h_i(t+Δt) - h0 + fact * (h_i(t+Δt) - h_j(t+Δt))^n = 0
                                            f += ((dh_fs_sample + dh_df_sample) * dh) as f64;
                                            // ∂h_i(t+Δt)/∂n = 1 + fact * n * (h_i(t+Δt) - h_j(t+Δt))^(n - 1)
                                            df += (n as SimdType * dh_fs_sample
                                                + q_ as SimdType * dh_df_sample)
                                                as f64;
                                        }
                                    },
                                );
                                // hn = h_i(t+Δt, k) - (h_i(t+Δt, k) - (h0 - fact * (h_i(t+Δt, k) - h_j(t+Δt))^n)) / ∂h_i/∂n(t+Δt, k)
                                let hn = new_h_i - f / df;
                                // errp = |(h_i(t+Δt, k) - (h0 - fact * (h_i(t+Δt, k) - h_j(t+Δt))^n)) / ∂h_i/∂n(t+Δt, k)|
                                errp = (hn - new_h_i).abs();
                                // h_i(t+∆t, k+1) = ...
                                new_h_i = new_h_i * (1.0 - omega) + hn * omega;
                            }
                            /* omega=0.875d0/n
                            tolp=1.d-3
                            errp=2.d0*tolp
                            h0=elev(ijk)
                            do while (errp.gt.tolp)
                              f=h(ijk)-h0
                              df=1.d0
                              if (ht(ijk).gt.ht(ijr)) then
                                fact = kfint(ijk)*dt*a(ijk)**m/length(ijk)**n
                                f=f+fact*max(0.d0,h(ijk)-h(ijr))**n
                                df=df+fact*n*max(0.d0,h(ijk)-h(ijr))**(n-1.d0)
                              endif
                              hn=h(ijk)-f/df
                              errp=abs(hn-h(ijk))
                              h(ijk)=h(ijk)*(1.d0-omega)+hn*omega
                            enddo */
                        }

                        lake_sill[stacki] = posi as isize;
                        lake_water_volume[stacki] = 0.0;

                        // If we dipped below the receiver's water level, set our height to the receiver's
                        // water level.
                        // NOTE: If we want erosion to proceed underwater, use h_j here instead of wh_j.
                        if new_h_i <= wh_j
                        /*h_j*/
                        {
                            if compute_stats {
                                ncorr += 1;
                            }
                            // NOTE: Why wh_j?
                            // Because in the next round, if the old height is still wh_j or under, it
                            // will be set precisely equal to the estimated height, meaning it
                            // effectively "vanishes" and just deposits sediment to its reciever.
                            // (This is probably related to criteria for block Gauss-Seidel, etc.).
                            // NOTE: If we want erosion to proceed underwater, use h_j here instead of
                            // wh_j.
                            new_h_i = wh_j;
                        } else {
                            if compute_stats && new_h_i > 0.0 {
                                let dxy = (uniform_idx_as_vec2(posi) - uniform_idx_as_vec2(posj))
                                    .map(|e| e as f64);
                                let neighbor_distance = (neighbor_coef * dxy).magnitude();
                                let dz = (new_h_i - wh_j).max(0.0);
                                let mag_slope = dz / neighbor_distance;

                                nland += 1;
                                sumsed_land += sed;
                                sumh += new_h_i;
                                sums += mag_slope;
                            }
                        }
                    } else {
                        new_h_i = old_elev_i;
                        let posj_stack = mstack_inv[posj];
                        let lposj = lake_sill[posj_stack];
                        lake_sill[stacki] = lposj;
                        if lposj >= 0 {
                            let lposj = lposj as usize;
                            lake_water_volume[lposj] += (wh_j - old_elev_i) as Compute;
                        }
                    }
                    // Set max_slope to this node's water height (max of receiver's water height and
                    // this node's height).
                    wh[posi] = wh_j.max(new_h_i) as Alt;
                    h_stack[stacki] = new_h_i as Alt;
                }
                if compute_stats {
                    sumsed += sed;
                    let h_i = h_stack[stacki];
                    if h_i > 0.0 {
                        minh = h_i.min(minh);
                    }
                    maxh = h_i.max(maxh);
                }

                // Add sum of squares of errors.
                sum_err +=
                    (iteration_error + h_stack[stacki] as Compute - h_p_i as Compute).powi(2);
            });
        log::debug!(
            "(Done erosion computation, time={:?}ms)",
            start_time.elapsed().as_millis()
        );

        err = (sum_err / mstack.len() as Compute).sqrt();
        log::debug!("(RMSE: {:?})", err);
        if max_g == 0.0 {
            err = 0.0;
        }
        if n_gs_stream_power_law == max_n_gs_stream_power_law {
            log::warn!(
                "Beware: Gauss-Siedel scheme not convergent: err={:?}, expected={:?}",
                err,
                tol
            );
        }
    }

    (&*mstack_inv, &mut *h)
        .into_par_iter()
        .enumerate()
        .for_each(|(posi, (&stacki, h))| {
            assert!(posi == mstack[stacki] as usize);
            *h = h_stack[stacki];
        });

    // update the basement
    //
    // NOTE: Despite this not quite applying since basement order and height order differ, we once
    // again borrow the implicit FastScape stack order.  If this becomes a problem we can easily
    // compute a separate stack order just for basement.
    // TODO: Consider taking advantage of multi-receiver flow here.
    b.par_iter_mut()
        .zip_eq(h.par_iter())
        .enumerate()
        .for_each(|(posi, (b, &h_i))| {
            let old_b_i = *b;
            let uplift_i = uplift(posi) as Alt;

            // First, add uplift...
            let mut new_b_i = old_b_i + uplift_i;

            let posj = dh[posi];
            // Sediment height normal to bedrock.  NOTE: Currently we can actually have sedment and
            // bedrock slope at different heights, meaning there's no uniform slope.  There are
            // probably more correct ways to account for this, such as averaging, integrating, or doing
            // things by mass / volume instead of height, but for now we use the time-honored
            // technique of ignoring the problem.
            let vertical_sed = (h_i - new_b_i).max(0.0) as f64;
            let h_normal = if posj < 0 {
                // Egress with no outgoing flows; for now, we assume this means normal and vertical
                // coincide.
                vertical_sed
            } else {
                let posj = posj as usize;
                let h_j = h[posj];
                let dxy = (uniform_idx_as_vec2(posi) - uniform_idx_as_vec2(posj)).map(|e| e as f64);
                let neighbor_distance_squared = (neighbor_coef * dxy).magnitude_squared();
                let dh = (h_i - h_j) as f64;
                // H_i_fact = (h_i - h_j) / (||p_i - p_j||^2 + (h_i - h_j)^2)
                let h_i_fact = dh / (neighbor_distance_squared + dh * dh);
                let h_i_vertical = 1.0 - h_i_fact * dh;
                // ||H_i|| = (h_i - b_i) * √((H_i_fact^2 * ||p_i - p_j||^2 + (1 - H_i_fact * (h_i - h_j))^2))
                vertical_sed
                    * (h_i_fact * h_i_fact * neighbor_distance_squared
                        + h_i_vertical * h_i_vertical)
                        .sqrt()
            };
            // Rate of sediment production: -∂z_b / ∂t = ε₀ * e^(-αH)
            let p_i = epsilon_0(posi) as f64 * dt * f64::exp(-alpha(posi) as f64 * h_normal);

            new_b_i -= p_i as Alt;

            // Clamp basement so it doesn't exceed height.
            new_b_i = new_b_i.min(h_i);
            *b = new_b_i;
        });
    log::debug!("Done updating basement and applying soil production...");

    // update the height to reflect sediment flux.
    if max_g > 0.0 {
        //  If max_g = 0.0, lake_silt will be too high during the first iteration since our
        //  initial estimate for h is very poor; however, the elevation estimate will have been
        //  unaffected by g.
        (&mut *h, &*mstack_inv)
            .into_par_iter()
            .enumerate()
            .for_each(|(posi, (h, &stacki))| {
                let lposi = lake_sill[stacki];
                if lposi >= 0 {
                    let lposi = lposi as usize;
                    if lake_water_volume[lposi] > 0.0 {
                        // +max(0.d0,min(lake_sediment(lake_sill(ij)),lake_water_volume(lake_sill(ij))))/
                        // lake_water_volume(lake_sill(ij))*(water(ij)-h(ij))
                        *h += (0.0.max(lake_silt[stacki].min(lake_water_volume[lposi]))
                            / lake_water_volume[lposi]
                            * (wh[posi] - *h) as Compute) as Alt;
                    }
                }
            });
    }
    // do ij=1,nn
    //   if (lake_sill(ij).ne.0) then
    //     if (lake_water_volume(lake_sill(ij)).gt.0.d0) h(ij)=h(ij) &
    //     +max(0.d0,min(lake_sediment(lake_sill(ij)),lake_water_volume(lake_sill(ij))))/ &
    //     lake_water_volume(lake_sill(ij))*(water(ij)-h(ij))
    //   endif
    // enddo

    log::debug!(
        "Done applying stream power (max height: {:?}) (avg height: {:?}) (min height: {:?}) (avg slope: {:?})\n        \
        (above talus angle, geom. mean slope [actual/critical/ratio]: {:?} / {:?} / {:?})\n        \
        (old avg sediment thickness [all/land]: {:?} / {:?})\n        \
        (num land: {:?}) (num thermal: {:?}) (num corrected: {:?})",
        maxh,
        avgz(sumh, nland),
        minh,
        avgz(sums, nland),
        geomz(prods_therm, ntherm),
        geomz(prodscrit_therm, ntherm),
        geomz(prods_therm - prodscrit_therm, ntherm),
        avgz(sumsed, newh.len()),
        avgz(sumsed_land, nland),
        nland,
        ntherm,
        ncorr,
    );

    // Apply thermal erosion.
    maxh = 0.0;
    minh = <Alt as Float>::infinity();
    sumh = 0.0;
    sums = 0.0;
    sumsed = 0.0;
    sumsed_land = 0.0;
    nland = 0usize;
    ncorr = 0usize;
    ntherm = 0usize;
    prods_therm = 0.0;
    prodscrit_therm = 0.0;
    newh.iter().for_each(|&posi| {
        let posi = posi as usize;
        let old_h_i = h[posi] as f64;
        let old_b_i = b[posi] as f64;
        let sed = (old_h_i - old_b_i) as f64;

        let max_slope = max_slopes[posi];
        let n = n_f(posi);
        max_slopes[posi] = if sed > sediment_thickness(n) && kdsed > 0.0 {
            // Sediment
            kdsed
        } else {
            // Bedrock
            kd(posi)
        };

        let posj = dh[posi];
        if posj < 0 {
            // Egress with no outgoing flows.
            if posj == -1 {
                panic!("Disconnected lake!");
            }
            // wh for oceans is always at least min_erosion_height.
            wh[posi] = min_erosion_height.max(old_h_i as Alt);
        } else {
            let posj = posj as usize;
            // Find the water height for this chunk's receiver; we only apply thermal erosion
            // for chunks above water.
            let wh_j = wh[posj] as f64;
            let mut new_h_i = old_h_i;
            if wh_j < old_h_i {
                // NOTE: Currently assuming that talus angle is not eroded once the substance is
                // totally submerged in water, and that talus angle if part of the substance is
                // in water is 0 (or the same as the dry part, if this is set to wh_j), but
                // actually that's probably not true.
                let old_h_j = h[posj] as f64;
                let h_j = old_h_j;
                // Test the slope.
                // Hacky version of thermal erosion: only consider lowest neighbor, don't redistribute
                // uplift to other neighbors.
                let (posk, h_k) = (posj, h_j);
                let (posk, h_k) = if h_k < h_j { (posk, h_k) } else { (posj, h_j) };
                let dxy = (uniform_idx_as_vec2(posi) - uniform_idx_as_vec2(posk)).map(|e| e as f64);
                let neighbor_distance = (neighbor_coef * dxy).magnitude();
                let dz = (new_h_i - h_k).max(0.0);
                let mag_slope = dz / neighbor_distance;
                if mag_slope >= max_slope {
                    let dtherm = 0.0;
                    new_h_i = new_h_i - dtherm;
                    if new_h_i <= wh_j {
                        if compute_stats {
                            ncorr += 1;
                        }
                    } else {
                        if compute_stats && new_h_i > 0.0 {
                            let dz = (new_h_i - h_j).max(0.0);
                            let slope = dz / neighbor_distance;
                            sums += slope;
                            nland += 1;
                            sumh += new_h_i;
                            sumsed_land += sed;
                        }
                    }
                    if compute_stats {
                        ntherm += 1;
                        prodscrit_therm += max_slope.ln();
                        prods_therm += mag_slope.ln();
                    }
                } else {
                    // Poorly emulating nonlinear hillslope transport as described by
                    // http://eps.berkeley.edu/~bill/papers/112.pdf.
                    // sqrt(3)/3*32*32/(128000/2)
                    // Also Perron-2011-Journal_of_Geophysical_Research__Earth_Surface.pdf
                    let slope_ratio = (mag_slope / max_slope).powi(2);
                    let slope_nonlinear_factor =
                        slope_ratio * ((3.0 - slope_ratio) / (1.0 - slope_ratio).powi(2));
                    max_slopes[posi] += (max_slopes[posi] * slope_nonlinear_factor).min(max_stable);
                    if compute_stats && new_h_i > 0.0 {
                        sums += mag_slope;
                        // Just use the computed rate.
                        nland += 1;
                        sumh += new_h_i;
                        sumsed_land += sed;
                    }
                }
            }
            // Set wh to this node's water height (max of receiver's water height and
            // this node's height).
            wh[posi] = wh_j.max(new_h_i) as Alt;
        }

        if compute_stats {
            sumsed += sed;
            let h_i = h[posi];
            if h_i > 0.0 {
                minh = h_i.min(minh);
            }
            maxh = h_i.max(maxh);
        }
    });
    log::debug!(
        "Done applying thermal erosion (max height: {:?}) (avg height: {:?}) (min height: {:?}) (avg slope: {:?})\n        \
        (above talus angle, geom. mean slope [actual/critical/ratio]: {:?} / {:?} / {:?})\n        \
        (avg sediment thickness [all/land]: {:?} / {:?})\n        \
        (num land: {:?}) (num thermal: {:?}) (num corrected: {:?})",
        maxh,
        avgz(sumh, nland),
        minh,
        avgz(sums, nland),
        geomz(prods_therm, ntherm),
        geomz(prodscrit_therm, ntherm),
        geomz(prods_therm - prodscrit_therm, ntherm),
        avgz(sumsed, newh.len()),
        avgz(sumsed_land, nland),
        nland,
        ntherm,
        ncorr,
    );

    // Apply hillslope diffusion.
    diffusion(
        nx,
        ny,
        nx as f64 * dx,
        ny as f64 * dy,
        dt,
        (),
        h,
        b,
        |posi| max_slopes[posi],
        -1.0,
    );
    log::debug!("Done applying diffusion.");
    log::debug!("Done eroding.");
}

/// The Planchon-Darboux algorithm for extracting drainage networks.
///
/// http://horizon.documentation.ird.fr/exl-doc/pleins_textes/pleins_textes_7/sous_copyright/010031925.pdf
///
/// See https://github.com/mewo2/terrain/blob/master/terrain.js
pub fn fill_sinks<F: Float + Send + Sync>(
    h: impl Fn(usize) -> F + Sync,
    is_ocean: impl Fn(usize) -> bool + Sync,
) -> Box<[F]> {
    // NOTE: We are using the "exact" version of depression-filling, which is slower but doesn't
    // change altitudes.
    let epsilon = F::zero();
    let infinity = F::infinity();
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
                debug_assert!(h <= F::zero());
                h
            } else {
                infinity
            }
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();

    loop {
        let mut changed = false;
        (0..newh.len()).for_each(|posi| {
            let nh = newh[posi];
            let oh = oldh[posi];
            if nh == oh {
                return;
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
        });
        if !changed {
            return newh;
        }
    }
}

/// Algorithm for finding and connecting lakes.  Assumes newh and downhill have already
/// been computed.  When a lake's value is negative, it is its own lake root, and when it is 0, it
/// is on the boundary of Ω.
///
/// Returns a 4-tuple containing:
/// - The first indirection vector (associating chunk indices with their lake's root node).
/// - A list of chunks on the boundary (non-lake egress points).
/// - The second indirection vector (associating chunk indices with their lake's adjacency list).
/// - The adjacency list (stored in a single vector), indexed by the second indirection vector.
pub fn get_lakes<F: Float>(
    h: impl Fn(usize) -> F,
    downhill: &mut [isize],
) -> (usize, Box<[i32]>, Box<[u32]>, F) {
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
    (&*downhill)
        .into_iter()
        .enumerate()
        .filter(|(_, &dh_idx)| dh_idx < 0)
        .for_each(|(chunk_idx, &dh)| {
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
                uphill(downhill, node as usize).for_each(|child| {
                    // lake_idx is the index of our lake root.
                    indirection[child] = chunk_idx as i32;
                    indirection_[child] = indirection_idx;
                    newh.push(child as u32);
                });
                cur += 1;
            }
            // Find the number of elements pushed.
            let length = (cur - start) * 8;
            // New lake root (lakes have indirection set to -length).
            indirection[chunk_idx] = -(length as i32);
            indirection_[chunk_idx] = indirection_idx;
        });
    assert_eq!(newh.len(), downhill.len());

    log::debug!("Old lake roots: {:?}", lake_roots.len());

    let newh = newh.into_boxed_slice();
    let mut maxh = -F::infinity();
    // Now, we know that the sum of all the indirection nodes will be the same as the number of
    // nodes.  We can allocate a *single* vector with 8 * nodes entries, to be used as storage
    // space, and augment our indirection vector with the starting index, resulting in a vector of
    // slices.  As we go, we replace each lake entry with its index in the new indirection buffer,
    // allowing
    newh.into_iter().for_each(|&chunk_idx_| {
        let chunk_idx = chunk_idx_ as usize;
        let lake_idx_ = indirection_[chunk_idx];
        let lake_idx = lake_idx_ as usize;
        let height = h(chunk_idx_ as usize);
        // While we're here, compute the max elevation difference from zero among all nodes.
        maxh = maxh.max(height.abs());
        // For every neighbor, check to see whether it is already set; if the neighbor is set,
        // its height is ≤ our height.  We should search through the edge list for the
        // neighbor's lake to see if there's an entry; if not, we insert, and otherwise we
        // get its height.  We do the same thing in our own lake's entry list.  If the maximum
        // of the heights we get out from this process is greater than the maximum of this
        // chunk and its neighbor chunk, we switch to this new edge.
        neighbors(chunk_idx).for_each(|neighbor_idx| {
            let neighbor_height = h(neighbor_idx);
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
                                let pass_height = h(neighbor_pass.1 as usize);
                                let neighbor_pass_height = h(pass.1 as usize);
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
        });
    });

    // Now it's time to calculate the lake connections graph T_L covering G_L.
    let mut candidates = BinaryHeap::with_capacity(indirection.len());
    // let mut pass_flows : Vec<i32> = vec![-1; indirection.len()];

    // We start by going through each pass, deleting the ones that point out of boundary nodes and
    // adding ones that point into boundary nodes from non-boundary nodes.
    lakes.iter_mut().for_each(|edge| {
        let edge: &mut (i32, u32) = edge;
        // Only consider valid elements.
        if edge.0 == -1 {
            return;
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
            return;
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
            let pass = h(from).max(h(to));
            candidates.push(Reverse((
                NotNan::new(pass).unwrap(),
                (edge.0 as u32, edge.1),
            )));
        }
    });

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
            let pass = h(edge.0 as usize).max(h(edge.1 as usize));
            // Put the reverse edge in candidates, sorted by height, then chunk idx, and finally
            // neighbor idx.
            candidates.push(Reverse((
                NotNan::new(pass).unwrap(),
                (edge.1, edge.0 as u32),
            )));
        }
    }
    log::debug!("Total lakes: {:?}", pass_flows_sorted.len());

    // Perform the bfs once again.
    #[derive(Clone, Copy, PartialEq)]
    enum Tag {
        UnParsed,
        InQueue,
        WithRcv,
    }
    let mut tag = vec![Tag::UnParsed; WORLD_SIZE.x * WORLD_SIZE.y];
    // TODO: Combine with adding to vector.
    let mut filling_queue = Vec::with_capacity(downhill.len());

    let mut newh = Vec::with_capacity(downhill.len());
    (&*boundary)
        .iter()
        .chain(pass_flows_sorted.iter())
        .for_each(|&chunk_idx| {
            // Find all the nodes uphill from this lake.  Since there is only one outgoing edge
            // in the "downhill" graph, this is guaranteed never to visit a node more than
            // once.
            let mut start = newh.len();
            // First, find the neighbor pass (assuming this is not the ocean).
            let neighbor_pass_idx = downhill[chunk_idx];
            let first_idx = if neighbor_pass_idx < 0 {
                // This is the ocean.
                newh.push(chunk_idx as u32);
                start += 1;
                chunk_idx
            } else {
                // This is a "real" lake.
                let neighbor_pass_idx = neighbor_pass_idx as usize;
                // Let's find our side of the pass.
                let pass_idx = -indirection[chunk_idx];
                // NOTE: Since only lakes are on the boundary, this should be a valid array index.
                assert!(pass_idx >= 0);
                let pass_idx = pass_idx as usize;
                // Now, we should recompute flow paths so downhill nodes are contiguous.

                /* // Carving strategy: reverse the path from the lake side of the pass to the
                // lake bottom, and also set the lake side of the pass's downhill to its
                // neighbor pass.
                let mut to_idx = neighbor_pass_idx;
                let mut from_idx = pass_idx;
                // NOTE: Since our side of the lake pass must be in the same basin as chunk_idx,
                // and chunk_idx is the basin bottom, we must reach it before we reach an ocean
                // node or other node with an invalid index.
                while from_idx != chunk_idx {
                    // Reverse this (from, to) edge by first replacing to_idx with from_idx,
                    // then replacing from_idx's downhill with the old to_idx, and finally
                    // replacing from_idx with from_idx's old downhill.
                    //
                    // println!("Reversing (lake={:?}): to={:?}, from={:?}, dh={:?}", chunk_idx, to_idx, from_idx, downhill[from_idx]);
                    from_idx = mem::replace(
                        &mut downhill[from_idx],
                        mem::replace(
                            &mut to_idx,
                            // NOTE: This cast should be valid since the node is either a path on the way
                            // to a lake bottom, or a lake bottom with an actual pass outwards.
                            from_idx
                        ) as isize,
                    ) as usize;
                }
                // Remember to set the actual lake's from_idx properly!
                downhill[from_idx] = to_idx as isize; */

                // TODO: Enqueue onto newh simultaneously with filling (this could help prevent
                // needing a special tag just for checking if we are already enqueued).
                // Filling strategy: nodes are assigned paths based on cost.
                {
                    assert!(tag[pass_idx] == Tag::UnParsed);

                    filling_queue.push(pass_idx);
                    tag[neighbor_pass_idx] = Tag::WithRcv;
                    tag[pass_idx] = Tag::InQueue;

                    let outflow_coords = uniform_idx_as_vec2(neighbor_pass_idx);
                    let elev = h(neighbor_pass_idx).max(h(pass_idx));

                    while let Some(node) = filling_queue.pop() {
                        let coords = uniform_idx_as_vec2(node);

                        let mut rcv = -1;
                        let mut rcv_cost = -f64::INFINITY; /*f64::EPSILON;*/
                        let outflow_distance = (outflow_coords - coords).map(|e| e as f64);

                        neighbors(node).for_each(|ineighbor| {
                            if indirection[ineighbor] != chunk_idx as i32
                                && ineighbor != chunk_idx
                                && ineighbor != neighbor_pass_idx
                                || h(ineighbor) > elev
                            {
                                return;
                            }
                            let dxy = (uniform_idx_as_vec2(ineighbor) - coords).map(|e| e as f64);
                            let neighbor_distance = /*neighbor_coef * */dxy;
                            let tag = &mut tag[ineighbor];
                            match *tag {
                                Tag::WithRcv => {
                                    // TODO: Remove outdated comment.
                                    //
                                    // vec_to_outflow ⋅ (vec_to_neighbor / |vec_to_neighbor|) = ||vec_to_outflow||cos Θ
                                    //   where θ is the angle between vec_to_outflow and vec_to_neighbor.
                                    //
                                    // Which is also the scalar component of vec_to_outflow in the
                                    // direction of vec_to_neighbor.
                                    let cost = outflow_distance
                                        .dot(neighbor_distance / neighbor_distance.magnitude());
                                    if cost > rcv_cost {
                                        rcv = ineighbor as isize;
                                        rcv_cost = cost;
                                    }
                                }
                                Tag::UnParsed => {
                                    filling_queue.push(ineighbor);
                                    *tag = Tag::InQueue;
                                }
                                _ => {}
                            }
                        });
                        assert!(rcv != -1);
                        downhill[node] = rcv;
                        tag[node] = Tag::WithRcv;
                    }
                }

                // Use our side of the pass as the initial node in the stack order.
                // TODO: Verify that this stack order will not "double reach" any lake chunks.
                neighbor_pass_idx
            };
            // New lake root
            let mut cur = start;
            let mut node = first_idx;
            loop {
                uphill(downhill, node as usize).for_each(|child| {
                    // lake_idx is the index of our lake root.
                    // Check to make sure child (flowing into us) is in the same lake.
                    if indirection[child] == chunk_idx as i32 || child == chunk_idx {
                        newh.push(child as u32);
                    }
                });

                if cur == newh.len() {
                    break;
                }
                node = newh[cur] as usize;
                cur += 1;
            }
        });
    assert_eq!(newh.len(), downhill.len());
    (boundary.len(), indirection, newh.into_boxed_slice(), maxh)
}

/// Iterate through set neighbors of multi-receiver flow.
pub fn mrec_downhill<'a>(
    mrec: &'a [u8],
    posi: usize,
) -> impl Clone + Iterator<Item = (usize, usize)> + 'a {
    let pos = uniform_idx_as_vec2(posi);
    let mrec_i = mrec[posi];
    NEIGHBOR_DELTA
        .iter()
        .enumerate()
        .filter(move |&(k, _)| (mrec_i >> k as isize) & 1 == 1)
        .map(move |(k, &(x, y))| {
            (
                k,
                vec2_as_uniform_idx(Vec2::new(pos.x + x as i32, pos.y + y as i32)),
            )
        })
}

/// Algorithm for computing multi-receiver flow.
///
/// * `h`: altitude
/// * `downhill`: single receiver
/// * `newh`: single receiver stack
/// * `wh`: buffer into which water height will be inserted.
/// * `nx`, `ny`: resolution in x and y directions.
/// * `dx`, `dy`: grid spacing in x- and y-directions
/// * `maxh`: maximum |height| among all nodes.
///
///
/// Updates the water height to a nearly planar surface, and returns a 3-tuple containing:
/// * A bitmask representing which neighbors are downhill.
/// * Stack order for multiple receivers (from top to bottom).
/// * The weight for each receiver, for each node.
pub fn get_multi_rec<F: fmt::Debug + Float + Sync + Into<Compute>>(
    h: impl Fn(usize) -> F + Sync,
    downhill: &[isize],
    newh: &[u32],
    wh: &mut [F],
    nx: usize,
    ny: usize,
    dx: Compute,
    dy: Compute,
    _maxh: F,
) -> (Box<[u8]>, Box<[u32]>, Box<[Computex8]>) {
    let nn = nx * ny;
    let dxdy = Vec2::new(dx, dy);

    /* // set bc
    let i1 = 0;
    let i2 = nx;
    let j1 = 0;
    let j2 = ny;
    let xcyclic = false;
    let ycyclic = false; */
    /*
      write (cbc,'(i4)') ibc
      i1=1
      i2=nx
      j1=1
      j2=ny
      if (cbc(4:4).eq.'1') i1=2
      if (cbc(2:2).eq.'1') i2=nx-1
      if (cbc(1:1).eq.'1') j1=2
      if (cbc(3:3).eq.'1') j2=ny-1
      xcyclic=.FALSE.
      ycyclic=.FALSE.
      if (cbc(4:4).ne.'1'.and.cbc(2:2).ne.'1') xcyclic=.TRUE.
      if (cbc(1:1).ne.'1'.and.cbc(3:3).ne.'1') ycyclic=.TRUE.
    */
    assert_eq!(nn, wh.len());

    // fill the local minima with a nearly planar surface
    // See https://matthew-brett.github.io/teaching/floating_error.html;
    // our absolute error is bounded by β^(e-(p-1)), where e is the exponent of the largest value we
    // care about.  In this case, since we are summing up to nn numbers, we are bounded from above
    // by nn * |maxh|; however, we only need to invoke this when we actually encounter a number, so
    // we compute it dynamically.
    // for nn + |maxh|
    // TODO: Consider that it's probably not possible to have a downhill path the size of the whole grid...
    // either measure explicitly (maybe in get_lakes) or work out a more precise upper bound (since
    // using nn * 2 * (maxh + epsilon) makes f32 not work very well).
    let deltah = F::epsilon() + F::epsilon();
    newh.into_iter().for_each(|&ijk| {
        let ijk = ijk as usize;
        let h_i = h(ijk);
        let ijr = downhill[ijk];
        wh[ijk] = if ijr >= 0 {
            let ijr = ijr as usize;
            let wh_j = wh[ijr];
            if wh_j >= h_i {
                let deltah = deltah * wh_j.abs();
                wh_j + deltah
            } else {
                h_i
            }
        } else {
            h_i
        };
    });

    let mut wrec = Vec::<Computex8>::with_capacity(nn);
    let mut mrec = Vec::with_capacity(nn);
    let mut don_vis = Vec::with_capacity(nn);

    // loop on all nodes
    (0..nn)
        .into_par_iter()
        .map(|ij| {
            // TODO: SIMDify?  Seems extremely amenable to that.
            let wh_ij = wh[ij];
            let mut mrec_ij = 0u8;
            let mut ndon_ij = 0u8;
            let neighbor_iter = |posi| {
                let pos = uniform_idx_as_vec2(posi);
                NEIGHBOR_DELTA
                    .iter()
                    .map(move |&(x, y)| Vec2::new(pos.x + x, pos.y + y))
                    .enumerate()
                    .filter(move |&(_, pos)| {
                        pos.x >= 0
                            && pos.y >= 0
                            && pos.x < WORLD_SIZE.x as i32
                            && pos.y < WORLD_SIZE.y as i32
                    })
                    .map(move |(k, pos)| (k, vec2_as_uniform_idx(pos)))
            };

            neighbor_iter(ij).for_each(|(k, ijk)| {
                let wh_ijk = wh[ijk];
                if wh_ij > wh_ijk {
                    // Set neighboring edge lower than this one as being downhill.
                    // NOTE: relying on at most 8 neighbors.
                    mrec_ij |= 1 << k;
                } else if wh_ijk > wh_ij {
                    // Avoiding ambiguous cases.
                    ndon_ij += 1;
                }
            });
            (mrec_ij, (ndon_ij, ndon_ij))
        })
        .unzip_into_vecs(&mut mrec, &mut don_vis);

    let czero = <Compute as Zero>::zero();
    let (wrec, stack) = rayon::join(
        || {
            (0..nn)
                .into_par_iter()
                .map(|ij| {
                    let mut sumweight = czero;
                    let mut wrec = [czero; 8];
                    let mut nrec = 0;
                    mrec_downhill(&mrec, ij).for_each(|(k, ijk)| {
                        let lrec_ijk = ((uniform_idx_as_vec2(ijk) - uniform_idx_as_vec2(ij))
                            .map(|e| e as Compute)
                            * dxdy)
                            .magnitude();
                        let wrec_ijk = (wh[ij] - wh[ijk]).into() / lrec_ijk;
                        // NOTE: To emulate single-direction flow, uncomment this line.
                        // let wrec_ijk = if ijk as isize == downhill[ij] { <Compute as One>::one() } else { <Compute as Zero>::zero() };
                        wrec[k] = wrec_ijk;
                        sumweight += wrec_ijk;
                        nrec += 1;
                    });
                    let slope = sumweight / (nrec as Compute).max(1.0);
                    let p_mfd_exp = 0.5 + 0.6 * slope;
                    sumweight = czero;
                    wrec.iter_mut().for_each(|wrec_k| {
                        let wrec_ijk = wrec_k.powf(p_mfd_exp);
                        sumweight += wrec_ijk;
                        *wrec_k = wrec_ijk;
                    });
                    if sumweight > czero {
                        wrec.iter_mut().for_each(|wrec_k| {
                            *wrec_k /= sumweight;
                        });
                    }
                    wrec
                })
                .collect_into_vec(&mut wrec);
            wrec
        },
        || {
            let mut stack = Vec::with_capacity(nn);
            let mut parse = Vec::with_capacity(nn);

            // we go through the nodes
            (0..nn).for_each(|ij| {
                let (ndon_ij, _) = don_vis[ij];
                // when we find a "summit" (ie a node that has no donors)
                // we parse it (put it in a stack called parse)
                if ndon_ij == 0 {
                    parse.push(ij);
                }
                // we go through the parsing stack
                while let Some(ijn) = parse.pop() {
                    // we add the node to the stack
                    stack.push(ijn as u32);
                    mrec_downhill(&mrec, ijn).for_each(|(_, ijr)| {
                        let (_, ref mut vis_ijr) = don_vis[ijr];
                        if *vis_ijr >= 1 {
                            *vis_ijr -= 1;
                            if *vis_ijr == 0 {
                                parse.push(ijr);
                            }
                        }
                    });
                }
            });

            assert_eq!(stack.len(), nn);
            stack
        },
    );

    (
        mrec.into_boxed_slice(),
        stack.into_boxed_slice(),
        wrec.into_boxed_slice(),
    )
}

/// Perform erosion n times.
pub fn do_erosion(
    _max_uplift: f32,
    n_steps: usize,
    seed: &RandomField,
    rock_strength_nz: &(impl NoiseFn<Point3<f64>> + Sync),
    oldh: impl Fn(usize) -> f32 + Sync,
    oldb: impl Fn(usize) -> f32 + Sync,
    is_ocean: impl Fn(usize) -> bool + Sync,
    uplift: impl Fn(usize) -> f64 + Sync,
    n: impl Fn(usize) -> f32 + Sync,
    theta: impl Fn(usize) -> f32 + Sync,
    kf: impl Fn(usize) -> f64 + Sync,
    kd: impl Fn(usize) -> f64 + Sync,
    g: impl Fn(usize) -> f32 + Sync,
    epsilon_0: impl Fn(usize) -> f32 + Sync,
    alpha: impl Fn(usize) -> f32 + Sync,
    // scaling factors
    height_scale: impl Fn(f32) -> Alt + Sync,
    k_d_scale: f64,
    k_da_scale: impl Fn(f64) -> f64,
) -> (Box<[Alt]>, Box<[Alt]> /*, Box<[Alt]>*/) {
    log::debug!("Initializing erosion arrays...");
    let oldh_ = (0..WORLD_SIZE.x * WORLD_SIZE.y)
        .into_par_iter()
        .map(|posi| oldh(posi) as Alt)
        .collect::<Vec<_>>()
        .into_boxed_slice();
    // Topographic basement (The height of bedrock, not including sediment).
    let mut b = (0..WORLD_SIZE.x * WORLD_SIZE.y)
        .into_par_iter()
        .map(|posi| oldb(posi) as Alt)
        .collect::<Vec<_>>()
        .into_boxed_slice();
    // Stream power law slope exponent--link between channel slope and erosion rate.
    let n = (0..WORLD_SIZE.x * WORLD_SIZE.y)
        .into_par_iter()
        .map(|posi| n(posi))
        .collect::<Vec<_>>()
        .into_boxed_slice();
    // Stream power law concavity index (θ = m/n), turned into an exponent on drainage
    // (which is a proxy for discharge according to Hack's Law).
    let m = (0..WORLD_SIZE.x * WORLD_SIZE.y)
        .into_par_iter()
        .map(|posi| theta(posi) * n[posi])
        .collect::<Vec<_>>()
        .into_boxed_slice();
    // Stream power law erodability constant for fluvial erosion (bedrock)
    let kf = (0..WORLD_SIZE.x * WORLD_SIZE.y)
        .into_par_iter()
        .map(|posi| kf(posi))
        .collect::<Vec<_>>()
        .into_boxed_slice();
    // Stream power law erodability constant for hillslope diffusion (bedrock)
    let kd = (0..WORLD_SIZE.x * WORLD_SIZE.y)
        .into_par_iter()
        .map(|posi| kd(posi))
        .collect::<Vec<_>>()
        .into_boxed_slice();
    // Deposition coefficient
    let g = (0..WORLD_SIZE.x * WORLD_SIZE.y)
        .into_par_iter()
        .map(|posi| g(posi))
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let epsilon_0 = (0..WORLD_SIZE.x * WORLD_SIZE.y)
        .into_par_iter()
        .map(|posi| epsilon_0(posi))
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let alpha = (0..WORLD_SIZE.x * WORLD_SIZE.y)
        .into_par_iter()
        .map(|posi| alpha(posi))
        .collect::<Vec<_>>()
        .into_boxed_slice();
    let mut wh = vec![0.0; WORLD_SIZE.x * WORLD_SIZE.y].into_boxed_slice();
    // TODO: Don't do this, maybe?
    // (To elaborate, maybe we should have varying uplift or compute it some other way).
    let uplift = (0..oldh_.len())
        .into_par_iter()
        .map(|posi| uplift(posi) as f32)
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
    let max_g = g
        .into_par_iter()
        .cloned()
        .max_by(|a, b| a.partial_cmp(&b).unwrap())
        .unwrap();
    log::debug!("Max uplift: {:?}", max_uplift);
    log::debug!("Max g: {:?}", max_g);
    // Height of terrain, including sediment.
    let mut h = oldh_;
    // Bedrock transport coefficients (diffusivity) in m^2 / year, for sediment.
    // For now, we set them all to be equal
    // on land, but in theory we probably want to at least differentiate between soil, bedrock, and
    // sediment.
    let kdsed = 1.5e-2 / 4.0;
    let kdsed = kdsed * k_d_scale;
    let n = |posi: usize| n[posi];
    let m = |posi: usize| m[posi];
    let kd = |posi: usize| kd[posi];
    let kf = |posi: usize| kf[posi];
    let g = |posi: usize| g[posi];
    let epsilon_0 = |posi: usize| epsilon_0[posi];
    let alpha = |posi: usize| alpha[posi];
    let height_scale = |n| height_scale(n);
    let k_da_scale = |q| k_da_scale(q);
    (0..n_steps).for_each(|i| {
        log::debug!("Erosion iteration #{:?}", i);
        erode(
            &mut h,
            &mut b,
            &mut wh,
            max_uplift,
            max_g,
            kdsed,
            seed,
            rock_strength_nz,
            |posi| uplift[posi],
            n,
            m,
            kf,
            kd,
            g,
            epsilon_0,
            alpha,
            |posi| is_ocean(posi),
            height_scale,
            k_da_scale,
        );
    });
    (h, b)
}
