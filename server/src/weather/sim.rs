use common::{
    grid::Grid,
    resources::TimeOfDay,
    terrain::{BiomeKind, TerrainChunkSize},
    time::DayPeriod,
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
    altitude: f32,
    water: f32,
    temperature_day: f32,
    temperature_night: f32,
}

#[derive(Clone, Copy, Default)]
struct Cell {
    wind: Vec2<f32>,
    temperature: f32,
    moisture: f32,
    cloud: f32,
}
/// Used to sample weather that isn't simulated
fn sample_cell(p: Vec2<i32>, time: f64) -> Cell {
    /*let noise = FastNoise::new(0b10110_101_1100_1111_10010_101_1110);
    // Feeding sensible values into the simulation
    Cell {
        wind: Vec2::new(
            noise.get(p.as_().with_z(time / 1000.0)),
            noise.get(p.as_().with_z(-time / 1000.0)),
        ) * ((noise.get(p.as_().with_z(time / 1000.0 + 200.0)) + 1.0) * 0.5).powf(2.0)
            * 30.0
            + 5.0,
        temperature: noise.get(p.as_().with_z(time / 1000.0 + 300.0)).powf(3.0) * 10.0
            + BASE_TEMPERATURE
            + 273.3, /* 10 -> 30 C */
        moisture: BASE_MOISTURE + noise.get(p.as_().with_z(time / 100.0 + 400.0)).powf(3.0) * 0.2,
        cloud: noise
            .get((p.as_() / 2.0).with_z(time / 100.0 + 400.0))
            .powf(2.0)
            * 1.0,
    } */
    Cell {
        wind: Vec2::new(10.0, 0.0),
        temperature: 20.0,
        moisture: 0.0,
        cloud: 0.0,
    }
}

pub struct WeatherSim {
    cells: Grid<Cell>,          // The variables used for simulation
    constants: Grid<Constants>, // The constants from the world used for simulation
    weather: Grid<Weather>,     // The current weather.
}

const BASE_MOISTURE: f32 = 1.0;
const BASE_TEMPERATURE: f32 = 20.0;
const WATER_BOILING_POINT: f32 = 373.3;
const MAX_WIND_SPEED: f32 = 60.0;
const CELL_SIZE: f32 = (CHUNKS_PER_CELL * TerrainChunkSize::RECT_SIZE.x) as f32;
pub(crate) const DT: f32 = CELL_SIZE / MAX_WIND_SPEED;

impl WeatherSim {
    pub fn new(size: Vec2<u32>, world: &World) -> Self {
        let size = size.as_();
        let mut this = Self {
            cells: Grid::new(size, Cell::default()),
            constants: Grid::from_raw(
                size,
                (0..size.x * size.y)
                    .map(|i| Vec2::new(i as i32 % size.x, i as i32 / size.x))
                    .map(|p| {
                        let add = |member: &mut f32, v| {
                            *member += v;
                            *member /= 2.0;
                        };
                        let mut temperature_night_r = 0.0;
                        let mut temperature_night = |v| {
                            add(&mut temperature_night_r, v);
                        };
                        let mut temperature_day_r = 0.0;
                        let mut temperature_day = |v| {
                            add(&mut temperature_day_r, v);
                        };
                        let mut altitude_r = 0.0;
                        let mut altitude = |v| {
                            add(&mut altitude_r, v);
                        };
                        let mut water_r = 0.0;
                        for y in 0..CHUNKS_PER_CELL as i32 {
                            for x in 0..CHUNKS_PER_CELL as i32 {
                                let chunk_pos = p * CHUNKS_PER_CELL as i32 + Vec2::new(x, y);
                                if let Some(chunk) = world.sim().get(chunk_pos) {
                                    let a =
                                        world.sim().get_gradient_approx(chunk_pos).unwrap_or(0.0);
                                    altitude(a);

                                    let height_p = 1.0
                                        - world.sim().get_alt_approx(chunk_pos).unwrap_or(0.0)
                                            / world.sim().max_height;

                                    let mut water = |v| {
                                        add(&mut water_r, v * height_p);
                                    };

                                    match chunk.get_biome() {
                                        BiomeKind::Desert => {
                                            water(0.0);
                                            temperature_night(11.0);
                                            temperature_day(30.0);
                                        },
                                        BiomeKind::Savannah => {
                                            water(0.02);
                                            temperature_night(13.0);
                                            temperature_day(26.0);
                                        },
                                        BiomeKind::Swamp => {
                                            water(0.8);
                                            temperature_night(16.0);
                                            temperature_day(24.0);
                                        },
                                        BiomeKind::Mountain => {
                                            water(0.01);
                                            temperature_night(13.0);
                                            temperature_day(14.0);
                                        },
                                        BiomeKind::Grassland => {
                                            water(0.05);
                                            temperature_night(15.0);
                                            temperature_day(20.0);
                                        },
                                        BiomeKind::Snowland => {
                                            water(0.005);
                                            temperature_night(-8.0);
                                            temperature_day(-1.0);
                                        },
                                        BiomeKind::Jungle => {
                                            water(0.4);
                                            temperature_night(20.0);
                                            temperature_day(27.0);
                                        },
                                        BiomeKind::Forest => {
                                            water(0.1);
                                            temperature_night(16.0);
                                            temperature_day(19.0);
                                        },
                                        BiomeKind::Taiga => {
                                            water(0.02);
                                            temperature_night(1.0);
                                            temperature_day(10.0);
                                        },
                                        BiomeKind::Lake => {
                                            water(1.0);
                                            temperature_night(20.0);
                                            temperature_day(18.0);
                                        },
                                        BiomeKind::Ocean => {
                                            water(0.98);
                                            temperature_night(19.0);
                                            temperature_day(17.0);
                                        },
                                        BiomeKind::Void => {
                                            water(0.0);
                                            temperature_night(20.0);
                                            temperature_day(20.0);
                                        },
                                    }
                                }
                            }
                        }
                        Constants {
                            altitude: altitude_r,
                            water: water_r,
                            temperature_day: temperature_day_r,
                            temperature_night: temperature_day_r,
                        }
                    })
                    .collect_vec(),
            ),
            weather: Grid::new(size, Weather::default()),
        };
        this.cells.iter_mut().for_each(|(point, cell)| {
            let time = 0.0;
            *cell = sample_cell(point, time);
        });
        this
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
    pub fn tick(&mut self, time_of_day: &TimeOfDay, dt: f32) {
        let time = time_of_day.0;
        let mut swap = Grid::new(self.cells.size(), Cell::default());
        // Dissipate wind, humidty and pressure
        // Dissipation is represented by the target cell expanding into the 8 adjacent
        // cells. The target cell’s contents are then distributed based the
        // percentage that overlaps the surrounding cell.
        for (point, cell) in self.cells.iter() {
            swap[point] = {
                let spread = [
                    (*cell, 0),
                    (self.get_cell(point + Vec2::new(1, 0), time), 1),
                    (self.get_cell(point + Vec2::new(-1, 0), time), 1),
                    (self.get_cell(point + Vec2::new(0, 1), time), 1),
                    (self.get_cell(point + Vec2::new(0, -1), time), 1),
                    // Diagonal so less overlap
                    (self.get_cell(point + Vec2::new(1, 1), time), 2),
                    (self.get_cell(point + Vec2::new(1, -1), time), 2),
                    (self.get_cell(point + Vec2::new(-1, 1), time), 2),
                    (self.get_cell(point + Vec2::new(-1, -1), time), 2),
                ];
                let mut cell = Cell::default();

                for (c, p) in spread {
                    let factor = |rate: f32| {
                        let rate = (1.0 + rate).powf(DT);
                        //              1.0
                        //      ___________________
                        //     /                   \
                        // +---+-------------------+---+
                        // | 2 |         1         | 2 |
                        // +---+-------------------+---+
                        // |   |                   |   |
                        // |   |                   |   |
                        // |   |                   |   |
                        // | 1 |         0         | 1 |
                        // |   |                   |   |
                        // |   |                   |   |
                        // |   |                   |   |
                        // +---+-------------------+---+  \
                        // | 2 |         1         | 2 |  | rate
                        // +---+-------------------+---+  /
                        // \___________________________/
                        //        2.0 * rate + 1.0
                        // area_0 = 1.0 * 1.0
                        // area_1 = rate * 1.0
                        // area_2 = rate * rate

                        let area = (1.0 + 2.0 * rate).powf(2.0);
                        match p {
                            0 => 1.0 * 1.0 / area,   // area_0 / area
                            1 => rate * 1.0 / area,  // area_1 / area
                            _ => rate * rate / area, // area_2/ area
                        }
                    };
                    //  0.0 <= dissipation rate <= 1.0 because we only spread to direct neighbours.
                    cell.wind += c.wind * factor(0.0055);
                    cell.temperature += c.temperature * factor(0.007);
                    cell.moisture += c.moisture * factor(0.003);
                    cell.cloud += c.cloud * factor(0.001);
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
                let cell = self.get_cell(point, time);
                let a = CELL_SIZE - cell.wind.x.abs() * DT;
                let b = CELL_SIZE - cell.wind.y.abs() * DT;
                let wind_dir = Vec2::new(cell.wind.x.signum(), cell.wind.y.signum()).as_();
                let spread = [
                    (point, a * b / (CELL_SIZE * CELL_SIZE)),
                    (
                        point + wind_dir.with_y(0),
                        (CELL_SIZE - a) * b / (CELL_SIZE * CELL_SIZE),
                    ),
                    (
                        point + wind_dir.with_x(0),
                        a * (CELL_SIZE - b) / (CELL_SIZE * CELL_SIZE),
                    ),
                    (
                        point + wind_dir,
                        (CELL_SIZE - a) * (CELL_SIZE - b) / (CELL_SIZE * CELL_SIZE),
                    ),
                ];
                for (p, factor) in spread {
                    if let Some(c) = swap.get_mut(p) {
                        c.wind += cell.wind * factor;
                        c.temperature += cell.temperature * factor;
                        c.moisture += cell.moisture * factor;
                        c.cloud += cell.cloud * factor;
                    }
                }
            }
        }
        self.cells = swap.clone();

        // TODO: wind curl

        // Evaporate moisture and condense clouds
        for (point, cell) in self.cells.iter_mut() {
            let dt = 1.0 - 0.96f32.powf(DT);
            let day_light = (1.0 - (1.0 - time_of_day.0 as f32 / 12.0 * 60.0 * 60.0).abs())
                * self.weather[point].cloud;

            cell.temperature = cell.temperature * (1.0 - dt)
                + dt * (self.constants[point].temperature_day * day_light
                    + self.constants[point].temperature_night * (1.0 - day_light));

            // Evaporate from ground.
            let temp_part = (cell.temperature / WATER_BOILING_POINT)
                //.powf(2.0)
                .clamp(0.0, 1.0);
            cell.moisture += (self.constants[point].water / 10.0) * temp_part * DT;

            // If positive condense moisture to clouds, if negative evaporate clouds into
            // moisture.
            let condensation = if cell.moisture > 1.0 {
                cell.moisture.powf(2.0 / 3.0)
            } else {
                cell.moisture
            } * temp_part
                * DT;

            cell.moisture -= condensation;
            cell.cloud += condensation;

            const CLOUD_MAX: f32 = 15.0;
            const RAIN_P: f32 = 0.96;
            let rain_cloud = temp_part * CLOUD_MAX;

            let rain_p_t = ((cell.cloud - rain_cloud) / (CLOUD_MAX - rain_cloud)).max(0.0);
            let rain = rain_p_t * (1.0 - RAIN_P.powf(DT));
            cell.cloud -= rain;
            cell.cloud = cell.cloud.max(0.0);

            self.weather[point].cloud = (cell.cloud / CLOUD_MAX).clamp(0.0, 1.0);
            self.weather[point].rain = rain_p_t;
            self.weather[point].wind = cell.wind;
        }

        // Maybe moisture condenses to clouds, which if they have a certain
        // amount they will release rain.
    }
}
