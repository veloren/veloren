use common::{
    event::EventBus,
    grid::Grid,
    outcome::Outcome,
    resources::TimeOfDay,
    weather::{Weather, WeatherGrid, CELL_SIZE, CHUNKS_PER_CELL},
};
use noise::{NoiseFn, SuperSimplex, Turbulence};
use rand::prelude::*;
use vek::*;
use world::World;

use crate::weather::WEATHER_DT;

fn cell_to_wpos_center(p: Vec2<i32>) -> Vec2<i32> { p * CELL_SIZE as i32 + CELL_SIZE as i32 / 2 }

#[derive(Clone)]
struct WeatherZone {
    weather: Weather,
    /// Time, in seconds this zone lives.
    time_to_live: f32,
}

struct CellConsts {
    humidity: f32,
}

pub struct WeatherSim {
    size: Vec2<u32>,
    consts: Grid<CellConsts>,
    zones: Grid<Option<WeatherZone>>,
}

impl WeatherSim {
    pub fn new(size: Vec2<u32>, world: &World) -> Self {
        Self {
            size,
            consts: Grid::from_raw(
                size.as_(),
                (0..size.x * size.y)
                    .map(|i| Vec2::new(i % size.x, i / size.x))
                    .map(|p| {
                        let mut humid_sum = 0.0;

                        for y in 0..CHUNKS_PER_CELL {
                            for x in 0..CHUNKS_PER_CELL {
                                let chunk_pos = p * CHUNKS_PER_CELL + Vec2::new(x, y);
                                if let Some(chunk) = world.sim().get(chunk_pos.as_()) {
                                    let env = chunk.get_environment();
                                    humid_sum += env.humid;
                                }
                            }
                        }
                        let average_humid = humid_sum / (CHUNKS_PER_CELL * CHUNKS_PER_CELL) as f32;
                        CellConsts {
                            humidity: average_humid.powf(0.2).min(1.0),
                        }
                    })
                    .collect::<Vec<_>>(),
            ),
            zones: Grid::new(size.as_(), None),
        }
    }

    /// Adds a weather zone as a circle at a position, with a given radius. Both
    /// of which should be in weather cell units
    pub fn add_zone(&mut self, weather: Weather, pos: Vec2<f32>, radius: f32, time: f32) {
        let min: Vec2<i32> = (pos - radius).as_::<i32>().map(|e| e.max(0));
        let max: Vec2<i32> = (pos + radius)
            .ceil()
            .as_::<i32>()
            .map2(self.size.as_::<i32>(), |a, b| a.min(b));
        for y in min.y..max.y {
            for x in min.x..max.x {
                let ipos = Vec2::new(x, y);
                let p = ipos.as_::<f32>();

                if p.distance_squared(pos) < radius.powi(2) {
                    self.zones[ipos] = Some(WeatherZone {
                        weather,
                        time_to_live: time,
                    });
                }
            }
        }
    }

    // Time step is cell size / maximum wind speed
    pub fn tick(
        &mut self,
        time_of_day: &TimeOfDay,
        outcomes: &EventBus<Outcome>,
        out: &mut WeatherGrid,
        world: &World,
    ) {
        let time = time_of_day.0;

        let base_nz = Turbulence::new(
            Turbulence::new(SuperSimplex::new())
                .set_frequency(0.2)
                .set_power(1.5),
        )
        .set_frequency(2.0)
        .set_power(0.2);

        let rain_nz = SuperSimplex::new();

        for (point, cell) in out.iter_mut() {
            if let Some(zone) = &mut self.zones[point] {
                *cell = zone.weather;
                zone.time_to_live -= WEATHER_DT;
                if zone.time_to_live <= 0.0 {
                    self.zones[point] = None;
                }
            } else {
                let wpos = cell_to_wpos_center(point);

                let pos = wpos.as_::<f64>() + time * 0.1;

                let space_scale = 7_500.0;
                let time_scale = 100_000.0;
                let spos = (pos / space_scale).with_z(time / time_scale);

                let avg_scale = 20_000.0;
                let avg_delay = 250_000.0;
                let pressure = ((base_nz
                    .get((pos / avg_scale).with_z(time / avg_delay).into_array())
                    + base_nz.get(
                        (pos / (avg_scale * 0.25))
                            .with_z(time / (avg_delay * 0.25))
                            .into_array(),
                    ) * 0.5)
                    * 0.5
                    + 1.0)
                    .clamped(0.0, 1.0) as f32
                    + 0.55
                    - self.consts[point].humidity * 0.6;

                const RAIN_CLOUD_THRESHOLD: f32 = 0.25;
                cell.cloud = (1.0 - pressure).max(0.0) * 0.5;
                cell.rain = ((1.0 - pressure - RAIN_CLOUD_THRESHOLD).max(0.0)
                    * self.consts[point].humidity
                    * 2.5)
                    .powf(0.75);
                cell.wind = Vec2::new(
                    rain_nz.get(spos.into_array()).powi(3) as f32,
                    rain_nz.get((spos + 1.0).into_array()).powi(3) as f32,
                ) * 200.0
                    * (1.0 - pressure);

                if cell.rain > 0.2 && cell.cloud > 0.15 && thread_rng().gen_bool(0.01) {
                    let wpos = wpos.map(|e| {
                        e as f32 + thread_rng().gen_range(-1.0..1.0) * CELL_SIZE as f32 * 0.5
                    });
                    outcomes.emit_now(Outcome::Lightning {
                        pos: wpos.with_z(world.sim().get_alt_approx(wpos.as_()).unwrap_or(0.0)),
                    });
                }
            }
        }
    }

    pub fn size(&self) -> Vec2<u32> { self.size }
}
