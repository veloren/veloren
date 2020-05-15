pub mod entity_sync;
pub mod message;
pub mod persistence;
pub mod sentinel;
pub mod subscription;
pub mod terrain;
pub mod terrain_sync;
pub mod waypoint;

use specs::DispatcherBuilder;
use std::{
    marker::PhantomData,
    time::{Duration, Instant},
};

pub type EntitySyncTimer = SysTimer<entity_sync::Sys>;
pub type MessageTimer = SysTimer<message::Sys>;
pub type SentinelTimer = SysTimer<sentinel::Sys>;
pub type SubscriptionTimer = SysTimer<subscription::Sys>;
pub type TerrainTimer = SysTimer<terrain::Sys>;
pub type TerrainSyncTimer = SysTimer<terrain_sync::Sys>;
pub type WaypointTimer = SysTimer<waypoint::Sys>;
pub type StatsPersistenceTimer = SysTimer<persistence::stats::Sys>;
pub type StatsPersistenceScheduler = SysScheduler<persistence::stats::Sys>;

// System names
// Note: commented names may be useful in the future
//const ENTITY_SYNC_SYS: &str = "server_entity_sync_sys";
//const SENTINEL_SYS: &str = "sentinel_sys";
//const SUBSCRIPTION_SYS: &str = "server_subscription_sys";
//const TERRAIN_SYNC_SYS: &str = "server_terrain_sync_sys";
const TERRAIN_SYS: &str = "server_terrain_sys";
const WAYPOINT_SYS: &str = "waypoint_sys";
const STATS_PERSISTENCE_SYS: &str = "stats_persistence_sys";

pub fn add_server_systems(dispatch_builder: &mut DispatcherBuilder) {
    dispatch_builder.add(terrain::Sys, TERRAIN_SYS, &[]);
    dispatch_builder.add(waypoint::Sys, WAYPOINT_SYS, &[]);
    dispatch_builder.add(persistence::stats::Sys, STATS_PERSISTENCE_SYS, &[]);
}

pub fn run_sync_systems(ecs: &mut specs::World) {
    use specs::RunNow;

    // Setup for entity sync
    // If I'm not mistaken, these two could be ran in parallel
    sentinel::Sys.run_now(ecs);
    subscription::Sys.run_now(ecs);

    // Sync
    terrain_sync::Sys.run_now(ecs);
    entity_sync::Sys.run_now(ecs);
}

/// Used to schedule systems to run at an interval
pub struct SysScheduler<S> {
    interval: Duration,
    last_run: Instant,
    _phantom: PhantomData<S>,
}

impl<S> SysScheduler<S> {
    pub fn every(interval: Duration) -> Self {
        Self {
            interval,
            last_run: Instant::now(),
            _phantom: PhantomData,
        }
    }

    pub fn should_run(&mut self) -> bool {
        if self.last_run.elapsed() > self.interval {
            self.last_run = Instant::now();

            true
        } else {
            false
        }
    }
}

impl<S> Default for SysScheduler<S> {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(30),
            last_run: Instant::now(),
            _phantom: PhantomData,
        }
    }
}

/// Used to keep track of how much time each system takes
pub struct SysTimer<S> {
    pub nanos: u64,
    start: Option<Instant>,
    _phantom: PhantomData<S>,
}

impl<S> SysTimer<S> {
    pub fn start(&mut self) {
        if self.start.is_some() {
            panic!("Timer already started");
        }
        self.start = Some(Instant::now());
    }

    pub fn end(&mut self) {
        self.nanos = self
            .start
            .take()
            .expect("Timer ended without starting it")
            .elapsed()
            .as_nanos() as u64;
    }
}

impl<S> Default for SysTimer<S> {
    fn default() -> Self {
        Self {
            nanos: 0,
            start: None,
            _phantom: PhantomData,
        }
    }
}
