use common::resources::TimeOfDay;
use common_ecs::{Origin, Phase, System};
use specs::{Read, Write, WriteExpect};

use crate::sys::SysScheduler;

use super::sim::WeatherSim;

#[derive(Default)]
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Read<'a, TimeOfDay>,
        WriteExpect<'a, WeatherSim>,
        Write<'a, SysScheduler<Self>>,
    );

    const NAME: &'static str = "weather::tick";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut common_ecs::Job<Self>,
        (game_time, mut sim, mut scheduler): Self::SystemData,
    ) {
        if scheduler.should_run() {
            sim.tick(&*game_time);
        }
    }
}
