use common::{
    comp,
    event::EventBus,
    outcome::Outcome,
    resources::{DeltaTime, ProgramTime, TimeOfDay},
    slowjob::{SlowJob, SlowJobPool},
    weather::{SharedWeatherGrid, WeatherGrid},
};
use common_ecs::{Origin, Phase, System};
use common_net::msg::ServerGeneral;
use rand::{seq::SliceRandom, thread_rng, Rng};
use specs::{Join, Read, ReadExpect, ReadStorage, Write, WriteExpect};
use std::{mem, sync::Arc};
use world::World;

use crate::client::Client;

use super::{
    sim::{LightningCells, WeatherSim},
    WEATHER_DT,
};

enum WeatherJobState {
    Working(SlowJob),
    Idle(WeatherSim),
    None,
}

pub struct WeatherJob {
    last_update: ProgramTime,
    weather_tx: crossbeam_channel::Sender<(WeatherGrid, LightningCells, WeatherSim)>,
    weather_rx: crossbeam_channel::Receiver<(WeatherGrid, LightningCells, WeatherSim)>,
    state: WeatherJobState,
}

#[derive(Default)]
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Read<'a, TimeOfDay>,
        Read<'a, ProgramTime>,
        Read<'a, DeltaTime>,
        Write<'a, LightningCells>,
        Write<'a, Option<WeatherJob>>,
        WriteExpect<'a, WeatherGrid>,
        WriteExpect<'a, SlowJobPool>,
        ReadExpect<'a, EventBus<Outcome>>,
        ReadExpect<'a, Arc<World>>,
        ReadStorage<'a, Client>,
        ReadStorage<'a, comp::Pos>,
    );

    const NAME: &'static str = "weather::tick";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut common_ecs::Job<Self>,
        (
            game_time,
            program_time,
            delta_time,
            mut lightning_cells,
            mut weather_job,
            mut grid,
            slow_job_pool,
            outcomes,
            world,
            clients,
            positions,
        ): Self::SystemData,
    ) {
        if let Some(weather_job) = match &mut *weather_job {
            Some(weather_job) => (program_time.0 - weather_job.last_update.0 >= WEATHER_DT as f64)
                .then_some(weather_job),
            None => {
                let (weather_tx, weather_rx) = crossbeam_channel::bounded(1);

                let weather_size = world.sim().get_size() / common::weather::CHUNKS_PER_CELL;
                let mut sim = WeatherSim::new(weather_size, &world);
                *grid = WeatherGrid::new(sim.size());
                *lightning_cells = sim.tick(*game_time, &mut grid);

                *weather_job = Some(WeatherJob {
                    last_update: *program_time,
                    weather_tx,
                    weather_rx,
                    state: WeatherJobState::Idle(sim),
                });

                None
            },
        } {
            if matches!(weather_job.state, WeatherJobState::Working(_))
            && let Ok((new_grid, new_lightning_cells, sim)) = weather_job.weather_rx.try_recv() {
                *grid = new_grid;
                *lightning_cells = new_lightning_cells;
                let mut lazy_msg = None;
                for client in clients.join() {
                    if lazy_msg.is_none() {
                        lazy_msg = Some(client.prepare(ServerGeneral::WeatherUpdate(
                            SharedWeatherGrid::from(&*grid),
                        )));
                    }
                    lazy_msg.as_ref().map(|msg| client.send_prepared(msg));
                }
                weather_job.state = WeatherJobState::Idle(sim);

            }

            if matches!(weather_job.state, WeatherJobState::Idle(_)) {
                let old_state = mem::replace(&mut weather_job.state, WeatherJobState::None);

                let WeatherJobState::Idle(mut sim) = old_state else {
                    unreachable!()
                };

                let weather_tx = weather_job.weather_tx.clone();
                let game_time = *game_time;
                let job = slow_job_pool.spawn("WEATHER", move || {
                    let mut grid = WeatherGrid::new(sim.size());
                    let lightning_cells = sim.tick(game_time, &mut grid);
                    weather_tx
                        .send((grid, lightning_cells, sim))
                        .expect("We should never send more than 1 of these.")
                });

                weather_job.state = WeatherJobState::Working(job);
            }
        }

        let mut outcome_emitter = outcomes.emitter();
        let mut rng = thread_rng();
        let num_cells = lightning_cells.cells.len() as f64 * 0.002 * delta_time.0 as f64;
        let num_cells = num_cells.floor() as u32 + rng.gen_bool(num_cells.fract()) as u32;

        for _ in 0..num_cells {
            let cell_pos = lightning_cells.cells.choose(&mut rng).expect(
                "This is non-empty, since we multiply with its len for the chance to do a \
                 lightning strike.",
            );
            let wpos = cell_pos.map(|e| {
                (e as f32 + thread_rng().gen_range(0.0..1.0)) * common::weather::CELL_SIZE as f32
            });
            outcome_emitter.emit(Outcome::Lightning {
                pos: wpos.with_z(world.sim().get_alt_approx(wpos.as_()).unwrap_or(0.0)),
            });
        }

        for (client, pos) in (&clients, &positions).join() {
            let weather = grid.get_interpolated(pos.0.xy());
            client.send_fallible(ServerGeneral::LocalWeatherUpdate(weather));
        }
    }
}
