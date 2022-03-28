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
    // Wind direction in block / second
    pub wind: Vec2<f32>,
}

impl Weather {
    pub fn new(cloud: f32, rain: f32, wind: Vec2<f32>) -> Self { Self { cloud, rain, wind } }

    pub fn get_kind(&self) -> WeatherKind {
        match (
            (self.cloud * 10.0) as i32,
            (self.rain * 10.0) as i32,
            (self.wind.magnitude() * 10.0) as i32,
        ) {
            // Over 24.5 m/s wind is a storm
            (_, _, 245..) => WeatherKind::Storm,
            (_, 1..=10, _) => WeatherKind::Rain,
            (4..=10, _, _) => WeatherKind::Cloudy,
            _ => WeatherKind::Clear,
        }
    }

    pub fn lerp(from: &Self, to: &Self, t: f32) -> Self {
        Self {
            cloud: f32::lerp(from.cloud, to.cloud, t),
            rain: f32::lerp(from.rain, to.rain, t),
            wind: Vec2::<f32>::lerp(from.wind, to.wind, t),
        }
    }

    // Get the rain direction for this weather
    pub fn rain_dir(&self) -> Vec3<f32> {
        // If this value is changed also change it in cloud-frag.glsl
        const FALL_RATE: f32 = 70.0;
        (-Vec3::unit_z() + self.wind / FALL_RATE).normalized()
    }
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

pub const CHUNKS_PER_CELL: u32 = 16;

pub const CELL_SIZE: u32 = CHUNKS_PER_CELL * TerrainChunkSize::RECT_SIZE.x;

/// How often the weather is updated, in seconds
pub const WEATHER_DT: f32 = 5.0;

// pub const MAX_WIND_SPEED: f32 = CELL_SIZE as f32 / WEATHER_DT;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherGrid {
    weather: Grid<Weather>,
}

impl WeatherGrid {
    pub fn new(size: Vec2<u32>) -> Self {
        Self {
            weather: Grid::new(size.as_(), Weather::default()),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (Vec2<i32>, &Weather)> { self.weather.iter() }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Vec2<i32>, &mut Weather)> {
        self.weather.iter_mut()
    }

    pub fn size(&self) -> Vec2<u32> { self.weather.size().as_() }

    /// Get the weather at a given world position by doing bilinear
    /// interpolation between four cells.
    pub fn get_interpolated(&self, wpos: Vec2<f32>) -> Weather {
        let cell_pos = wpos / CELL_SIZE as f32;
        let rpos = cell_pos.map(|e| e.fract());
        let cell_pos = cell_pos.map(|e| e.floor());

        let wpos = cell_pos.as_::<i32>();
        Weather::lerp(
            &Weather::lerp(
                self.weather.get(wpos).unwrap_or(&Weather::default()),
                self.weather
                    .get(wpos + Vec2::unit_x())
                    .unwrap_or(&Weather::default()),
                rpos.x,
            ),
            &Weather::lerp(
                self.weather
                    .get(wpos + Vec2::unit_x())
                    .unwrap_or(&Weather::default()),
                self.weather
                    .get(wpos + Vec2::one())
                    .unwrap_or(&Weather::default()),
                rpos.x,
            ),
            rpos.y,
        )
    }
}
