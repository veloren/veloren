use serde::{Deserialize, Serialize};

/// A resource that stores the time of day.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, Default)]
pub struct TimeOfDay(pub f64);

/// A resource that stores the tick (i.e: physics) time.
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct Time(pub f64);

/// A resource that stores the time since the previous tick.
#[derive(Default)]
pub struct DeltaTime(pub f32);
