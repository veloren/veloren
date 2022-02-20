use std::{
    ops::{Add, Deref, DerefMut, Div, Mul},
    sync::Arc,
};

use common::{
    grid::Grid,
    resources::TimeOfDay,
    terrain::{BiomeKind, TerrainChunkSize},
    vol::RectVolSize,
    weather::{Weather, CHUNKS_PER_CELL},
};
use itertools::Itertools;
use vek::*;
use world::{
    util::{FastNoise, Sampler},
    World,
};

#[derive(Default)]
pub struct Constants {
    drag: f32, // How much drag therfe is on wind. Caused by slopes.
    humidity: Option<f32>,
    temperature_day: Option<f32>,
    temperature_night: Option<f32>,
}

#[derive(Clone, Copy, Default)]
struct Cell {
    wind: Vec2<f32>,
    temperature: f32,
    moisture: f32,
    rain: f32,
}
/// Used to sample weather that isn't simulated
fn sample_cell(p: Vec2<i32>, time: f64) -> Cell {
    let noise = FastNoise::new(0b10110_101_1100_1111_10010_101_1110);

    Cell {
        wind: Vec2::new(noise.get(p.with_z(0).as_()), noise.get(p.with_z(100).as_()))
            * ((noise.get(p.with_z(200).as_()) + 1.0) * 0.5).powf(4.0)
            * 30.0,
        temperature: noise.get(p.with_z(300).as_()).powf(3.0) * 10.0 + 20.0, // 10 -> 30
        moisture: BASE_HUMIDITY,
        rain: ((noise.get(p.with_z(400).as_()) + 1.0) * 0.5).powf(4.0) * MAX_RAIN,
    }
}

pub struct WeatherSim {
    cells: Grid<Cell>,
    constants: Grid<Constants>,

    weather: Grid<Weather>, // The current weather.
}

const BASE_HUMIDITY: f32 = 1.6652;
const BASE_TEMPERATURE: f32 = 20.0;
const MAX_RAIN: f32 = 100.0;

impl WeatherSim {
    pub fn new(size: Vec2<u32>, world: &World) -> Self {
        let size = size.as_();
        Self {
            cells: Grid::new(size, Cell::default()),
            constants: Grid::from_raw(
                size,
                (0..size.x * size.y)
                    .map(|i| Vec2::new(i as i32 % size.x, i as i32 / size.x))
                    .map(|p| {
                        let mut c = Constants::default();
                        for y in 0..CHUNKS_PER_CELL as i32 {
                            for x in 0..CHUNKS_PER_CELL as i32 {
                                let chunk_pos = p * CHUNKS_PER_CELL as i32 + Vec2::new(x, y);
                                if let Some(chunk) = world.sim().get(chunk_pos) {
                                    c.drag +=
                                        world.sim().get_gradient_approx(chunk_pos).unwrap_or(0.0);
                                    if chunk.is_underwater() {
                                        c.humidity =
                                            Some(c.humidity.unwrap_or(0.0) + BASE_HUMIDITY);
                                        c.temperature_night =
                                            Some(c.temperature_night.unwrap_or(0.0) + 0.01);
                                    } else {
                                        c.temperature_day =
                                            Some(c.temperature_day.unwrap_or(0.0) + 0.01);
                                    }
                                    match chunk.get_biome() {
                                        BiomeKind::Desert => {
                                            c.temperature_day =
                                                Some(c.temperature_day.unwrap_or(0.0) + 0.01);
                                            c.humidity = Some(
                                                c.humidity.unwrap_or(0.0) - 10.0 * BASE_HUMIDITY,
                                            );
                                        },
                                        BiomeKind::Swamp => {
                                            c.humidity = Some(
                                                c.humidity.unwrap_or(0.0) + 2.0 * BASE_HUMIDITY,
                                            );
                                        },
                                        _ => {},
                                    }
                                }
                            }
                        }
                        c
                    })
                    .collect_vec(),
            ),
            weather: Grid::new(size, Weather::default()),
        }
    }

    pub fn get_weather(&self) -> &Grid<Weather> { &self.weather }

    pub fn get_weather_at(&self, chunk: Vec2<i32>) -> Option<&Weather> {
        self.weather.get(chunk / CHUNKS_PER_CELL as i32)
    }

    fn get_cell(&self, p: Vec2<i32>, time: f64) -> Cell {
        *self.cells.get(p).unwrap_or(&sample_cell(p, time))
    }

    // https://minds.wisconsin.edu/bitstream/handle/1793/66950/LitzauSpr2013.pdf
    // Time step is cell size / maximum wind speed
    pub fn tick(&mut self, world: &World, time_of_day: &TimeOfDay, dt: f32) {
        const MAX_WIND_SPEED: f32 = 127.0;
        let cell_size: Vec2<f32> = (CHUNKS_PER_CELL * TerrainChunkSize::RECT_SIZE).as_();
        let dt = cell_size.x / MAX_WIND_SPEED;
        let mut swap = Grid::new(self.cells.size(), Cell::default());
        // Dissipate wind, humidty and pressure
        //Dispersion is represented by the target cell expanding into the 8 adjacent
        // cells. The target cell’s contents are then distributed based the
        // percentage that overlaps the surrounding cell.
        for (point, cell) in self.cells.iter() {
            swap[point] = {
                let spread = [
                    (*cell, 1. / 4.),
                    (
                        self.get_cell(point + Vec2::new(1, 0), time_of_day.0),
                        1. / 8.,
                    ),
                    (
                        self.get_cell(point + Vec2::new(-1, 0), time_of_day.0),
                        1. / 8.,
                    ),
                    (
                        self.get_cell(point + Vec2::new(0, 1), time_of_day.0),
                        1. / 8.,
                    ),
                    (
                        self.get_cell(point + Vec2::new(0, -1), time_of_day.0),
                        1. / 8.,
                    ),
                    // Diagonal so less overlap
                    (
                        self.get_cell(point + Vec2::new(1, 1), time_of_day.0),
                        1. / 16.,
                    ),
                    (
                        self.get_cell(point + Vec2::new(1, -1), time_of_day.0),
                        1. / 16.,
                    ),
                    (
                        self.get_cell(point + Vec2::new(-1, 1), time_of_day.0),
                        1. / 16.,
                    ),
                    (
                        self.get_cell(point + Vec2::new(-1, -1), time_of_day.0),
                        1. / 16.,
                    ),
                ];
                let mut cell = Cell::default();

                for (c, factor) in spread {
                    cell.wind += c.wind * factor;
                    cell.temperature += c.temperature * factor;
                    cell.moisture += c.moisture * factor;
                }

                cell
            }
        }
        self.cells = swap.clone();
        swap.iter_mut().for_each(|(_, cell)| {
            *cell = Cell::default();
        });

        // Wind spread
        // Wind is modeled by taking the target cell
        // contents and moving it to different cells.
        // We assume wind will not travel more than 1 cell per tick.

        // Need to spread wind from outside simulation to simulate that the veloren
        // island is not in a box.
        for y in -1..=self.cells.size().y {
            for x in -1..=self.cells.size().x {
                let point = Vec2::new(x, y);
                let cell = self.get_cell(point, time_of_day.0);
                let a = cell_size.x - cell.wind.x.abs();
                let b = cell_size.y - cell.wind.y.abs();
                let wind_dir = Vec2::new(cell.wind.x.signum(), cell.wind.y.signum()).as_();
                let spread = [
                    (point, a * b / (cell_size.x * cell_size.y)),
                    (
                        point + wind_dir.with_y(0),
                        (cell_size.x - a) * b / (cell_size.x * cell_size.y),
                    ),
                    (
                        point + wind_dir.with_x(0),
                        a * (cell_size.y - b) / (cell_size.x * cell_size.y),
                    ),
                    (
                        point + wind_dir,
                        (cell_size.x - a) * (cell_size.y - b) / (cell_size.x * cell_size.y),
                    ),
                ];
                for (p, factor) in spread {
                    if let Some(c) = swap.get_mut(p) {
                        c.wind += cell.wind * factor;
                        c.temperature += cell.temperature * factor;
                        c.moisture += cell.moisture * factor;
                        c.rain += cell.rain * factor;
                    }
                }
            }
        }
        self.cells = swap.clone();

        // TODO: wind curl, rain condesnsing from moisture. And interacting with world
        // elements.

        // Evaporate moisture and condense clouds
        for (point, cell) in self.cells.iter_mut() {
            // r = rain, m = moisture
            // ∆r = Rcond − Rrain.
            // ∆m = −Rcond + V.
            // Rcond = δ(T0(m) − T)H(T0(m) − T) − γ(T − T0(m))H(T − T0(m)).
            // T0(m) = (100−m) / 5

            // V = B(T − Tv(h))H(T − Tv(h))
            // γ = 0.5, δ = 0.25, B = 0.5 and Tv(h) = 20◦C

            // TODO: make these parameters depend on the world.
            // TODO: figure out what these variables mean.
            let gamma = 0.5;
            let delta = 0.25;
            let b = 0.5;
            let h = 1.0;
            let t_v = BASE_TEMPERATURE;

            let rain_fall_max = 0.05;

            let evaporation = b * (cell.temperature - t_v) * h * (cell.temperature - t_v);

            let dew_point = (100.0 - cell.moisture) / 5.0;

            let condensation =
                delta * (dew_point - cell.temperature) * h * (dew_point - cell.temperature)
                    - gamma * (cell.temperature - dew_point) * h * (cell.temperature - dew_point);
            cell.rain += condensation;
            if cell.rain > rain_fall_max {
                cell.rain -= rain_fall_max;
                self.weather[point].rain = 1.0;
            } else {
                self.weather[point].rain = 0.5;
            }
            cell.moisture += evaporation - condensation;

            self.weather[point].cloud = (cell.rain / MAX_RAIN).clamp(0.0, 1.0);
            self.weather[point].wind = cell.wind;
        }

        // Maybe moisture condenses to clouds, which if they have a certain
        // amount they will release rain.
    }

    fn update_info(&mut self) {
        let w = self
            .cells
            .iter()
            .map(|(p, c)| {
                (
                    p,
                    Weather::new(
                        //if p.x % 2 == p.y % 2 { 1.0 } else { 0.0 },
                        (self.constants[p].humidity.unwrap_or(0.0) * 100.0).clamp(0.0, 1.0),
                        0.0,
                        c.wind,
                    ),
                )
            })
            .collect_vec();
        w.iter().for_each(|&(p, w)| {
            self.weather[p] = w;
        });
    }
}
