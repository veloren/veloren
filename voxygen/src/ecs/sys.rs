pub mod floater;
mod interpolation;

use specs::DispatcherBuilder;

// System names
const FLOATER_SYS: &str = "floater_voxygen_sys";
const INTERPOLATION_SYS: &str = "interpolation_voxygen_sys";

pub fn add_local_systems(dispatch_builder: &mut DispatcherBuilder) {
    dispatch_builder.add(interpolation::Sys, INTERPOLATION_SYS, &[
        common::sys::PHYS_SYS,
    ]);
    dispatch_builder.add(floater::Sys, FLOATER_SYS, &[INTERPOLATION_SYS]);
}
