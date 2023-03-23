use serde::{Deserialize, Serialize};

/// Per-server constant data (configs) that stays the same for the server's
/// life.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerConstants {
    pub day_cycle_coefficient: f64,
}
impl Default for ServerConstants {
    fn default() -> Self {
        ServerConstants {
            // == 30.0 via server settings (the default)
            day_cycle_coefficient: 24.0 * 2.0,
        }
    }
}
