use std::fmt;

use serde::{Deserialize, Serialize};
use vek::{Lerp, Vec2, Vec3};

use crate::{grid::Grid, terrain::TerrainChunkSize, vol::RectVolSize};

/// Weather::default is Clear, 0 degrees C and no wind
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Weather {
    /// Clouds currently in the area between 0 and 1
    pub cloud: f32,
    /// Rain per time, between 0 and 1
    pub rain: f32,
    /// Wind velocity in block / second
    pub wind: Vec2<f32>,
}

impl Weather {
    pub fn new(cloud: f32, rain: f32, wind: Vec2<f32>) -> Self { Self { cloud, rain, wind } }

    pub fn get_kind(&self) -> WeatherKind {
        // Over 24.5 m/s wind is a storm
        if self.wind.magnitude_squared() >= 24.5f32.powi(2) {
            WeatherKind::Storm
        } else if (0.1..=1.0).contains(&self.rain) {
            WeatherKind::Rain
        } else if (0.2..=1.0).contains(&self.cloud) {
            WeatherKind::Cloudy
        } else {
            WeatherKind::Clear
        }
    }

    pub fn lerp_unclamped(&self, to: &Self, t: f32) -> Self {
        Self {
            cloud: f32::lerp_unclamped(self.cloud, to.cloud, t),
            rain: f32::lerp_unclamped(self.rain, to.rain, t),
            wind: Vec2::<f32>::lerp_unclamped(self.wind, to.wind, t),
        }
    }

    // Get the rain velocity for this weather
    pub fn rain_vel(&self) -> Vec3<f32> {
        const FALL_RATE: f32 = 30.0;
        self.wind.with_z(-FALL_RATE)
    }

    // Get the wind velocity for this weather
    pub fn wind_vel(&self) -> Vec2<f32> { self.wind }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum WeatherKind {
    Clear,
    Cloudy,
    Rain,
    Storm,
}

impl fmt::Display for WeatherKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WeatherKind::Clear => write!(f, "Clear"),
            WeatherKind::Cloudy => write!(f, "Cloudy"),
            WeatherKind::Rain => write!(f, "Rain"),
            WeatherKind::Storm => write!(f, "Storm"),
        }
    }
}

// How many chunks wide a weather cell is.
// So one weather cell has (CHUNKS_PER_CELL * CHUNKS_PER_CELL) chunks.
pub const CHUNKS_PER_CELL: u32 = 16;

pub const CELL_SIZE: u32 = CHUNKS_PER_CELL * TerrainChunkSize::RECT_SIZE.x;

#[derive(Debug, Clone)]
pub struct WeatherGrid {
    weather: Grid<Weather>,
}

/// Weather that's compressed in order to send it to the client.
#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CompressedWeather {
    cloud: u8,
    rain: u8,
}

impl CompressedWeather {
    pub fn lerp_unclamped(&self, to: &CompressedWeather, t: f32) -> Weather {
        Weather {
            cloud: f32::lerp_unclamped(self.cloud as f32, to.cloud as f32, t) / 255.0,
            rain: f32::lerp_unclamped(self.rain as f32, to.rain as f32, t) / 255.0,
            wind: Vec2::zero(),
        }
    }
}

impl From<Weather> for CompressedWeather {
    fn from(weather: Weather) -> Self {
        Self {
            cloud: (weather.cloud * 255.0).round() as u8,
            rain: (weather.rain * 255.0).round() as u8,
        }
    }
}

impl From<CompressedWeather> for Weather {
    fn from(weather: CompressedWeather) -> Self {
        Self {
            cloud: weather.cloud as f32 / 255.0,
            rain: weather.rain as f32 / 255.0,
            wind: Vec2::zero(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedWeatherGrid {
    weather: Grid<CompressedWeather>,
}

impl From<&WeatherGrid> for SharedWeatherGrid {
    fn from(value: &WeatherGrid) -> Self {
        Self {
            weather: Grid::from_raw(
                value.weather.size(),
                value
                    .weather
                    .raw()
                    .iter()
                    .copied()
                    .map(CompressedWeather::from)
                    .collect::<Vec<_>>(),
            ),
        }
    }
}

impl From<&SharedWeatherGrid> for WeatherGrid {
    fn from(value: &SharedWeatherGrid) -> Self {
        Self {
            weather: Grid::from_raw(
                value.weather.size(),
                value
                    .weather
                    .raw()
                    .iter()
                    .copied()
                    .map(Weather::from)
                    .collect::<Vec<_>>(),
            ),
        }
    }
}

impl SharedWeatherGrid {
    pub fn new(size: Vec2<u32>) -> Self {
        size.map(|e| debug_assert!(i32::try_from(e).is_ok()));
        Self {
            weather: Grid::new(size.as_(), CompressedWeather::default()),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (Vec2<i32>, &CompressedWeather)> {
        self.weather.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Vec2<i32>, &mut CompressedWeather)> {
        self.weather.iter_mut()
    }

    pub fn size(&self) -> Vec2<u32> { self.weather.size().as_() }
}

/// Transforms a world position to cell coordinates. Where (0.0, 0.0) in cell
/// coordinates is the center of the weather cell located at (0, 0) in the grid.
fn to_cell_pos(wpos: Vec2<f32>) -> Vec2<f32> { wpos / CELL_SIZE as f32 - 0.5 }

// TODO: Move consts from world to common to avoid duplication
const LOCALITY: [Vec2<i32>; 9] = [
    Vec2::new(0, 0),
    Vec2::new(0, 1),
    Vec2::new(1, 0),
    Vec2::new(0, -1),
    Vec2::new(-1, 0),
    Vec2::new(1, 1),
    Vec2::new(1, -1),
    Vec2::new(-1, 1),
    Vec2::new(-1, -1),
];

impl WeatherGrid {
    pub fn new(size: Vec2<u32>) -> Self {
        size.map(|e| debug_assert!(i32::try_from(e).is_ok()));
        Self {
            weather: Grid::new(size.as_(), Weather::default()),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (Vec2<i32>, &Weather)> { self.weather.iter() }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Vec2<i32>, &mut Weather)> {
        self.weather.iter_mut()
    }

    pub fn size(&self) -> Vec2<u32> { self.weather.size().as_() }

    pub fn get(&self, cell_pos: Vec2<u32>) -> Weather {
        self.weather
            .get(cell_pos.as_())
            .copied()
            .unwrap_or_default()
    }

    /// Get the weather at a given world position by doing bilinear
    /// interpolation between four cells.
    pub fn get_interpolated(&self, wpos: Vec2<f32>) -> Weather {
        let cell_pos = to_cell_pos(wpos);
        let rpos = cell_pos.map(|e| e.fract() + (1.0 - e.signum()) / 2.0);
        let cell_pos = cell_pos.map(|e| e.floor());

        let cpos = cell_pos.as_::<i32>();
        Weather::lerp_unclamped(
            &Weather::lerp_unclamped(
                self.weather.get(cpos).unwrap_or(&Weather::default()),
                self.weather
                    .get(cpos + Vec2::unit_x())
                    .unwrap_or(&Weather::default()),
                rpos.x,
            ),
            &Weather::lerp_unclamped(
                self.weather
                    .get(cpos + Vec2::unit_y())
                    .unwrap_or(&Weather::default()),
                self.weather
                    .get(cpos + Vec2::one())
                    .unwrap_or(&Weather::default()),
                rpos.x,
            ),
            rpos.y,
        )
    }

    /// Get the max weather near a position
    pub fn get_max_near(&self, wpos: Vec2<f32>) -> Weather {
        let cell_pos: Vec2<i32> = to_cell_pos(wpos).as_();
        LOCALITY
            .iter()
            .map(|l| {
                self.weather
                    .get(cell_pos + l)
                    .cloned()
                    .unwrap_or_default()
            })
            .reduce(|a, b| Weather {
                cloud: a.cloud.max(b.cloud),
                rain: a.rain.max(b.rain),
                wind: a.wind.map2(b.wind, |a, b| a.max(b)),
            })
            // There will always be 9 elements in locality
            .unwrap()
    }
}
