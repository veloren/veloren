use crate::system::CpuTimeline;
use std::{collections::HashMap, sync::Mutex};

#[derive(Default)]
pub struct SysMetrics {
    pub stats: Mutex<HashMap<String, CpuTimeline>>,
}

#[derive(Default)]
pub struct PhysicsMetrics {
    pub entity_entity_collision_checks: u64,
    pub entity_entity_collisions: u64,
}
