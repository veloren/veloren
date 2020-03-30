mod bag;
mod buttons;
mod character_window;
mod chat;
mod esc_menu;
mod img_ids;
mod item_imgs;
mod map;
mod minimap;
mod quest;
mod settings_window;
mod skillbar;
mod social;
mod spell;

use crate::{ecs::comp::HpFloaterList, hud::img_ids::ImgsRot, ui::img_ids::Rotations};
pub use settings_window::ScaleChange;
use std::time::Duration;

use bag::Bag;
use buttons::Buttons;
use character_window::CharacterWindow;
use chat::Chat;
use chrono::NaiveTime;
use esc_menu::EscMenu;
use img_ids::Imgs;
use item_imgs::ItemImgs;
use map::Map;
use minimap::MiniMap;
use quest::Quest;
use serde::{Deserialize, Serialize};
use settings_window::{SettingsTab, SettingsWindow};
use skillbar::Skillbar;
use social::{Social, SocialTab};
use spell::Spell;

use crate::{
    ecs::comp as vcomp,
    i18n::{i18n_asset_key, LanguageMetadata, VoxygenLocalization},
    render::{AaMode, CloudMode, Consts, FluidMode, Globals, Renderer},
    scene::camera::{self, Camera},
    ui::{fonts::ConrodVoxygenFonts, Graphic, Ingameable, ScaleMode, Ui},
    window::{Event as WinEvent, GameInput},
    GlobalState,
};
use client::{Client, Event as ClientEvent};
use common::{assets::load_expect, comp, terrain::TerrainChunk, vol::RectRasterableVol};
use conrod_core::{
    text::cursor::Index,
    widget::{self, Button, Image, Rectangle, Text},
    widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, Widget,
};
use specs::{Join, WorldExt};
use std::collections::VecDeque;
use vek::*;

const XP_COLOR: Color = Color::Rgba(0.59, 0.41, 0.67, 1.0);
const TEXT_COLOR: Color = Color::Rgba(1.0, 1.0, 1.0, 1.0);
const TEXT_BG: Color = Color::Rgba(0.0, 0.0, 0.0, 1.0);
//const TEXT_COLOR_GREY: Color = Color::Rgba(1.0, 1.0, 1.0, 0.5);
const MENU_BG: Color = Color::Rgba(0.0, 0.0, 0.0, 0.4);
//const TEXT_COLOR_2: Color = Color::Rgba(0.0, 0.0, 0.0, 1.0);
const TEXT_COLOR_3: Color = Color::Rgba(1.0, 1.0, 1.0, 0.1);
//const BG_COLOR: Color = Color::Rgba(1.0, 1.0, 1.0, 0.8);
const HP_COLOR: Color = Color::Rgba(0.33, 0.63, 0.0, 1.0);
const LOW_HP_COLOR: Color = Color::Rgba(0.93, 0.59, 0.03, 1.0);
const CRITICAL_HP_COLOR: Color = Color::Rgba(0.79, 0.19, 0.17, 1.0);
const MANA_COLOR: Color = Color::Rgba(0.29, 0.62, 0.75, 0.9);
//const FOCUS_COLOR: Color = Color::Rgba(1.0, 0.56, 0.04, 1.0);
//const RAGE_COLOR: Color = Color::Rgba(0.5, 0.04, 0.13, 1.0);
const META_COLOR: Color = Color::Rgba(1.0, 1.0, 0.0, 1.0);
const TELL_COLOR: Color = Color::Rgba(0.98, 0.71, 1.0, 1.0);
const PRIVATE_COLOR: Color = Color::Rgba(1.0, 1.0, 0.0, 1.0); // Difference between private and tell?
const BROADCAST_COLOR: Color = Color::Rgba(0.28, 0.83, 0.71, 1.0);
const GAME_UPDATE_COLOR: Color = Color::Rgba(1.0, 1.0, 0.0, 1.0);
const SAY_COLOR: Color = Color::Rgba(1.0, 1.0, 1.0, 1.0);
const GROUP_COLOR: Color = Color::Rgba(0.47, 0.84, 1.0, 1.0);
const FACTION_COLOR: Color = Color::Rgba(0.24, 1.0, 0.48, 1.0);
const KILL_COLOR: Color = Color::Rgba(1.0, 0.17, 0.17, 1.0);

/// Distance at which nametags are visible
const NAMETAG_RANGE: f32 = 40.0;
/// Time nametags stay visible after doing damage even if they are out of range
/// in seconds
const NAMETAG_DMG_TIME: f32 = 60.0;
/// Range damaged triggered nametags can be seen
const NAMETAG_DMG_RANGE: f32 = 120.0;

widget_ids! {
    struct Ids {
        // Crosshair
        crosshair_inner,
        crosshair_outer,

        // Character Names
        name_tags[],
        name_tags_bgs[],
        levels[],
        levels_skull[],
        // Health Bars
        health_bars[],
        mana_bars[],
        health_bar_fronts[],
        health_bar_backs[],

        // SCT
        player_scts[],
        player_sct_bgs[],
        sct_exp_bgs[],
        sct_exps[],
        sct_lvl_bg,
        sct_lvl,
        hurt_bg,
        death_bg,
        sct_bgs[],
        scts[],

        // Intro Text
        intro_bg,
        intro_text,
        intro_close,
        intro_close_2,
        intro_close_3,
        intro_close_4,
        intro_close_5,
        intro_check,
        intro_check_text,

        // Test
        bag_space_add,

        // Debug
        debug_bg,
        fps_counter,
        ping,
        coordinates,
        velocity,
        loaded_distance,
        time,
        entity_count,
        num_chunks,
        num_figures,

        // Game Version
        version,

        // Help
        help,
        help_info,
        debug_info,

        // Window Frames
        window_frame_0,
        window_frame_1,
        window_frame_2,
        window_frame_3,
        window_frame_4,
        window_frame_5,

        button_help2,
        button_help3,

        // External
        chat,
        map,
        world_map,
        character_window,
        minimap,
        bag,
        social,
        quest,
        spell,
        skillbar,
        buttons,
        esc_menu,
        small_window,
        social_window,
        settings_window,

        // Free look indicator
        free_look_txt,
        free_look_bg,
    }
}

pub struct DebugInfo {
    pub tps: f64,
    pub ping_ms: f64,
    pub coordinates: Option<comp::Pos>,
    pub velocity: Option<comp::Vel>,
    pub num_chunks: u32,
    pub num_visible_chunks: u32,
    pub num_figures: u32,
    pub num_figures_visible: u32,
}

pub enum Event {
    SendMessage(String),
    AdjustMousePan(u32),
    AdjustMouseZoom(u32),
    ToggleZoomInvert(bool),
    ToggleMouseYInvert(bool),
    AdjustViewDistance(u32),
    AdjustMusicVolume(f32),
    AdjustSfxVolume(f32),
    ChangeAudioDevice(String),
    ChangeMaxFPS(u32),
    ChangeFOV(u16),
    ChangeGamma(f32),
    AdjustWindowSize([u16; 2]),
    ToggleFullscreen,
    ChangeAaMode(AaMode),
    ChangeCloudMode(CloudMode),
    ChangeFluidMode(FluidMode),
    CrosshairTransp(f32),
    ChatTransp(f32),
    CrosshairType(CrosshairType),
    ToggleXpBar(XpBar),
    Intro(Intro),
    ToggleBarNumbers(BarNumbers),
    ToggleShortcutNumbers(ShortcutNumbers),
    Sct(bool),
    SctPlayerBatch(bool),
    SctDamageBatch(bool),
    ToggleDebug(bool),
    UiScale(ScaleChange),
    CharacterSelection,
    UseInventorySlot(usize),
    SwapInventorySlots(usize, usize),
    DropInventorySlot(usize),
    Logout,
    Quit,
    ChangeLanguage(LanguageMetadata),
    ChangeFreeLookBehavior(PressBehavior),
}

// TODO: Are these the possible layouts we want?
// TODO: Maybe replace this with bitflags.
// `map` is not here because it currently is displayed over the top of other
// open windows.
#[derive(PartialEq)]
pub enum Windows {
    Settings, // Display settings window.
    None,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum CrosshairType {
    Round,
    RoundEdges,
    Edges,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Intro {
    Show,
    Never,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum XpBar {
    Always,
    OnGain,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum BarNumbers {
    Values,
    Percent,
    Off,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ShortcutNumbers {
    On,
    Off,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum PressBehavior {
    Toggle = 0,
    Hold = 1,
}

pub struct Show {
    ui: bool,
    intro: bool,
    help: bool,
    debug: bool,
    bag: bool,
    social: bool,
    spell: bool,
    quest: bool,
    character_window: bool,
    esc_menu: bool,
    open_windows: Windows,
    map: bool,
    mini_map: bool,
    ingame: bool,
    settings_tab: SettingsTab,
    social_tab: SocialTab,
    want_grab: bool,
    free_look: bool,
}
impl Show {
    fn bag(&mut self, open: bool) {
        self.bag = open;
        self.want_grab = !open;
    }

    fn toggle_bag(&mut self) { self.bag(!self.bag); }

    fn map(&mut self, open: bool) {
        self.map = open;
        self.bag = false;
        self.want_grab = true;
    }

    fn character_window(&mut self, open: bool) {
        self.character_window = open;
        self.bag = false;
        self.want_grab = !open;
    }

    fn social(&mut self, open: bool) {
        self.social = open;
        self.spell = false;
        self.quest = false;
        self.want_grab = !open;
    }

    fn spell(&mut self, open: bool) {
        self.social = false;
        self.spell = open;
        self.quest = false;
        self.want_grab = !open;
    }

    fn quest(&mut self, open: bool) {
        self.social = false;
        self.spell = false;
        self.quest = open;
        self.want_grab = !open;
    }

    fn toggle_map(&mut self) { self.map(!self.map) }

    fn toggle_mini_map(&mut self) { self.mini_map = !self.mini_map; }

    fn toggle_char_window(&mut self) { self.character_window = !self.character_window }

    fn settings(&mut self, open: bool) {
        self.open_windows = if open {
            Windows::Settings
        } else {
            Windows::None
        };
        self.bag = false;
        self.social = false;
        self.spell = false;
        self.quest = false;
        self.want_grab = !open;
    }

    fn toggle_settings(&mut self) {
        match self.open_windows {
            Windows::Settings => self.settings(false),
            _ => self.settings(true),
        };
    }

    fn toggle_help(&mut self) { self.help = !self.help }

    fn toggle_ui(&mut self) { self.ui = !self.ui; }

    fn toggle_windows(&mut self, global_state: &mut GlobalState) {
        if self.bag
            || self.esc_menu
            || self.map
            || self.social
            || self.quest
            || self.spell
            || self.character_window
            || match self.open_windows {
                Windows::None => false,
                _ => true,
            }
        {
            self.bag = false;
            self.esc_menu = false;
            self.map = false;
            self.social = false;
            self.quest = false;
            self.spell = false;
            self.character_window = false;
            self.open_windows = Windows::None;
            self.want_grab = true;

            // Unpause the game if we are on singleplayer
            if let Some(singleplayer) = global_state.singleplayer.as_ref() {
                singleplayer.pause(false);
            };
        } else {
            self.esc_menu = true;
            self.want_grab = false;

            // Pause the game if we are on singleplayer
            if let Some(singleplayer) = global_state.singleplayer.as_ref() {
                singleplayer.pause(true);
            };
        }
    }

    fn open_setting_tab(&mut self, tab: SettingsTab) {
        self.open_windows = Windows::Settings;
        self.esc_menu = false;
        self.settings_tab = tab;
        self.bag = false;
        self.want_grab = false;
    }

    fn toggle_social(&mut self) {
        self.social = !self.social;
        self.spell = false;
        self.quest = false;
    }

    fn open_social_tab(&mut self, social_tab: SocialTab) {
        self.social_tab = social_tab;
        self.spell = false;
        self.quest = false;
    }

    fn toggle_spell(&mut self) {
        self.spell = !self.spell;
        self.social = false;
        self.quest = false;
    }

    fn toggle_quest(&mut self) {
        self.quest = !self.quest;
        self.spell = false;
        self.social = false;
    }
}

pub struct Hud {
    ui: Ui,
    ids: Ids,
    world_map: (/* Id */ Rotations, Vec2<u32>),
    imgs: Imgs,
    item_imgs: ItemImgs,
    fonts: ConrodVoxygenFonts,
    rot_imgs: ImgsRot,
    new_messages: VecDeque<ClientEvent>,
    show: Show,
    never_show: bool,
    intro: bool,
    intro_2: bool,
    to_focus: Option<Option<widget::Id>>,
    force_ungrab: bool,
    force_chat_input: Option<String>,
    force_chat_cursor: Option<Index>,
    pulse: f32,
    velocity: f32,
    voxygen_i18n: std::sync::Arc<VoxygenLocalization>,
}

impl Hud {
    pub fn new(global_state: &mut GlobalState, client: &Client) -> Self {
        let window = &mut global_state.window;
        let settings = &global_state.settings;

        let mut ui = Ui::new(window).unwrap();
        ui.set_scaling_mode(settings.gameplay.ui_scale);
        // Generate ids.
        let ids = Ids::new(ui.id_generator());
        // Load world map
        let world_map = (
            ui.add_graphic_with_rotations(Graphic::Image(client.world_map.0.clone())),
            client.world_map.1,
        );
        // Load images.
        let imgs = Imgs::load(&mut ui).expect("Failed to load images!");
        // Load rotation images.
        let rot_imgs = ImgsRot::load(&mut ui).expect("Failed to load rot images!");
        // Load item images.
        let item_imgs = ItemImgs::new(&mut ui);
        // Load language
        let voxygen_i18n = load_expect::<VoxygenLocalization>(&i18n_asset_key(
            &global_state.settings.language.selected_language,
        ));
        // Load fonts.
        let fonts = ConrodVoxygenFonts::load(&voxygen_i18n.fonts, &mut ui)
            .expect("Impossible to load fonts!");

        Self {
            ui,
            imgs,
            world_map,
            rot_imgs,
            item_imgs,
            fonts,
            ids,
            new_messages: VecDeque::new(),
            intro: false,
            intro_2: false,
            show: Show {
                help: false,
                intro: true,
                debug: false,
                bag: false,
                esc_menu: false,
                open_windows: Windows::None,
                map: false,
                ui: true,
                social: false,
                quest: false,
                spell: false,
                character_window: false,
                mini_map: true,
                settings_tab: SettingsTab::Interface,
                social_tab: SocialTab::Online,
                want_grab: true,
                ingame: true,
                free_look: false,
            },
            to_focus: None,
            never_show: false,
            force_ungrab: false,
            force_chat_input: None,
            force_chat_cursor: None,
            pulse: 0.0,
            velocity: 0.0,
            voxygen_i18n,
        }
    }

    pub fn update_language(&mut self, voxygen_i18n: std::sync::Arc<VoxygenLocalization>) {
        self.voxygen_i18n = voxygen_i18n;
        self.fonts = ConrodVoxygenFonts::load(&self.voxygen_i18n.fonts, &mut self.ui)
            .expect("Impossible to load fonts!");
    }

    fn update_layout(
        &mut self,
        client: &Client,
        global_state: &GlobalState,
        debug_info: DebugInfo,
        dt: Duration,
    ) -> Vec<Event> {
        let mut events = Vec::new();
        let (ref mut ui_widgets, ref mut tooltip_manager) = self.ui.set_widgets();
        // pulse time for pulsating elements
        self.pulse = self.pulse + dt.as_secs_f32();
        self.velocity = match debug_info.velocity {
            Some(velocity) => velocity.0.magnitude(),
            None => 0.0,
        };

        let version = format!(
            "{}-{}",
            env!("CARGO_PKG_VERSION"),
            common::util::GIT_VERSION.to_string()
        );

        if self.show.ingame {
            let ecs = client.state().ecs();
            let pos = ecs.read_storage::<comp::Pos>();
            let stats = ecs.read_storage::<comp::Stats>();
            let energy = ecs.read_storage::<comp::Energy>();
            let hp_floater_lists = ecs.read_storage::<vcomp::HpFloaterList>();
            let interpolated = ecs.read_storage::<vcomp::Interpolated>();
            let players = ecs.read_storage::<comp::Player>();
            let scales = ecs.read_storage::<comp::Scale>();
            let bodies = ecs.read_storage::<comp::Body>();
            let entities = ecs.entities();
            let me = client.entity();
            let own_level = stats
                .get(client.entity())
                .map_or(0, |stats| stats.level.level());
            //self.input = client.read_storage::<comp::ControllerInputs>();
            if let Some(stats) = stats.get(me) {
                // Hurt Frame
                let hp_percentage =
                    stats.health.current() as f32 / stats.health.maximum() as f32 * 100.0;
                if hp_percentage < 10.0 && !stats.is_dead {
                    let hurt_fade =
                        (self.pulse * (10.0 - hp_percentage as f32) * 0.1/* speed factor */).sin()
                            * 0.5
                            + 0.6; //Animation timer
                    Image::new(self.imgs.hurt_bg)
                        .wh_of(ui_widgets.window)
                        .middle_of(ui_widgets.window)
                        .graphics_for(ui_widgets.window)
                        .color(Some(Color::Rgba(1.0, 1.0, 1.0, hurt_fade)))
                        .set(self.ids.hurt_bg, ui_widgets);
                }
                // Death Frame
                if stats.is_dead {
                    Image::new(self.imgs.death_bg)
                        .wh_of(ui_widgets.window)
                        .middle_of(ui_widgets.window)
                        .graphics_for(ui_widgets.window)
                        .color(Some(Color::Rgba(0.0, 0.0, 0.0, 1.0)))
                        .set(self.ids.death_bg, ui_widgets);
                }
                // Crosshair
                if !self.show.help && !stats.is_dead {
                    Image::new(
                        // TODO: Do we want to match on this every frame?
                        match global_state.settings.gameplay.crosshair_type {
                            CrosshairType::Round => self.imgs.crosshair_outer_round,
                            CrosshairType::RoundEdges => self.imgs.crosshair_outer_round_edges,
                            CrosshairType::Edges => self.imgs.crosshair_outer_edges,
                        },
                    )
                    .w_h(21.0 * 1.5, 21.0 * 1.5)
                    .middle_of(ui_widgets.window)
                    .color(Some(Color::Rgba(
                        1.0,
                        1.0,
                        1.0,
                        global_state.settings.gameplay.crosshair_transp,
                    )))
                    .set(self.ids.crosshair_outer, ui_widgets);
                    Image::new(self.imgs.crosshair_inner)
                        .w_h(21.0 * 2.0, 21.0 * 2.0)
                        .middle_of(self.ids.crosshair_outer)
                        .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.6)))
                        .set(self.ids.crosshair_inner, ui_widgets);
                }
            }

            // Nametags and healthbars

            // Max amount the sct font size increases when "flashing"
            const FLASH_MAX: f32 = 25.0;
            const BARSIZE: f64 = 2.0;
            const MANA_BAR_HEIGHT: f64 = BARSIZE * 1.5;
            const MANA_BAR_Y: f64 = MANA_BAR_HEIGHT / 2.0;
            // Get player position.
            let player_pos = client
                .state()
                .ecs()
                .read_storage::<comp::Pos>()
                .get(client.entity())
                .map_or(Vec3::zero(), |pos| pos.0);
            let mut name_id_walker = self.ids.name_tags.walk();
            let mut name_id_bg_walker = self.ids.name_tags_bgs.walk();
            let mut level_id_walker = self.ids.levels.walk();
            let mut level_skull_id_walker = self.ids.levels_skull.walk();
            let mut health_id_walker = self.ids.health_bars.walk();
            let mut mana_id_walker = self.ids.mana_bars.walk();
            let mut health_back_id_walker = self.ids.health_bar_backs.walk();
            let mut health_front_id_walker = self.ids.health_bar_fronts.walk();
            let mut sct_bg_id_walker = self.ids.sct_bgs.walk();
            let mut sct_id_walker = self.ids.scts.walk();

            // Render Health Bars
            for (pos, stats, energy, height_offset, hp_floater_list) in (
                &entities,
                &pos,
                interpolated.maybe(),
                &stats,
                &energy,
                scales.maybe(),
                &bodies,
                &hp_floater_lists,
            )
                .join()
                .filter(|(entity, _, _, stats, _, _, _, _)| {
                    *entity != me && !stats.is_dead
                    //&& stats.health.current() != stats.health.maximum()
                })
                // Don't show outside a certain range
                .filter(|(_, pos, _, _, _, _, _, hpfl)| {
                    pos.0.distance_squared(player_pos)
                        < (if hpfl
                            .time_since_last_dmg_by_me
                            .map_or(false, |t| t < NAMETAG_DMG_TIME)
                        {
                            NAMETAG_DMG_RANGE
                        } else {
                            NAMETAG_RANGE
                        })
                        .powi(2)
                })
                .map(|(_, pos, interpolated, stats, energy, scale, body, f)| {
                    (
                        interpolated.map_or(pos.0, |i| i.pos),
                        stats,
                        energy,
                        // TODO: when body.height() is more accurate remove the 2.0
                        body.height() * 2.0 * scale.map_or(1.0, |s| s.0),
                        f,
                    )
                })
            {
                let back_id = health_back_id_walker.next(
                    &mut self.ids.health_bar_backs,
                    &mut ui_widgets.widget_id_generator(),
                );
                let health_bar_id = health_id_walker.next(
                    &mut self.ids.health_bars,
                    &mut ui_widgets.widget_id_generator(),
                );
                let mana_bar_id = mana_id_walker.next(
                    &mut self.ids.mana_bars,
                    &mut ui_widgets.widget_id_generator(),
                );
                let front_id = health_front_id_walker.next(
                    &mut self.ids.health_bar_fronts,
                    &mut ui_widgets.widget_id_generator(),
                );
                let hp_percentage =
                    stats.health.current() as f64 / stats.health.maximum() as f64 * 100.0;
                let energy_percentage = energy.current() as f64 / energy.maximum() as f64 * 100.0;
                let hp_ani = (self.pulse * 4.0/* speed factor */).cos() * 0.5 + 1.0; //Animation timer
                let crit_hp_color: Color = Color::Rgba(0.79, 0.19, 0.17, hp_ani);

                let ingame_pos = pos + Vec3::unit_z() * height_offset;

                // Background
                Image::new(self.imgs.enemy_health_bg)
                    .w_h(84.0 * BARSIZE, 10.0 * BARSIZE)
                    .x_y(0.0, MANA_BAR_Y + 6.5) //-25.5)
                    .color(Some(Color::Rgba(0.1, 0.1, 0.1, 0.8)))
                    .position_ingame(ingame_pos)
                    .set(back_id, ui_widgets);

                // % HP Filling
                Image::new(self.imgs.enemy_bar)
                    .w_h(73.0 * (hp_percentage / 100.0) * BARSIZE, 6.0 * BARSIZE)
                    .x_y(
                        (4.5 + (hp_percentage / 100.0 * 36.45 - 36.45)) * BARSIZE,
                        MANA_BAR_Y + 7.5,
                    )
                    .color(Some(if hp_percentage <= 25.0 {
                        crit_hp_color
                    } else if hp_percentage <= 50.0 {
                        LOW_HP_COLOR
                    } else {
                        HP_COLOR
                    }))
                    .position_ingame(ingame_pos)
                    .set(health_bar_id, ui_widgets);
                // % Mana Filling
                Rectangle::fill_with(
                    [
                        72.0 * (energy.current() as f64 / energy.maximum() as f64) * BARSIZE,
                        MANA_BAR_HEIGHT,
                    ],
                    MANA_COLOR,
                )
                .x_y(
                    ((3.5 + (energy_percentage / 100.0 * 36.5)) - 36.45) * BARSIZE,
                    MANA_BAR_Y, //-32.0,
                )
                .position_ingame(ingame_pos)
                .set(mana_bar_id, ui_widgets);

                // Foreground
                Image::new(self.imgs.enemy_health)
                    .w_h(84.0 * BARSIZE, 10.0 * BARSIZE)
                    .x_y(0.0, MANA_BAR_Y + 6.5) //-25.5)
                    .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.99)))
                    .position_ingame(ingame_pos)
                    .set(front_id, ui_widgets);

                // Enemy SCT
                if let Some(floaters) = Some(hp_floater_list)
                    .filter(|fl| !fl.floaters.is_empty() && global_state.settings.gameplay.sct)
                    .map(|l| &l.floaters)
                {
                    // Colors
                    const WHITE: Rgb<f32> = Rgb::new(1.0, 0.9, 0.8);
                    const LIGHT_OR: Rgb<f32> = Rgb::new(1.0, 0.925, 0.749);
                    const LIGHT_MED_OR: Rgb<f32> = Rgb::new(1.0, 0.85, 0.498);
                    const MED_OR: Rgb<f32> = Rgb::new(1.0, 0.776, 0.247);
                    const DARK_ORANGE: Rgb<f32> = Rgb::new(1.0, 0.7, 0.0);
                    const RED_ORANGE: Rgb<f32> = Rgb::new(1.0, 0.349, 0.0);
                    const DAMAGE_COLORS: [Rgb<f32>; 6] = [
                        WHITE,
                        LIGHT_OR,
                        LIGHT_MED_OR,
                        MED_OR,
                        DARK_ORANGE,
                        RED_ORANGE,
                    ];
                    // Largest value that select the first color is 40, then it shifts colors
                    // every 5
                    let font_col = |font_size: u32| {
                        DAMAGE_COLORS[(font_size.saturating_sub(36) / 5).min(5) as usize]
                    };

                    if global_state.settings.gameplay.sct_damage_batch {
                        let number_speed = 50.0; // Damage number speed
                        let sct_bg_id = sct_bg_id_walker
                            .next(&mut self.ids.sct_bgs, &mut ui_widgets.widget_id_generator());
                        let sct_id = sct_id_walker
                            .next(&mut self.ids.scts, &mut ui_widgets.widget_id_generator());
                        // Calculate total change
                        // Ignores healing
                        let hp_damage = floaters.iter().fold(0, |acc, f| {
                            if f.hp_change < 0 {
                                acc + f.hp_change
                            } else {
                                acc
                            }
                        });
                        let max_hp_frac = hp_damage.abs() as f32 / stats.health.maximum() as f32;
                        let timer = floaters
                            .last()
                            .expect("There must be at least one floater")
                            .timer;
                        // Increase font size based on fraction of maximum health
                        // "flashes" by having a larger size in the first 100ms
                        let font_size = 30
                            + (max_hp_frac * 30.0) as u32
                            + if timer < 0.1 {
                                (FLASH_MAX * (1.0 - timer / 0.1)) as u32
                            } else {
                                0
                            };
                        let font_col = font_col(font_size);
                        // Timer sets the widget offset
                        let y = (timer as f64 / crate::ecs::sys::floater::HP_SHOWTIME as f64
                            * number_speed)
                            + 100.0;
                        // Timer sets text transparency
                        let fade = ((crate::ecs::sys::floater::HP_SHOWTIME - timer) * 0.25) + 0.2;

                        Text::new(&format!("{}", (hp_damage).abs()))
                            .font_size(font_size)
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(Color::Rgba(0.0, 0.0, 0.0, fade))
                            .x_y(0.0, y - 3.0)
                            .position_ingame(ingame_pos)
                            .set(sct_bg_id, ui_widgets);
                        Text::new(&format!("{}", hp_damage.abs()))
                            .font_size(font_size)
                            .font_id(self.fonts.cyri.conrod_id)
                            .x_y(0.0, y)
                            .color(if hp_damage < 0 {
                                Color::Rgba(font_col.r, font_col.g, font_col.b, fade)
                            } else {
                                Color::Rgba(0.1, 1.0, 0.1, fade)
                            })
                            .position_ingame(ingame_pos)
                            .set(sct_id, ui_widgets);
                    } else {
                        for floater in floaters {
                            let number_speed = 250.0; // Single Numbers Speed
                            let sct_bg_id = sct_bg_id_walker
                                .next(&mut self.ids.sct_bgs, &mut ui_widgets.widget_id_generator());
                            let sct_id = sct_id_walker
                                .next(&mut self.ids.scts, &mut ui_widgets.widget_id_generator());
                            // Calculate total change
                            let max_hp_frac =
                                floater.hp_change.abs() as f32 / stats.health.maximum() as f32;
                            // Increase font size based on fraction of maximum health
                            // "flashes" by having a larger size in the first 100ms
                            let font_size = 30
                                + (max_hp_frac * 30.0) as u32
                                + if floater.timer < 0.1 {
                                    (FLASH_MAX * (1.0 - floater.timer / 0.1)) as u32
                                } else {
                                    0
                                };
                            let font_col = font_col(font_size);
                            // Timer sets the widget offset
                            let y = (floater.timer as f64
                                / crate::ecs::sys::floater::HP_SHOWTIME as f64
                                * number_speed)
                                + 100.0;
                            // Timer sets text transparency
                            let fade = ((crate::ecs::sys::floater::HP_SHOWTIME - floater.timer)
                                * 0.25)
                                + 0.2;

                            Text::new(&format!("{}", (floater.hp_change).abs()))
                                .font_size(font_size)
                                .font_id(self.fonts.cyri.conrod_id)
                                .color(if floater.hp_change < 0 {
                                    Color::Rgba(0.0, 0.0, 0.0, fade)
                                } else {
                                    Color::Rgba(0.1, 1.0, 0.1, 0.0)
                                })
                                .x_y(0.0, y - 3.0)
                                .position_ingame(ingame_pos)
                                .set(sct_bg_id, ui_widgets);
                            Text::new(&format!("{}", (floater.hp_change).abs()))
                                .font_size(font_size)
                                .font_id(self.fonts.cyri.conrod_id)
                                .x_y(0.0, y)
                                .color(if floater.hp_change < 0 {
                                    Color::Rgba(font_col.r, font_col.g, font_col.b, fade)
                                } else {
                                    Color::Rgba(0.1, 1.0, 0.1, 0.0)
                                })
                                .position_ingame(ingame_pos)
                                .set(sct_id, ui_widgets);
                        }
                    }
                }
            }

            if global_state.settings.gameplay.sct {
                // Render Player SCT numbers
                let mut player_sct_bg_id_walker = self.ids.player_sct_bgs.walk();
                let mut player_sct_id_walker = self.ids.player_scts.walk();
                if let (Some(HpFloaterList { floaters, .. }), Some(stats)) = (
                    hp_floater_lists
                        .get(me)
                        .filter(|fl| !fl.floaters.is_empty()),
                    stats.get(me),
                ) {
                    if global_state.settings.gameplay.sct_player_batch {
                        let number_speed = 100.0; // Player Batched Numbers Speed
                        let player_sct_bg_id = player_sct_bg_id_walker.next(
                            &mut self.ids.player_sct_bgs,
                            &mut ui_widgets.widget_id_generator(),
                        );
                        let player_sct_id = player_sct_id_walker.next(
                            &mut self.ids.player_scts,
                            &mut ui_widgets.widget_id_generator(),
                        );
                        // Calculate total change
                        // Ignores healing
                        let hp_damage = floaters.iter().fold(0, |acc, f| f.hp_change.min(0) + acc);
                        let max_hp_frac = hp_damage.abs() as f32 / stats.health.maximum() as f32;
                        let timer = floaters
                            .last()
                            .expect("There must be at least one floater")
                            .timer;
                        // Increase font size based on fraction of maximum health
                        // "flashes" by having a larger size in the first 100ms
                        let font_size = 30
                            + (max_hp_frac * 30.0) as u32
                            + if timer < 0.1 {
                                (FLASH_MAX * (1.0 - timer / 0.1)) as u32
                            } else {
                                0
                            };
                        // Timer sets the widget offset
                        let y = timer as f64 * number_speed * -1.0;
                        // Timer sets text transparency
                        let hp_fade =
                            ((crate::ecs::sys::floater::MY_HP_SHOWTIME - timer) * 0.25) + 0.2;
                        Text::new(&format!("{}", (hp_damage).abs()))
                            .font_size(font_size)
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(if hp_damage < 0 {
                                Color::Rgba(0.0, 0.0, 0.0, hp_fade)
                            } else {
                                Color::Rgba(0.0, 0.0, 0.0, 0.0)
                            })
                            .mid_bottom_with_margin_on(ui_widgets.window, 297.0 + y)
                            .set(player_sct_bg_id, ui_widgets);
                        Text::new(&format!("{}", (hp_damage).abs()))
                            .font_size(font_size)
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(if hp_damage < 0 {
                                Color::Rgba(1.0, 0.1, 0.0, hp_fade)
                            } else {
                                Color::Rgba(0.0, 0.0, 0.0, 0.0)
                            })
                            .mid_bottom_with_margin_on(ui_widgets.window, 300.0 + y)
                            .set(player_sct_id, ui_widgets);
                    };
                    for floater in floaters {
                        // Healing always single numbers so just skip damage when in batch mode

                        if global_state.settings.gameplay.sct_player_batch && floater.hp_change < 0
                        {
                            continue;
                        }
                        let number_speed = 50.0; // Player Heal Speed
                        let player_sct_bg_id = player_sct_bg_id_walker.next(
                            &mut self.ids.player_sct_bgs,
                            &mut ui_widgets.widget_id_generator(),
                        );
                        let player_sct_id = player_sct_id_walker.next(
                            &mut self.ids.player_scts,
                            &mut ui_widgets.widget_id_generator(),
                        );
                        let max_hp_frac =
                            floater.hp_change.abs() as f32 / stats.health.maximum() as f32;
                        // Increase font size based on fraction of maximum health
                        // "flashes" by having a larger size in the first 100ms
                        let font_size = 30
                            + (max_hp_frac * 30.0) as u32
                            + if floater.timer < 0.1 {
                                (FLASH_MAX * (1.0 - floater.timer / 0.1)) as u32
                            } else {
                                0
                            };
                        // Timer sets the widget offset
                        let y = if floater.hp_change < 0 {
                            floater.timer as f64
                            * number_speed
                            * floater.hp_change.signum() as f64
                            //* -1.0
                            + 300.0
                                - ui_widgets.win_h * 0.5
                        } else {
                            floater.timer as f64
                                * number_speed
                                * floater.hp_change.signum() as f64
                                * -1.0
                                + 300.0
                                - ui_widgets.win_h * 0.5
                        };
                        // Healing is offset randomly
                        let x = if floater.hp_change < 0 {
                            0.0
                        } else {
                            (floater.rand as f64 - 0.5) * 0.2 * ui_widgets.win_w
                        };
                        // Timer sets text transparency
                        let hp_fade = ((crate::ecs::sys::floater::MY_HP_SHOWTIME - floater.timer)
                            * 0.25)
                            + 0.2;
                        Text::new(&format!("{}", (floater.hp_change).abs()))
                            .font_size(font_size)
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(Color::Rgba(0.0, 0.0, 0.0, hp_fade))
                            .x_y(x, y - 3.0)
                            .set(player_sct_bg_id, ui_widgets);
                        Text::new(&format!("{}", (floater.hp_change).abs()))
                            .font_size(font_size)
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(if floater.hp_change < 0 {
                                Color::Rgba(1.0, 0.1, 0.0, hp_fade)
                            } else {
                                Color::Rgba(0.1, 1.0, 0.1, hp_fade)
                            })
                            .x_y(x, y)
                            .set(player_sct_id, ui_widgets);
                    }
                }
                // EXP Numbers
                if let (Some(floaters), Some(stats)) = (
                    Some(&*ecs.read_resource::<crate::ecs::MyExpFloaterList>())
                        .map(|l| &l.floaters)
                        .filter(|f| !f.is_empty()),
                    stats.get(me),
                ) {
                    // TODO replace with setting
                    let batched_sct = false;
                    if batched_sct {
                        let number_speed = 50.0; // Number Speed for Cumulated EXP
                        let player_sct_bg_id = player_sct_bg_id_walker.next(
                            &mut self.ids.player_sct_bgs,
                            &mut ui_widgets.widget_id_generator(),
                        );
                        let player_sct_id = player_sct_id_walker.next(
                            &mut self.ids.player_scts,
                            &mut ui_widgets.widget_id_generator(),
                        );
                        // Sum xp change
                        let exp_change = floaters.iter().fold(0, |acc, f| f.exp_change + acc);
                        // Can't fail since we filtered out empty lists above
                        let (timer, rand) = floaters
                            .last()
                            .map(|f| (f.timer, f.rand))
                            .expect("Impossible");
                        // Increase font size based on fraction of maximum health
                        // "flashes" by having a larger size in the first 100ms
                        let font_size_xp = 30
                            + (exp_change.abs() as f32 / stats.exp.maximum() as f32 * 50.0) as u32
                            + if timer < 0.1 {
                                (FLASH_MAX * (1.0 - timer / 0.1)) as u32
                            } else {
                                0
                            };

                        let y = timer as f64 * number_speed; // Timer sets the widget offset
                        let fade = ((4.0 - timer as f32) * 0.25) + 0.2; // Timer sets text transparency

                        Text::new(&format!("{} Exp", exp_change))
                            .font_size(font_size_xp)
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(Color::Rgba(0.0, 0.0, 0.0, fade))
                            .x_y(
                                ui_widgets.win_w * (0.5 * rand.0 as f64 - 0.25),
                                ui_widgets.win_h * (0.15 * rand.1 as f64) + y - 3.0,
                            )
                            .set(player_sct_bg_id, ui_widgets);
                        Text::new(&format!("{} Exp", exp_change))
                            .font_size(font_size_xp)
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(Color::Rgba(0.59, 0.41, 0.67, fade))
                            .x_y(
                                ui_widgets.win_w * (0.5 * rand.0 as f64 - 0.25),
                                ui_widgets.win_h * (0.15 * rand.1 as f64) + y,
                            )
                            .set(player_sct_id, ui_widgets);
                    } else {
                        for floater in floaters {
                            let number_speed = 50.0; // Number Speed for Single EXP
                            let player_sct_bg_id = player_sct_bg_id_walker.next(
                                &mut self.ids.player_sct_bgs,
                                &mut ui_widgets.widget_id_generator(),
                            );
                            let player_sct_id = player_sct_id_walker.next(
                                &mut self.ids.player_scts,
                                &mut ui_widgets.widget_id_generator(),
                            );
                            // Increase font size based on fraction of maximum health
                            // "flashes" by having a larger size in the first 100ms
                            let font_size_xp = 30
                                + (floater.exp_change.abs() as f32 / stats.exp.maximum() as f32
                                    * 50.0) as u32
                                + if floater.timer < 0.1 {
                                    (FLASH_MAX * (1.0 - floater.timer / 0.1)) as u32
                                } else {
                                    0
                                };

                            let y = floater.timer as f64 * number_speed; // Timer sets the widget offset
                            let fade = ((4.0 - floater.timer as f32) * 0.25) + 0.2; // Timer sets text transparency

                            Text::new(&format!("{} Exp", floater.exp_change))
                                .font_size(font_size_xp)
                                .font_id(self.fonts.cyri.conrod_id)
                                .color(Color::Rgba(0.0, 0.0, 0.0, fade))
                                .x_y(
                                    ui_widgets.win_w * (0.5 * floater.rand.0 as f64 - 0.25),
                                    ui_widgets.win_h * (0.15 * floater.rand.1 as f64) + y - 3.0,
                                )
                                .set(player_sct_bg_id, ui_widgets);
                            Text::new(&format!("{} Exp", floater.exp_change))
                                .font_size(font_size_xp)
                                .font_id(self.fonts.cyri.conrod_id)
                                .color(Color::Rgba(0.59, 0.41, 0.67, fade))
                                .x_y(
                                    ui_widgets.win_w * (0.5 * floater.rand.0 as f64 - 0.25),
                                    ui_widgets.win_h * (0.15 * floater.rand.1 as f64) + y,
                                )
                                .set(player_sct_id, ui_widgets);
                        }
                    }
                }
            }

            // Render Name Tags
            for (pos, name, level, height_offset) in (
                &entities,
                &pos,
                interpolated.maybe(),
                &stats,
                players.maybe(),
                scales.maybe(),
                &bodies,
                &hp_floater_lists,
            )
                .join()
                .filter(|(entity, _, _, stats, _, _, _, _)| *entity != me && !stats.is_dead)
                // Don't show outside a certain range
                .filter(|(_, pos, _, _, _, _, _, hpfl)| {
                    pos.0.distance_squared(player_pos)
                        < (if hpfl
                            .time_since_last_dmg_by_me
                            .map_or(false, |t| t < NAMETAG_DMG_TIME)
                        {
                            NAMETAG_DMG_RANGE
                        } else {
                            NAMETAG_RANGE
                        })
                        .powi(2)
                })
                .map(|(_, pos, interpolated, stats, player, scale, body, _)| {
                    // TODO: This is temporary
                    // If the player used the default character name display their name instead
                    let name = if stats.name == "Character Name" {
                        player.map_or(&stats.name, |p| &p.alias)
                    } else {
                        &stats.name
                    };
                    (
                        interpolated.map_or(pos.0, |i| i.pos),
                        format!("{}", name),
                        stats.level,
                        body.height() * 2.0 * scale.map_or(1.0, |s| s.0),
                    )
                })
            {
                let name_id = name_id_walker.next(
                    &mut self.ids.name_tags,
                    &mut ui_widgets.widget_id_generator(),
                );
                let name_bg_id = name_id_bg_walker.next(
                    &mut self.ids.name_tags_bgs,
                    &mut ui_widgets.widget_id_generator(),
                );
                let level_id = level_id_walker
                    .next(&mut self.ids.levels, &mut ui_widgets.widget_id_generator());
                let level_skull_id = level_skull_id_walker.next(
                    &mut self.ids.levels_skull,
                    &mut ui_widgets.widget_id_generator(),
                );

                let ingame_pos = pos + Vec3::unit_z() * height_offset;

                // Name
                Text::new(&name)
                    .font_id(self.fonts.cyri.conrod_id)
                    .font_size(30)
                    .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
                    .x_y(-1.0, MANA_BAR_Y + 48.0)
                    .position_ingame(ingame_pos)
                    .set(name_bg_id, ui_widgets);
                Text::new(&name)
                    .font_id(self.fonts.cyri.conrod_id)
                    .font_size(30)
                    .color(Color::Rgba(0.61, 0.61, 0.89, 1.0))
                    .x_y(0.0, MANA_BAR_Y + 50.0)
                    .position_ingame(ingame_pos)
                    .set(name_id, ui_widgets);

                // Level
                const LOW: Color = Color::Rgba(0.54, 0.81, 0.94, 0.4);
                const HIGH: Color = Color::Rgba(1.0, 0.0, 0.0, 1.0);
                const EQUAL: Color = Color::Rgba(1.0, 1.0, 1.0, 1.0);
                let op_level = level.level();
                let level_str = format!("{}", op_level);
                // Change visuals of the level display depending on the player level/opponent
                // level
                let level_comp = op_level as i64 - own_level as i64;
                // + 10 level above player -> skull
                // + 5-10 levels above player -> high
                // -5 - +5 levels around player level -> equal
                // - 5 levels below player -> low
                Text::new(if level_comp < 10 { &level_str } else { "?" })
                    .font_id(self.fonts.cyri.conrod_id)
                    .font_size(if op_level > 9 && level_comp < 10 {
                        14
                    } else {
                        15
                    })
                    .color(if level_comp > 4 {
                        HIGH
                    } else if level_comp < -5 {
                        LOW
                    } else {
                        EQUAL
                    })
                    .x_y(-37.0 * BARSIZE, MANA_BAR_Y + 9.0)
                    .position_ingame(ingame_pos)
                    .set(level_id, ui_widgets);
                if level_comp > 9 {
                    let skull_ani = ((self.pulse * 0.7/* speed factor */).cos() * 0.5 + 0.5) * 10.0; //Animation timer
                    Image::new(if skull_ani as i32 == 1 && rand::random::<f32>() < 0.9 {
                        self.imgs.skull_2
                    } else {
                        self.imgs.skull
                    })
                    .w_h(18.0 * BARSIZE, 18.0 * BARSIZE)
                    .x_y(-39.0 * BARSIZE, MANA_BAR_Y + 7.0)
                    .color(Some(Color::Rgba(1.0, 1.0, 1.0, 1.0)))
                    .position_ingame(ingame_pos)
                    .set(level_skull_id, ui_widgets);
                }
            }
        }

        // Introduction Text
        let intro_text = &self.voxygen_i18n.get("hud.welcome");
        if self.show.intro && !self.show.esc_menu && !self.intro_2 {
            match global_state.settings.gameplay.intro_show {
                Intro::Show => {
                    Rectangle::fill_with([800.0, 850.0], Color::Rgba(0.0, 0.0, 0.0, 0.80))
                        .top_left_with_margins_on(ui_widgets.window, 180.0, 10.0)
                        .floating(true)
                        .set(self.ids.intro_bg, ui_widgets);
                    Text::new(intro_text)
                        .top_left_with_margins_on(self.ids.intro_bg, 10.0, 10.0)
                        .font_size(self.fonts.cyri.scale(20))
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(TEXT_COLOR)
                        .set(self.ids.intro_text, ui_widgets);
                    if Button::image(self.imgs.button)
                        .w_h(100.0, 50.0)
                        .mid_bottom_with_margin_on(self.ids.intro_bg, 10.0)
                        .label(&self.voxygen_i18n.get("common.close"))
                        .label_font_size(self.fonts.cyri.scale(20))
                        .label_font_id(self.fonts.cyri.conrod_id)
                        .label_color(TEXT_COLOR)
                        .hover_image(self.imgs.button_hover)
                        .press_image(self.imgs.button_press)
                        .set(self.ids.intro_close, ui_widgets)
                        .was_clicked()
                    {
                        if self.never_show {
                            events.push(Event::Intro(Intro::Never));
                            self.never_show = !self.never_show;
                            self.intro = false;
                            self.intro_2 = false;
                        } else {
                            self.show.intro = !self.show.intro;
                            self.intro = false;
                            self.intro_2 = false;
                        }
                    }
                    if Button::image(if self.never_show {
                        self.imgs.checkbox_checked
                    } else {
                        self.imgs.checkbox
                    })
                    .w_h(20.0, 20.0)
                    .right_from(self.ids.intro_close, 10.0)
                    .hover_image(if self.never_show {
                        self.imgs.checkbox_checked_mo
                    } else {
                        self.imgs.checkbox_mo
                    })
                    .press_image(self.imgs.checkbox_press)
                    .set(self.ids.intro_check, ui_widgets)
                    .was_clicked()
                    {
                        self.never_show = !self.never_show
                    };
                    Text::new(&self.voxygen_i18n.get("hud.do_not_show_on_startup"))
                        .right_from(self.ids.intro_check, 10.0)
                        .font_size(self.fonts.cyri.scale(10))
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(TEXT_COLOR)
                        .set(self.ids.intro_check_text, ui_widgets);
                    // X-button
                    if Button::image(self.imgs.close_button)
                        .w_h(40.0, 40.0)
                        .hover_image(self.imgs.close_button_hover)
                        .press_image(self.imgs.close_button_press)
                        .top_right_with_margins_on(self.ids.intro_bg, 0.0, 0.0)
                        .color(Color::Rgba(1.0, 1.0, 1.0, 0.8))
                        .set(self.ids.intro_close_4, ui_widgets)
                        .was_clicked()
                    {
                        if self.never_show {
                            events.push(Event::Intro(Intro::Never));
                            self.never_show = !self.never_show;
                            self.intro = false;
                            self.intro_2 = false;
                        } else {
                            self.show.intro = !self.show.intro;
                            self.intro = false;
                            self.intro_2 = false;
                        }
                    };
                },
                Intro::Never => {},
            }
        }

        if self.intro_2 && !self.show.esc_menu {
            Rectangle::fill_with([800.0, 850.0], Color::Rgba(0.0, 0.0, 0.0, 0.80))
                .top_left_with_margins_on(ui_widgets.window, 180.0, 10.0)
                .floating(true)
                .set(self.ids.intro_bg, ui_widgets);
            Text::new(intro_text)
                .top_left_with_margins_on(self.ids.intro_bg, 10.0, 10.0)
                .font_size(self.fonts.cyri.scale(20))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(self.ids.intro_text, ui_widgets);
            if Button::image(self.imgs.button)
                .w_h(100.0, 50.0)
                .mid_bottom_with_margin_on(self.ids.intro_bg, 10.0)
                .label(&self.voxygen_i18n.get("common.close"))
                .label_font_size(self.fonts.cyri.scale(20))
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_color(TEXT_COLOR)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .set(self.ids.intro_close_3, ui_widgets)
                .was_clicked()
            {
                self.intro_2 = false;
            }
            // X-button
            if Button::image(self.imgs.close_button)
                .w_h(40.0, 40.0)
                .hover_image(self.imgs.close_button_hover)
                .press_image(self.imgs.close_button_press)
                .top_right_with_margins_on(self.ids.intro_bg, 0.0, 0.0)
                .color(Color::Rgba(1.0, 1.0, 1.0, 0.8))
                .set(self.ids.intro_close_4, ui_widgets)
                .was_clicked()
            {
                if self.never_show {
                    events.push(Event::Intro(Intro::Never));
                    self.never_show = !self.never_show;
                    self.intro = false;
                    self.intro_2 = false;
                } else {
                    self.show.intro = !self.show.intro;
                    self.intro = false;
                    self.intro_2 = false;
                }
            };
        }

        // Display debug window.
        if global_state.settings.gameplay.toggle_debug {
            // Alpha Version
            Text::new(&version)
                .top_left_with_margins_on(ui_widgets.window, 5.0, 5.0)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(self.ids.version, ui_widgets);
            // Ticks per second
            Text::new(&format!("FPS: {:.0}", debug_info.tps))
                .color(TEXT_COLOR)
                .down_from(self.ids.version, 5.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .set(self.ids.fps_counter, ui_widgets);
            // Ping
            Text::new(&format!("Ping: {:.0}ms", debug_info.ping_ms))
                .color(TEXT_COLOR)
                .down_from(self.ids.fps_counter, 5.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .set(self.ids.ping, ui_widgets);
            // Player's position
            let coordinates_text = match debug_info.coordinates {
                Some(coordinates) => format!(
                    "Coordinates: ({:.0}, {:.0}, {:.0})",
                    coordinates.0.x, coordinates.0.y, coordinates.0.z,
                ),
                None => "Player has no Pos component".to_owned(),
            };
            Text::new(&coordinates_text)
                .color(TEXT_COLOR)
                .down_from(self.ids.ping, 5.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .set(self.ids.coordinates, ui_widgets);
            // Player's velocity
            let velocity_text = match debug_info.velocity {
                Some(velocity) => format!(
                    "Velocity: ({:.1}, {:.1}, {:.1}) [{:.1} u/s]",
                    velocity.0.x,
                    velocity.0.y,
                    velocity.0.z,
                    velocity.0.magnitude()
                ),
                None => "Player has no Vel component".to_owned(),
            };
            Text::new(&velocity_text)
                .color(TEXT_COLOR)
                .down_from(self.ids.coordinates, 5.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .set(self.ids.velocity, ui_widgets);
            // Loaded distance
            Text::new(&format!(
                "View distance: {:.2} blocks ({:.2} chunks)",
                client.loaded_distance(),
                client.loaded_distance() / TerrainChunk::RECT_SIZE.x as f32,
            ))
            .color(TEXT_COLOR)
            .down_from(self.ids.velocity, 5.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(14))
            .set(self.ids.loaded_distance, ui_widgets);
            // Time
            let time_in_seconds = client.state().get_time_of_day();
            let current_time = NaiveTime::from_num_seconds_from_midnight(
                // Wraps around back to 0s if it exceeds 24 hours (24 hours = 86400s)
                (time_in_seconds as u64 % 86400) as u32,
                0,
            );
            Text::new(&format!(
                "Time: {}",
                current_time.format("%H:%M").to_string()
            ))
            .color(TEXT_COLOR)
            .down_from(self.ids.loaded_distance, 5.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(14))
            .set(self.ids.time, ui_widgets);

            // Number of entities
            let entity_count = client.state().ecs().entities().join().count();
            Text::new(&format!("Entity count: {}", entity_count))
                .color(TEXT_COLOR)
                .down_from(self.ids.time, 5.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .set(self.ids.entity_count, ui_widgets);

            // Number of chunks
            Text::new(&format!(
                "Chunks: {} ({} visible)",
                debug_info.num_chunks, debug_info.num_visible_chunks,
            ))
            .color(TEXT_COLOR)
            .down_from(self.ids.entity_count, 5.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(14))
            .set(self.ids.num_chunks, ui_widgets);

            // Number of figures
            Text::new(&format!(
                "Figures: {} ({} visible)",
                debug_info.num_figures, debug_info.num_figures_visible,
            ))
            .color(TEXT_COLOR)
            .down_from(self.ids.num_chunks, 5.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(14))
            .set(self.ids.num_figures, ui_widgets);

            // Help Window
            Text::new(
                &self
                    .voxygen_i18n
                    .get("hud.press_key_to_toggle_keybindings_fmt")
                    .replace(
                        "{key}",
                        &format!("{:?}", global_state.settings.controls.help),
                    ),
            )
            .color(TEXT_COLOR)
            .down_from(self.ids.num_figures, 5.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(14))
            .set(self.ids.help_info, ui_widgets);
            // Info about Debug Shortcut
            Text::new(
                &self
                    .voxygen_i18n
                    .get("hud.press_key_to_toggle_debug_info_fmt")
                    .replace(
                        "{key}",
                        &format!("{:?}", global_state.settings.controls.toggle_debug),
                    ),
            )
            .color(TEXT_COLOR)
            .down_from(self.ids.help_info, 5.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(14))
            .set(self.ids.debug_info, ui_widgets);
        } else {
            // Help Window
            Text::new(
                &self
                    .voxygen_i18n
                    .get("hud.press_key_to_show_keybindings_fmt")
                    .replace(
                        "{key}",
                        &format!("{:?}", global_state.settings.controls.help),
                    ),
            )
            .color(TEXT_COLOR)
            .top_left_with_margins_on(ui_widgets.window, 5.0, 5.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(16))
            .set(self.ids.help_info, ui_widgets);
            // Info about Debug Shortcut
            Text::new(
                &self
                    .voxygen_i18n
                    .get("hud.press_key_to_show_debug_info_fmt")
                    .replace(
                        "{key}",
                        &format!("{:?}", global_state.settings.controls.toggle_debug),
                    ),
            )
            .color(TEXT_COLOR)
            .down_from(self.ids.help_info, 5.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(12))
            .set(self.ids.debug_info, ui_widgets);
        }

        // Help Text
        if self.show.help && !self.show.map && !self.show.esc_menu {
            Image::new(self.imgs.help)
                .middle_of(ui_widgets.window)
                .w_h(1260.0 * 1.2, 519.0 * 1.2)
                .set(self.ids.help, ui_widgets);
            // Show tips
            if Button::image(self.imgs.button)
                .w_h(120.0, 50.0)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .label(&self.voxygen_i18n.get("hud.show_tips"))
                .label_font_size(self.fonts.cyri.scale(20))
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_color(TEXT_COLOR)
                .mid_bottom_with_margin_on(self.ids.help, 20.0)
                .set(self.ids.button_help3, ui_widgets)
                .was_clicked()
            {
                self.show.help = false;
                self.show.intro = false;
                self.intro = false;
                self.intro_2 = true;
            };
            // X-button
            if Button::image(self.imgs.close_button)
                .w_h(40.0, 40.0)
                .hover_image(self.imgs.close_button_hover)
                .press_image(self.imgs.close_button_press)
                .top_right_with_margins_on(self.ids.help, 0.0, 0.0)
                .color(Color::Rgba(1.0, 1.0, 1.0, 0.8))
                .set(self.ids.button_help2, ui_widgets)
                .was_clicked()
            {
                self.show.help = false;
            };
        }

        // Bag button and nearby icons
        match Buttons::new(
            &self.show.open_windows,
            self.show.map,
            self.show.bag,
            &self.imgs,
            &self.fonts,
        )
        .set(self.ids.buttons, ui_widgets)
        {
            Some(buttons::Event::ToggleBag) => self.show.toggle_bag(),
            Some(buttons::Event::ToggleSettings) => self.show.toggle_settings(),
            Some(buttons::Event::ToggleCharacter) => self.show.toggle_char_window(),
            Some(buttons::Event::ToggleSocial) => self.show.toggle_social(),
            Some(buttons::Event::ToggleSpell) => self.show.toggle_spell(),
            Some(buttons::Event::ToggleQuest) => self.show.toggle_quest(),
            Some(buttons::Event::ToggleMap) => self.show.toggle_map(),
            None => {},
        }

        // MiniMap
        match MiniMap::new(
            &self.show,
            client,
            &self.imgs,
            &self.rot_imgs,
            &self.world_map,
            &self.fonts,
        )
        .set(self.ids.minimap, ui_widgets)
        {
            Some(minimap::Event::Toggle) => self.show.toggle_mini_map(),
            None => {},
        }

        // Bag contents
        if self.show.bag {
            match Bag::new(
                client,
                &self.imgs,
                &self.item_imgs,
                &self.fonts,
                &self.rot_imgs,
                tooltip_manager,
                self.pulse,
            )
            .set(self.ids.bag, ui_widgets)
            {
                Some(bag::Event::HudEvent(event)) => events.push(event),
                Some(bag::Event::Close) => {
                    self.show.bag(false);
                    self.force_ungrab = true;
                },
                None => {},
            }
        }

        // Skillbar
        // Get player stats
        let ecs = client.state().ecs();
        let stats = ecs.read_storage::<comp::Stats>();
        let energy = ecs.read_storage::<comp::Energy>();
        let character_state = ecs.read_storage::<comp::CharacterState>();
        let entity = client.entity();
        let controller = ecs.read_storage::<comp::Controller>();
        if let (Some(stats), Some(energy), Some(character_state), Some(controller)) = (
            stats.get(entity),
            energy.get(entity),
            character_state.get(entity),
            controller.get(entity).map(|c| &c.inputs),
        ) {
            Skillbar::new(
                global_state,
                &self.imgs,
                &self.fonts,
                &stats,
                &energy,
                &character_state,
                self.pulse,
                &controller,
            )
            .set(self.ids.skillbar, ui_widgets);
        }

        // Chat box
        match Chat::new(
            &mut self.new_messages,
            global_state,
            &self.imgs,
            &self.fonts,
        )
        .and_then(self.force_chat_input.take(), |c, input| c.input(input))
        .and_then(self.force_chat_cursor.take(), |c, pos| c.cursor_pos(pos))
        .set(self.ids.chat, ui_widgets)
        {
            Some(chat::Event::SendMessage(message)) => {
                events.push(Event::SendMessage(message));
            },
            Some(chat::Event::Focus(focus_id)) => {
                self.to_focus = Some(Some(focus_id));
            },
            None => {},
        }

        self.new_messages = VecDeque::new();

        // Windows

        // Char Window will always appear at the left side. Other Windows default to the
        // left side, but when the Char Window is opened they will appear to the right
        // of it.

        // Settings
        if let Windows::Settings = self.show.open_windows {
            for event in SettingsWindow::new(
                &global_state,
                &self.show,
                &self.imgs,
                &self.fonts,
                &self.voxygen_i18n,
            )
            .set(self.ids.settings_window, ui_widgets)
            {
                match event {
                    settings_window::Event::Sct(sct) => {
                        events.push(Event::Sct(sct));
                    },
                    settings_window::Event::SctPlayerBatch(sct_player_batch) => {
                        events.push(Event::SctPlayerBatch(sct_player_batch));
                    },
                    settings_window::Event::SctDamageBatch(sct_damage_batch) => {
                        events.push(Event::SctDamageBatch(sct_damage_batch));
                    },
                    settings_window::Event::ToggleHelp => self.show.help = !self.show.help,
                    settings_window::Event::ToggleDebug => self.show.debug = !self.show.debug,
                    settings_window::Event::ChangeTab(tab) => self.show.open_setting_tab(tab),
                    settings_window::Event::Close => {
                        // Unpause the game if we are on singleplayer so that we can logout
                        if let Some(singleplayer) = global_state.singleplayer.as_ref() {
                            singleplayer.pause(false);
                        };

                        self.show.settings(false)
                    },
                    settings_window::Event::AdjustMousePan(sensitivity) => {
                        events.push(Event::AdjustMousePan(sensitivity));
                    },
                    settings_window::Event::AdjustMouseZoom(sensitivity) => {
                        events.push(Event::AdjustMouseZoom(sensitivity));
                    },
                    settings_window::Event::ChatTransp(chat_transp) => {
                        events.push(Event::ChatTransp(chat_transp));
                    },
                    settings_window::Event::ToggleZoomInvert(zoom_inverted) => {
                        events.push(Event::ToggleZoomInvert(zoom_inverted));
                    },
                    settings_window::Event::ToggleMouseYInvert(mouse_y_inverted) => {
                        events.push(Event::ToggleMouseYInvert(mouse_y_inverted));
                    },
                    settings_window::Event::AdjustViewDistance(view_distance) => {
                        events.push(Event::AdjustViewDistance(view_distance));
                    },
                    settings_window::Event::CrosshairTransp(crosshair_transp) => {
                        events.push(Event::CrosshairTransp(crosshair_transp));
                    },
                    settings_window::Event::Intro(intro_show) => {
                        events.push(Event::Intro(intro_show));
                    },
                    settings_window::Event::AdjustMusicVolume(music_volume) => {
                        events.push(Event::AdjustMusicVolume(music_volume));
                    },
                    settings_window::Event::AdjustSfxVolume(sfx_volume) => {
                        events.push(Event::AdjustSfxVolume(sfx_volume));
                    },
                    settings_window::Event::MaximumFPS(max_fps) => {
                        events.push(Event::ChangeMaxFPS(max_fps));
                    },
                    settings_window::Event::ChangeAudioDevice(name) => {
                        events.push(Event::ChangeAudioDevice(name));
                    },
                    settings_window::Event::CrosshairType(crosshair_type) => {
                        events.push(Event::CrosshairType(crosshair_type));
                    },
                    settings_window::Event::ToggleXpBar(xp_bar) => {
                        events.push(Event::ToggleXpBar(xp_bar));
                    },
                    settings_window::Event::ToggleBarNumbers(bar_numbers) => {
                        events.push(Event::ToggleBarNumbers(bar_numbers));
                    },
                    settings_window::Event::ToggleShortcutNumbers(shortcut_numbers) => {
                        events.push(Event::ToggleShortcutNumbers(shortcut_numbers));
                    },
                    settings_window::Event::UiScale(scale_change) => {
                        events.push(Event::UiScale(scale_change));
                    },
                    settings_window::Event::AdjustFOV(new_fov) => {
                        events.push(Event::ChangeFOV(new_fov));
                    },
                    settings_window::Event::AdjustGamma(new_gamma) => {
                        events.push(Event::ChangeGamma(new_gamma));
                    },
                    settings_window::Event::ChangeAaMode(new_aa_mode) => {
                        events.push(Event::ChangeAaMode(new_aa_mode));
                    },
                    settings_window::Event::ChangeCloudMode(new_cloud_mode) => {
                        events.push(Event::ChangeCloudMode(new_cloud_mode));
                    },
                    settings_window::Event::ChangeFluidMode(new_fluid_mode) => {
                        events.push(Event::ChangeFluidMode(new_fluid_mode));
                    },
                    settings_window::Event::ChangeLanguage(language) => {
                        events.push(Event::ChangeLanguage(language));
                    },
                    settings_window::Event::ToggleFullscreen => {
                        events.push(Event::ToggleFullscreen);
                    },
                    settings_window::Event::AdjustWindowSize(new_size) => {
                        events.push(Event::AdjustWindowSize(new_size));
                    },
                    settings_window::Event::ChangeFreeLookBehavior(behavior) => {
                        events.push(Event::ChangeFreeLookBehavior(behavior));
                    },
                }
            }
        }

        // Social Window
        if self.show.social {
            for event in Social::new(
                &self.show,
                client,
                &self.imgs,
                &self.fonts,
                &self.voxygen_i18n,
            )
            .set(self.ids.social_window, ui_widgets)
            {
                match event {
                    social::Event::Close => self.show.social(false),
                    social::Event::ChangeSocialTab(social_tab) => {
                        self.show.open_social_tab(social_tab)
                    },
                }
            }
        }

        // Character Window
        if self.show.character_window {
            let ecs = client.state().ecs();
            let stats = ecs.read_storage::<comp::Stats>();
            let player_stats = stats.get(client.entity()).unwrap();
            match CharacterWindow::new(
                &self.show,
                &player_stats,
                &self.imgs,
                &self.fonts,
                &self.voxygen_i18n,
            )
            .set(self.ids.character_window, ui_widgets)
            {
                Some(character_window::Event::Close) => {
                    self.show.character_window(false);
                    self.force_ungrab = true;
                },
                None => {},
            }
        }

        // Spellbook
        if self.show.spell {
            match Spell::new(
                &self.show,
                client,
                &self.imgs,
                &self.fonts,
                &self.voxygen_i18n,
            )
            .set(self.ids.spell, ui_widgets)
            {
                Some(spell::Event::Close) => {
                    self.show.spell(false);
                    self.force_ungrab = true;
                },
                None => {},
            }
        }

        // Quest Log
        if self.show.quest {
            match Quest::new(
                &self.show,
                client,
                &self.imgs,
                &self.fonts,
                &self.voxygen_i18n,
            )
            .set(self.ids.quest, ui_widgets)
            {
                Some(quest::Event::Close) => {
                    self.show.quest(false);
                    self.force_ungrab = true;
                },
                None => {},
            }
        }
        // Map
        if self.show.map {
            match Map::new(
                &self.show,
                client,
                &self.imgs,
                &self.rot_imgs,
                &self.world_map,
                &self.fonts,
                self.pulse,
                self.velocity,
            )
            .set(self.ids.map, ui_widgets)
            {
                Some(map::Event::Close) => {
                    self.show.map(false);
                    self.force_ungrab = true;
                },
                None => {},
            }
        }

        if self.show.esc_menu {
            match EscMenu::new(&self.imgs, &self.fonts, &self.voxygen_i18n)
                .set(self.ids.esc_menu, ui_widgets)
            {
                Some(esc_menu::Event::OpenSettings(tab)) => {
                    self.show.open_setting_tab(tab);
                },
                Some(esc_menu::Event::Close) => {
                    self.show.esc_menu = false;
                    self.show.want_grab = true;
                    self.force_ungrab = false;

                    // Unpause the game if we are on singleplayer
                    if let Some(singleplayer) = global_state.singleplayer.as_ref() {
                        singleplayer.pause(false);
                    };
                },
                Some(esc_menu::Event::Logout) => {
                    // Unpause the game if we are on singleplayer so that we can logout
                    if let Some(singleplayer) = global_state.singleplayer.as_ref() {
                        singleplayer.pause(false);
                    };

                    events.push(Event::Logout);
                },
                Some(esc_menu::Event::Quit) => events.push(Event::Quit),
                Some(esc_menu::Event::CharacterSelection) => {
                    // Unpause the game if we are on singleplayer so that we can logout
                    if let Some(singleplayer) = global_state.singleplayer.as_ref() {
                        singleplayer.pause(false);
                    };

                    events.push(Event::CharacterSelection)
                },
                None => {},
            }
        }

        // Free look indicator
        if self.show.free_look {
            Text::new(&self.voxygen_i18n.get("hud.free_look_indicator"))
                .color(TEXT_BG)
                .mid_top_with_margin_on(ui_widgets.window, 100.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(20))
                .set(self.ids.free_look_bg, ui_widgets);
            Text::new(&self.voxygen_i18n.get("hud.free_look_indicator"))
                .color(KILL_COLOR)
                .top_left_with_margins_on(self.ids.free_look_bg, -1.0, -1.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(20))
                .set(self.ids.free_look_txt, ui_widgets);
        }

        events
    }

    pub fn new_message(&mut self, msg: ClientEvent) { self.new_messages.push_back(msg); }

    pub fn scale_change(&mut self, scale_change: ScaleChange) -> ScaleMode {
        let scale_mode = match scale_change {
            ScaleChange::Adjust(scale) => ScaleMode::Absolute(scale),
            ScaleChange::ToAbsolute => self.ui.scale().scaling_mode_as_absolute(),
            ScaleChange::ToRelative => self.ui.scale().scaling_mode_as_relative(),
        };
        self.ui.set_scaling_mode(scale_mode);
        scale_mode
    }

    // Checks if a TextEdit widget has the keyboard captured.
    fn typing(&self) -> bool {
        if let Some(id) = self.ui.widget_capturing_keyboard() {
            self.ui
                .widget_graph()
                .widget(id)
                .filter(|c| {
                    c.type_id == std::any::TypeId::of::<<widget::TextEdit as Widget>::State>()
                })
                .is_some()
        } else {
            false
        }
    }

    pub fn handle_event(&mut self, event: WinEvent, global_state: &mut GlobalState) -> bool {
        let cursor_grabbed = global_state.window.is_cursor_grabbed();
        let handled = match event {
            WinEvent::Ui(event) => {
                if (self.typing() && event.is_keyboard() && self.show.ui)
                    || !(cursor_grabbed && event.is_keyboard_or_mouse())
                {
                    self.ui.handle_event(event);
                }
                true
            },
            WinEvent::InputUpdate(GameInput::ToggleInterface, true) if !self.typing() => {
                self.show.toggle_ui();
                true
            },
            WinEvent::InputUpdate(GameInput::ToggleCursor, true) if !self.typing() => {
                self.force_ungrab = !self.force_ungrab;
                true
            },
            _ if !self.show.ui => false,
            WinEvent::Zoom(_) => !cursor_grabbed && !self.ui.no_widget_capturing_mouse(),

            WinEvent::InputUpdate(GameInput::Enter, true) => {
                self.ui.focus_widget(if self.typing() {
                    None
                } else {
                    Some(self.ids.chat)
                });
                true
            },
            WinEvent::InputUpdate(GameInput::Escape, true) => {
                if self.typing() {
                    self.ui.focus_widget(None);
                } else {
                    // Close windows on esc
                    self.show.toggle_windows(global_state);
                }
                true
            },

            // Press key while not typing
            WinEvent::InputUpdate(key, true) if !self.typing() => match key {
                GameInput::Command => {
                    self.force_chat_input = Some("/".to_owned());
                    self.force_chat_cursor = Some(Index { line: 0, char: 1 });
                    self.ui.focus_widget(Some(self.ids.chat));
                    true
                },
                GameInput::Map => {
                    self.show.toggle_map();
                    true
                },
                GameInput::Bag => {
                    self.show.toggle_bag();
                    true
                },
                GameInput::QuestLog => {
                    self.show.toggle_quest();
                    true
                },
                GameInput::CharacterWindow => {
                    self.show.toggle_char_window();
                    true
                },
                GameInput::Social => {
                    self.show.toggle_social();
                    true
                },
                GameInput::Spellbook => {
                    self.show.toggle_spell();
                    true
                },
                GameInput::Settings => {
                    self.show.toggle_settings();
                    true
                },
                GameInput::Help => {
                    self.show.toggle_help();
                    true
                },
                GameInput::ToggleDebug => {
                    global_state.settings.gameplay.toggle_debug =
                        !global_state.settings.gameplay.toggle_debug;
                    true
                },
                GameInput::ToggleIngameUi => {
                    self.show.ingame = !self.show.ingame;
                    true
                },
                _ => false,
            },
            // Else the player is typing in chat
            WinEvent::InputUpdate(_key, _) => self.typing(),
            WinEvent::Char(_) => self.typing(),
            WinEvent::Focused(state) => {
                self.force_ungrab = !state;
                true
            },

            _ => false,
        };
        // Handle cursor grab.
        global_state
            .window
            .grab_cursor(!self.force_ungrab && self.show.want_grab);

        handled
    }

    pub fn maintain(
        &mut self,
        client: &Client,
        global_state: &mut GlobalState,
        debug_info: DebugInfo,
        camera: &Camera,
        dt: Duration,
    ) -> Vec<Event> {
        if let Some(maybe_id) = self.to_focus.take() {
            self.ui.focus_widget(maybe_id);
        }
        let events = self.update_layout(client, global_state, debug_info, dt);
        let camera::Dependents {
            view_mat, proj_mat, ..
        } = camera.dependents();
        self.ui.maintain(
            &mut global_state.window.renderer_mut(),
            Some(proj_mat * view_mat),
        );

        // Check if item images need to be reloaded
        self.item_imgs.reload_if_changed(&mut self.ui);

        events
    }

    pub fn render(&self, renderer: &mut Renderer, globals: &Consts<Globals>) {
        // Don't show anything if the UI is toggled off.
        if self.show.ui {
            self.ui.render(renderer, Some(globals));
        }
    }

    pub fn free_look(&mut self, free_look: bool) { self.show.free_look = free_look; }
}
