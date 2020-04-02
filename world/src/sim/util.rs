use super::WORLD_SIZE;
use bitvec::prelude::{bitbox, bitvec, BitBox};
use common::{terrain::TerrainChunkSize, vol::RectVolSize};
use noise::{MultiFractal, NoiseFn, Perlin, Point2, Point3, Point4, Seedable};
use num::Float;
use rayon::prelude::*;
use std::{f32, f64, ops::Mul, u32};
use vek::*;

/// Calculates the smallest distance along an axis (x, y) from an edge of
/// the world.  This value is maximal at WORLD_SIZE / 2 and minimized at the
/// extremes (0 or WORLD_SIZE on one or more axes).  It then divides the
/// quantity by cell_size, so the final result is 1 when we are not in a cell
/// along the edge of the world, and ranges between 0 and 1 otherwise (lower
/// when the chunk is closer to the edge).
pub fn map_edge_factor(posi: usize) -> f32 {
    uniform_idx_as_vec2(posi)
        .map2(WORLD_SIZE.map(|e| e as i32), |e, sz| {
            (sz / 2 - (e - sz / 2).abs()) as f32 / (16.0 / 1024.0 * sz as f32)
        })
        .reduce_partial_min()
        .max(0.0)
        .min(1.0)
}

/// Computes the cumulative distribution function of the weighted sum of k
/// independent, uniformly distributed random variables between 0 and 1.  For
/// each variable i, we use weights[i] as the weight to give samples[i] (the
/// weights should all be positive).
///
/// If the precondition is met, the distribution of the result of calling this
/// function will be uniformly distributed while preserving the same information
/// that was in the original average.
///
/// For N > 33 the function will no longer return correct results since we will
/// overflow u32.
///
/// NOTE:
///
/// Per [1], the problem of determing the CDF of
/// the sum of uniformly distributed random variables over *different* ranges is
/// considerably more complicated than it is for the same-range case.
/// Fortunately, it also provides a reference to [2], which contains a complete
/// derivation of an exact rule for the density function for this case.  The CDF
/// is just the integral of the cumulative distribution function [3],
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
    // instead, and figure out K by calling count_ones(), so we can compute the
    // result in O(2^N) iterations.
    let x: f64 = weights
        .iter()
        .zip(samples.iter())
        .map(|(&weight, &sample)| weight as f64 * sample as f64)
        .sum();

    let mut y = 0.0f64;
    for subset in 0u32..(1 << N) {
        // Number of set elements
        let k = subset.count_ones();
        // Add together exactly the set elements to get B_subset
        let z = weights
            .iter()
            .enumerate()
            .filter(|(i, _)| subset & (1 << i) as u32 != 0)
            .map(|(_, &k)| k as f64)
            .sum::<f64>();
        // Compute max(0, x - B_subset)^N
        let z = (x - z).max(0.0).powi(N as i32);
        // The parity of k determines whether the sum is negated.
        y += if k & 1 == 0 { z } else { -z };
    }

    // Divide by the product of the weights.
    y /= weights.iter().map(|&k| k as f64).product::<f64>();

    // Remember to multiply by 1 / N! at the end.
    (y / (1..(N as i32) + 1).product::<i32>() as f64) as f32
}

/// First component of each element of the vector is the computed CDF of the
/// noise function at this index (i.e. its position in a sorted list of value
/// returned by the noise function applied to every chunk in the game).  Second
/// component is the cached value of the noise function that generated the
/// index.
///
/// NOTE: Length should always be WORLD_SIZE.x * WORLD_SIZE.y.
pub type InverseCdf<F = f32> = Box<[(f32, F)]>;

/// Computes the position Vec2 of a SimChunk from an index, where the index was
/// generated by uniform_noise.
pub fn uniform_idx_as_vec2(idx: usize) -> Vec2<i32> {
    Vec2::new((idx % WORLD_SIZE.x) as i32, (idx / WORLD_SIZE.x) as i32)
}

/// Computes the index of a Vec2 of a SimChunk from a position, where the index
/// is generated by uniform_noise.  NOTE: Both components of idx should be
/// in-bounds!
pub fn vec2_as_uniform_idx(idx: Vec2<i32>) -> usize {
    (idx.y as usize * WORLD_SIZE.x + idx.x as usize) as usize
}

/// Compute inverse cumulative distribution function for arbitrary function f,
/// the hard way.  We pre-generate noise values prior to worldgen, then sort
/// them in order to determine the correct position in the sorted order.  That
/// lets us use `(index + 1) / (WORLDSIZE.y * WORLDSIZE.x)` as a uniformly
/// distributed (from almost-0 to 1) regularization of the chunks.  That is, if
/// we apply the computed "function" F⁻¹(x, y) to (x, y) and get out p, it means
/// that approximately (100 * p)% of chunks have a lower value for F⁻¹ than p.
/// The main purpose of doing this is to make sure we are using the entire range
/// we want, and to allow us to apply the numerous results about distributions
/// on uniform functions to the procedural noise we generate, which lets us much
/// more reliably control the *number* of features in the world while still
/// letting us play with the *shape* of those features, without having arbitrary
/// cutoff points / discontinuities (which tend to produce ugly-looking /
/// unnatural terrain).
///
/// As a concrete example, before doing this it was very hard to tweak humidity
/// so that either most of the world wasn't dry, or most of it wasn't wet, by
/// combining the billow noise function and the computed altitude.  This is
/// because the billow noise function has a very unusual distribution that is
/// heavily skewed towards 0.  By correcting for this tendency, we can start
/// with uniformly distributed billow noise and altitudes and combine them to
/// get uniformly distributed humidity, while still preserving the existing
/// shapes that the billow noise and altitude functions produce.
///
/// f takes an index, which represents the index corresponding to this chunk in
/// any any SimChunk vector returned by uniform_noise, and (for convenience) the
/// float-translated version of those coordinates.
/// f should return a value with no NaNs.  If there is a NaN, it will panic.
/// There are no other conditions on f.  If f returns None, the value will be
/// set to NaN, and will be ignored for the purposes of computing the uniform
/// range.
///
/// Returns a vec of (f32, f32) pairs consisting of the percentage of chunks
/// with a value lower than this one, and the actual noise value (we don't need
/// to cache it, but it makes ensuring that subsequent code that needs the noise
/// value actually uses the same one we were using here easier).  Also returns
/// the "inverted index" pointing from a position to a noise.
pub fn uniform_noise<F: Float + Send>(
    f: impl Fn(usize, Vec2<f64>) -> Option<F> + Sync,
) -> (InverseCdf<F>, Box<[(usize, F)]>) {
    let mut noise = (0..WORLD_SIZE.x * WORLD_SIZE.y)
        .into_par_iter()
        .filter_map(|i| {
            f(
                i,
                (uniform_idx_as_vec2(i) * TerrainChunkSize::RECT_SIZE.map(|e| e as i32))
                    .map(|e| e as f64),
            )
            .map(|res| (i, res))
        })
        .collect::<Vec<_>>();

    // sort_unstable_by is equivalent to sort_by here since we include a unique
    // index in the comparison.  We could leave out the index, but this might
    // make the order not reproduce the same way between different versions of
    // Rust (for example).
    noise.par_sort_unstable_by(|f, g| (f.1, f.0).partial_cmp(&(g.1, g.0)).unwrap());

    // Construct a vector that associates each chunk position with the 1-indexed
    // position of the noise in the sorted vector (divided by the vector length).
    // This guarantees a uniform distribution among the samples (excluding those
    // that returned None, which will remain at zero).
    let mut uniform_noise = vec![(0.0, F::nan()); WORLD_SIZE.x * WORLD_SIZE.y].into_boxed_slice();
    // NOTE: Consider using try_into here and elsewhere in this function, since
    // i32::MAX technically doesn't fit in an f32 (even if we should never reach
    // that limit).
    let total = noise.len() as f32;
    for (noise_idx, &(chunk_idx, noise_val)) in noise.iter().enumerate() {
        uniform_noise[chunk_idx] = ((1 + noise_idx) as f32 / total, noise_val);
    }
    (uniform_noise, noise.into_boxed_slice())
}

/// Iterate through all cells adjacent and including four chunks whose top-left
/// point is posi. This isn't just the immediate neighbors of a chunk plus the
/// center, because it is designed to cover neighbors of a point in the chunk's
/// "interior."
///
/// This is what's used during cubic interpolation, for example, as it
/// guarantees that for any point between the given chunk (on the top left) and
/// its top-right/down-right/down neighbors, the twelve chunks surrounding this
/// box (its "perimeter") are also inspected.
pub fn local_cells(posi: usize) -> impl Clone + Iterator<Item = usize> {
    let pos = uniform_idx_as_vec2(posi);
    // NOTE: want to keep this such that the chunk index is in ascending order!
    let grid_size = 3i32;
    let grid_bounds = 2 * grid_size + 1;
    (0..grid_bounds * grid_bounds)
        .into_iter()
        .map(move |index| {
            Vec2::new(
                pos.x + (index % grid_bounds) - grid_size,
                pos.y + (index / grid_bounds) - grid_size,
            )
        })
        .filter(|pos| {
            pos.x >= 0 && pos.y >= 0 && pos.x < WORLD_SIZE.x as i32 && pos.y < WORLD_SIZE.y as i32
        })
        .map(vec2_as_uniform_idx)
}

// NOTE: want to keep this such that the chunk index is in ascending order!
pub const NEIGHBOR_DELTA: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

/// Iterate through all cells adjacent to a chunk.
pub fn neighbors(posi: usize) -> impl Clone + Iterator<Item = usize> {
    let pos = uniform_idx_as_vec2(posi);
    NEIGHBOR_DELTA
        .iter()
        .map(move |&(x, y)| Vec2::new(pos.x + x, pos.y + y))
        .filter(|pos| {
            pos.x >= 0 && pos.y >= 0 && pos.x < WORLD_SIZE.x as i32 && pos.y < WORLD_SIZE.y as i32
        })
        .map(vec2_as_uniform_idx)
}

// Note that we should already have okay cache locality since we have a grid.
pub fn uphill<'a>(dh: &'a [isize], posi: usize) -> impl Clone + Iterator<Item = usize> + 'a {
    neighbors(posi).filter(move |&posj| dh[posj] == posi as isize)
}

/// Compute the neighbor "most downhill" from all chunks.
///
/// TODO: See if allocating in advance is worthwhile.
pub fn downhill<F: Float>(
    h: impl Fn(usize) -> F + Sync,
    is_ocean: impl Fn(usize) -> bool + Sync,
) -> Box<[isize]> {
    // Constructs not only the list of downhill nodes, but also computes an ordering
    // (visiting nodes in order from roots to leaves).
    (0..WORLD_SIZE.x * WORLD_SIZE.y)
        .into_par_iter()
        .map(|posi| {
            let nh = h(posi);
            if is_ocean(posi) {
                -2
            } else {
                let mut best = -1;
                let mut besth = nh;
                for nposi in neighbors(posi) {
                    let nbh = h(nposi);
                    if nbh < besth {
                        besth = nbh;
                        best = nposi as isize;
                    }
                }
                best
            }
        })
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

/* /// Bilinear interpolation.
///
/// Linear interpolation in both directions (i.e. quadratic interpolation).
fn get_interpolated_bilinear<T, F>(&self, pos: Vec2<i32>, mut f: F) -> Option<T>
    where
        T: Copy + Default + Signed + Float + Add<Output = T> + Mul<f32, Output = T>,
        F: FnMut(Vec2<i32>) -> Option<T>,
{
    // (i) Find downhill for all four points.
    // (ii) Compute distance from each downhill point and do linear interpolation on
    // their heights. (iii) Compute distance between each neighboring point
    // and do linear interpolation on       their distance-interpolated
    // heights.

    // See http://articles.adsabs.harvard.edu/cgi-bin/nph-iarticle_query?1990A%26A...239..443S&defaultprint=YES&page_ind=0&filetype=.pdf
    //
    // Note that these are only guaranteed monotone in one dimension; fortunately,
    // that is sufficient for our purposes.
    let pos = pos.map2(TerrainChunkSize::RECT_SIZE, |e, sz: u32| {
        e as f64 / sz as f64
    });

    // Orient the chunk in the direction of the most downhill point of the four.  If
    // there is no "most downhill" point, then we don't care.
    let x0 = pos.map2(Vec2::new(0, 0), |e, q| e.max(0.0) as i32 + q);
    let y0 = f(x0)?;

    let x1 = pos.map2(Vec2::new(1, 0), |e, q| e.max(0.0) as i32 + q);
    let y1 = f(x1)?;

    let x2 = pos.map2(Vec2::new(0, 1), |e, q| e.max(0.0) as i32 + q);
    let y2 = f(x2)?;

    let x3 = pos.map2(Vec2::new(1, 1), |e, q| e.max(0.0) as i32 + q);
    let y3 = f(x3)?;

    let z0 = y0
        .mul(1.0 - pos.x.fract() as f32)
        .mul(1.0 - pos.y.fract() as f32);
    let z1 = y1.mul(pos.x.fract() as f32).mul(1.0 - pos.y.fract() as f32);
    let z2 = y2.mul(1.0 - pos.x.fract() as f32).mul(pos.y.fract() as f32);
    let z3 = y3.mul(pos.x.fract() as f32).mul(pos.y.fract() as f32);

    Some(z0 + z1 + z2 + z3)
} */

/// Find all ocean tiles from a height map, using an inductive definition of
/// ocean as one of:
/// - posi is at the side of the world (map_edge_factor(posi) == 0.0)
/// - posi has a neighboring ocean tile, and has a height below sea level
///   (oldh(posi) <= 0.0).
pub fn get_oceans<F: Float>(oldh: impl Fn(usize) -> F + Sync) -> BitBox {
    // We can mark tiles as ocean candidates by scanning row by row, since the top
    // edge is ocean, the sides are connected to it, and any subsequent ocean
    // tiles must be connected to it.
    let mut is_ocean = bitbox![0; WORLD_SIZE.x * WORLD_SIZE.y];
    let mut stack = Vec::new();
    let mut do_push = |pos| {
        let posi = vec2_as_uniform_idx(pos);
        if oldh(posi) <= F::zero() {
            stack.push(posi);
        }
    };
    for x in 0..WORLD_SIZE.x as i32 {
        do_push(Vec2::new(x, 0));
        do_push(Vec2::new(x, WORLD_SIZE.y as i32 - 1));
    }
    for y in 1..WORLD_SIZE.y as i32 - 1 {
        do_push(Vec2::new(0, y));
        do_push(Vec2::new(WORLD_SIZE.x as i32 - 1, y));
    }
    while let Some(chunk_idx) = stack.pop() {
        // println!("Ocean chunk {:?}: {:?}", uniform_idx_as_vec2(chunk_idx),
        // oldh(chunk_idx));
        if *is_ocean.at(chunk_idx) {
            continue;
        }
        *is_ocean.at(chunk_idx) = true;
        stack.extend(neighbors(chunk_idx).filter(|&neighbor_idx| {
            // println!("Ocean neighbor: {:?}: {:?}", uniform_idx_as_vec2(neighbor_idx),
            // oldh(neighbor_idx));
            oldh(neighbor_idx) <= F::zero()
        }));
    }
    is_ocean
}

/// Finds the horizon map for sunlight for the given chunks.
pub fn get_horizon_map<F: Float + Sync, A: Send, H: Send>(
    lgain: F,
    bounds: Aabr<i32>,
    minh: F,
    maxh: F,
    h: impl Fn(usize) -> F + Sync,
    to_angle: impl Fn(F) -> A + Sync,
    to_height: impl Fn(F) -> H + Sync,
) -> Result<[(Vec<A>, Vec<H>); 2], ()> {
    let map_size = Vec2::<i32>::from(bounds.size()).map(|e| e as usize);
    let map_len = map_size.product();

    // Now, do the raymarching.
    let chunk_x = if let Vec2 { x: Some(x), .. } = TerrainChunkSize::RECT_SIZE.map(F::from) {
        x
    } else {
        return Err(());
    };
    // let epsilon = F::epsilon() * if let x = F::from(map_size.x) { x } else {
    // return Err(()) };
    let march = |dx: isize, maxdx: fn(isize) -> isize| {
        let mut angles = Vec::with_capacity(map_len);
        let mut heights = Vec::with_capacity(map_len);
        (0..map_len)
            .into_par_iter()
            .map(|posi| {
                let wposi =
                    bounds.min + Vec2::new((posi % map_size.x) as i32, (posi / map_size.x) as i32);
                if wposi.reduce_partial_min() < 0
                    || wposi.y as usize >= WORLD_SIZE.x
                    || wposi.y as usize >= WORLD_SIZE.y
                {
                    return (to_angle(F::zero()), to_height(F::zero()));
                }
                let posi = vec2_as_uniform_idx(wposi);
                // March in the given direction.
                let maxdx = maxdx(wposi.x as isize);
                let mut slope = F::zero();
                let mut max_height = F::zero();
                let h0 = h(posi);
                if h0 >= minh {
                    let maxdz = maxh - h0;
                    let posi = posi as isize;
                    for deltax in 1..maxdx {
                        let posj = (posi + deltax * dx) as usize;
                        let deltax = chunk_x * F::from(deltax).unwrap();
                        let h_j_est = slope * deltax;
                        if h_j_est > maxdz {
                            break;
                        }
                        let h_j_act = h(posj) - h0;
                        if
                        /* h_j_est - h_j_act <= epsilon */
                        h_j_est <= h_j_act {
                            slope = h_j_act / deltax;
                            max_height = h_j_act;
                        }
                    }
                }
                let a = slope * lgain;
                let h = h0 + max_height;
                (to_angle(a), to_height(h))
            })
            .unzip_into_vecs(&mut angles, &mut heights);
        (angles, heights)
    };
    let west = march(-1, |x| x);
    let east = march(1, |x| (WORLD_SIZE.x - x as usize) as isize);
    Ok([west, east])
}

/// A 2-dimensional vector, for internal use.
type Vector2<T> = [T; 2];
/// A 3-dimensional vector, for internal use.
type Vector3<T> = [T; 3];
/// A 4-dimensional vector, for internal use.
type Vector4<T> = [T; 4];

#[inline]
fn zip_with2<T, U, V, F>(a: Vector2<T>, b: Vector2<U>, f: F) -> Vector2<V>
where
    T: Copy,
    U: Copy,
    F: Fn(T, U) -> V,
{
    let (ax, ay) = (a[0], a[1]);
    let (bx, by) = (b[0], b[1]);
    [f(ax, bx), f(ay, by)]
}

#[inline]
fn zip_with3<T, U, V, F>(a: Vector3<T>, b: Vector3<U>, f: F) -> Vector3<V>
where
    T: Copy,
    U: Copy,
    F: Fn(T, U) -> V,
{
    let (ax, ay, az) = (a[0], a[1], a[2]);
    let (bx, by, bz) = (b[0], b[1], b[2]);
    [f(ax, bx), f(ay, by), f(az, bz)]
}

#[inline]
fn zip_with4<T, U, V, F>(a: Vector4<T>, b: Vector4<U>, f: F) -> Vector4<V>
where
    T: Copy,
    U: Copy,
    F: Fn(T, U) -> V,
{
    let (ax, ay, az, aw) = (a[0], a[1], a[2], a[3]);
    let (bx, by, bz, bw) = (b[0], b[1], b[2], b[3]);
    [f(ax, bx), f(ay, by), f(az, bz), f(aw, bw)]
}

#[inline]
fn mul2<T>(a: Vector2<T>, b: T) -> Vector2<T>
where
    T: Copy + Mul<T, Output = T>,
{
    zip_with2(a, const2(b), Mul::mul)
}

#[inline]
fn mul3<T>(a: Vector3<T>, b: T) -> Vector3<T>
where
    T: Copy + Mul<T, Output = T>,
{
    zip_with3(a, const3(b), Mul::mul)
}

#[inline]
fn mul4<T>(a: Vector4<T>, b: T) -> Vector4<T>
where
    T: Copy + Mul<T, Output = T>,
{
    zip_with4(a, const4(b), Mul::mul)
}

#[inline]
fn const2<T: Copy>(x: T) -> Vector2<T> { [x, x] }

#[inline]
fn const3<T: Copy>(x: T) -> Vector3<T> { [x, x, x] }

#[inline]
fn const4<T: Copy>(x: T) -> Vector4<T> { [x, x, x, x] }

fn build_sources(seed: u32, octaves: usize) -> Vec<Perlin> {
    let mut sources = Vec::with_capacity(octaves);
    for x in 0..octaves {
        sources.push(Perlin::new().set_seed(seed + x as u32));
    }
    sources
}

/// Noise function that outputs hybrid Multifractal noise.
///
/// The result of this multifractal noise is that valleys in the noise should
/// have smooth bottoms at all altitudes.
#[derive(Clone, Debug)]
pub struct HybridMulti {
    /// Total number of frequency octaves to generate the noise with.
    ///
    /// The number of octaves control the _amount of detail_ in the noise
    /// function. Adding more octaves increases the detail, with the drawback
    /// of increasing the calculation time.
    pub octaves: usize,

    /// The number of cycles per unit length that the noise function outputs.
    pub frequency: f64,

    /// A multiplier that determines how quickly the frequency increases for
    /// each successive octave in the noise function.
    ///
    /// The frequency of each successive octave is equal to the product of the
    /// previous octave's frequency and the lacunarity value.
    ///
    /// A lacunarity of 2.0 results in the frequency doubling every octave. For
    /// almost all cases, 2.0 is a good value to use.
    pub lacunarity: f64,

    /// A multiplier that determines how quickly the amplitudes diminish for
    /// each successive octave in the noise function.
    ///
    /// The amplitude of each successive octave is equal to the product of the
    /// previous octave's amplitude and the persistence value. Increasing the
    /// persistence produces "rougher" noise.
    ///
    /// H = 1.0 - fractal increment = -ln(persistence) / ln(lacunarity).  For
    /// a fractal increment between 0 (inclusive) and 1 (exclusive), keep
    /// persistence between 1 / lacunarity (inclusive, for low fractal
    /// dimension) and 1 (exclusive, for high fractal dimension).
    pub persistence: f64,

    /// An offset that is added to the output of each sample of the underlying
    /// Perlin noise function.  Because each successive octave is weighted in
    /// part by the previous signal's output, increasing the offset will weight
    /// the output more heavily towards 1.0.
    pub offset: f64,

    seed: u32,
    sources: Vec<Perlin>,
}

impl HybridMulti {
    pub const DEFAULT_FREQUENCY: f64 = 2.0;
    pub const DEFAULT_LACUNARITY: f64 = /* std::f64::consts::PI * 2.0 / 3.0 */2.0;
    pub const DEFAULT_OCTAVES: usize = 6;
    pub const DEFAULT_OFFSET: f64 = /* 0.25 *//* 0.5*/ 0.7;
    // -ln(2^(-0.25))/ln(2) = 0.25
    // 2^(-0.25) ~ 13/16
    pub const DEFAULT_PERSISTENCE: f64 = /* 0.25 *//* 0.5*/ 13.0 / 16.0;
    pub const DEFAULT_SEED: u32 = 0;
    pub const MAX_OCTAVES: usize = 32;

    pub fn new() -> Self {
        Self {
            seed: Self::DEFAULT_SEED,
            octaves: Self::DEFAULT_OCTAVES,
            frequency: Self::DEFAULT_FREQUENCY,
            lacunarity: Self::DEFAULT_LACUNARITY,
            persistence: Self::DEFAULT_PERSISTENCE,
            offset: Self::DEFAULT_OFFSET,
            sources: build_sources(Self::DEFAULT_SEED, Self::DEFAULT_OCTAVES),
        }
    }

    pub fn set_offset(self, offset: f64) -> Self { Self { offset, ..self } }
}

impl Default for HybridMulti {
    fn default() -> Self { Self::new() }
}

impl MultiFractal for HybridMulti {
    fn set_octaves(self, mut octaves: usize) -> Self {
        if self.octaves == octaves {
            return self;
        }

        octaves = octaves.max(1).min(Self::MAX_OCTAVES);
        Self {
            octaves,
            sources: build_sources(self.seed, octaves),
            ..self
        }
    }

    fn set_frequency(self, frequency: f64) -> Self { Self { frequency, ..self } }

    fn set_lacunarity(self, lacunarity: f64) -> Self { Self { lacunarity, ..self } }

    fn set_persistence(self, persistence: f64) -> Self {
        Self {
            persistence,
            ..self
        }
    }
}

impl Seedable for HybridMulti {
    fn set_seed(self, seed: u32) -> Self {
        if self.seed == seed {
            return self;
        }

        Self {
            seed,
            sources: build_sources(seed, self.octaves),
            ..self
        }
    }

    fn seed(&self) -> u32 { self.seed }
}

/// 2-dimensional `HybridMulti` noise
impl NoiseFn<Point2<f64>> for HybridMulti {
    fn get(&self, mut point: Point2<f64>) -> f64 {
        // First unscaled octave of function; later octaves are scaled.
        point = mul2(point, self.frequency);
        // Offset and bias to scale into [offset - 1.0, 1.0 + offset] range.
        let bias = 1.0;
        let mut result = (self.sources[0].get(point) + self.offset) * bias * self.persistence;
        let mut exp_scale = 1.0;
        let mut scale = self.persistence;
        let mut weight = result;

        // Spectral construction inner loop, where the fractal is built.
        for x in 1..self.octaves {
            // Prevent divergence.
            weight = weight.min(1.0);

            // Raise the spatial frequency.
            point = mul2(point, self.lacunarity);

            // Get noise value, and scale it to the [offset - 1.0, 1.0 + offset] range.
            let mut signal = (self.sources[x].get(point) + self.offset) * bias;

            // Scale the amplitude appropriately for this frequency.
            exp_scale *= self.persistence;
            signal *= exp_scale;

            // Add it in, weighted by previous octave's noise value.
            result += weight * signal;

            // Update the weighting value.
            weight *= signal;
            scale += exp_scale;
        }

        // Scale the result to the [-1,1] range
        (result / scale) / bias - self.offset
    }
}

/// 3-dimensional `HybridMulti` noise
impl NoiseFn<Point3<f64>> for HybridMulti {
    fn get(&self, mut point: Point3<f64>) -> f64 {
        // First unscaled octave of function; later octaves are scaled.
        point = mul3(point, self.frequency);
        // Offset and bias to scale into [offset - 1.0, 1.0 + offset] range.
        let bias = 1.0;
        let mut result = (self.sources[0].get(point) + self.offset) * bias * self.persistence;
        let mut exp_scale = 1.0;
        let mut scale = self.persistence;
        let mut weight = result;

        // Spectral construction inner loop, where the fractal is built.
        for x in 1..self.octaves {
            // Prevent divergence.
            weight = weight.min(1.0);

            // Raise the spatial frequency.
            point = mul3(point, self.lacunarity);

            // Get noise value, and scale it to the [0, 1.0] range.
            let mut signal = (self.sources[x].get(point) + self.offset) * bias;

            // Scale the amplitude appropriately for this frequency.
            exp_scale *= self.persistence;
            signal *= exp_scale;

            // Add it in, weighted by previous octave's noise value.
            result += weight * signal;

            // Update the weighting value.
            weight *= signal;
            scale += exp_scale;
        }

        // Scale the result to the [-1,1] range
        (result / scale) / bias - self.offset
    }
}

/// 4-dimensional `HybridMulti` noise
impl NoiseFn<Point4<f64>> for HybridMulti {
    fn get(&self, mut point: Point4<f64>) -> f64 {
        // First unscaled octave of function; later octaves are scaled.
        point = mul4(point, self.frequency);
        // Offset and bias to scale into [offset - 1.0, 1.0 + offset] range.
        let bias = 1.0;
        let mut result = (self.sources[0].get(point) + self.offset) * bias * self.persistence;
        let mut exp_scale = 1.0;
        let mut scale = self.persistence;
        let mut weight = result;

        // Spectral construction inner loop, where the fractal is built.
        for x in 1..self.octaves {
            // Prevent divergence.
            weight = weight.min(1.0);

            // Raise the spatial frequency.
            point = mul4(point, self.lacunarity);

            // Get noise value, and scale it to the [0, 1.0] range.
            let mut signal = (self.sources[x].get(point) + self.offset) * bias;

            // Scale the amplitude appropriately for this frequency.
            exp_scale *= self.persistence;
            signal *= exp_scale;

            // Add it in, weighted by previous octave's noise value.
            result += weight * signal;

            // Update the weighting value.
            weight *= signal;
            scale += exp_scale;
        }

        // Scale the result to the [-1,1] range
        (result / scale) / bias - self.offset
    }
}

/// Noise function that applies a scaling factor and a bias to the output value
/// from the source function.
///
/// The function retrieves the output value from the source function, multiplies
/// it with the scaling factor, adds the bias to it, then outputs the value.
pub struct ScaleBias<'a, F: 'a> {
    /// Outputs a value.
    pub source: &'a F,

    /// Scaling factor to apply to the output value from the source function.
    /// The default value is 1.0.
    pub scale: f64,

    /// Bias to apply to the scaled output value from the source function.
    /// The default value is 0.0.
    pub bias: f64,
}

impl<'a, F> ScaleBias<'a, F> {
    pub fn new(source: &'a F) -> Self {
        ScaleBias {
            source,
            scale: 1.0,
            bias: 0.0,
        }
    }

    pub fn set_scale(self, scale: f64) -> Self { ScaleBias { scale, ..self } }

    pub fn set_bias(self, bias: f64) -> Self { ScaleBias { bias, ..self } }
}

impl<'a, F: NoiseFn<T> + 'a, T> NoiseFn<T> for ScaleBias<'a, F> {
    #[cfg(not(target_os = "emscripten"))]
    fn get(&self, point: T) -> f64 { (self.source.get(point)).mul_add(self.scale, self.bias) }

    #[cfg(target_os = "emscripten")]
    fn get(&self, point: T) -> f64 { (self.source.get(point) * self.scale) + self.bias }
}
