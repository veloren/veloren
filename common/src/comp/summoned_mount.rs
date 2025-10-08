use serde::{Deserialize, Serialize};

/// Marker attached to a spawned, single-use mount so we can clean it up on dismount.
/// No ECS types here, so this comp can live in `common` without extra deps.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct SummonedMount;
