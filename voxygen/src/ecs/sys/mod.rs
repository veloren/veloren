pub mod floater;
mod interpolation;

use common_ecs::{dispatch, System};
use specs::DispatcherBuilder;

pub fn add_local_systems(dispatch_builder: &mut DispatcherBuilder) {
    dispatch::<interpolation::Sys>(dispatch_builder, &[&common_systems::phys::Sys::sys_name()]);
    dispatch::<floater::Sys>(dispatch_builder, &[&interpolation::Sys::sys_name()]);
}
