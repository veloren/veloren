use crate::util::SpatialGrid;

/// Cached [`SpatialGrid`] for reuse within different ecs systems during a tick.
/// This is used to accelerate queries on entities within a specific area.
/// Updated within the physics system [`crate::sys::phys::Sys`] after new entity
/// positions are calculated for the tick. So any position modifications outside
/// the physics system will not be reflected here until the next tick when the
/// physics system runs.
pub struct CachedSpatialGrid(pub SpatialGrid);

impl Default for CachedSpatialGrid {
    fn default() -> Self {
        let lg2_cell_size = 5; // 32
        let lg2_large_cell_size = 6; // 64
        let radius_cutoff = 8;

        let spatial_grid = SpatialGrid::new(lg2_cell_size, lg2_large_cell_size, radius_cutoff);

        Self(spatial_grid)
    }
}
