pub mod agent;
pub mod character_state;
pub mod controller;
mod mount;
pub mod phys;
mod projectile;
mod stats;

// External
use specs::DispatcherBuilder;

// System names
pub const CHARACTER_STATE_SYS: &str = "character_state_sys";
pub const AGENT_SYS: &str = "agent_sys";
pub const CONTROLLER_SYS: &str = "controller_sys";
pub const MOUNT_SYS: &str = "mount_sys";
pub const PHYS_SYS: &str = "phys_sys";
pub const PROJECTILE_SYS: &str = "projectile_sys";
pub const STATS_SYS: &str = "stats_sys";

pub fn add_local_systems(dispatch_builder: &mut DispatcherBuilder) {
    dispatch_builder.add(agent::Sys, AGENT_SYS, &[]);
    dispatch_builder.add(mount::Sys, MOUNT_SYS, &[AGENT_SYS]);
    dispatch_builder.add(controller::Sys, CONTROLLER_SYS, &[AGENT_SYS, MOUNT_SYS]);
    dispatch_builder.add(character_state::Sys, CHARACTER_STATE_SYS, &[CONTROLLER_SYS]);
    dispatch_builder.add(stats::Sys, STATS_SYS, &[]);
    dispatch_builder.add(phys::Sys, PHYS_SYS, &[CONTROLLER_SYS, MOUNT_SYS, STATS_SYS]);
    dispatch_builder.add(projectile::Sys, PROJECTILE_SYS, &[PHYS_SYS]);
}
