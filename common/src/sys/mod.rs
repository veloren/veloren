pub mod agent;
pub mod control;
pub mod phys;
pub mod action_state;
mod stats;

// External
use specs::DispatcherBuilder;

// System names
const AGENT_SYS: &str = "agent_sys";
const CONTROL_SYS: &str = "control_sys";
const MOVEMENT_SYS: &str = "movement_sys";
const ACTION_STATE_SYS: &str = "action_state_sys";
const STATS_SYS: &str = "stats_sys";

pub fn add_local_systems(dispatch_builder: &mut DispatcherBuilder) {
    dispatch_builder.add(agent::Sys, AGENT_SYS, &[]);
    dispatch_builder.add(control::Sys, CONTROL_SYS, &[]);
    dispatch_builder.add(phys::Sys, MOVEMENT_SYS, &[]);
    dispatch_builder.add(action_state::Sys, ACTION_STATE_SYS, &[]);
    dispatch_builder.add(stats::Sys, STATS_SYS, &[ACTION_STATE_SYS]);
}
