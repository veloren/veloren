#![feature(label_break_value, bool_to_option)]

pub mod agent;
mod aura;
mod beam;
mod buff;
pub mod character_behavior;
pub mod controller;
pub mod melee;
mod mount;
pub mod phys;
mod projectile;
mod shockwave;
pub mod state;
mod stats;

// External
use specs::DispatcherBuilder;

// System names
pub const CHARACTER_BEHAVIOR_SYS: &str = "character_behavior_sys";
pub const MELEE_SYS: &str = "melee_sys";
pub const AGENT_SYS: &str = "agent_sys";
pub const BEAM_SYS: &str = "beam_sys";
pub const CONTROLLER_SYS: &str = "controller_sys";
pub const MOUNT_SYS: &str = "mount_sys";
pub const PHYS_SYS: &str = "phys_sys";
pub const PROJECTILE_SYS: &str = "projectile_sys";
pub const SHOCKWAVE_SYS: &str = "shockwave_sys";
pub const STATS_SYS: &str = "stats_sys";
pub const BUFFS_SYS: &str = "buffs_sys";
pub const AURAS_SYS: &str = "auras_sys";

pub fn add_local_systems(dispatch_builder: &mut DispatcherBuilder) {
    dispatch_builder.add(agent::Sys, AGENT_SYS, &[]);
    dispatch_builder.add(mount::Sys, MOUNT_SYS, &[AGENT_SYS]);
    dispatch_builder.add(controller::Sys, CONTROLLER_SYS, &[AGENT_SYS, MOUNT_SYS]);
    dispatch_builder.add(character_behavior::Sys, CHARACTER_BEHAVIOR_SYS, &[
        CONTROLLER_SYS,
    ]);
    dispatch_builder.add(stats::Sys, STATS_SYS, &[]);
    dispatch_builder.add(buff::Sys, BUFFS_SYS, &[]);
    dispatch_builder.add(phys::Sys, PHYS_SYS, &[CONTROLLER_SYS, MOUNT_SYS, STATS_SYS]);
    dispatch_builder.add(projectile::Sys, PROJECTILE_SYS, &[PHYS_SYS]);
    dispatch_builder.add(shockwave::Sys, SHOCKWAVE_SYS, &[PHYS_SYS]);
    dispatch_builder.add(beam::Sys, BEAM_SYS, &[PHYS_SYS]);
    dispatch_builder.add(melee::Sys, MELEE_SYS, &[PROJECTILE_SYS]);
    dispatch_builder.add(aura::Sys, AURAS_SYS, &[]);
}
