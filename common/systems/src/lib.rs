#![feature(btree_drain_filter)]
#![allow(clippy::option_map_unit_fn)]

mod aura;
mod beam;
mod buff;
pub mod character_behavior;
pub mod controller;
mod interpolation;
pub mod melee;
mod mount;
pub mod phys;
pub mod projectile;
mod shockwave;
mod stats;

// External
use common_ecs::{dispatch, System};
use specs::DispatcherBuilder;

pub fn add_local_systems(dispatch_builder: &mut DispatcherBuilder) {
    //TODO: don't run interpolation on server
    dispatch::<interpolation::Sys>(dispatch_builder, &[]);
    dispatch::<mount::Sys>(dispatch_builder, &[]);
    dispatch::<controller::Sys>(dispatch_builder, &[&mount::Sys::sys_name()]);
    dispatch::<character_behavior::Sys>(dispatch_builder, &[&controller::Sys::sys_name()]);
    dispatch::<buff::Sys>(dispatch_builder, &[]);
    dispatch::<stats::Sys>(dispatch_builder, &[&buff::Sys::sys_name()]);
    dispatch::<phys::Sys>(dispatch_builder, &[
        &interpolation::Sys::sys_name(),
        &controller::Sys::sys_name(),
        &mount::Sys::sys_name(),
        &stats::Sys::sys_name(),
    ]);
    dispatch::<projectile::Sys>(dispatch_builder, &[&phys::Sys::sys_name()]);
    dispatch::<shockwave::Sys>(dispatch_builder, &[&phys::Sys::sys_name()]);
    dispatch::<beam::Sys>(dispatch_builder, &[&phys::Sys::sys_name()]);
    dispatch::<aura::Sys>(dispatch_builder, &[]);
}
