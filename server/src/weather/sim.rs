use common::{
    resources::TimeOfDay,
    weather::{WeatherGrid, CELL_SIZE},
};
use noise::{NoiseFn, SuperSimplex, Turbulence};
use vek::*;
use world::World;

fn cell_to_wpos(p: Vec2<i32>) -> Vec2<i32> { p * CELL_SIZE as i32 }

pub struct WeatherSim {
    size: Vec2<u32>,
}

impl WeatherSim {
    pub fn new(size: Vec2<u32>, _world: &World) -> Self { Self { size } }

    // Time step is cell size / maximum wind speed
    pub fn tick(&mut self, time_of_day: &TimeOfDay, out: &mut WeatherGrid) {
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
            let wpos = cell_to_wpos(point);

            let pos = wpos.as_::<f64>() + time as f64 * 0.1;

            let space_scale = 7_500.0;
            let time_scale = 100_000.0;
            let spos = (pos / space_scale).with_z(time as f64 / time_scale);

            let pressure = (base_nz.get(spos.into_array()) * 0.5 + 1.0).clamped(0.0, 1.0) as f32;

            const RAIN_CLOUD_THRESHOLD: f32 = 0.26;
            cell.cloud = (1.0 - pressure) * 0.5;
            cell.rain = (1.0 - pressure - RAIN_CLOUD_THRESHOLD).max(0.0).powf(1.0);
            cell.wind = Vec2::new(
                rain_nz.get(spos.into_array()).powi(3) as f32,
                rain_nz.get((spos + 1.0).into_array()).powi(3) as f32,
            ) * 200.0
                * (1.0 - pressure);
        }
    }

    pub fn size(&self) -> Vec2<u32> { self.size }
}
