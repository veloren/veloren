#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct ViewDistances {
    pub terrain: u32,
    /// Server will clamp this to `terrain` if it is larger.
    ///
    /// NOTE: Importantly, the server still loads entities in the `terrain` view
    /// distance (at least currently, please update this if you change it!),
    /// but the syncing to the client is done based on the entity view
    /// distance.
    pub entity: u32,
}

impl ViewDistances {
    /// Clamps the terrain view distance to an optional max and clamps the
    /// entity view distance to the resulting terrain view distance.
    ///
    /// Also ensures both are at a minimum of 1 (unless the provided max is 0).
    pub fn clamp(self, max: Option<u32>) -> Self {
        let terrain = self.terrain.clamp(1, max.unwrap_or(u32::MAX));
        Self {
            terrain,
            entity: self.entity.clamp(1, terrain),
        }
    }
}
