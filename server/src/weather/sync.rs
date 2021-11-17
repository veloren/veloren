use common_ecs::{Origin, Phase, System};
use common_net::msg::ServerGeneral;
use specs::{Join, ReadExpect, ReadStorage, Write};

use crate::{client::Client, sys::SysScheduler};

use super::sim::WeatherSim;

#[derive(Default)]
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        ReadExpect<'a, WeatherSim>,
        Write<'a, SysScheduler<Self>>,
        ReadStorage<'a, Client>,
    );

    const NAME: &'static str = "weather::sync";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(job: &mut common_ecs::Job<Self>, (sim, mut scheduler, clients): Self::SystemData) {
        if scheduler.should_run() {
            let mut lazy_msg = None;
            for client in clients.join() {
                if lazy_msg.is_none() {
                    lazy_msg = Some(
                        client.prepare(ServerGeneral::WeatherUpdate(sim.get_weather().clone())),
                    );
                }
                lazy_msg.as_ref().map(|msg| client.send_prepared(msg));
            }
        }
    }
}
