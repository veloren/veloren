use common::weather::WeatherGrid;
use common_ecs::{Origin, Phase, System};
use common_net::msg::ServerGeneral;
use specs::{Join, ReadExpect, ReadStorage, Write};

use crate::{client::Client, sys::SysScheduler};

#[derive(Default)]
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        ReadExpect<'a, WeatherGrid>,
        Write<'a, SysScheduler<Self>>,
        ReadStorage<'a, Client>,
    );

    const NAME: &'static str = "weather::sync";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut common_ecs::Job<Self>,
        (weather_grid, mut scheduler, clients): Self::SystemData,
    ) {
        if scheduler.should_run() {
            let mut lazy_msg = None;
            for client in clients.join() {
                if lazy_msg.is_none() {
                    lazy_msg =
                        Some(client.prepare(ServerGeneral::WeatherUpdate(weather_grid.clone())));
                }
                lazy_msg.as_ref().map(|msg| client.send_prepared(msg));
            }
        }
    }
}
