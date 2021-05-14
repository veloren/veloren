use crate::{
    hud::{BarNumbers, BuffPosition, CrosshairType, Intro, ShortcutNumbers, XpBar},
    ui::ScaleMode,
};
use serde::{Deserialize, Serialize};
use vek::*;

/// `InterfaceSettings` contains UI, HUD and Map options.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct InterfaceSettings {
    pub toggle_debug: bool,
    pub sct: bool,
    pub sct_player_batch: bool,
    pub sct_damage_batch: bool,
    pub speech_bubble_dark_mode: bool,
    pub speech_bubble_icon: bool,
    pub crosshair_transp: f32,
    pub crosshair_type: CrosshairType,
    pub intro_show: Intro,
    pub xp_bar: XpBar,
    pub shortcut_numbers: ShortcutNumbers,
    pub buff_position: BuffPosition,
    pub bar_numbers: BarNumbers,
    pub ui_scale: ScaleMode,
    pub map_zoom: f64,
    pub map_drag: Vec2<f64>,
    pub map_show_topo_map: bool,
    pub map_show_difficulty: bool,
    pub map_show_towns: bool,
    pub map_show_dungeons: bool,
    pub map_show_castles: bool,
    pub loading_tips: bool,
    pub map_show_caves: bool,
    pub map_show_trees: bool,
    pub map_show_peaks: bool,
    pub minimap_show: bool,
    pub minimap_face_north: bool,
    pub minimap_zoom: f64,
}

impl Default for InterfaceSettings {
    fn default() -> Self {
        Self {
            toggle_debug: false,
            sct: true,
            sct_player_batch: false,
            sct_damage_batch: false,
            speech_bubble_dark_mode: false,
            speech_bubble_icon: true,
            crosshair_transp: 0.6,
            crosshair_type: CrosshairType::Round,
            intro_show: Intro::Show,
            xp_bar: XpBar::Always,
            shortcut_numbers: ShortcutNumbers::On,
            buff_position: BuffPosition::Bar,
            bar_numbers: BarNumbers::Values,
            ui_scale: ScaleMode::RelativeToWindow([1920.0, 1080.0].into()),
            map_zoom: 10.0,
            map_drag: Vec2 { x: 0.0, y: 0.0 },
            map_show_topo_map: false,
            map_show_difficulty: true,
            map_show_towns: true,
            map_show_dungeons: true,
            map_show_castles: false,
            loading_tips: true,
            map_show_caves: true,
            map_show_trees: false,
            map_show_peaks: false,
            minimap_show: true,
            minimap_face_north: false,
            minimap_zoom: 10.0,
        }
    }
}
