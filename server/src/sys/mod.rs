pub mod agent;
pub mod entity_sync;
pub mod invite_timeout;
pub mod metrics;
pub mod msg;
pub mod object;
pub mod persistence;
pub mod sentinel;
pub mod subscription;
pub mod terrain;
pub mod terrain_sync;
pub mod waypoint;

use common_ecs::{dispatch, run_now};
use specs::DispatcherBuilder;
use std::{
    marker::PhantomData,
    time::{Duration, Instant},
};

pub type PersistenceScheduler = SysScheduler<persistence::Sys>;

pub fn add_server_systems(dispatch_builder: &mut DispatcherBuilder) {
    dispatch::<agent::Sys>(dispatch_builder, &[]);
    dispatch::<terrain::Sys>(dispatch_builder, &[]);
    dispatch::<waypoint::Sys>(dispatch_builder, &[]);
    dispatch::<invite_timeout::Sys>(dispatch_builder, &[]);
    dispatch::<persistence::Sys>(dispatch_builder, &[]);
    dispatch::<object::Sys>(dispatch_builder, &[]);
}

pub fn run_sync_systems(ecs: &mut specs::World) {
    // Setup for entity sync
    // If I'm not mistaken, these two could be ran in parallel
    run_now::<sentinel::Sys>(ecs);
    run_now::<subscription::Sys>(ecs);

    // Sync
    run_now::<terrain_sync::Sys>(ecs);
    run_now::<entity_sync::Sys>(ecs);
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
