pub mod fast_noise;
pub mod gen_cache;
pub mod map_array;
pub mod map_vec;
pub mod math;
pub mod random;
pub mod sampler;
pub mod seed_expan;
pub mod small_cache;
pub mod structure;
pub mod unit_chooser;

// Reexports
pub use self::{
    fast_noise::{FastNoise, FastNoise2d},
    map_vec::MapVec,
    random::{RandomField, RandomPerm},
    sampler::{Sampler, SamplerMut},
    small_cache::SmallCache,
    structure::StructureGen2d,
    unit_chooser::UnitChooser,
};

pub use common::grid::Grid;

use fxhash::{FxHasher32, FxHasher64};
use hashbrown::{HashMap, HashSet};
use std::hash::BuildHasherDefault;
use vek::*;

// Deterministic HashMap and HashSet
pub type DHashMap<K, V> = HashMap<K, V, BuildHasherDefault<FxHasher64>>;
pub type DHashSet<T> = HashSet<T, BuildHasherDefault<FxHasher32>>;

pub fn attempt<T>(max_iters: usize, mut f: impl FnMut() -> Option<T>) -> Option<T> {
    (0..max_iters).find_map(|_| f())
}

pub fn close(x: f32, tgt: f32, falloff: f32) -> f32 {
    (1.0 - (x - tgt).abs() / falloff).max(0.0).powf(0.125)
}

pub fn close_fast(x: f32, tgt: f32, falloff: f32, falloff_strength: i32) -> f32 {
    (1.0 - ((x - tgt) / falloff).powi(falloff_strength * 2)).max(0.0)
}

pub const CARDINALS: [Vec2<i32>; 4] = [
    Vec2::new(0, 1),
    Vec2::new(1, 0),
    Vec2::new(0, -1),
    Vec2::new(-1, 0),
];

pub const DIRS: [Vec2<i32>; 8] = [
    Vec2::new(1, 0),
    Vec2::new(1, 1),
    Vec2::new(0, 1),
    Vec2::new(-1, 1),
    Vec2::new(-1, 0),
    Vec2::new(-1, -1),
    Vec2::new(0, -1),
    Vec2::new(1, -1),
];

pub const DIAGONALS: [Vec2<i32>; 4] = [
    Vec2::new(-1, -1),
    Vec2::new(1, -1),
    Vec2::new(-1, 1),
    Vec2::new(1, 1),
];

pub const NEIGHBORS: [Vec2<i32>; 8] = [
    Vec2::new(1, 0),
    Vec2::new(1, 1),
    Vec2::new(0, 1),
    Vec2::new(-1, 1),
    Vec2::new(-1, 0),
    Vec2::new(-1, -1),
    Vec2::new(0, -1),
    Vec2::new(1, -1),
];

pub const NEIGHBORS3: [Vec3<i32>; 26] = [
    Vec3::new(0, 0, -1),
    Vec3::new(0, 0, 1),
    Vec3::new(0, -1, 0),
    Vec3::new(0, -1, -1),
    Vec3::new(0, -1, 1),
    Vec3::new(0, 1, 0),
    Vec3::new(0, 1, -1),
    Vec3::new(0, 1, 1),
    Vec3::new(-1, 0, 0),
    Vec3::new(-1, 0, -1),
    Vec3::new(-1, 0, 1),
    Vec3::new(-1, -1, 0),
    Vec3::new(-1, -1, -1),
    Vec3::new(-1, -1, 1),
    Vec3::new(-1, 1, 0),
    Vec3::new(-1, 1, -1),
    Vec3::new(-1, 1, 1),
    Vec3::new(1, 0, 0),
    Vec3::new(1, 0, -1),
    Vec3::new(1, 0, 1),
    Vec3::new(1, -1, 0),
    Vec3::new(1, -1, -1),
    Vec3::new(1, -1, 1),
    Vec3::new(1, 1, 0),
    Vec3::new(1, 1, -1),
    Vec3::new(1, 1, 1),
];

pub const LOCALITY: [Vec2<i32>; 9] = [
    Vec2::new(0, 0),
    Vec2::new(0, 1),
    Vec2::new(1, 0),
    Vec2::new(0, -1),
    Vec2::new(-1, 0),
    Vec2::new(1, 1),
    Vec2::new(1, -1),
    Vec2::new(-1, 1),
    Vec2::new(-1, -1),
];

pub const CARDINAL_LOCALITY: [Vec2<i32>; 5] = [
    Vec2::new(0, 0),
    Vec2::new(0, 1),
    Vec2::new(1, 0),
    Vec2::new(0, -1),
    Vec2::new(-1, 0),
];

pub const SQUARE_4: [Vec2<i32>; 4] = [
    Vec2::new(0, 0),
    Vec2::new(1, 0),
    Vec2::new(0, 1),
    Vec2::new(1, 1),
];

pub const SQUARE_9: [Vec2<i32>; 9] = [
    Vec2::new(-1, -1),
    Vec2::new(0, -1),
    Vec2::new(1, -1),
    Vec2::new(-1, 0),
    Vec2::new(0, 0),
    Vec2::new(1, 0),
    Vec2::new(-1, 1),
    Vec2::new(0, 1),
    Vec2::new(1, 1),
];
