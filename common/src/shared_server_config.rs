use serde::{Serialize, Deserialize};

/// Per-server constant data (configs) that stays the same for the server's life.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerConstants {
    pub day_cycle_coefficient: f64,
}