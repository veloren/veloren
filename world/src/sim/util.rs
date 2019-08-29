use crate::{
    config::CONFIG,
    util::{RandomField, RandomPerm, Sampler},
};
use noise::{Point3, NoiseFn};
use std::{f32, mem, u32};
use super::WORLD_SIZE;
use common::{terrain::TerrainChunkSize, vol::VolSize};
use vek::*;

/// Calculates the smallest distance along an axis (x, y) from an edge of
/// the world.  This value is maximal at WORLD_SIZE / 2 and minimized at the extremes
/// (0 or WORLD_SIZE on one or more axes).  It then divides the quantity by cell_size,
/// so the final result is 1 when we are not in a cell along the edge of the world, and
/// ranges between 0 and 1 otherwise (lower when the chunk is closer to the edge).
pub fn map_edge_factor(posi: usize) -> f32 {
    uniform_idx_as_vec2(posi)
        .map2(WORLD_SIZE.map(|e| e as i32), |e, sz| {
            (sz / 2 - (e - sz / 2).abs()) as f32 / 16.0
        })
        .reduce_partial_min()
        .max(0.0)
        .min(1.0)
}

/// Computes the cumulative distribution function of the weighted sum of k independent,
/// uniformly distributed random variables between 0 and 1.  For each variable i, we use weights[i]
/// as the weight to give samples[i] (the weights should all be positive).
///
/// If the precondition is met, the distribution of the result of calling this function will be
/// uniformly distributed while preserving the same information that was in the original average.
///
/// For N > 33 the function will no longer return correct results since we will overflow u32.
///
/// NOTE:
///
/// Per [1], the problem of determing the CDF of
/// the sum of uniformly distributed random variables over *different* ranges is considerably more
/// complicated than it is for the same-range case.  Fortunately, it also provides a reference to
/// [2], which contains a complete derivation of an exact rule for the density function for
/// this case.  The CDF is just the integral of the cumulative distribution function [3],
/// which we use to convert this into a CDF formula.
///
/// This allows us to sum weighted, uniform, independent random variables.
///
/// At some point, we should probably contribute this back to stats-rs.
///
/// 1. https://www.r-bloggers.com/sums-of-random-variables/,
/// 2. Sadooghi-Alvandi, S., A. Nematollahi, & R. Habibi, 2009.
///    On the Distribution of the Sum of Independent Uniform Random Variables.
///    Statistical Papers, 50, 171-175.
/// 3. hhttps://en.wikipedia.org/wiki/Cumulative_distribution_function
pub fn cdf_irwin_hall<const N: usize>(weights: &[f32; N], samples: [f32; N]) -> f32 {
    // Let J_k = {(j_1, ... , j_k) : 1 ≤ j_1 < j_2 < ··· < j_k ≤ N }.
    //
    // Let A_N = Π{k = 1 to n}a_k.
    //
    // The density function for N ≥ 2 is:
    //
    //   1/(A_N * (N - 1)!) * (x^(N-1) + Σ{k = 1 to N}((-1)^k *
    //   Σ{(j_1, ..., j_k) ∈ J_k}(max(0, x - Σ{l = 1 to k}(a_(j_l)))^(N - 1))))
    //
    // So the cumulative distribution function is its integral, i.e. (I think)
    //
    // 1/(product{k in A}(k) * N!) * (x^N + sum(k in 1 to N)((-1)^k *
    // sum{j in Subsets[A, {k}]}(max(0, x - sum{l in j}(l))^N)))
    //
    // which is also equivalent to
    //
    //   (letting B_k = { a in Subsets[A, {k}] : sum {l in a} l }, B_(0,1) = 0 and
    //            H_k = { i : 1 ≤ 1 ≤ N! / (k! * (N - k)!) })
    //
    //   1/(product{k in A}(k) * N!) * sum(k in 0 to N)((-1)^k *
    //   sum{l in H_k}(max(0, x - B_(k,l))^N))
    //
    // We should be able to iterate through the whole power set
    // instead, and figure out K by calling count_ones(), so we can compute the result in O(2^N)
    // iterations.
    let x: f32 = weights
        .iter()
        .zip(samples.iter())
        .map(|(weight, sample)| weight * sample)
        .sum();

    let mut y = 0.0f32;
    for subset in 0u32..(1 << N) {
        // Number of set elements
        let k = subset.count_ones();
        // Add together exactly the set elements to get B_subset
        let z = weights
            .iter()
            .enumerate()
            .filter(|(i, _)| subset & (1 << i) as u32 != 0)
            .map(|(_, k)| k)
            .sum::<f32>();
        // Compute max(0, x - B_subset)^N
        let z = (x - z).max(0.0).powi(N as i32);
        // The parity of k determines whether the sum is negated.
        y += if k & 1 == 0 { z } else { -z };
    }

    // Divide by the product of the weights.
    y /= weights.iter().product::<f32>();

    // Remember to multiply by 1 / N! at the end.
    y / (1..=N as i32).product::<i32>() as f32
}

/// First component of each element of the vector is the computed CDF of the noise function at this
/// index (i.e. its position in a sorted list of value returned by the noise function applied to
/// every chunk in the game).  Second component is the cached value of the noise function that
/// generated the index.
///
/// NOTE: Length should always be WORLD_SIZE.x * WORLD_SIZE.y.
pub type InverseCdf = Box<[(f32, f32)]>;

/// Computes the position Vec2 of a SimChunk from an index, where the index was generated by
/// uniform_noise.
pub fn uniform_idx_as_vec2(idx: usize) -> Vec2<i32> {
    Vec2::new((idx % WORLD_SIZE.x) as i32, (idx / WORLD_SIZE.x) as i32)
}

/// Computes the index of a Vec2 of a SimChunk from a position, where the index is generated by
/// uniform_noise.  NOTE: Both components of idx should be in-bounds!
pub fn vec2_as_uniform_idx(idx: Vec2<i32>) -> usize {
    (idx.y as usize * WORLD_SIZE.x + idx.x as usize) as usize
}

/// Compute inverse cumulative distribution function for arbitrary function f, the hard way.  We
/// pre-generate noise values prior to worldgen, then sort them in order to determine the correct
/// position in the sorted order.  That lets us use `(index + 1) / (WORLDSIZE.y * WORLDSIZE.x)` as
/// a uniformly distributed (from almost-0 to 1) regularization of the chunks.  That is, if we
/// apply the computed "function" F⁻¹(x, y) to (x, y) and get out p, it means that approximately
/// (100 * p)% of chunks have a lower value for F⁻¹ than p.  The main purpose of doing this is to
/// make sure we are using the entire range we want, and to allow us to apply the numerous results
/// about distributions on uniform functions to the procedural noise we generate, which lets us
/// much more reliably control the *number* of features in the world while still letting us play
/// with the *shape* of those features, without having arbitrary cutoff points / discontinuities
/// (which tend to produce ugly-looking / unnatural terrain).
///
/// As a concrete example, before doing this it was very hard to tweak humidity so that either most
/// of the world wasn't dry, or most of it wasn't wet, by combining the billow noise function and
/// the computed altitude.  This is because the billow noise function has a very unusual
/// distribution that is heavily skewed towards 0.  By correcting for this tendency, we can start
/// with uniformly distributed billow noise and altitudes and combine them to get uniformly
/// distributed humidity, while still preserving the existing shapes that the billow noise and
/// altitude functions produce.
///
/// f takes an index, which represents the index corresponding to this chunk in any any SimChunk
/// vector returned by uniform_noise, and (for convenience) the float-translated version of those
/// coordinates.
/// f should return a value with no NaNs.  If there is a NaN, it will panic.  There are no other
/// conditions on f.  If f returns None, the value will be set to 0.0, and will be ignored for the
/// purposes of computing the uniform range.
///
/// Returns a vec of (f32, f32) pairs consisting of the percentage of chunks with a value lower than
/// this one, and the actual noise value (we don't need to cache it, but it makes ensuring that
/// subsequent code that needs the noise value actually uses the same one we were using here
/// easier).
pub fn uniform_noise(f: impl Fn(usize, Vec2<f64>) -> Option<f32>) -> InverseCdf {
    let mut noise = (0..WORLD_SIZE.x * WORLD_SIZE.y)
        .filter_map(|i| {
            (f(
                i,
                (uniform_idx_as_vec2(i) * TerrainChunkSize::SIZE.map(|e| e as i32))
                    .map(|e| e as f64),
            )
            .map(|res| (i, res)))
        })
        .collect::<Vec<_>>();

    // sort_unstable_by is equivalent to sort_by here since we include a unique index in the
    // comparison.  We could leave out the index, but this might make the order not
    // reproduce the same way between different versions of Rust (for example).
    noise.sort_unstable_by(|f, g| (f.1, f.0).partial_cmp(&(g.1, g.0)).unwrap());

    // Construct a vector that associates each chunk position with the 1-indexed
    // position of the noise in the sorted vector (divided by the vector length).
    // This guarantees a uniform distribution among the samples (excluding those that returned
    // None, which will remain at zero).
    let mut uniform_noise = vec![(0.0, 0.0); WORLD_SIZE.x * WORLD_SIZE.y].into_boxed_slice();
    let total = noise.len() as f32;
    for (noise_idx, (chunk_idx, noise_val)) in noise.into_iter().enumerate() {
        uniform_noise[chunk_idx] = ((1 + noise_idx) as f32 / total, noise_val);
    }
    uniform_noise
}

/// Iterate through all cells adjacent to a chunk.
fn neighbors(posi: usize) -> impl Clone + Iterator<Item=usize> {
    let pos = uniform_idx_as_vec2(posi);
    [(-1,-1), (0,-1), (1,-1), (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0)]
        .into_iter()
        .map(move |&(x, y)| Vec2::new(pos.x + x, pos.y + y))
        .filter(|pos| pos.x >= 0 && pos.y >= 0 &&
                      pos.x < WORLD_SIZE.x as i32 && pos.y < WORLD_SIZE.y as i32)
        .map(vec2_as_uniform_idx)
}

/// Compute the neighbor "most downhill" from all chunks.
pub fn downhill(h: &[f32]) -> Box<[isize]> {
    h.iter().enumerate().map(|(posi, &nh)| {
        if map_edge_factor(posi) == 0.0 || nh <= 0.0 {
            -2
        } else {
            let mut best = -1;
            let mut besth = nh;
            for nposi in neighbors(posi) {
                let nbh = h[nposi];
                if nbh < besth {
                    besth = nbh;
                    best = nposi as isize;
                }
            }
            best
        }
    }).collect::<Vec<_>>().into_boxed_slice()
}

/// Sort the chunk indices by (increasing) height.
pub fn height_sorted(h: &[f32]) -> Box<[usize]> {
    let mut newh = (0..h.len()).collect::<Vec<_>>().into_boxed_slice();

    // Sort by altitude.
    newh.sort_unstable_by(|&i, &j| h[i].partial_cmp(&h[j]).unwrap());
    newh
}

/// Compute the water flux at all chunks, given a list of chunk indices sorted by increasing
/// height.
pub fn get_flux(newh: &[usize], downhill: &[isize]) -> Box<[f32]> {
    /* let mut newh = h.iter().enumerate().collect::<Vec<_>>();

    // Sort by altitude
    newh.sort_unstable_by(|f, g| (f.1, f.0).partial_cmp(&(g.1, g.0)).unwrap()); */

    // FIXME: Make the below work.  For now, we just use constant flux.
    // Initially, flux is determined by rainfall.  We currently treat this as the same as humidity,
    // so we just use humidity as a proxy.  The total flux across the whole map is normalize to
    // 1.0, and we expect the average flux to be 0.5.  To figure out how far from normal any given
    // chunk is, we use its logit.
    let base_flux = 1.0 / ((WORLD_SIZE.x * WORLD_SIZE.y) as f32);
    let mut flux = vec![base_flux ; WORLD_SIZE.x * WORLD_SIZE.y].into_boxed_slice();
    for &chunk_idx in newh.into_iter().rev() {
        let downhill_idx = downhill[chunk_idx];
        if downhill_idx >= 0 {
            flux[downhill_idx as usize] += flux[chunk_idx];
        }
    }
    flux
    /* var dh = downhill(h);
    var idxs = [];
    var flux = zero(h.mesh);
    for (var i = 0; i < h.length; i++) {
        idxs[i] = i;
        flux[i] = 1/h.length;
    }
    idxs.sort(function (a, b) {
        return h[b] - h[a];
    });
    for (var i = 0; i < h.length; i++) {
        var j = idxs[i];
        if (dh[j] >= 0) {
            flux[dh[j]] += flux[j];
        }
    }
    return flux; */
}

/// trislope algorithm
fn tri_slope(h: &[f32], posi: usize, seed: &RandomField) -> Vec2<f32> {
    // Compute a random contiguous group of 3 adjacent vertices.
    let pos = uniform_idx_as_vec2(posi);
    let start = seed.get(Vec3::new(pos.x, pos.y, /*(h[posi] * CONFIG.mountain_scale) as i32*/0)) & 0x7;
    let mut neighbors = neighbors(posi).cycle().skip(start as usize);
    let nb0 = if let Some(n) = neighbors.next() { n } else { return Vec2::new(0.0, 0.0) };
    let nb1 = if let Some(n) = neighbors.next() { n } else { return Vec2::new(0.0, 0.0) };
    let nb2 = if let Some(n) = neighbors.next() { n } else { return Vec2::new(0.0, 0.0) };
    // Compute the approximate slope of this location from three points.
    /* let nb0 = if let Some(n) = neighbors.next() { n } else { return Vec2::new(0.0, 0.0) };
    let nb1 = if let Some(n) = neighbors.next() { n } else { return Vec2::new(0.0, 0.0) };
    let nb2 = if let Some(n) = neighbors.next() { n } else { return Vec2::new(0.0, 0.0) }; */


    /* let (nb0, nb1, nb2) = if map_edge_factor(posi) < 1.0 {
        let nb0 = if let Some(n) = neighbors.next() { n } else { return Vec2::new(0.0, 0.0) };
        (nb0, nb1, nb2)
    } else {
        let start = seed.get(Vec3::new(pos.x, pos.y, /*(h[posi] * CONFIG.mountain_scale) as i32*/0)) & 0x7;
        let end = (rand + 3) & 0x7;
        if end < start {

        }
        let mut nidx1;
        let mut nidx2;
        let mut nidx3;
        loop {
            nidx1 = (rand & 0x7);
            nidx2 = (rand & (0x7 << 3)) >> 3;
            nidx3 = (rand & (0x7 << 6)) >> 6;
            // If any of them match, permute and loop.
            if nidx1 == nidx2 || nidx2 == nidx3 || nidx1 == nidx3 {
                rand = RandomPerm::new(0).get(rand);
            } else {
                break;
            }
        }
        // Sort.
        if nidx2 < nidx1 {
            mem::swap(&mut nidx1, &mut nidx2);
        }
        if nidx3 < nidx1 {
            mem::swap(&mut nidx1, &mut nidx3);
        }
        if nidx3 < nidx2 {
            mem::swap(&mut nidx2, &mut nidx3);
        }
        let mut neighbors = neighbors.skip(nidx1 as usize);
        let nb0 = neighbors.next().unwrap();
        let mut neighbors = neighbors.skip(nidx2 as usize - 1 - nidx1 as usize);
        let nb1 = neighbors.next().unwrap();
        let mut neighbors = neighbors.skip(nidx3 as usize - 1 - nidx2 as usize);
        let nb2 = neighbors.next().unwrap();
        (nb0, nb1, nb2)
    }; */
    let mk_point = |n|
        (uniform_idx_as_vec2(n) * TerrainChunkSize::SIZE.map(|e| e as i32)).map(|e| e as f32);
    let p0 = mk_point(nb0);
    let p1 = mk_point(nb1);
    let p2 = mk_point(nb2);

    let x1 = p1.x - p0.x;
    let x2 = p2.x - p0.x;
    let y1 = p1.y - p0.y;
    let y2 = p2.y - p0.y;

    // x_new = x1 * x_old + x2 * y_old
    // y_new = y1 * x_old + y2 * y_old
    //
    // |det| = area of parallelogram from (0, 0), (x1, y1), (x2, y2), (x1+x2, y1+y2).
    // det = *oriented* area (negative when angle between first and second vector defining the
    // parallelogram turns in a clockwise direction).

    let det = x1 * y2 - y1 * x2;
    let h1 = (h[nb1] - h[nb0]) * CONFIG.mountain_scale;
    let h2 = (h[nb2] - h[nb0]) * CONFIG.mountain_scale;

    Vec2::new((y2 * h1 - y1 * h2) / det, (-x2 * h1 + x1 * h2) / det)
    /* var nbs = neighbours(h.mesh, i);
    if (nbs.length != 3) return [0,0];
    var p0 = h.mesh.vxs[nbs[0]];
    var p1 = h.mesh.vxs[nbs[1]];
    var p2 = h.mesh.vxs[nbs[2]];

    var x1 = p1[0] - p0[0];
    var x2 = p2[0] - p0[0];
    var y1 = p1[1] - p0[1];
    var y2 = p2[1] - p0[1];

    var det = x1 * y2 - x2 * y1;
    var h1 = h[nbs[1]] - h[nbs[0]];
    var h2 = h[nbs[2]] - h[nbs[0]];

    return [(y2 * h1 - y1 * h2) / det,
            (-x2 * h1 + x1 * h2) / det]; */
}

/* /// Compute the slope at all chunks.
fn get_slope(h: &[f32], newh: &[usize], downhill: &[isize], seed: &RandomField) -> Box<[f32]> {
    h.iter().enumerate().map(|(posi, &nh)| {
        // let s = tri_slope(h, posi, seed);
        if downhill[posi] < 0 {
            0.0
        } else {
            let zdist = (nh - h[downhill[posi] as usize]) * CONFIG.mountain_scale;
            let dist = Vec2::new(TerrainChunkSize::SIZE.x as f32, TerrainChunkSize::SIZE.y as f32);
            // let dist = Vec3::new(TerrainChunkSize::SIZE.x as f32, TerrainChunkSize::SIZE.y as f32, zdist);
            zdist / dist.magnitude()
        }
        // s.magnitude()
        // FIXME: make this work properly.
    }).collect::<Vec<_>>().into_boxed_slice()
    /* var dh = downhill(h);
    var slope = zero(h.mesh);
    for (var i = 0; i < h.length; i++) {
        var s = trislope(h, i);
        slope[i] = Math.sqrt(s[0] * s[0] + s[1] * s[1]);
        continue;
        if (dh[i] < 0) {
            slope[i] = 0;
        } else {
            slope[i] = (h[i] - h[dh[i]]) / distance(h.mesh, i, dh[i]);
        }
    }
    return slope; */
} */

// dh(p) / dt = u(p)−kA(p)^m * s(p)^n
//
// Problem with treating as a grid:
//
//
//  a  b
//
//  Angle is such that center of a and center of b are at height x... so a forms a parallelogram,
//  as does b, with the edges touching.
//
//  Problem: suppose a, b, and c aren't colinnear, and a, b, and c are touching?
//
//
//  a b c
//
//  They need to "average out" to something.  Impossible to tesselate in general!
//
//  Lines between points works, but then the "area" isn't being computed correctly.
//
//  1024*1024/8 = 131,072
//    _
//  / /
// /_/
//
// s(p) = ∇h(p).

/// Compute the maximum slope at a point.
fn get_max_slope(posi: usize, z: f32, rock_strength_nz: &impl NoiseFn<Point3<f64>>) -> f32 {
    const MIN_MAX_ANGLE : f32 = 6.0 / 360.0 * 2.0 * f32::consts::PI;
    const MAX_MAX_ANGLE : f32 = 54.0 / 360.0 * 2.0 * f32::consts::PI;
    const MAX_ANGLE_RANGE : f32 = MAX_MAX_ANGLE - MIN_MAX_ANGLE;
    let wposf = (uniform_idx_as_vec2(posi) * TerrainChunkSize::SIZE.map(|e| e as i32))
        .map(|e| e as f64);
    // let wposf = uniform_idx_as_vec2(posi).map(|e| e as f64) / WORLD_SIZE.map(|e| e as f64);
    let wposz = (z * CONFIG.mountain_scale) as f64;
    // let wposz = h[posi] as f64;
    // Normalized to be between 6 and and 54 degrees.
    let rock_strength = (rock_strength_nz.get([wposf.x, wposf.y, wposz]) * 0.5 + 0.5)
        .min(1.0)
        .max(0.0) as f32;
    /* if rock_strength < 0.0 || rock_strength > 1.0 {
        println!("Huh strength?: {:?}", rock_strength);
    } */
    let max_slope = (rock_strength * MAX_ANGLE_RANGE + MIN_MAX_ANGLE).tan();
    /* if max_slope > 1.48 || max_slope < 0.0 {
        println!("Huh? {:?}", max_slope);
    } */
    max_slope
}

/* /// Compute erosion rates for all chunks.
fn erosion_rate(k: f32, h: &[f32], downhill: &[isize], seed: &RandomField,
                rock_strength_nz: &impl NoiseFn<Point3<f64>>,
                uplift: impl Fn(usize) -> f32) -> Box<[f32]> {
    let newh = height_sorted(h);
    let area = get_flux(&newh, downhill);
    // let slope = get_slope(h, newh, downhill, seed);
    assert!(h.len() == downhill.len() &&
            downhill.len() == /*flux*/area.len()/* &&
            flux.len() == slope.len()*/);
    // max angle of slope depends on rock strength, which is computed from noise function.
    // let max_slope = f32::consts::FRAC_PI_6.tan();
    // // Would normally multiply by 2PI, but we already need to multiply by 0.5 when we compute
    // // max_slope from a number in range [-1, 1], so these cancel out.
    // tan((56-6)/360*0*pi+6/360*pi)
    // tan((56-6)/360*0*2*pi+6/360*2*pi)
    /* let min_max_angle = 6.0 / 360.0 * 2.0 * f32::consts::PI;
    let max_max_angle = 54.0 / 360.0 * 2.0 * f32::consts::PI;
    let max_angle_range = max_max_angle - min_max_angle; */
    const NEIGHBOR_DISTANCE : f32 = TerrainChunkSize::SIZE.map(|e| e as f32).magnitude();
    let mut rate = vec![0.0; h.len()].into_boxed_slice();
    // Iterate in ascending height order.
    for &posi in &*newh {
        let posj = downhill[posi];
        rate[posi] = if posj < 0 {
            0.0 // Egress with no outgoing flows.
        } else {
            let posj = posj as usize;
            let dist = Vec2::new(TerrainChunkSize::SIZE.x as f32, TerrainChunkSize::SIZE.y as f32);
            // let dist = Vec3::new(TerrainChunkSize::SIZE.x as f32, TerrainChunkSize::SIZE.y as f32, zdist);
            // zdist / dist.magnitude()

            // Has an outgoing flow edge (posi, posj).
            // flux(i) = k * A[i]^m / ((p(i) - p(j)).magnitude()), and δt = 1
            let flux = k * area[posi].sqrt() / NEIGHBOR_DISTANCE;
            // h[i](t + dt) = h[i](t) + δt * (uplift[i] + flux(i) * h[j](t + δt)) / (1 + flux(i) * δt).
            // NOTE: posj has already been computed since it's downhill from us.
            let h_j = h[posj] + rate[posj];
            let dh_i = (uplift(posi) + flux * h_j) / (1 + flux);
            // Test the slope.
            let old_h_i = h[posi];
            let new_h_i = old_h_i + dh_i;
            let dz = (new_h_i - h_j) * CONFIG.mountain_scale;
            let max_slope = get_max_slope(posi, old_h_i, rock_strength_nz);
            if dz / NEIGHBOR_DISTANCE > max_slope {
                // Thermal erosion says this can't happen, so we reduce dh_i to make the slope
                // exactly max_slope.
                // max_slope = (old_h_i + dh - h_j) / NEIGHBOR_DISTANCE
                // dh = max_slope * NEIGHBOR_DISTANCE + h_j - old_h_i.
                max_slope * NEIGHBOR_DISTANCE + h_j - old_h_i
            } else {
                // Just use the computed rate.
                dh_i
            }
        };
    }
    rate
    /* for (posi, (flux, &slope)) in flux.iter().zip(slope.iter()).enumerate() {
        let max_slope = get_max_slope(posi, h[posi], rock_strenth_nz);
        //
    }
    flux.iter().zip(slope.iter()).enumerate().map(|(posi, (flux, &slope))| {
        let max_slope = get_max_slope(posi, h[posi], rock_strenth_nz);
        // Note: slope is already guaranteed positive, I think?
        let slope = slope.min(max_slope);
        // height / hmax = tectonic uplift at i.
        // let u = height / hmax;
        // dh(p) / dt = u(p)−kA(p)^m * s(p)^n
        let river = /*k * */flux.sqrt() * slope;
        river
        // let river = flux.sqrt() * slope;
        //let creep = slope * slope;
        // 1 = 2.244u / k
        // k = 2.244u / 1 = 2.244
        //
        // k = 2.244*5.010e-4/2000 ~ 5.6110e-7
        //
        // k = 2.244*amount
        //
        // (5.010e-4 = max tectonic uplift in meters / year).
        //
        // k =
        //
        // 2.5e5
        //river
        //    /0−kA(p)^m * s(p)^n/
        //(1000.0 * river/* + creep*/).min(200.0)
    }).collect::<Vec<_>>().into_boxed_slice() */
} */

/// Erode all chunks by amount.
///
/// Our equation is:
///
///   dh(p) / dt = uplift(p)−k * A(p)^m * slope(p)^n
///
///   where A(p) is the drainage area at p, m and n are constants
///   (we choose m = 0.5 and n = 1), and k is a constant.  We choose
///
///     k = 2.244 * uplift.max() / (desired_max_height)
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
fn erode(h: &mut [f32], erosion_base: f32, max_uplift: f32, seed: &RandomField,
         rock_strength_nz: &impl NoiseFn<Point3<f64>>,
         uplift: impl Fn(usize) -> f32) {
    let dh = downhill(h);
    /* // 1. Sort nodes in h by height.
    let mut newh = h.iter().enumerate().collect::<Vec<_>>();
    newh.sort_unstable_by(|f, g| (f.1, f.0).partial_cmp(&(g.1, g.0)).unwrap());
    // 2. Iterate through in reverse and compute drainage area. */
    let mmaxh = 1.0;
    let k = erosion_base + 2.244 / mmaxh * max_uplift;
    let maxh = *h.iter().max_by( |a, b| a.partial_cmp(&b).unwrap()).unwrap();
    println!("Eroding... (max height: {:?})", maxh);
    /* let er = erosion_rate(k, h, &dh, seed, rock_strength_nz, uplift);
    let maxh = *h.iter().max_by( |a, b| a.partial_cmp(&b).unwrap()).unwrap();
    // let maxr = er.iter().max_by( |a, b| a.partial_cmp(&b).unwrap()).unwrap();
    println!("Eroding... (max height: {:?})", maxh);
    // println!("Erosion rate: {:?}", maxr);
    // 1.0 = max mountain height
    // k = 2.244 * (max uplift/dt) / (max height)
    assert!(h.len() == er.len());
    for (posi, (nh, er)) in h.iter_mut().zip(er.iter()).enumerate() {
        /* // height / hmax * max uplift/dt = tectonic uplift at i (it really should be a known
        // constant, I think).  Also this is a very imprecise solution.
        // Actually we let tectonic uplfit be the uplift value at this point.
        // let pos = uniform_idx_as_vec2(posi);
        let u = uplift(posi);
        // let uplift_base = (seed.get(Vec3::new(pos.x, pos.y, 0)) as f64 / u32::MAX as f64) as f32;
        // let u = uplift_base * amount;
        // let u = (*nh / maxh) * amount;
        // dh(p) / dt = u(p)−kA(p)^m * s(p)^n
        *nh = (*nh + u - k * er)/*.max(0.0)*/;//.min(1.0);// * /*(er / maxr)*/er; */
        *nh += *er;
    }
    /* for (var i = 0; i < h.length; i++) {
        newh[i] = h[i] - amount * (er[i] / maxr);
    } */ */
    let newh = height_sorted(h);
    let area = get_flux(&newh, &dh);
    // let slope = get_slope(h, newh, downhill, seed);
    assert!(h.len() == dh.len() &&
            dh.len() == /*flux*/area.len()/* &&
            flux.len() == slope.len()*/);
    // max angle of slope depends on rock strength, which is computed from noise function.
    // let max_slope = f32::consts::FRAC_PI_6.tan();
    // // Would normally multiply by 2PI, but we already need to multiply by 0.5 when we compute
    // // max_slope from a number in range [-1, 1], so these cancel out.
    // tan((56-6)/360*0*pi+6/360*pi)
    // tan((56-6)/360*0*2*pi+6/360*2*pi)
    /* let min_max_angle = 6.0 / 360.0 * 2.0 * f32::consts::PI;
    let max_max_angle = 54.0 / 360.0 * 2.0 * f32::consts::PI;
    let max_angle_range = max_max_angle - min_max_angle; */
    let neighbor_distance = TerrainChunkSize::SIZE.map(|e| e as f32).magnitude();
    // let mut rate = vec![0.0; h.len()].into_boxed_slice();
    // Iterate in ascending height order.
    for &posi in &*newh {
        let posj = dh[posi];
        if posj < 0 {
            // Egress with no outgoing flows.
            // println!("Shouldn't happen often: {:?}", uniform_idx_as_vec2(posi));
            // 0.0 // Egress with no outgoing flows.
        } else {
            let posj = posj as usize;
            let dist = Vec2::new(TerrainChunkSize::SIZE.x as f32, TerrainChunkSize::SIZE.y as f32);
            // let dist = Vec3::new(TerrainChunkSize::SIZE.x as f32, TerrainChunkSize::SIZE.y as f32, zdist);
            // zdist / dist.magnitude()

            // Has an outgoing flow edge (posi, posj).
            // flux(i) = k * A[i]^m / ((p(i) - p(j)).magnitude()), and δt = 1
            let flux = k * area[posi].sqrt() / neighbor_distance;
            // h[i](t + dt) = (h[i](t) + δt * (uplift[i] + flux(i) * h[j](t + δt))) / (1 + flux(i) * δt).
            // NOTE: posj has already been computed since it's downhill from us.
            let h_j = h[posj];
            let old_h_i = h[posi];
            let new_h_i = (old_h_i + (uplift(posi) + flux * h_j)) / (1.0 + flux);
            // Test the slope.
            let dz = (new_h_i - h_j) * CONFIG.mountain_scale;
            /* if dz < 0.0 {
                println!("Huh?: {:?}", dz);
            } */
            let max_slope = get_max_slope(posi, new_h_i, rock_strength_nz);
            /*rate[posi] =*/
            h[posi] = if dz.abs() / neighbor_distance > max_slope {
                // println!("{:?}", max_slope);
                // Thermal erosion says this can't happen, so we reduce dh_i to make the slope
                // exactly max_slope.
                // max_slope = (old_h_i + dh - h_j) * CONFIG.mountain_scale / NEIGHBOR_DISTANCE
                // dh = max_slope * NEIGHBOR_DISTANCE / CONFIG.mountain_scale + h_j - old_h_i.
                dz.signum() * max_slope * neighbor_distance / CONFIG.mountain_scale + h_j/* - old_h_i*/
            } else {
                // Just use the computed rate.
                new_h_i
            }
        }
    }
}

/// The Planchon-Darboux algorithm for extracting drainage networks.
///
/// http://horizon.documentation.ird.fr/exl-doc/pleins_textes/pleins_textes_7/sous_copyright/010031925.pdf
///
/// See https://github.com/mewo2/terrain/blob/master/terrain.js
pub fn fill_sinks(h: Box<[f32]>/*, epsilon: f64*/) -> Box<[f32]> {
    //let epsilon = 1e-5f32;
    let epsilon = 1e-7f32 / CONFIG.mountain_scale;
    let infinity = f32::INFINITY;
    let mut newh = h.iter().enumerate().map(|(posi, &h)| {
        let is_near_edge = map_edge_factor(posi) < 1.0 || h < 5.0 / CONFIG.mountain_scale;
        if is_near_edge {
            h
        } else {
            infinity
        }
    }).collect::<Vec<_>>().into_boxed_slice();
    /* let newh = vec![0.0; WORLD_SIZE.x * WORLD_SIZE.y].into_boxed_slice();
    assert!(newh.len() == h.len());
    for (posi, (newh, (_, h))) in newh.iter_mut().zip(h).enumerate() {
        let is_near_edge = map_edge_factor(posi) < 1.0;
        if is_near_edge {
            *newh = h;
        } else {
            *newh = infinity;
        }
    } */

    loop {
        let mut changed = false;
        for posi in (0..newh.len()) {
            let nh = newh[posi];
            let oh = h[posi];
            if nh == oh {
                continue;
            }
            for nposi in neighbors(posi) {
                let onbh = newh[nposi];
                let nbh = onbh + epsilon;
                if oh >= nbh {
                    newh[posi] = oh;
                    changed = true;
                    break;
                }
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
    /* let newh = zero(h.mesh);
    for (var i = 0; i < h.length; i++) {
        if (isnearedge(h.mesh, i)) {
            newh[i] = h[i];
        } else {
            newh[i] = infinity;
        }
    }
    while (true) {
        var changed = false;
        for (var i = 0; i < h.length; i++) {
            if (newh[i] == h[i]) continue;
            var nbs = neighbours(h.mesh, i);
            for (var j = 0; j < nbs.length; j++) {
                if (h[i] >= newh[nbs[j]] + epsilon) {
                    newh[i] = h[i];
                    changed = true;
                    break;
                }
                var oh = newh[nbs[j]] + epsilon;
                if ((newh[i] > oh) && (oh > h[i])) {
                    newh[i] = oh;
                    changed = true;
                }
            }
        }
        if (!changed) return newh;
    } */
}

/// Perform erosion n times.
pub fn do_erosion(h: &InverseCdf/*, epsilon: f64*/, erosion_base: f32, /*amount: f32, */n: usize,
                  seed: &RandomField, rock_strength_nz: &impl NoiseFn<Point3<f64>>,
                  uplift: impl Fn(usize) -> f32) -> Box<[f32]> {
    let max_uplift = (0..h.len())
        .map( |posi| uplift(posi))
        .max_by( |a, b| a.partial_cmp(&b).unwrap()).unwrap();
    let mut h = fill_sinks(h.iter().map(|&(_, h)| h).collect::<Vec<_>>().into_boxed_slice());
    for i in 0..n {
        erode(&mut h, /*amount*/erosion_base, max_uplift, seed, rock_strength_nz, |posi| uplift(posi));
        h = fill_sinks(h);
    }
    h
}
