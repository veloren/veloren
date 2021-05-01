use crate::{render::RenderMode, window::FullScreenSettings};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum Fps {
    Max(u32),
    Unlimited,
}

pub fn get_fps(max_fps: Fps) -> u32 {
    match max_fps {
        Fps::Max(x) => x,
        Fps::Unlimited => u32::MAX,
    }
}

impl fmt::Display for Fps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Fps::Max(x) => write!(f, "{}", x),
            Fps::Unlimited => write!(f, "Unlimited"),
        }
    }
}

/// `GraphicsSettings` contains settings related to framerate and in-game
/// visuals.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphicsSettings {
    pub view_distance: u32,
    pub sprite_render_distance: u32,
    pub particles_enabled: bool,
    pub lossy_terrain_compression: bool,
    pub figure_lod_render_distance: u32,
    pub max_fps: Fps,
    pub fov: u16,
    pub gamma: f32,
    pub exposure: f32,
    pub ambiance: f32,
    pub render_mode: RenderMode,
    pub window_size: [u16; 2],
    pub fullscreen: FullScreenSettings,
    pub lod_detail: u32,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            view_distance: 10,
            sprite_render_distance: 100,
            particles_enabled: true,
            lossy_terrain_compression: false,
            figure_lod_render_distance: 300,
            max_fps: Fps::Max(60),
            fov: 70,
            gamma: 1.0,
            exposure: 1.0,
            ambiance: 10.0,
            render_mode: RenderMode::default(),
            window_size: [1280, 720],
            fullscreen: FullScreenSettings::default(),
            lod_detail: 250,
        }
    }
}
