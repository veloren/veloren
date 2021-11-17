use std::sync::Arc;

use common::resources::TimeOfDay;
use common_ecs::{Origin, Phase, System};
use specs::{Read, ReadExpect, Write, WriteExpect};

use crate::sys::SysScheduler;

use super::sim::WeatherSim;

#[derive(Default)]
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Read<'a, TimeOfDay>,
        ReadExpect<'a, Arc<world::World>>,
        WriteExpect<'a, WeatherSim>,
        Write<'a, SysScheduler<Self>>,
    );

    const NAME: &'static str = "weather::tick";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        job: &mut common_ecs::Job<Self>,
        (game_time, world, mut sim, mut scheduler): Self::SystemData,
    ) {
        if scheduler.should_run() {
            sim.tick(&*world, &*game_time, 1.0);
        }
    }
}
