use crate::vsystem::CpuTimeline;
use std::{
    collections::HashMap,
    sync::{atomic::AtomicU64, Mutex},
};

#[derive(Default)]
pub struct SysMetrics {
    pub stats: Mutex<HashMap<String, CpuTimeline>>,
    pub agent_ns: AtomicU64,
    pub mount_ns: AtomicU64,
    pub controller_ns: AtomicU64,
    pub character_behavior_ns: AtomicU64,
    pub stats_ns: AtomicU64,
    pub phys_ns: AtomicU64,
    pub projectile_ns: AtomicU64,
    pub melee_ns: AtomicU64,
}

#[derive(Default)]
pub struct PhysicsMetrics {
    pub entity_entity_collision_checks: u64,
    pub entity_entity_collisions: u64,
}
