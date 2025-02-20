use crate::{
    ColumnSample, IndexRef,
    column::ColumnGen,
    sim::{self, SimChunk},
    util::Sampler,
};
use common::{terrain::TerrainChunkSize, vol::RectVolSize};
use vek::*;

/// A wrapper type that may contain a reference to a generated world. If not,
/// default values will be provided.
pub struct Land<'a> {
    sim: Option<&'a sim::WorldSim>,
}

impl<'a> Land<'a> {
    pub fn empty() -> Self { Self { sim: None } }

    pub fn size(&self) -> Vec2<u32> { self.sim.map_or(Vec2::one(), |s| s.get_size()) }

    pub fn from_sim(sim: &'a sim::WorldSim) -> Self { Self { sim: Some(sim) } }

    pub fn get_interpolated<T>(&self, wpos: Vec2<i32>, f: impl FnMut(&SimChunk) -> T) -> T
    where
        T: Copy + Default + std::ops::Add<Output = T> + std::ops::Mul<f32, Output = T>,
    {
        self.sim
            .and_then(|sim| sim.get_interpolated(wpos, f))
            .unwrap_or_default()
    }

    /// See `WorldSim::get_surface_alt_approx`.
    pub fn get_surface_alt_approx(&self, wpos: Vec2<i32>) -> f32 {
        self.sim
            .map(|sim| sim.get_surface_alt_approx(wpos))
            .unwrap_or(0.0)
    }

    pub fn get_alt_approx(&self, wpos: Vec2<i32>) -> f32 {
        self.sim
            .and_then(|sim| sim.get_alt_approx(wpos))
            .unwrap_or(0.0)
    }

    pub fn get_downhill(&self, wpos: Vec2<i32>) -> Vec2<i32> {
        self.sim
            .and_then(|sim| sim.get_wpos(wpos))
            .and_then(|c| c.downhill)
            .unwrap_or(Vec2::zero())
    }

    pub fn get_gradient_approx(&self, wpos: Vec2<i32>) -> f32 {
        self.sim
            .and_then(|sim| sim.get_gradient_approx(self.wpos_chunk_pos(wpos)))
            .unwrap_or(0.0)
    }

    pub fn wpos_chunk_pos(&self, wpos: Vec2<i32>) -> Vec2<i32> {
        wpos.map2(TerrainChunkSize::RECT_SIZE, |e, sz| e.div_euclid(sz as i32))
    }

    pub fn get_chunk(&self, chunk_pos: Vec2<i32>) -> Option<&sim::SimChunk> {
        self.sim.and_then(|sim| sim.get(chunk_pos))
    }

    pub fn get_chunk_wpos(&self, wpos: Vec2<i32>) -> Option<&sim::SimChunk> {
        self.sim.and_then(|sim| sim.get_wpos(wpos))
    }

    pub fn get_nearest_path(
        &self,
        wpos: Vec2<i32>,
    ) -> Option<(f32, Vec2<f32>, sim::Path, Vec2<f32>)> {
        self.sim.and_then(|sim| sim.get_nearest_path(wpos))
    }

    pub fn column_sample<'sample>(
        &'sample self,
        wpos: Vec2<i32>,
        index: IndexRef<'sample>,
    ) -> Option<ColumnSample<'sample>> {
        self.sim
            .and_then(|sim| ColumnGen::new(sim).get((wpos, index, None)))
    }
}
