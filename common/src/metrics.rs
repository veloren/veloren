use std::sync::atomic::AtomicI64;

#[derive(Default)]
pub struct SysMetrics {
    pub agent_ns: AtomicI64,
    pub mount_ns: AtomicI64,
    pub controller_ns: AtomicI64,
    pub character_behavior_ns: AtomicI64,
    pub stats_ns: AtomicI64,
    pub phys_ns: AtomicI64,
    pub projectile_ns: AtomicI64,
    pub melee_ns: AtomicI64,
}

#[derive(Default)]
pub struct PhysicsMetrics {
    pub entity_entity_collision_checks: i64,
    pub entity_entity_collisions: i64,
}
