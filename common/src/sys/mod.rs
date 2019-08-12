mod agent;
mod animation;
mod combat;
mod controller;
mod movement;
mod phys;
mod stats;

// External
use specs::DispatcherBuilder;

// System names
const AGENT_SYS: &str = "agent_sys";
const CONTROLLER_SYS: &str = "controller_sys";
const PHYS_SYS: &str = "phys_sys";
const MOVEMENT_SYS: &str = "movement_sys";
const COMBAT_SYS: &str = "combat_sys";
const ANIMATION_SYS: &str = "animation_sys";
const STATS_SYS: &str = "stats_sys";

pub fn add_local_systems(dispatch_builder: &mut DispatcherBuilder) {
    dispatch_builder.add(agent::Sys, AGENT_SYS, &[]);
    dispatch_builder.add(controller::Sys, CONTROLLER_SYS, &[AGENT_SYS]);
    dispatch_builder.add(movement::Sys, MOVEMENT_SYS, &[CONTROLLER_SYS]);
    dispatch_builder.add(phys::Sys, PHYS_SYS, &[MOVEMENT_SYS]);
    dispatch_builder.add(combat::Sys, COMBAT_SYS, &[PHYS_SYS]);
    dispatch_builder.add(animation::Sys, ANIMATION_SYS, &[PHYS_SYS]);
    dispatch_builder.add(stats::Sys, STATS_SYS, &[COMBAT_SYS]);
}
