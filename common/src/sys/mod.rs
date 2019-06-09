pub mod actions;
pub mod agent;
pub mod animation;
pub mod controller;
pub mod inputs;
pub mod phys;
mod stats;

// External
use specs::DispatcherBuilder;

// System names
const AGENT_SYS: &str = "agent_sys";
const CONTROLLER_SYS: &str = "controller_sys";
const INPUTS_SYS: &str = "inputs_sys";
const ACTIONS_SYS: &str = "actions_sys";
const PHYS_SYS: &str = "phys_sys";
const ANIMATION_SYS: &str = "animation_sys";
const STATS_SYS: &str = "stats_sys";

pub fn add_local_systems(dispatch_builder: &mut DispatcherBuilder) {
    dispatch_builder.add(agent::Sys, AGENT_SYS, &[]);
    dispatch_builder.add(phys::Sys, PHYS_SYS, &[]);
    dispatch_builder.add(actions::Sys, ACTIONS_SYS, &[]);
    dispatch_builder.add(controller::Sys, CONTROLLER_SYS, &[]);
    dispatch_builder.add(inputs::Sys, INPUTS_SYS, &[]);
    dispatch_builder.add(animation::Sys, ANIMATION_SYS, &[]);
    dispatch_builder.add(stats::Sys, STATS_SYS, &[INPUTS_SYS]);
}
