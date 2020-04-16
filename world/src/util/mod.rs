pub mod fast_noise;
pub mod grid;
pub mod hash_cache;
pub mod random;
pub mod sampler;
pub mod seed_expan;
pub mod small_cache;
pub mod structure;
pub mod unit_chooser;

// Reexports
pub use self::{
    fast_noise::FastNoise,
    grid::Grid,
    hash_cache::HashCache,
    random::{RandomField, RandomPerm},
    sampler::{Sampler, SamplerMut},
    small_cache::SmallCache,
    structure::StructureGen2d,
    unit_chooser::UnitChooser,
};

pub fn attempt<T>(max_iters: usize, mut f: impl FnMut() -> Option<T>) -> Option<T> {
    (0..max_iters).find_map(|_| f())
}
