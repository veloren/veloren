use crate::hud::{AutoPressBehavior, PressBehavior};
use serde::{Deserialize, Serialize};

/// `GameplaySettings` contains sensitivity and gameplay options.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GameplaySettings {
    pub pan_sensitivity: u32,
    pub zoom_sensitivity: u32,
    pub camera_clamp_angle: u32,
    pub zoom_inversion: bool,
    pub mouse_y_inversion: bool,
    pub smooth_pan_enable: bool,
    pub free_look_behavior: PressBehavior,
    pub auto_walk_behavior: PressBehavior,
    pub camera_clamp_behavior: PressBehavior,
    pub zoom_lock_behavior: AutoPressBehavior,
    pub stop_auto_walk_on_input: bool,
    pub auto_camera: bool,
    pub bow_zoom: bool,
    pub zoom_lock: bool,
}

impl Default for GameplaySettings {
    fn default() -> Self {
        Self {
            pan_sensitivity: 100,
            zoom_sensitivity: 100,
            camera_clamp_angle: 45,
            zoom_inversion: false,
            mouse_y_inversion: false,
            smooth_pan_enable: false,
            free_look_behavior: PressBehavior::Toggle,
            auto_walk_behavior: PressBehavior::Toggle,
            camera_clamp_behavior: PressBehavior::Toggle,
            zoom_lock_behavior: AutoPressBehavior::Auto,
            stop_auto_walk_on_input: true,
            auto_camera: false,
            bow_zoom: true,
            zoom_lock: false,
        }
    }
}
