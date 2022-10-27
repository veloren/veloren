use crate::{render::RenderMode, window::FullScreenSettings};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
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
    pub terrain_view_distance: u32,
    pub entity_view_distance: u32,
    pub lod_distance: u32,
    pub sprite_render_distance: u32,
    pub particles_enabled: bool,
    pub weapon_trails_enabled: bool,
    pub figure_lod_render_distance: u32,
    pub max_fps: Fps,
    pub max_background_fps: Fps,
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
            terrain_view_distance: 10,
            entity_view_distance: client::MAX_SELECTABLE_VIEW_DISTANCE,
            lod_distance: 200,
            sprite_render_distance: 100,
            particles_enabled: true,
            weapon_trails_enabled: true,
            figure_lod_render_distance: 300,
            max_fps: Fps::Max(60),
            max_background_fps: Fps::Max(30),
            fov: 70,
            gamma: 1.0,
            exposure: 1.0,
            ambiance: 0.5,
            render_mode: RenderMode::default(),
            window_size: [1280, 720],
            fullscreen: FullScreenSettings::default(),
            lod_detail: 250,
        }
    }
}

impl GraphicsSettings {
    pub fn into_minimal(self) -> Self {
        use crate::render::*;
        Self {
            terrain_view_distance: 4,
            entity_view_distance: 4,
            lod_distance: 0,
            sprite_render_distance: 80,
            figure_lod_render_distance: 100,
            lod_detail: 80,
            render_mode: RenderMode {
                aa: AaMode::FxUpscale,
                cloud: CloudMode::Minimal,
                reflection: ReflectionMode::Low,
                fluid: FluidMode::Low,
                lighting: LightingMode::Lambertian,
                shadow: ShadowMode::None,
                rain_occlusion: ShadowMapMode { resolution: 0.25 },
                bloom: BloomMode::Off,
                point_glow: 0.0,
                upscale_mode: UpscaleMode { factor: 0.35 },
                ..self.render_mode
            },
            ..self
        }
    }

    pub fn into_low(self) -> Self {
        use crate::render::*;
        Self {
            terrain_view_distance: 7,
            entity_view_distance: 7,
            lod_distance: 75,
            sprite_render_distance: 125,
            figure_lod_render_distance: 200,
            lod_detail: 200,
            render_mode: RenderMode {
                aa: AaMode::FxUpscale,
                cloud: CloudMode::Low,
                reflection: ReflectionMode::Medium,
                fluid: FluidMode::Low,
                lighting: LightingMode::Lambertian,
                shadow: ShadowMode::Cheap,
                rain_occlusion: ShadowMapMode { resolution: 0.25 },
                bloom: BloomMode::Off,
                point_glow: 0.35,
                upscale_mode: UpscaleMode { factor: 0.65 },
                ..self.render_mode
            },
            ..self
        }
    }

    pub fn into_medium(self) -> Self {
        use crate::render::*;
        Self {
            terrain_view_distance: 10,
            entity_view_distance: 10,
            lod_distance: 150,
            sprite_render_distance: 250,
            figure_lod_render_distance: 350,
            lod_detail: 300,
            render_mode: RenderMode {
                aa: AaMode::Fxaa,
                cloud: CloudMode::Medium,
                reflection: ReflectionMode::High,
                fluid: FluidMode::Medium,
                lighting: LightingMode::BlinnPhong,
                shadow: ShadowMode::Map(ShadowMapMode { resolution: 0.75 }),
                rain_occlusion: ShadowMapMode { resolution: 0.25 },
                bloom: BloomMode::On(BloomConfig {
                    factor: BloomFactor::Medium,
                    uniform_blur: false,
                }),
                point_glow: 0.35,
                upscale_mode: UpscaleMode { factor: 0.85 },
                ..self.render_mode
            },
            ..self
        }
    }

    pub fn into_high(self) -> Self {
        use crate::render::*;
        Self {
            terrain_view_distance: 16,
            entity_view_distance: 16,
            lod_distance: 200,
            sprite_render_distance: 350,
            figure_lod_render_distance: 450,
            lod_detail: 375,
            render_mode: RenderMode {
                aa: AaMode::Fxaa,
                cloud: CloudMode::Medium,
                reflection: ReflectionMode::High,
                fluid: FluidMode::Medium,
                lighting: LightingMode::Ashikhmin,
                shadow: ShadowMode::Map(ShadowMapMode { resolution: 1.0 }),
                rain_occlusion: ShadowMapMode { resolution: 0.5 },
                bloom: BloomMode::On(BloomConfig {
                    factor: BloomFactor::Medium,
                    uniform_blur: true,
                }),
                point_glow: 0.35,
                upscale_mode: UpscaleMode { factor: 1.0 },
                ..self.render_mode
            },
            ..self
        }
    }

    pub fn into_ultra(self) -> Self {
        use crate::render::*;
        Self {
            terrain_view_distance: 16,
            entity_view_distance: 16,
            lod_distance: 450,
            sprite_render_distance: 800,
            figure_lod_render_distance: 600,
            lod_detail: 500,
            render_mode: RenderMode {
                aa: AaMode::Fxaa,
                cloud: CloudMode::High,
                reflection: ReflectionMode::High,
                fluid: FluidMode::High,
                lighting: LightingMode::Ashikhmin,
                shadow: ShadowMode::Map(ShadowMapMode { resolution: 1.75 }),
                rain_occlusion: ShadowMapMode { resolution: 0.5 },
                bloom: BloomMode::On(BloomConfig {
                    factor: BloomFactor::Medium,
                    uniform_blur: true,
                }),
                point_glow: 0.35,
                upscale_mode: UpscaleMode { factor: 1.25 },
                ..self.render_mode
            },
            ..self
        }
    }
}
