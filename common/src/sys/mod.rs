pub mod agent;
pub mod anim;
pub mod control;
pub mod phys;

// External
use specs::DispatcherBuilder;

// System names
const AGENT_SYS: &str = "agent_sys";
const CONTROL_SYS: &str = "control_sys";
const MOVEMENT_SYS: &str = "movement_sys";
const ANIM_SYS: &str = "anim_sys";

pub fn add_local_systems(dispatch_builder: &mut DispatcherBuilder) {
    dispatch_builder.add(agent::Sys, AGENT_SYS, &[]);
    dispatch_builder.add(control::Sys, CONTROL_SYS, &[]);
    dispatch_builder.add(phys::Sys, MOVEMENT_SYS, &[]);
    dispatch_builder.add(anim::Sys, ANIM_SYS, &[]);
}
