use common::{
    resources::TimeOfDay,
    weather::{WeatherGrid, CELL_SIZE},
};
use noise::{NoiseFn, SuperSimplex, Turbulence};
use vek::*;
use world::World;



/*
#[derive(Clone, Copy, Default)]
struct Cell {
    wind: Vec2<f32>,
    temperature: f32,
    moisture: f32,
    cloud: f32,
}
#[derive(Default)]
pub struct Constants {
    alt: f32,
    normal: Vec3<f32>,
    humid: f32,
    temp: f32,
}

/// Used to sample weather that isn't simulated
fn sample_cell(_p: Vec2<i32>, _time: f64) -> Cell {
    Cell {
        wind: Vec2::new(20.0, 20.0),
        temperature: 0.5,
        moisture: 0.1,
        cloud: 0.0,
    }
}

#[derive(Clone, Copy, Default)]
pub struct WeatherInfo {
    pub lightning_chance: f32,
}

fn sample_plane_normal(points: &[Vec3<f32>]) -> Option<Vec3<f32>> {
    if points.len() < 3 {
        return None;
    }
    let sum = points.iter().cloned().sum::<Vec3<f32>>();
    let centroid = sum / (points.len() as f32);

    let (xx, xy, xz, yy, yz, zz) = {
        let (mut xx, mut xy, mut xz, mut yy, mut yz, mut zz) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        for p in points {
            let p = *p - centroid;
            xx += p.x * p.x;
            xy += p.x * p.y;
            xz += p.x * p.z;
            yy += p.y * p.y;
            yz += p.y * p.z;
            zz += p.z * p.z;
        }
        (xx, xy, xz, yy, yz, zz)
    };

    let det_x: f32 = yy * zz - yz * yz;
    let det_y: f32 = xx * zz - xz * xz;
    let det_z: f32 = xx * yy - xy * xy;

    let det_max = det_x.max(det_y).max(det_z);
    if det_max <= 0.0 {
        None
    } else if det_max == det_x {
        Some(Vec3::new(det_x, xz * yz - xy * zz, xy * yz - xz * yy).normalized())
    } else if det_max == det_y {
        Some(Vec3::new(xy * yz - xy * zz, det_y, xy * xz - yz * xx).normalized())
    } else {
        Some(Vec3::new(xy * yz - xz * yy, xy * xz - yz * xx, det_z).normalized())
    }
}
*/
fn cell_to_wpos(p: Vec2<i32>) -> Vec2<i32> { p * CELL_SIZE as i32 }

pub struct WeatherSim {
    // cells: Grid<Cell>,       // The variables used for simulation
    // consts: Grid<Constants>, // The constants from the world used for simulation
    // info: Grid<WeatherInfo>,
    size: Vec2<u32>,
}

impl WeatherSim {
    pub fn new(size: Vec2<u32>, _world: &World) -> Self {
        /*
        let size = size.as_();
        let this = Self {
            cells: Grid::new(size, Cell::default()),
            consts: Grid::from_raw(
                size,
                (0..size.x * size.y)
                    .map(|i| Vec2::new(i % size.x, i / size.x))
                    .map(|p| {
                        let mut temp_sum = 0.0;

                        let mut alt_sum = 0.0;

                        let mut humid_sum = 1000.0;

                        let mut points: Vec<Vec3<f32>> =
                            Vec::with_capacity((CHUNKS_PER_CELL * CHUNKS_PER_CELL) as usize);
                        for y in 0..CHUNKS_PER_CELL as i32 {
                            for x in 0..CHUNKS_PER_CELL as i32 {
                                let chunk_pos = p * CHUNKS_PER_CELL as i32 + Vec2::new(x, y);
                                if let Some(chunk) = world.sim().get(chunk_pos) {
                                    let wpos = chunk_pos * TerrainChunkSize::RECT_SIZE.as_();
                                    let a = world.sim().get_alt_approx(wpos).unwrap_or(0.0);
                                    alt_sum += a;

                                    let wpos = wpos.as_().with_z(a);
                                    points.push(wpos);

                                    let height_p = 1.0 - a / world.sim().max_height;

                                    let env = chunk.get_environment();
                                    temp_sum += env.temp;
                                    humid_sum += (env.humid * (1.0 + env.near_water)) * height_p;
                                }
                            }
                        }
                        Constants {
                            alt: alt_sum / (CHUNKS_PER_CELL * CHUNKS_PER_CELL) as f32,
                            humid: humid_sum / (CHUNKS_PER_CELL * CHUNKS_PER_CELL) as f32,
                            temp: temp_sum / (CHUNKS_PER_CELL * CHUNKS_PER_CELL) as f32,
                            normal: sample_plane_normal(&points).unwrap(),
                        }
                    })
                    .collect_vec(),
            ),
            info: Grid::new(size, WeatherInfo::default()),
        };
        this.cells.iter_mut().for_each(|(point, cell)| {
            let time = 0.0;
            *cell = sample_cell(point, time);
        });
        this
        */
        Self { size }
    }

    /*
    fn get_cell(&self, p: Vec2<i32>, time: f64) -> Cell {
        *self.cells.get(p).unwrap_or(&sample_cell(p, time))
    }
    */

    // https://minds.wisconsin.edu/bitstream/handle/1793/66950/LitzauSpr2013.pdf
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
        /*
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
                            _ => rate * rate / area, // area_2 / area
                        }
                    };
                    //  0.0 <= dissipation rate <= 1.0 because we only spread to direct neighbours.
                    cell.wind += c.wind * factor(0.009);
                    cell.temperature += c.temperature * factor(0.01);
                    cell.moisture += c.moisture * factor(0.008);
                    cell.cloud += c.cloud * factor(0.005);
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

        // Evaporate moisture and condense clouds
        for (point, cell) in swap.iter() {
            let dt = 1.0 - 0.96f32.powf(DT);
            let day_light = ((2.0
                * ((time_of_day.0 as f32 / (24.0 * 60.0 * 60.0)) % 1.0 - 0.5).abs())
                * (1.0 - self.weather[point].cloud / CLOUD_MAX))
                .clamp(0.0, 1.0);

            self.cells[point].temperature =
                cell.temperature * (1.0 - dt) + dt * (self.consts[point].temp + 0.1 * day_light);

            let temp_part = ((cell.temperature + WATER_BOILING_POINT)
                / (WATER_BOILING_POINT * 2.0))
                .powf(4.0)
                .clamp(0.0, 1.0);

            // Drag wind based on pressure difference.
            // note: pressure scales linearly with temperature in this simulation
            self.cells[point].wind = cell.wind
                + [
                    Vec2::new(1, 0),
                    Vec2::new(1, 1),
                    Vec2::new(0, 1),
                    Vec2::new(-1, 0),
                    Vec2::new(-1, -1),
                    Vec2::new(0, -1),
                    Vec2::new(1, -1),
                    Vec2::new(-1, 1),
                ]
                .iter()
                .filter(|&&p| swap.get(p).is_some())
                .map(|&p| {
                    let diff =
                        (swap.get(p).unwrap().temperature - cell.temperature) / WATER_BOILING_POINT;
                    p.as_().normalized() * diff * DT
                })
                .sum::<Vec2<f32>>();

            // Curve wind based on topography
            if let Some(xy) = if self.consts[point].normal.z < 1.0 {
                Some(self.consts[point].normal.xy().normalized())
            } else {
                None
            } {
                if self.cells[point].wind.dot(xy) > 0.0 {
                    const WIND_CHECK_START: f32 = 500.0;
                    const WIND_CHECK_STOP: f32 = 3000.0;
                    let alt_m = (self.consts[point].alt - WIND_CHECK_START)
                        / (WIND_CHECK_STOP - WIND_CHECK_START);
                    let reflected = self.cells[point].wind.reflected(xy) * (1.0 - 0.9f32.powf(DT));
                    if reflected.x.is_nan() || reflected.y.is_nan() {
                        panic!("ref is nan");
                    }
                    if self.cells[point].wind.x.is_nan() || self.cells[point].wind.y.is_nan() {
                        panic!("wind is nan");
                    }
                    let drag = (1.0 - alt_m) * self.consts[point].normal.z;
                    self.cells[point].wind = self.cells[point].wind * drag
                        + reflected * alt_m * (1.0 - self.consts[point].normal.z);
                }
            }

            // If positive condense moisture to clouds, if negative evaporate clouds into
            // moisture.
            let condensation = cell.moisture * (1.0 - 0.99f32.powf((1.0 - temp_part) * DT))
                - cell.cloud * (1.0 - 0.98f32.powf(temp_part * DT)) / CLOUD_MAX;

            const CLOUD_MAX: f32 = 15.0;
            const RAIN_P: f32 = 0.96;
            let rain_cloud = temp_part * CLOUD_MAX;

            let rain_p_t = ((cell.cloud - rain_cloud) / (CLOUD_MAX - rain_cloud)).max(0.0);
            let rain = rain_p_t * (1.0 - RAIN_P.powf(DT));

            // Evaporate from ground.
            self.cells[point].moisture =
                cell.moisture + (self.consts[point].humid / 100.0) * temp_part * DT - condensation;

            self.cells[point].cloud = (cell.cloud + condensation - rain).max(0.0);

            self.weather[point].cloud = (cell.cloud / CLOUD_MAX).clamp(0.0, 1.0);
            self.weather[point].rain = rain_p_t;
            self.weather[point].wind = cell.wind;

            self.info[point].lightning_chance = self.weather[point].cloud.powf(2.0)
                * (self.weather[point].rain * 0.9 + 0.1)
                * temp_part;
        }
        */
    }

    pub fn size(&self) -> Vec2<u32> { self.size }
}
