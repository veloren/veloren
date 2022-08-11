use common::weather::CHUNKS_PER_CELL;
use common_ecs::{dispatch, System};
use common_state::State;
use specs::DispatcherBuilder;
use std::time::Duration;

use crate::sys::SysScheduler;

mod sim;
mod sync;
mod tick;

pub use sim::WeatherSim;

/// How often the weather is updated, in seconds
const WEATHER_DT: f32 = 5.0;

pub fn add_server_systems(dispatch_builder: &mut DispatcherBuilder) {
    dispatch::<tick::Sys>(dispatch_builder, &[]);
    dispatch::<sync::Sys>(dispatch_builder, &[&tick::Sys::sys_name()]);
}

#[cfg(feature = "worldgen")]
pub fn init(state: &mut State, world: &world::World) {
    let weather_size = world.sim().get_size() / CHUNKS_PER_CELL;
    let sim = WeatherSim::new(weather_size, world);
    state.ecs_mut().insert(sim);

    // NOTE: If weather computations get too heavy, this should not block the main
    // thread.
    state
        .ecs_mut()
        .insert(SysScheduler::<tick::Sys>::every(Duration::from_secs_f32(
            WEATHER_DT,
        )));
    state
        .ecs_mut()
        .insert(SysScheduler::<sync::Sys>::every(Duration::from_secs_f32(
            WEATHER_DT,
        )));
}
