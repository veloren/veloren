#![feature(label_break_value, bool_to_option, option_unwrap_none)]

mod aura;
mod beam;
mod buff;
pub mod character_behavior;
pub mod controller;
pub mod melee;
mod mount;
pub mod phys;
#[cfg(feature = "plugins")] pub mod plugin;
mod projectile;
mod shockwave;
pub mod state;
mod stats;

// External
use common::vsystem::{dispatch, VSystem};
use specs::DispatcherBuilder;

pub fn add_local_systems(dispatch_builder: &mut DispatcherBuilder) {
    dispatch::<mount::Sys>(dispatch_builder, &[]);
    dispatch::<controller::Sys>(dispatch_builder, &[&mount::Sys::sys_name()]);
    dispatch::<character_behavior::Sys>(dispatch_builder, &[&controller::Sys::sys_name()]);
    dispatch::<stats::Sys>(dispatch_builder, &[]);
    dispatch::<buff::Sys>(dispatch_builder, &[]);
    dispatch::<phys::Sys>(dispatch_builder, &[
        &controller::Sys::sys_name(),
        &mount::Sys::sys_name(),
        &stats::Sys::sys_name(),
    ]);
    dispatch::<projectile::Sys>(dispatch_builder, &[&phys::Sys::sys_name()]);
    dispatch::<shockwave::Sys>(dispatch_builder, &[&phys::Sys::sys_name()]);
    dispatch::<beam::Sys>(dispatch_builder, &[&phys::Sys::sys_name()]);
    dispatch::<melee::Sys>(dispatch_builder, &[&projectile::Sys::sys_name()]);
    dispatch::<aura::Sys>(dispatch_builder, &[]);
}
