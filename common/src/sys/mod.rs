pub mod agent;
pub mod anim;
pub mod control;
pub mod phys;

// External
use specs::DispatcherBuilder;

// System names
const AGENT_SYS: &str = "agent_sys";
const CONTROL_SYS: &str = "control_sys";
const PHYS_SYS: &str = "phys_sys";
const ANIM_SYS: &str = "anim_sys";

pub fn add_local_systems(dispatch_builder: &mut DispatcherBuilder) {
    dispatch_builder.add(agent::Sys, AGENT_SYS, &[]);
    dispatch_builder.add(phys::Sys, PHYS_SYS, &[]);
    dispatch_builder.add(control::Sys, CONTROL_SYS, &["phys_sys"]);
    dispatch_builder.add(anim::Sys, ANIM_SYS, &[]);
}
