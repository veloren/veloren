use crate::sim;
use common::{terrain::TerrainChunkSize, vol::RectVolSize};
use vek::*;

/// A wrapper type that may contain a reference to a generated world. If not,
/// default values will be provided.
pub struct Land<'a> {
    sim: Option<&'a sim::WorldSim>,
}

impl<'a> Land<'a> {
    pub fn empty() -> Self { Self { sim: None } }

    pub fn from_sim(sim: &'a sim::WorldSim) -> Self { Self { sim: Some(sim) } }

    pub fn get_alt_approx(&self, wpos: Vec2<i32>) -> f32 {
        self.sim
            .and_then(|sim| sim.get_alt_approx(wpos))
            .unwrap_or(0.0)
    }

    pub fn get_gradient_approx(&self, wpos: Vec2<i32>) -> f32 {
        self.sim
            .and_then(|sim| {
                sim.get_gradient_approx(
                    wpos.map2(TerrainChunkSize::RECT_SIZE, |e, sz| e.div_euclid(sz as i32)),
                )
            })
            .unwrap_or(0.0)
    }

    pub fn get_chunk_at(&self, wpos: Vec2<i32>) -> Option<&sim::SimChunk> {
        self.sim.and_then(|sim| sim.get_wpos(wpos))
    }
}
