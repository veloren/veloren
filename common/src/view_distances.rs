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
    pub fn clamp(self, max: Option<u32>) -> Self {
        let terrain = max.unwrap_or(u32::MAX).min(self.terrain);
        Self {
            terrain,
            entity: self.entity.min(terrain),
        }
    }
}
