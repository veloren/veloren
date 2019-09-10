use crate::{
    config::CONFIG,
    util::RandomField,
};
use noise::{Point3, NoiseFn};
use ordered_float::NotNan;
use rayon::prelude::*;
use std::{
    cmp::Reverse,
    collections::BinaryHeap,
    f32,
    mem,
    u32,
};
use super::WORLD_SIZE;
use common::{terrain::TerrainChunkSize, vol::RectVolSize};
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
pub fn uniform_noise(f: impl Fn(usize, Vec2<f64>) -> Option<f32> + Sync) -> InverseCdf {
    let mut noise = (0..WORLD_SIZE.x * WORLD_SIZE.y)
        .into_par_iter()
        .filter_map(|i| {
            (f(
                i,
                (uniform_idx_as_vec2(i) * TerrainChunkSize::RECT_SIZE.map(|e| e as i32))
                    .map(|e| e as f64),
            )
            .map(|res| (i, res)))
        })
        .collect::<Vec<_>>();

    // sort_unstable_by is equivalent to sort_by here since we include a unique index in the
    // comparison.  We could leave out the index, but this might make the order not
    // reproduce the same way between different versions of Rust (for example).
    noise.par_sort_unstable_by(|f, g| (f.1, f.0).partial_cmp(&(g.1, g.0)).unwrap());

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
pub fn neighbors(posi: usize) -> impl Clone + Iterator<Item=usize> {
    let pos = uniform_idx_as_vec2(posi);
    // NOTE: want to keep this such that the chunk index is in ascending order!
    [(-1,-1), (0,-1), (1,-1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)]
        .into_iter()
        .map(move |&(x, y)| Vec2::new(pos.x + x, pos.y + y))
        .filter(|pos| pos.x >= 0 && pos.y >= 0 &&
                      pos.x < WORLD_SIZE.x as i32 && pos.y < WORLD_SIZE.y as i32)
        .map(vec2_as_uniform_idx)
}

// Note that we should already have okay cache locality since we have a grid.
pub fn uphill<'a>(dh: &'a [isize], posi: usize) -> impl Clone + Iterator<Item=usize> + 'a {
    neighbors(posi).filter(move |&posj| dh[posj] == posi as isize)
}

/// Compute the neighbor "most downhill" from all chunks.
///
/// TODO: See if allocating in advance is worthwhile.
pub fn downhill(h: &[f32], oh: impl Fn(usize) -> f32 + Sync) -> Box<[isize]> {
    // Constructs not only the list of downhill nodes, but also computes an ordering (visiting
    // nodes in order from roots to leaves).
    h.par_iter().enumerate().map(|(posi, &nh)| {
        if map_edge_factor(posi) == 0.0 || oh(posi) <= 0.0 {
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

/* /// Construct an initial list of chunk indices.
pub fn alt_positions() -> Box<[u32]> {
    (0..(WORLD_SIZE.x * WORLD_SIZE.y) as u32).collect::<Vec<_>>().into_boxed_slice()
}

/// Sort the chunk indices by (increasing) height.
pub fn sort_by_height(h: &[f32], newh: &mut [u32]) {
    // We trade off worse cache locality (not keeping the key with the height) for hopefully much
    // faster sorts after the first time (since we expect height orders to be mostly unchanged
    // after the first iteration or two).
    newh.par_sort_unstable_by(|&i, &j| h[i as usize].partial_cmp(&h[j as usize]).unwrap());
    /* let mut newh = (0..h.len()).collect::<Vec<_>>().into_boxed_slice();

    // Sort by altitude.
    newh.sort_unstable_by(|&i, &j| h[i].partial_cmp(&h[j]).unwrap());
    newh */
} */

/// Compute the water flux at all chunks, given a list of chunk indices sorted by increasing
/// height.
pub fn get_flux(newh: &[u32], downhill: &[isize], _boundary_len: usize) -> Box<[f32]> {
    /* let mut newh = h.iter().enumerate().collect::<Vec<_>>();

    // Sort by altitude
    newh.sort_unstable_by(|f, g| (f.1, f.0).partial_cmp(&(g.1, g.0)).unwrap()); */

    // FIXME: Make the below work.  For now, we just use constant flux.
    // Initially, flux is determined by rainfall.  We currently treat this as the same as humidity,
    // so we just use humidity as a proxy.  The total flux across the whole map is normalize to
    // 1.0, and we expect the average flux to be 0.5.  To figure out how far from normal any given
    // chunk is, we use its logit.
    // NOTE: If there are no non-boundary chunks, we just set base_flux to 1.0; this should still
    // work fine because in that case there's no erosion anyway.
    let base_flux = 1.0 / ((WORLD_SIZE.x * WORLD_SIZE.y) as f32);
    // let base_flux = 1.0 / ((WORLD_SIZE.x * WORLD_SIZE.y - boundary_len).max(1) as f32);
    let mut flux = vec![base_flux ; WORLD_SIZE.x * WORLD_SIZE.y].into_boxed_slice();
    for &chunk_idx in newh.into_iter().rev() {
        let chunk_idx = chunk_idx as usize;
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

/* /// trislope algorithm
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
        (uniform_idx_as_vec2(n) * TerrainChunkSize::RECT_SIZE.map(|e| e as i32)).map(|e| e as f32);
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
} */

/* /// Compute the slope at all chunks.
fn get_slope(h: &[f32], newh: &[usize], downhill: &[isize], seed: &RandomField) -> Box<[f32]> {
    h.iter().enumerate().map(|(posi, &nh)| {
        // let s = tri_slope(h, posi, seed);
        if downhill[posi] < 0 {
            0.0
        } else {
            let zdist = (nh - h[downhill[posi] as usize]) * CONFIG.mountain_scale;
            let dist = Vec2::new(TerrainChunkSize::RECT_SIZE.x as f32, TerrainChunkSize::RECT_SIZE.y as f32);
            // let dist = Vec3::new(TerrainChunkSize::RECT_SIZE.x as f32, TerrainChunkSize::RECT_SIZE.y as f32, zdist);
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

/// Precompute the maximum slope at all points.
///
/// TODO: See if allocating in advance is worthwhile.
fn get_max_slope(h: &[f32], rock_strength_nz: &(impl NoiseFn<Point3<f64>> + Sync)) -> Box<[f32]> {
    const MIN_MAX_ANGLE : f32 = 6.0 / 360.0 * 2.0 * f32::consts::PI;
    const MAX_MAX_ANGLE : f32 = 54.0 / 360.0 * 2.0 * f32::consts::PI;
    const MAX_ANGLE_RANGE : f32 = MAX_MAX_ANGLE - MIN_MAX_ANGLE;
    h.par_iter().enumerate().map(|(posi, &z)| {
        let wposf = (uniform_idx_as_vec2(posi) * TerrainChunkSize::RECT_SIZE.map(|e| e as i32))
            .map(|e| e as f64);
        // let wposf = uniform_idx_as_vec2(posi)
        //     .map(|e| e as f64) / WORLD_SIZE.map(|e| e as f64);
        let wposz = (z * CONFIG.mountain_scale) as f64;
        // let wposz = h[posi] as f64;
        // Normalized to be between 6 and and 54 degrees.
        let rock_strength = (rock_strength_nz.get([wposf.x, wposf.y, wposz]) * 0.5 + 0.5)
            .min(1.0)
            .max(0.0) as f32;
        /* if rock_strength < 0.0 || rock_strength > 1.0 {
            println!("Huh strength?: {:?}", rock_strength);
        } */
        // Powering rock_strength^((1.25 - z)^6) means the maximum angle increases with z, but
        // not too fast.  At z = 0.25 the angle is not affected at all, below it the angle is
        // lower, and above it the angle is higher.
        //
        // Normal distribution:
        //
        // f(x | μ, σ²) = 1 / √(2π * σ²) * e^(-(x - μ)^2 / (2σ²))
        //
        // (Probability density)
        //
        // e^((z - 0.25)^2 / ())
        //
        // 0.5^((1.25-0.25)^2)
        //
        // 1/(sqrt(2*pi*0.1^2)) * e^(-(1.25-0.25)^2/(2*0.1^2))
        //
        // 1/(sqrt(2*pi*0.1^2)) * e^(-(0.25-0.25)^2/(2*0.1^2))
        //
        // 1/(sqrt(2*pi*0.1^2)) * e^(-(0.25-0.25)^2/(2*0.1^2))
        //
        // 1/(sqrt(2*pi*0.1^2)) * e^(-(0.25-0.25)^2/(2*0.1^2))
        //
        // 0.5^((1.25 - 0.5) * (1.25 - 0.5).abs() / (2 * 0.1^2))
        //
        // (0.5^((1.25 - 0.25) * sqrt((1.25 - 0.25)^2) / (2 * 0.1^2))) *(56-6)+6
        //
        // (0.5^((1.25 - 0.25) * sqrt((1.25 - 0.25)^2)) *(56-6)+6
        //
        // (0.5^((1.25 - 0.25)^2) *(56-6)+6)
        //
        // 1/(sqrt(2*pi*(-0.1)^2)) * e^(-(1.0 - 0.25) * sqrt((1.0 - 0.25)^2) / (2 * (-0.1)^2))
        //
        // 1/(sqrt(2*pi*0.1^2)) * e^(-(1.25-0.25))
        //
        // (0.5^((1.25 - 0.25)^6)) *(56-6)+6
        //
        // (0.5^((1.25 - 0.25)*4)) *(56-6)+6
        //
        // ln((0.9 / (1 - 0.9))/(0.25 / (1 - 0.25)))
        //
        // ln((0.5 / (1 - 0.5))/(0.25 / (1 - 0.25)))
        //
        // ln((0.5 / (1 - 0.5))/(0.25 / (1 - 0.25)))
        //
        // ln((0.01 / (1 - 0.01))/(0.25 / (1 - 0.25)))
        //
        // ln(0.5/(1-0.5))+0.25*ln((0.01 / (1 - 0.01))/(0.25 / (1 - 0.25)))
        //
        //
        // 0.25*ln((0.05 / (1 - 0.05))/(0.25 / (1 - 0.25)))
        //
        // 0.5 + 0.5 * tanh(ln(1 / (1 - 0.05) - 1) / (2 * (sqrt(3)/pi)))
        // 0.5 + 0.5 * tanh((1 * (ln(1 / (1 - 0.5) - 1)) + 1 * (ln(1 / (1 - 0.05) - 1) - ln(1 / (1 - 0.25) - 1))) / (2 * (sqrt(3)/pi)))
        //
        // (0.5 + 0.5 * tanh((1 * (ln(1 / (1 - 0.5) - 1)) + 1 * (ln(1 / (1 - 0.05) - 1) - ln(1 / (1 - 0.25) - 1))) / (2 * (sqrt(3)/pi)))) * (56 - 6) + 6
        //
        // (0.5 + 0.5 * tanh((0.5 * (ln(1 / (1 - 0.9) - 1)) + 1 * (ln(1 / (1 - 0.05) - 1) - ln(1 / (1 - 0.25) - 1))) / (2 * (sqrt(3)/pi)))) * (56 - 6) + 6
        //
        // (0.5 + 0.5 * tanh((0.5 * (ln(1 / (1 - 0.9) - 1)) + 1 * (ln(1 / (1 - 0.45) - 1) - ln(1 / (1 - 0.25) - 1))) / (2 * (sqrt(3)/pi)))) * (56 - 6) + 6
        //
        // .2*3+.25 = 0.85
        //
        // (0.5 + 0.5 * tanh((0.5 * (ln(1 / (1 - 0.9) - 1)) + 0.5 * (ln(1 / (1 - 0.45) - 1) - ln(1 / (1 - 0.25) - 1))) / (2 * (sqrt(3)/pi)))) * (56 - 6) + 6
        //
        // (0.5 + 0.5 * tanh((0.5 * (ln(1 / (1 - 0.5) - 1)) + 0.5 * (ln(1 / (1 - 0.45) - 1) - ln(1 / (1 - 0.15) - 1))) / (2 * (sqrt(3)/pi)))) * (56 - 6) + 6
        //
        // (0.5 + 0.5 * tanh((1 * (ln(1 / (1 - 0.5) - 1)) + 1 * (ln(1 / (1 - 0.05) - 1) - ln(1 / (1 - 0.1) - 1))) / (2 * (sqrt(3)/pi)))) * (56 - 6) + 6
        //
        // (0.5 + 0.5 * tanh((0.5 * (ln(1 / (1 - 0.75) - 1)) + 1 * (ln(1 / (1 - 0.1) - 1) - ln(1 / (1 - 0.1) - 1))) / (2 * (sqrt(3)/pi)))) * (56 - 6) + 6
        //
        // (0.5 + 0.5 * tanh((0.5 * (ln(1 / (1 - 0.5) - 1)) + 2 * (ln(1 / (1 - 0.15) - 1) - ln(1 / (1 - 0.1) - 1))) / (2 * (sqrt(3)/pi)))) * (56 - 6) + 6
        //
        // tanh(3.29/(2*sqrt(3)/pi))
        //
        //
        // Logistic regression.  Make sure x ∈ (0, 1).
        let logit = |x: f32| x.ln() - (-x).ln_1p();
        // 0.5 + 0.5 * tanh(ln(1 / (1 - 0.1) - 1) / (2 * (sqrt(3)/pi)))
        let logistic_2_base = 3.0f32.sqrt() * f32::consts::FRAC_2_PI;
        // Assumes μ = 0, σ = 1
        let logistic_cdf = |x: f32| (x / logistic_2_base).tanh() * 0.5 + 0.5;

        // We do log-odds against 0.1, so that our log odds are 0 when x = 0.1, lower when x is
        // lower, and higher when x is higher.
        let log_odds = |x: f32| logit(x) - logit(/*0.10*/0.25);
        //
        // 0.9^((1.25-0.0)*4)*(56-6)+6
        //
        // Want: values from (0, 0.25) to be compressed.
        //
        // x / (1 - x)
        //
        // Odds ratio: p1 / (1 - p1) / (p2 / (1 - p2))
        //
        // > 1 means p1 is more likely, < 1 means p2 is more likely.
        //
        // (0.5 / (1 - 0.5)) / (0.25 / (1 - 0.25))
        //
        // (z / (1 - 0.5)) / (0.25 / (1 - 0.25)) gives us a good z estimate since it will be
        //
        // 0.25 / (1 - 0.25)
        // plausible cliffs effect.  Taking the square root makes it increase more rapidly than it
        // would otherwise, so that at height 0.4 we are already looking at 0.8 of the maximum.
        let rock_strength = logistic_cdf(/*0.25*/0.5 * logit(rock_strength.min(0.95).max(0.05)) +
                                         /*3.0*/4.0 * log_odds(z.min(/*0.15*/0.3).max(0.2)));
        // let height_factor = z.min(1.0).max(0.0).powf(0.25);
        let max_slope = (rock_strength * MAX_ANGLE_RANGE/* * height_factor*/ + MIN_MAX_ANGLE).tan();
        /* if max_slope > 1.48 || max_slope < 0.0 {
            println!("Huh? {:?}", max_slope);
        } */
        max_slope
    }).collect::<Vec<_>>().into_boxed_slice()
}
/*
/// Compute the maximum slope at a point.
fn get_max_slope(posi: usize, z: f32, rock_strength_nz: &impl NoiseFn<Point3<f64>>) -> f32 {
    const MIN_MAX_ANGLE : f32 = 6.0 / 360.0 * 2.0 * f32::consts::PI;
    const MAX_MAX_ANGLE : f32 = 54.0 / 360.0 * 2.0 * f32::consts::PI;
    const MAX_ANGLE_RANGE : f32 = MAX_MAX_ANGLE - MIN_MAX_ANGLE;
    let wposf = (uniform_idx_as_vec2(posi) * TerrainChunkSize::RECT_SIZE.map(|e| e as i32))
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
*/

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
    const NEIGHBOR_DISTANCE : f32 = TerrainChunkSize::RECT_SIZE.map(|e| e as f32).magnitude();
    let mut rate = vec![0.0; h.len()].into_boxed_slice();
    // Iterate in ascending height order.
    for &posi in &*newh {
        let posj = downhill[posi];
        rate[posi] = if posj < 0 {
            0.0 // Egress with no outgoing flows.
        } else {
            let posj = posj as usize;
            let dist = Vec2::new(TerrainChunkSize::RECT_SIZE.x as f32, TerrainChunkSize::RECT_SIZE.y as f32);
            // let dist = Vec3::new(TerrainChunkSize::RECT_SIZE.x as f32, TerrainChunkSize::RECT_SIZE.y as f32, zdist);
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
fn erode(h: &mut [f32], erosion_base: f32, max_uplift: f32, _seed: &RandomField,
         rock_strength_nz: &(impl NoiseFn<Point3<f64>> + Sync),
         uplift: impl Fn(usize) -> f32, oldh: impl Fn(usize) -> f32 + Sync) {
    println!("Done draining...");
    let mmaxh = 1.0;
    let k = erosion_base + 2.244 / mmaxh * max_uplift;
    let ((dh, newh, area), max_slope) = rayon::join(
        || {
            /* let (dh, ()) = rayon::join(
                || {
                    let mut dh = downhill(h);
                    println!("Computed downhill...");
                    let lakes = get_lakes(&h, &mut dh);
                    println!("Got lakes...");
                    dh
                },
                || {
                    sort_by_height(h, newh);
                    println!("Sorted... (max height: {:?}",
                             newh.last().map(|&posi| h[posi as usize]));
                },
            ); */
            let mut dh = downhill(h, |posi| oldh(posi));
            println!("Computed downhill...");
            let (boundary_len, _, newh) = get_lakes(&h, &mut dh);
            println!("Got lakes...");
            let area = get_flux(&newh, &dh, boundary_len);
            println!("Got flux...");
            /*let (area, _) = rayon::join(
                || {
                    let flux = get_flux(newh, &dh);
                    println!("Got flux...");
                    flux
                },
                || {
                },
            );*/
            (dh, newh, area)
        },
        || {
            let max_slope = get_max_slope(h, rock_strength_nz);
            println!("Got max slopes...");
            max_slope
        },
    );
    /* // 1. Sort nodes in h by height.
    let mut newh = h.iter().enumerate().collect::<Vec<_>>();
    newh.sort_unstable_by(|f, g| (f.1, f.0).partial_cmp(&(g.1, g.0)).unwrap());
    // 2. Iterate through in reverse and compute drainage area. */
    // let maxh = *h.iter().max_by( |a, b| a.partial_cmp(&b).unwrap()).unwrap();
    // println!("Computed downhill...");
    // println!("Eroding... (max height: {:?})", maxh);
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
    /* let max_slope = get_max_slope(h, rock_strength_nz);
    println!("Got max slopes..."); */
    //let newh = height_sorted(h);
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
    let neighbor_coef =
        Vec2::new(TerrainChunkSize::RECT_SIZE.x as f32, TerrainChunkSize::RECT_SIZE.y as f32);
    // let neighbor_distance = TerrainChunkSize::RECT_SIZE.map(|e| e as f32).magnitude();
    // let mut rate = vec![0.0; h.len()].into_boxed_slice();
    // Iterate in ascending height order.
    let mut maxh = 0.0;
    for &posi in &*newh {
        let posi = posi as usize;
        let posj = dh[posi];
        if posj < 0 {
            if posj == -1 {
                panic!("Disconnected lake!");
            }
            // Egress with no outgoing flows.
            // println!("Shouldn't happen often: {:?}", uniform_idx_as_vec2(posi));
            // 0.0 // Egress with no outgoing flows.
        } else {
            let posj = posj as usize;
            let dxy = (uniform_idx_as_vec2(posi) - uniform_idx_as_vec2(posj)).map(|e| e as f32);
            // let dist = Vec3::new(TerrainChunkSize::RECT_SIZE.x as f32, TerrainChunkSize::RECT_SIZE.y as f32, zdist);
            // zdist / dist.magnitude()

            // Has an outgoing flow edge (posi, posj).
            // flux(i) = k * A[i]^m / ((p(i) - p(j)).magnitude()), and δt = 1
            let neighbor_distance = (neighbor_coef * dxy).magnitude();
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
            // let max_slope = get_max_slope(posi, old_h_i/*new_h_i*/, rock_strength_nz);
            let max_slope = max_slope[posi];
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
        maxh = h[posi].max(maxh);
    }
    println!("Done eroding (max height: {:?})", maxh);
}

/// The Planchon-Darboux algorithm for extracting drainage networks.
///
/// http://horizon.documentation.ird.fr/exl-doc/pleins_textes/pleins_textes_7/sous_copyright/010031925.pdf
///
/// See https://github.com/mewo2/terrain/blob/master/terrain.js
pub fn fill_sinks(h: impl Fn(usize) -> f32 + Sync,
                  oh: impl Fn(usize) -> f32 + Sync/*, epsilon: f64*/) -> Box<[f32]> {
    //let epsilon = 1e-5f32;
    let epsilon = 1e-7f32 / CONFIG.mountain_scale;
    let infinity = f32::INFINITY;
    let range = 0..WORLD_SIZE.x * WORLD_SIZE.y;
    let mut newh = range.into_par_iter().map(|posi| {
        let h = h(posi);
        let is_near_edge = map_edge_factor(posi) /*< 1.0*/== 0.0 ||
            oh(posi) /*< 5.0 / CONFIG.mountain_scale*/<= 0.0 / CONFIG.mountain_scale;
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
        for posi in 0..newh.len() {
            let nh = newh[posi];
            let oh = h(posi);
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

/* #[derive(Clone,Copy,Debug)]
/// A set of lakes, such that you can find the chunk representing the bottom of a lake for any
/// chunk index.
pub struct LakeSet {
    ///
    indirection: Vec<i32>,
}
/// Lake array.
///
/// If the inner value is negative, this is a lake (
pub struct LakeIndex(i32); */

/* /// The Planchon-Darboux algorithm for extracting drainage networks.
///
/// http://horizon.documentation.ird.fr/exl-doc/pleins_textes/pleins_textes_7/sous_copyright/010031925.pdf
///
/// See https://github.com/mewo2/terrain/blob/master/terrain.js */
/// Algorithm for finding and connecting lakes.  Assumes newh and downhill have already
/// been computed.  When a lake's value is negative, it is its own lake root, and when it is 0, it
/// is on the boundary of Ω.
///
/// Returns a 4-tuple containing:
/// - The first indirection vector (associating chunk indices with their lake's root node).
/// - A list of chunks on the boundary (non-lake egress points).
/// - The second indirection vector (associating chunk indices with their lake's adjacency list).
/// - The adjacency list (stored in a single vector), indexed by the second indirection vector.
pub fn get_lakes(/*newh: &[u32], */h: &[f32], downhill: &mut [isize]) -> /*(Box<[i32]>, Vec<usize>, Vec<i32>, Vec<(i32, u32)>)*/(usize, Box<[i32]>, Box<[u32]>) {
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
    // let mut indirection = Vec::with_capacity(WORLD_SIZE.x * WORLD_SIZE.y);

    let mut newh = Vec::with_capacity(downhill.len());

    // Now, we know that the sum of all the indirection nodes will be the same as the number of
    // nodes.  We can allocate a *single* vector with 8 * nodes entries, to be used as storage
    // space, and augment our indirection vector with the starting index, resulting in a vector of
    // slices.  As we go, we replace each lake entry with its index in the new indirection buffer,
    // allowing
    let mut lakes = vec![(-1, 0); /*(indirection.len() - boundary.len())*/indirection.len() * 8];
    let mut indirection_ = vec![0u32; indirection.len()];
    /* // First, find all the lakes.  We can actually do this in parallel, sort of! (since each lake
    // can push onto a private vector, and we can then merge at the end).
    dh
        .par_iter()
        .enumerate()
        .filter(dh < 0)
        .map(|(chunk_idx, dh)| {
            // Find all the nodes uphill from this lake.  Since there is only one outgoing edge
            // in the "downhill" graph, this is guaranteed never to visit a node more than
            // once.
            let mut start = newh.len();
            if dh == -2 {
                    // On the boundary, add to the boundary vector.
                    boundary.push(chunk_idx);
                    // Still considered a lake root, though.
            }
        }) */
    let mut lake_roots = Vec::with_capacity(downhill.len()); // Test
    for (chunk_idx, &dh) in (&*downhill).into_iter().enumerate().filter(|(_, &dh_idx)| dh_idx < 0) {
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

    println!("Old lake roots: {:?}", lake_roots.len());

    let newh = newh.into_boxed_slice();
    // let mut indirection = indirection.into_boxed_slice();

    // let mut indirection = vec![/*-1i32*/0i32; WORLD_SIZE.x * WORLD_SIZE.y].into_boxed_slice();

    /* for &chunk_idx_ in newh.into_iter() {
        let chunk_idx = chunk_idx_ as usize;
        match downhill[chunk_idx] {
            -1 => {
                // New lake root, initialized to -1 (length = 1).
                indirection[chunk_idx] = -1;
            }
            -2 => {
                // On the boundary, add to the boundary vector.
                boundary.push(chunk_idx);
                // Still considered a lake root, though.
                indirection[chunk_idx] = -1;
            }
            downhill_idx_ => {
                // This is not a lake--it has a downhill edge.
                let downhill_idx = downhill_idx_ as usize;
                // First, we need to find out what lake we're in; we can do that by just looking up
                // the lake in downhill_
                let lake_idx = indirection[downhill_idx];
                /*if lake_idx == -1 {
                    // Not in a lake, not on the boundary, so do nothing.
                } else {*/
                    let lake_idx = if lake_idx >= 0 {
                        // lake_idx is a normal non-lake chunk, use its lake as ours.
                        lake_idx
                    } else {
                        // downhill is actually our lake, and -lake_idx represents a count.
                        downhill_idx as i32
                    };
                    // lake_idx is the index of the lake.  Set our lake and decrement the lake's
                    // entry.
                    indirection[chunk_idx] = lake_idx;
                    indirection[lake_idx as usize] -= 1;
                /*}*/
            }
        }
    } */

    // Now, we know that the sum of all the indirection nodes will be the same as the number of
    // nodes.  We can allocate a *single* vector with 8 * nodes entries, to be used as storage
    // space, and augment our indirection vector with the starting index, resulting in a vector of
    // slices.  As we go, we replace each lake entry with its index in the new indirection buffer,
    // allowing
    /* let mut lakes = vec![(-1, 0); /*(indirection.len() - boundary.len())*/indirection.len() * 8];
    let mut start = 0;
    let mut indirection_ =  vec![-1i32; indirection.len()]; */
    for &chunk_idx_ in newh.into_iter() {
        let chunk_idx = chunk_idx_ as usize;
        // let indirection_idx = indirection[chunk_idx];
        /*if indirection_idx == -1 {
            // Not in a lake, not on the boundary, so do nothing.
        } else {*/
            // Find our lake, just like before.
            /* let lake_idx_ = if indirection_idx >= 0 {
                // indirection_idx is a normal non-lake chunk, so its lake_idx should store the
                // start index into the lakes vector!
                indirection_[indirection_idx as usize]
            } else {
                // -indirection_idx represents a count (of at least 1).
                let size = -indirection_idx;
                // Our own index will be start.
                let lake_idx = start;
                // NOTE: Since lake_idx is non-negative, the cast to u32 is fine.
                let lake_idx_ = lake_idx as i32;
                // We reserve size * 8 slots, one for each potential neighbor, since that is an
                // upper bound on the number of neighboring lakes (this is clearly wasteful, but
                // why compress if you don't have a demonstrated need?).
                start += size * 8;
                // Since this is a lake, it has no neighbors of lower height, so we know that
                // nobody has added an entry for this lake yet.  Thus, we can unconditionally add
                // ourselves to any adjacent, processed entries' lists, and add them to ours.
                lake_idx_
            };
            // Set our indirection pointer to the new lake index.
            indirection_[chunk_idx] = lake_idx_;
            let lake_idx = lake_idx_ as usize; */
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
                if /*neighbor_lake_idx_ >= 0*//*lakes[neighbor_lake_idx].0 >= 0*/neighbor_lake_idx_ < lake_idx_ /*&& lake_idx_ != neighbor_lake_idx_*/ {
                    /* let (lake_chunk_idx, lake_len) = {
                        let indirection_idx = indirection[chunk_idx];
                        if indirection_idx >= 0 {
                            (indirection_idx as usize, (-indirection[indirection_idx as usize]) as usize)
                        } else {
                            (chunk_idx as usize, (-indirection_idx) as usize)
                        }
                    };
                    let (neighbor_lake_chunk_idx, neighbor_lake_len) = {
                        let indirection_idx = indirection[neighbor_idx];
                        if indirection_idx >= 0 {
                            (indirection_idx as usize, (-indirection[indirection_idx as usize]) as usize)
                        } else {
                            (neighbor_idx as usize, (-indirection_idx) as usize)
                        }
                    }; */
                    // let neighbor_lake_idx = neighbor_lake_idx_ as usize;
                    // We found an adjacent node that is not on the boundary and has already
                    // been processed, and also has a non-matching lake.  Therefore we can use
                    // split_at_mut to get disjoint slices.
                    let (lake, neighbor_lake) = /*if neighbor_lake_idx < lake_idx*/ {
                        // println!("Okay, {:?} < {:?}", neighbor_lake_idx, lake_idx);
                        let (neighbor_lake, lake) = lakes.split_at_mut(lake_idx);
                        (/*&mut lake[..lake_len]*/lake,
                         &mut neighbor_lake[neighbor_lake_idx..],
                         /* &mut neighbor_lake[neighbor_lake_idx..
                                            neighbor_lake_idx + neighbor_lake_len] */)
                    }/* else {
                        let (lake, neighbor_lake) = lakes.split_at_mut(neighbor_lake_idx);
                        (&mut lake[lake_idx..], neighbor_lake)
                    }*/;

                    // We don't actually need to know the real length here, because we've reserved
                    // enough spaces that we should always either find a -1 (available slot) or an
                    // entry for this chunk.
                    'outer: for pass in lake.iter_mut() {
                        if pass.0 == -1 {
                            /* let indirection_idx = indirection[chunk_idx];
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
                            println!("Adding edge {:?} between lakes {:?}.",
                                   ((chunk_idx, uniform_idx_as_vec2(chunk_idx as usize)),
                                    (neighbor_idx, uniform_idx_as_vec2(neighbor_idx as usize))),
                                   ((lake_chunk_idx,
                                     uniform_idx_as_vec2(lake_chunk_idx as usize),
                                     lake_idx_),
                                    (neighbor_lake_chunk_idx,
                                     uniform_idx_as_vec2(neighbor_lake_chunk_idx as usize),
                                     neighbor_lake_idx_)),
                            ); */

                            // println!("One time, in my mind, one time... (neighbor lake={:?} lake={:?})", neighbor_lake_idx, lake_idx_);
                            *pass = (chunk_idx_ as i32, neighbor_idx as u32);
                            // Should never run out of -1s in the neighbor lake if we didn't find
                            // the neighbor lake in our lake.
                            *neighbor_lake
                                .iter_mut()
                                .filter( |neighbor_pass| neighbor_pass.0 == -1)
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
                                    if height.max(neighbor_height) <
                                       pass_height.max(neighbor_pass_height) {
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
                            panic!("For edge {:?} between lakes {:?}, couldn't find partner \
                                    for pass {:?}. \
                                    Should never happen; maybe forgot to set both edges?",
                                   ((chunk_idx, uniform_idx_as_vec2(chunk_idx as usize)),
                                    (neighbor_idx, uniform_idx_as_vec2(neighbor_idx as usize))),
                                   ((lake_chunk_idx,
                                     uniform_idx_as_vec2(lake_chunk_idx as usize),
                                     lake_idx_),
                                    (neighbor_lake_chunk_idx,
                                     uniform_idx_as_vec2(neighbor_lake_chunk_idx as usize),
                                     neighbor_lake_idx_)),
                                   ((pass.0, uniform_idx_as_vec2(pass.0 as usize)),
                                    (pass.1, uniform_idx_as_vec2(pass.1 as usize))),
                            );
                        }
                    }
                    /*    lake.iter_mut()
                            .find_map(|(mut pass_lake_idx_, mut pass_chunk_idx_)| {
                                if pass_lake_idx_ == lake_idx_ {
                                    if h[pass_chunk_idx_ as usize] < height {
                                    }
                                    Some(())
                                } else if pass_lake_idx == -1 {
                                    *pass_lake_idx_ = lake_idx_;
                                    *pass_chunk_idx_ = chunk_idx_;
                                    Some(())
                                } else {
                                    None
                                }
                            })
                            // Should never run out of -1s.
                            .unwrap();*/

                    /* let lake_height = do_height(lake, neighbor_lake_idx_, neighbor_idx_);

                    // Since we are a lake root, we have no downhill nodes, so
                    // that means neighbor_idx is also a lake root.  Moreover, since we are
                    // generating this lake for the first time, there can't yet be an edge in
                    // neighbor_idx's list, so we can unconditionally add ourselves to it, and
                    // make chunk_idx the closest pass without checking the height.
                    // NOTE: neighbor_lake_idx can't overflow isize, because non-negative lake
                    // indices are always taken from the length of an allocated vector, and
                    // vector lengths are guaranteed to fit in isize.
                    // NOTE: Since neighbor_lake_idx is non-negative, the implicit cast from
                    // isize to usize is fine.
                    lakes[neighbor_lake_idx as usize].push((lake_idx_, chunk_idx_));
                    // We also push the neighbor onto our own pass list.
                    // NOTE: This can't overflow i32 because WORLD_SIZE.x * WORLD_SIZE.y is
                    // (assumed to) fit in an i32, and we can have at most
                    // WORLD_SIZE.x * WORLD_SIZE.y chunks (so neighbor_idx here is at most
                    // WORLD_SIZE.x * WORLD_SIZE.y - 1).
                    // NOTE: Since neighbor_idx is non-negative, the implicit cast from i32 to
                    // u32 is fine.
                    // NOTE: Since neighbor_lake_idx is non-negative, the cast to u32 is fine.
                    lakes[lake_idx].push((neighbor_lake_idx as u32, neighbor_idx as u32));*/
                }
            }
        /*}*/
    }

    // Now it's time to calculate the lake connections graph T_L covering G_L.
    let mut candidates = BinaryHeap::with_capacity(indirection.len());
    // let mut pass_flows : Vec<i32> = vec![-1; indirection.len()];

    // We start by going through each pass, deleting the ones that point out of boundary nodes and
    // adding ones that point into boundary nodes from non-boundary nodes.
    for edge in &mut lakes {
        let edge : &mut (i32, u32) = edge;
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
            candidates.push(Reverse((NotNan::new(pass).unwrap(), (edge.0 as u32, edge.1))));
        }
    }

    /* // We start with the nodes on the boundary.
    for &chunk_idx in &boundary {
        // No egress from boundary nodes (i.e. river mouths).
        debug_assert!(downhill[chunk_idx] == -2);
        // pass_flows[chunk_idx] = -2;
        let lake_idx = indirection_[chunk_idx] as usize;
        // Delete all outgoing edges.
        let lake_idx_len = indirection[chunk_idx];
        let max_len = -if lake_idx_len < 0 {
            lake_idx_len
        } else {
            indirection[lake_idx_len as usize]
        } as usize;
        // let max_len = -indirection[indirection[chunk_idx]] as usize;
        for edge in lakes[lake_idx..lake_idx + max_len].iter_mut() {
            // Delete the old edge, and remember it.
            let edge = mem::replace(edge, (-1, 0));
            // Don't fall off the end of the list.
            if edge.0 == -1 {
                break;
            }
            // Don't add incoming pointers from lakes on boundary nodes.
            let indirection_idx = indirection[edge.1 as usize];
            let neighbor_lake_idx = if indirection_idx < 0 {
                edge.1 as usize
            } else {
                indirection_idx as usize
            };
            if downhill[neighbor_lake_idx] == -2 {
                continue;
            }
            // Find the pass height
            let pass = h[edge.0 as usize].max(h[edge.1 as usize]);
            // Put the reverse edge in candidates, sorted by height.
            candidates.push((NotNan::new(pass).unwrap(), (edge.1, edge.0)));
        }
    } */

    // let mut pass_flows_sorted : Vec<(u32, u32)> = Vec::with_capacity(indirection.len());
    let mut pass_flows_sorted : Vec<usize> = Vec::with_capacity(indirection.len());

    // Now all passes pointing to the boundary are in candidates.
    // As long as there are still candidates, we continue...
    'outer_final_pass: while let Some(Reverse((_, (chunk_idx, neighbor_idx)))) = candidates.pop() {
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
        downhill[lake_chunk_idx as usize] = neighbor_idx as isize;
        // pass_flows[chunk_idx as usize] = neighbor_idx as i32;
        // Add this edge to the sorted list.
        pass_flows_sorted.push(lake_chunk_idx as usize);
        // pass_flows_sorted.push((chunk_idx as u32, neighbor_idx as u32));
        for edge in &mut lakes[lake_idx..lake_idx + max_len] {
        // for edge in lakes[lake_idx..].iter_mut().take(max_len) {
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
            candidates.push(Reverse((NotNan::new(pass).unwrap(), (edge.1, edge.0 as u32))));
        }
        // println!("I am a pass: {:?}", (uniform_idx_as_vec2(chunk_idx as usize), uniform_idx_as_vec2(neighbor_idx as usize)));
    }

    // Now just regenerate the downhill graph through the sorted lakes to generate a BFS-sorted tree.

    // Now that pass_flows are sorted, we can easily run a BFS.

    // Perofrm a breadth-first search of the tree.

    /* // Then, for each node in T_L, all the outgoing edges pointing out of T_L are removed from G_L,
    // and all the (remaining) incoming edges pointing into T_L become candidate arcs.
    // The candidate arcs are kept sorted by pass height.  We always choose the lowest candidate
    // arc (i, j) to become a new edge in T_L, removing it in the process.  When we choose it, we
    // remove all other edges pointing out of i from G_L, and ignore candidates when they flow out
    // of an already-handled node.  In this way we can use a BinaryHeap rather than a BTreeMap.
    let mut candidates =
        lakes
        .iter()
        .cloned() // Not expensive in this case.
        .filter(|&(chunk_idx, _)| chunk_idx >= 0)
        .collect::<Vec<_>>();

    // Our assumption here is that using a binary heap won't actually be that much faster than
    // sorting--nor will a BTree--because they will be sorted by height, and we need to be able
    // to efficiently remove edges by posi.
    candidates
        .par_sort_unstable_by(|f, g| (h[f.0 as usize].max(h[f.1 as usize]), f.0, f.1)
                              .partial_cmp(&(h[g.0 as usize].max(h[g.1 as usize]), g.0, g.1)
                              .unwrap());

    // Remove edges from boundary.
    for (chunk_idx, neighbor_idx) in candidates {
        //
    } */
    /*
    // Now it's time to calculate the lake connections graph T_L covering G_L.
    // We start with the nodes on the boundary.
    // Then, for each node in T_L, all the outgoing edges pointing out of T_L are removed from G_L,
    // and all the (remaining) incoming edges pointing into T_L become candidate arcs.
    // The candidate arcs are kept sorted by pass height.  We always choose the lowest candidate
    // arc (i, j) to become a new edge in T_L, removing it in the process.  When we choose it, we
    // remove all other edges pointing out of i from G_L, and ignore candidates when they flow out
    // of an already-handled node.  In this way we can use a BinaryHeap rather than a BTreeMap.
    let mut candidates =
        lakes
        .iter()
        .cloned() // Not expensive in this case.
        .enumerate()
        .filter(|&(_, (chunk_idx, _))| chunk_idx >= 0)
        .collect::<Vec<_>>();

    // Our assumption here is that using a binary heap won't actually be that much faster than
    // sorting--nor will a BTree--because they will be sorted by height, and we need to be able
    // to efficiently remove edges by posi.
    candidates
        .par_sort_unstable_by(|(_, f), (_, g)|
                              (h[f.0 as usize].max(h[f.1 as usize]), f.0, f.1)
                              .partial_cmp(&(h[g.0 as usize].max(h[g.1 as usize]), g.0, g.1))
                                           .unwrap())*/
    /* lakes
        .drain();
        .par_sort_unstable_by(|f, g| {
            if f.0 >= 0 && g.0 >= 0 {
                (f.1, f.0).partial_cmp(&(g.1, g.0)).unwrap()
            } else {

            });
            }

    let mut candidates = lakes
        .iter()
        .filter(|& &(chunk_idx, _)| chunk_idx >= 0)
        .map(|&(chunk_idx, neighbor_idx)|
             (indirection_[chunk_idx as usize],
              indirection_[neighbor_idx as usize]))


    let mut candidates = lakes
        .iter()
        .filter(|& &(chunk_idx, _)| chunk_idx >= 0)
        .map(|&(chunk_idx, neighbor_idx)|
             (NotNan::new(h[chunk_idx as usize].max(h[neighbor_idx as usize])).unwrap(),
              indirection_[chunk_idx as usize],
              indirection_[neighbor_idx as usize]))
        /* .flat_map(|(chunk_idx, indirection_idx)| {
            let lake_idx = indirection_[indirection_idx as usize];
            lake[lake_idx as usize].iter()
                .map(|(neighbor_lake_idx, pass)|
                     (h[chunk_idx].max(h[pass]), lake_idx, neighbor_lake_idx))
        }) */
        .collect::<BinaryHeap::<_>>(); */

    /* let mut candidates = indirection
        .iter()
        .enumerate()
        .filter(|(_, indirection_idx)| indirection_idx <= 0)
        .flat_map(|(chunk_idx, indirection_idx)| {
            let lake_idx = indirection_[indirection_idx as usize];
            lake[lake_idx as usize].iter()
                .map(|(neighbor_lake_idx, pass)|
                     (h[chunk_idx].max(h[pass]), lake_idx, neighbor_lake_idx))
        })
        .collect::<BinaryHeap>(); */
    println!("Total lakes: {:?}", pass_flows_sorted.len());

    // Perform the bfs once again.
    // let mut newh_position = vec![0usize; downhill.len()]; // Assertion
    let mut newh = Vec::with_capacity(downhill.len());
    // for (chunk_idx, &dh) in (&*downhill).into_iter().enumerate().filter(|(_, &dh_idx)| dh_idx < 0) {}
    (&*boundary).iter().chain(pass_flows_sorted.iter()).for_each(|&chunk_idx| {
        // Find all the nodes uphill from this lake.  Since there is only one outgoing edge
        // in the "downhill" graph, this is guaranteed never to visit a node more than
        // once.
        let start = newh.len();
        // New lake root
        newh.push(chunk_idx as u32);
        let mut cur = start;
        while cur < newh.len() {
            let node = newh[cur as usize];
            // newh_position[node as usize] = cur as usize;

            for child in uphill(downhill, node as usize) {
                // lake_idx is the index of our lake root.
                // Check to make sure child (flowing into us) isn't a lake.
                if indirection[child] /*== chunk_idx as i32*/>= 0 /* Note: equal to chunk_idx should be same */ {
                    assert!(h[child] >= h[node as usize]);
                    newh.push(child as u32);
                } else {
                    /* println!("wrong {:?} {:?}: indirection={:?}",
                             uniform_idx_as_vec2(node as usize),
                             uniform_idx_as_vec2(child as usize),
                             indirection[child]); */
                }
            }
            cur += 1;
        }
    });
    // Assertion
    // assert!(downhill.iter().enumerate().all(|(chunk_idx, &dh)| dh == -2 || newh_position[dh as usize] < newh_position[chunk_idx]));
    assert_eq!(newh.len(), downhill.len());
    // (indirection, boundary, indirection_, lakes)
    (boundary.len(), indirection, newh.into_boxed_slice())

    // Now, we have a bunch of computed lake information; in particular, we have passes for all
    // entries in the lake.

    /* indirection.iter_mut().map(|&lake_idx| {

    });

    //
    // an upper bound on the maximum number of
    // adjacent lakes to this node.
    // also store a , contiguous vector that w
    // Iterates in ascending height order through the nodes in newh, to ensure that lake bottoms
    // always appear before their higher neighbors.  If a chunk has no downhill chunk, set its lake
    // index to its own index.  Otherwise, set its lake index to its downhill chunk's lake index
    // (which we know is already set since we're going in ascending height order).
    // lake

    // NOTE: Even though we could use a two-level contiguous vector for representing lakes, with
    // each node containing an explicit adjacency list, we choose to use a BTreeMap instead.  This
    // way we hopefully avoid lots of little allocations (which can be a bit slow to clean up),
    // *and* we get both quick access and sorted access.

    // We start by computing a set of boundary nodes, and the initial set of passes.
    let mut boundary = Vec::with_capacity(downhill.len());

    // Construct a BTree...
    let mut passes = BTreeMap::new();

    // Iterate through each node.  Extend the
    boundary.extend(downhill.iter().filter(|&dh| dh == -2 ));

    //

    // We start by iterating through downhill and inserting into the BTreeMap.
    // use an explicit adjacency list within each finding edges, we choose to use a
    // BTreeMap.  This is because
    // NOTE: Asymptotically speaking it is a very bad choice to use a vector with an adjacency list
    // for our passes, rather than some other data structure.  Currently we are operating under the
    // assumption that in practice, this is in fact a very *good* idea, but we can fix it if
    // necessary.  A proper fix would probably stick to one t
    let mut passes : Vec<(u32, Vec<(u32, u32)>)> = vec![];
    // Associates each node to a lake index in passes.  If the lake is -2, this node is on
    // the boundary of Ω.  If it is -1, it hasn't been assigned yet.
    let mut lake = vec![-1 ; WORLD_SIZE.x * WORLD_SIZE.y].into_boxed_slice();
    // Iterates in ascending height order through the nodes in newh, to ensure that lake bottoms
    // always appear before their higher neighbors.  If a chunk has no downhill chunk, set its lake
    // index to its own index.  Otherwise, set its lake index to its downhill chunk's lake index
    // (which we know is already set since we're going in ascending height order).
    for &chunk_idx_ in newh.into_iter().rev() {
        let chunk_idx = chunk_idx_ as usize;
        let downhill_idx_ = downhill[chunk_idx];
        lakes[chunk_idx] = match downhill_idx_ {
            -2 => {
                // On the boundary.
                lake[chunk_idx] = -2;
            },
            -1 => {
                // New lake root.  The lake index is its new location in the pass table.
                // NOTE: This can't overflow i32 because WORLD_SIZE.x * WORLD_SIZE.y is (assumed
                // to) fit in an i32, and we can have at most WORLD_SIZE.x * WORLD_SIZE.y lakes (so
                // lake_idx here is at most WORLD_SIZE.x * WORLD_SIZE.y - 1).
                let lake_idx = passes.len() as i32;
                // NOTE: Since lake_idx is non-negative, the cast to u32 is fine.
                let lake_idx_ = lake_idx as u32;
                let mut edges = vec![];
                // We know we have no downhill neighbors, and don't care about neighbors on Ω.
                // Moreover, if we are *downhill* of any of our neighbors, then that node will find
                // this one when it checks its own downhill neighbors (which it can safely do since
                // downhill neighbors have already had their lakes set).  Therefore, we just need
                // to search for adjacent lakes (that have already been assigned, so they must have
                // a non-negative lake index) and set those up as passes.
                //
                // Note that since in our graphs we only have up to 8 neighbors, performing this
                // iteration is *not* proportional to n² or nything.  Even if we allowed adjacent
                // edges to any point in the graph, we will only scan each pair of connected nodes
                // at most twice, so we're at leats linear in O(m).
                for neighbor_idx in neighbors(chunk_idx) {
                    let neighbor_lake_idx = lake[neighbor_idx];
                    if  neighbor_lake_idx >= 0 {
                        // We found an adjacent node that is not on the boundary and has already
                        // been processed; since we are a lake root, we have no downhill nodes, so
                        // that means neighbor_idx is also a lake root.  Moreover, since we are
                        // generating this lake for the first time, there can't yet be an edge in
                        // neighbor_idx's list, so we can unconditionally add ourselves to it, and
                        // make chunk_idx the closest pass without checking the height.
                        // NOTE: neighbor_lake_idx can't overflow isize, because non-negative lake
                        // indices are always taken from the length of an allocated vector, and
                        // vector lengths are guaranteed to fit in isize.
                        // NOTE: Since neighbor_lake_idx is non-negative, the implicit cast from
                        // isize to usize is fine.
                        passes[neighbor_lake_idx as usize].1.push((lake_idx_, chunk_idx_));
                        // We also push the neighbor onto our own pass list.
                        // NOTE: This can't overflow i32 because WORLD_SIZE.x * WORLD_SIZE.y is
                        // (assumed to) fit in an i32, and we can have at most
                        // WORLD_SIZE.x * WORLD_SIZE.y chunks (so neighbor_idx here is at most
                        // WORLD_SIZE.x * WORLD_SIZE.y - 1).
                        // NOTE: Since neighbor_idx is non-negative, the implicit cast from i32 to
                        // u32 is fine.
                        // NOTE: Since neighbor_lake_idx is non-negative, the cast to u32 is fine.
                        edges.push((neighbor_lake_idx as u32, neighbor_idx as u32));
                    }
                }
                // Now, push the new lake entry into the table.
                passes.push((lake_idx_, edges));
                // Finally, we return lake_idx as the lake index for this chunk.
                lake_idx
            },
            _ => {
                // This is not a lake--it has a downhill edge.
                // First, we need to find out what lake we're in; we can do that by just looking up
                // the lake in downhill_
                let downhill_idx = downhill_idx_ as usize;
            }
        };
        if downhill_idx == -2 {
            // On the boundary.
        }
        lake[chunk_idx] = if downhill_idx >= 0 {
            // There is a node downhill from this one (whose lake was therefore already computed).
            let lake = lake[downhill_idx as usize];
            if lake < 0 {
                // The chunk at downhill_idx is the lake root
                downhill_idx
            } else {
                lake
            }
        } else if downhill_idx == -2 {
            // This is on the boundary of Ω, so we don't need to connect them to anything.
            0
        } else {
            // This is a new lake root.
            // We store an index to a vector of adjacent passes.
            let lake_index = passes.len();
            // Push a brand new
            passes.push();
            // and with 1 added.

            // Associated with each j is the maximum height of a chunk in this lake that is
            // adjacent to
            // is adjacent .  While we could use a hash map or some other data structure for this,
            // for now we just use a vector.  Pass height is defined as the minimum
            chunk_idx
        }
    }
    lake */
}

/// Perform erosion n times.
pub fn do_erosion(/*oldh: &InverseCdf, *//*, epsilon: f64*//*newh: &mut [u32],*/
                  erosion_base: f32, /*amount: f32, */n: usize,
                  seed: &RandomField, rock_strength_nz: &(impl NoiseFn<Point3<f64>> + Sync),
                  oldh: impl Fn(usize) -> f32 + Sync,
                  uplift: impl Fn(usize) -> f32 + Sync) -> Box<[f32]> {
    let oldh_ = (0..WORLD_SIZE.x * WORLD_SIZE.y).into_par_iter()
        .map(|posi| oldh(posi)).collect::<Vec<_>>().into_boxed_slice();
    let max_uplift = (0..oldh_.len())
        .into_par_iter()
        .map( |posi| uplift(posi))
        .max_by( |a, b| a.partial_cmp(&b).unwrap()).unwrap();
    // Start by filling in deep depressions, to make for a more realistic initial river network.
    // let mut h = fill_sinks(&oldh_, |posi| oldh[posi].1 );
    let mut h = oldh_;
    for _ in 0..n {
        erode(&mut h, /*newh*//*&h, *//*amount*/erosion_base, max_uplift, seed,
              rock_strength_nz, |posi| uplift(posi), |posi| oldh(posi));
        // h = fill_sinks(&h);
    }
    h
}
