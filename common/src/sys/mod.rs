pub mod agent;
mod cleanup;
pub mod combat;
pub mod controller;
pub mod movement;
pub mod phys;
mod projectile;
mod stats;

// External
use specs::DispatcherBuilder;

// System names
const AGENT_SYS: &str = "agent_sys";
const CONTROLLER_SYS: &str = "controller_sys";
const PHYS_SYS: &str = "phys_sys";
const MOVEMENT_SYS: &str = "movement_sys";
const PROJECTILE_SYS: &str = "projectile_sys";
const COMBAT_SYS: &str = "combat_sys";
const STATS_SYS: &str = "stats_sys";
const CLEANUP_SYS: &str = "cleanup_sys";

pub fn add_local_systems(dispatch_builder: &mut DispatcherBuilder) {
    dispatch_builder.add(agent::Sys, AGENT_SYS, &[]);
    dispatch_builder.add(controller::Sys, CONTROLLER_SYS, &[AGENT_SYS]);
    dispatch_builder.add(movement::Sys, MOVEMENT_SYS, &[]);
    dispatch_builder.add(combat::Sys, COMBAT_SYS, &[CONTROLLER_SYS]);
    dispatch_builder.add(stats::Sys, STATS_SYS, &[COMBAT_SYS]);
    dispatch_builder.add(
        phys::Sys,
        PHYS_SYS,
        &[CONTROLLER_SYS, MOVEMENT_SYS, COMBAT_SYS, STATS_SYS],
    );
    dispatch_builder.add(projectile::Sys, PROJECTILE_SYS, &[PHYS_SYS]);
    dispatch_builder.add(cleanup::Sys, CLEANUP_SYS, &[PHYS_SYS]);
}
