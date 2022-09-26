use crate::{
    hud::{BarNumbers, BuffPosition, CrosshairType, Intro, ShortcutNumbers, XpBar},
    ui::ScaleMode,
};
use common::comp::skillset::SkillGroupKind;
use serde::{Deserialize, Serialize};

/// `InterfaceSettings` contains UI, HUD and Map options.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct InterfaceSettings {
    pub toggle_debug: bool,
    pub toggle_egui_debug: bool,
    pub toggle_hitboxes: bool,
    pub toggle_chat: bool,
    pub toggle_hotkey_hints: bool,
    pub sct: bool,
    pub sct_damage_rounding: bool,
    pub sct_dmg_accum_duration: f32,
    pub sct_inc_dmg: bool,
    pub sct_inc_dmg_accum_duration: f32,
    pub speech_bubble_self: bool,
    pub speech_bubble_dark_mode: bool,
    pub speech_bubble_icon: bool,
    pub crosshair_opacity: f32,
    pub crosshair_type: CrosshairType,
    pub intro_show: Intro,
    pub xp_bar: XpBar,
    pub shortcut_numbers: ShortcutNumbers,
    pub buff_position: BuffPosition,
    pub bar_numbers: BarNumbers,
    pub always_show_bars: bool,
    pub enable_poise_bar: bool,
    pub ui_scale: ScaleMode,
    pub map_zoom: f64,
    pub map_show_topo_map: bool,
    pub map_show_difficulty: bool,
    pub map_show_towns: bool,
    pub map_show_dungeons: bool,
    pub map_show_castles: bool,
    pub map_show_bridges: bool,
    pub loading_tips: bool,
    pub map_show_caves: bool,
    pub map_show_trees: bool,
    pub map_show_peaks: bool,
    pub map_show_biomes: bool,
    pub map_show_voxel_map: bool,
    pub minimap_show: bool,
    pub minimap_face_north: bool,
    pub minimap_zoom: f64,
    pub accum_experience: bool,
    pub xp_bar_skillgroup: Option<SkillGroupKind>,
}

impl Default for InterfaceSettings {
    fn default() -> Self {
        Self {
            toggle_debug: false,
            toggle_egui_debug: false,
            toggle_hitboxes: false,
            toggle_chat: true,
            toggle_hotkey_hints: true,
            sct: true,
            sct_damage_rounding: false,
            sct_dmg_accum_duration: 0.45,
            sct_inc_dmg: true,
            sct_inc_dmg_accum_duration: 0.45,
            speech_bubble_self: true,
            speech_bubble_dark_mode: false,
            speech_bubble_icon: true,
            crosshair_opacity: 0.6,
            crosshair_type: CrosshairType::Round,
            intro_show: Intro::Show,
            xp_bar: XpBar::Always,
            shortcut_numbers: ShortcutNumbers::On,
            buff_position: BuffPosition::Bar,
            bar_numbers: BarNumbers::Values,
            always_show_bars: false,
            enable_poise_bar: false,
            ui_scale: ScaleMode::RelativeToWindow([1920.0, 1080.0].into()),
            map_zoom: 10.0,
            map_show_topo_map: true,
            map_show_difficulty: true,
            map_show_towns: true,
            map_show_dungeons: true,
            map_show_castles: false,
            map_show_bridges: false,
            loading_tips: true,
            map_show_caves: true,
            map_show_trees: false,
            map_show_peaks: false,
            map_show_biomes: false,
            map_show_voxel_map: true,
            minimap_show: true,
            minimap_face_north: true,
            minimap_zoom: 160.0,
            accum_experience: true,
            xp_bar_skillgroup: Some(SkillGroupKind::General),
        }
    }
}

#[cfg(feature = "egui-ui")]
impl InterfaceSettings {
    pub fn egui_enabled(&self) -> bool { self.toggle_egui_debug }
}
