mod bag;
mod buttons;
mod chat;
mod esc_menu;
mod hotbar;
mod img_ids;
mod item_imgs;
mod map;
mod minimap;
mod overhead;
mod popup;
mod settings_window;
mod skillbar;
mod slots;
mod social;
mod spell;

use crate::{ecs::comp::HpFloaterList, hud::img_ids::ImgsRot, ui::img_ids::Rotations};
pub use hotbar::{SlotContents as HotbarSlotContents, State as HotbarState};

pub use settings_window::ScaleChange;
use std::time::Duration;

use bag::Bag;
use buttons::Buttons;
use chat::Chat;
use chrono::NaiveTime;
use esc_menu::EscMenu;
use img_ids::Imgs;
use item_imgs::ItemImgs;
use map::Map;
use minimap::MiniMap;
use popup::Popup;
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
    ui::{fonts::ConrodVoxygenFonts, slot, Graphic, Ingameable, ScaleMode, Ui},
    window::{Event as WinEvent, GameInput},
    GlobalState,
};
use client::Client;
use common::{assets::load_expect, comp, sync::Uid, terrain::TerrainChunk, vol::RectRasterableVol};
use conrod_core::{
    text::cursor::Index,
    widget::{self, Button, Image, Text},
    widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, Widget,
};
use specs::{Join, WorldExt};
use std::{
    collections::{HashMap, VecDeque},
    time::Instant,
};
use vek::*;

const XP_COLOR: Color = Color::Rgba(0.59, 0.41, 0.67, 1.0);
const TEXT_COLOR: Color = Color::Rgba(1.0, 1.0, 1.0, 1.0);
const TEXT_BG: Color = Color::Rgba(0.0, 0.0, 0.0, 1.0);
//const TEXT_COLOR_GREY: Color = Color::Rgba(1.0, 1.0, 1.0, 0.5);
const MENU_BG: Color = Color::Rgba(0.0, 0.0, 0.0, 0.4);
//const TEXT_COLOR_2: Color = Color::Rgba(0.0, 0.0, 0.0, 1.0);
const TEXT_COLOR_3: Color = Color::Rgba(1.0, 1.0, 1.0, 0.1);
const TEXT_BIND_CONFLICT_COLOR: Color = Color::Rgba(1.0, 0.0, 0.0, 1.0);
const BLACK: Color = Color::Rgba(0.0, 0.0, 0.0, 1.0);
//const BG_COLOR: Color = Color::Rgba(1.0, 1.0, 1.0, 0.8);
const HP_COLOR: Color = Color::Rgba(0.33, 0.63, 0.0, 1.0);
const LOW_HP_COLOR: Color = Color::Rgba(0.93, 0.59, 0.03, 1.0);
const CRITICAL_HP_COLOR: Color = Color::Rgba(0.79, 0.19, 0.17, 1.0);
const MANA_COLOR: Color = Color::Rgba(0.29, 0.62, 0.75, 0.9);
//const FOCUS_COLOR: Color = Color::Rgba(1.0, 0.56, 0.04, 1.0);
//const RAGE_COLOR: Color = Color::Rgba(0.5, 0.04, 0.13, 1.0);

// Chat Colors
/// Color for chat command errors (yellow !)
const ERROR_COLOR: Color = Color::Rgba(1.0, 1.0, 0.0, 1.0);
/// Color for chat command info (blue i)
const INFO_COLOR: Color = Color::Rgba(0.28, 0.83, 0.71, 1.0);
/// Online color
const ONLINE_COLOR: Color = Color::Rgba(0.3, 1.0, 0.3, 1.0);
/// Offline color
const OFFLINE_COLOR: Color = Color::Rgba(1.0, 0.3, 0.3, 1.0);
/// Color for a private message from another player
const TELL_COLOR: Color = Color::Rgba(0.98, 0.71, 1.0, 1.0);
/// Color for local chat
const SAY_COLOR: Color = Color::Rgba(1.0, 0.8, 0.8, 1.0);
/// Color for group chat
const GROUP_COLOR: Color = Color::Rgba(0.47, 0.84, 1.0, 1.0);
/// Color for factional chat
const FACTION_COLOR: Color = Color::Rgba(0.24, 1.0, 0.48, 1.0);
/// Color for regional chat
const REGION_COLOR: Color = Color::Rgba(0.8, 1.0, 0.8, 1.0);
/// Color for death messages
const KILL_COLOR: Color = Color::Rgba(1.0, 0.17, 0.17, 1.0);
/// Color for global messages
const WORLD_COLOR: Color = Color::Rgba(0.95, 1.0, 0.95, 1.0);
/// Color for collected loot messages
const LOOT_COLOR: Color = Color::Rgba(0.69, 0.57, 1.0, 1.0);

// UI Color-Theme
const UI_MAIN: Color = Color::Rgba(0.61, 0.70, 0.70, 1.0); // Greenish Blue
//const UI_MAIN: Color = Color::Rgba(0.1, 0.1, 0.1, 0.97); // Dark
const UI_HIGHLIGHT_0: Color = Color::Rgba(0.79, 1.09, 1.09, 1.0);
//const UI_DARK_0: Color = Color::Rgba(0.25, 0.37, 0.37, 1.0);

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

        overheads[],

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

        // Alpha Disclaimer
        alpha_text,

        // Debug
        debug_bg,
        fps_counter,
        ping,
        coordinates,
        velocity,
        orientation,
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
        popup,
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

        // Auto walk indicator
        auto_walk_txt,
        auto_walk_bg,

        // Example Quest
        quest_bg,
        q_headline_bg,
        q_headline,
        q_text_bg,
        q_text,
        accept_button,
    }
}

pub struct DebugInfo {
    pub tps: f64,
    pub ping_ms: f64,
    pub coordinates: Option<comp::Pos>,
    pub velocity: Option<comp::Vel>,
    pub ori: Option<comp::Ori>,
    pub num_chunks: u32,
    pub num_visible_chunks: u32,
    pub num_figures: u32,
    pub num_figures_visible: u32,
}

pub struct HudInfo {
    pub is_aiming: bool,
    pub is_first_person: bool,
}

pub enum Event {
    SendMessage(String),
    AdjustMousePan(u32),
    AdjustMouseZoom(u32),
    ToggleZoomInvert(bool),
    ToggleMouseYInvert(bool),
    ToggleSmoothPan(bool),
    AdjustViewDistance(u32),
    AdjustSpriteRenderDistance(u32),
    AdjustFigureLoDRenderDistance(u32),
    AdjustMusicVolume(f32),
    AdjustSfxVolume(f32),
    ChangeAudioDevice(String),
    ChangeMaxFPS(u32),
    ChangeFOV(u16),
    ChangeGamma(f32),
    MapZoom(f64),
    AdjustWindowSize([u16; 2]),
    ToggleFullscreen,
    ChangeAaMode(AaMode),
    ChangeCloudMode(CloudMode),
    ChangeFluidMode(FluidMode),
    CrosshairTransp(f32),
    ChatTransp(f32),
    ChatCharName(bool),
    CrosshairType(CrosshairType),
    ToggleXpBar(XpBar),
    Intro(Intro),
    ToggleBarNumbers(BarNumbers),
    ToggleShortcutNumbers(ShortcutNumbers),
    Sct(bool),
    SctPlayerBatch(bool),
    SctDamageBatch(bool),
    SpeechBubbleDarkMode(bool),
    SpeechBubbleIcon(bool),
    ToggleDebug(bool),
    UiScale(ScaleChange),
    CharacterSelection,
    UseSlot(comp::slot::Slot),
    SwapSlots(comp::slot::Slot, comp::slot::Slot),
    DropSlot(comp::slot::Slot),
    ChangeHotbarState(HotbarState),
    Ability3(bool),
    Logout,
    Quit,
    ChangeLanguage(LanguageMetadata),
    ChangeBinding(GameInput),
    ResetBindings,
    ChangeFreeLookBehavior(PressBehavior),
    ChangeAutoWalkBehavior(PressBehavior),
    ChangeStopAutoWalkOnInput(bool),
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
    esc_menu: bool,
    open_windows: Windows,
    map: bool,
    mini_map: bool,
    ingame: bool,
    settings_tab: SettingsTab,
    social_tab: SocialTab,
    want_grab: bool,
    stats: bool,
    free_look: bool,
    auto_walk: bool,
}
impl Show {
    fn bag(&mut self, open: bool) {
        self.bag = open;
        self.map = false;
        self.want_grab = !open;
    }

    fn toggle_bag(&mut self) { self.bag(!self.bag); }

    fn map(&mut self, open: bool) {
        self.map = open;
        self.bag = false;
        self.want_grab = !open;
    }

    fn social(&mut self, open: bool) {
        self.social = open;
        self.spell = false;
        self.want_grab = !open;
    }

    fn spell(&mut self, open: bool) {
        self.social = false;
        self.spell = open;
        self.want_grab = !open;
    }

    fn toggle_map(&mut self) { self.map(!self.map) }

    fn toggle_mini_map(&mut self) { self.mini_map = !self.mini_map; }

    fn settings(&mut self, open: bool) {
        self.open_windows = if open {
            Windows::Settings
        } else {
            Windows::None
        };
        self.bag = false;
        self.social = false;
        self.spell = false;
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
            || self.spell
            || self.help
            || self.intro
            || match self.open_windows {
                Windows::None => false,
                _ => true,
            }
        {
            self.bag = false;
            self.esc_menu = false;
            self.help = false;
            self.intro = false;
            self.map = false;
            self.social = false;
            self.spell = false;
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
    }

    fn open_social_tab(&mut self, social_tab: SocialTab) {
        self.social_tab = social_tab;
        self.spell = false;
    }

    fn toggle_spell(&mut self) {
        self.spell = !self.spell;
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
    new_messages: VecDeque<comp::ChatMsg>,
    new_notifications: VecDeque<common::msg::Notification>,
    speech_bubbles: HashMap<Uid, comp::SpeechBubble>,
    show: Show,
    //never_show: bool,
    //intro: bool,
    //intro_2: bool,
    to_focus: Option<Option<widget::Id>>,
    force_ungrab: bool,
    force_chat_input: Option<String>,
    force_chat_cursor: Option<Index>,
    tab_complete: Option<String>,
    pulse: f32,
    velocity: f32,
    voxygen_i18n: std::sync::Arc<VoxygenLocalization>,
    slot_manager: slots::SlotManager,
    hotbar: hotbar::State,
    events: Vec<Event>,
    crosshair_opacity: f32,
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
        let item_imgs = ItemImgs::new(&mut ui, imgs.not_found);
        // Load language.
        let voxygen_i18n = load_expect::<VoxygenLocalization>(&i18n_asset_key(
            &global_state.settings.language.selected_language,
        ));
        // Load fonts.
        let fonts = ConrodVoxygenFonts::load(&voxygen_i18n.fonts, &mut ui)
            .expect("Impossible to load fonts!");
        // Get the server name.
        let server = &client.server_info.name;
        // Get the id, unwrap is safe because this CANNOT be None at this
        // point.
        let character_id = client.active_character_id.unwrap();
        // Create a new HotbarState from the persisted slots.
        let hotbar_state =
            HotbarState::new(global_state.profile.get_hotbar_slots(server, character_id));

        let slot_manager = slots::SlotManager::new(ui.id_generator(), Vec2::broadcast(40.0));

        Self {
            ui,
            imgs,
            world_map,
            rot_imgs,
            item_imgs,
            fonts,
            ids,
            new_messages: VecDeque::new(),
            new_notifications: VecDeque::new(),
            speech_bubbles: HashMap::new(),
            //intro: false,
            //intro_2: false,
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
                spell: false,
                mini_map: true,
                settings_tab: SettingsTab::Interface,
                social_tab: SocialTab::Online,
                want_grab: true,
                ingame: true,
                stats: false,
                free_look: false,
                auto_walk: false,
            },
            to_focus: None,
            //never_show: false,
            force_ungrab: false,
            force_chat_input: None,
            force_chat_cursor: None,
            tab_complete: None,
            pulse: 0.0,
            velocity: 0.0,
            voxygen_i18n,
            slot_manager,
            hotbar: hotbar_state,
            events: Vec::new(),
            crosshair_opacity: 0.0,
        }
    }

    pub fn update_language(&mut self, voxygen_i18n: std::sync::Arc<VoxygenLocalization>) {
        self.voxygen_i18n = voxygen_i18n;
        self.fonts = ConrodVoxygenFonts::load(&self.voxygen_i18n.fonts, &mut self.ui)
            .expect("Impossible to load fonts!");
    }

    #[allow(clippy::assign_op_pattern)] // TODO: Pending review in #587
    #[allow(clippy::single_match)] // TODO: Pending review in #587
    fn update_layout(
        &mut self,
        client: &Client,
        global_state: &GlobalState,
        debug_info: DebugInfo,
        dt: Duration,
        info: HudInfo,
        camera: &Camera,
    ) -> Vec<Event> {
        let mut events = std::mem::replace(&mut self.events, Vec::new());
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
            let uids = ecs.read_storage::<common::sync::Uid>();
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
                // Alpha Disclaimer
                Text::new(&format!("Veloren Pre-Alpha {}", env!("CARGO_PKG_VERSION")))
                    .font_id(self.fonts.cyri.conrod_id)
                    .font_size(self.fonts.cyri.scale(10))
                    .color(TEXT_COLOR)
                    .mid_top_with_margin_on(ui_widgets.window, 2.0)
                    .set(self.ids.alpha_text, ui_widgets);

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
                let show_crosshair = (info.is_aiming || info.is_first_person) && !stats.is_dead;
                self.crosshair_opacity = Lerp::lerp(
                    self.crosshair_opacity,
                    if show_crosshair { 1.0 } else { 0.0 },
                    5.0 * dt.as_secs_f32(),
                );

                if !self.show.help {
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
                        self.crosshair_opacity * global_state.settings.gameplay.crosshair_transp,
                    )))
                    .set(self.ids.crosshair_outer, ui_widgets);
                    Image::new(self.imgs.crosshair_inner)
                        .w_h(21.0 * 2.0, 21.0 * 2.0)
                        .middle_of(self.ids.crosshair_outer)
                        .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.6)))
                        .set(self.ids.crosshair_inner, ui_widgets);
                }
            }

            // Max amount the sct font size increases when "flashing"
            const FLASH_MAX: f32 = 25.0;

            // Get player position.
            let player_pos = client
                .state()
                .ecs()
                .read_storage::<comp::Pos>()
                .get(client.entity())
                .map_or(Vec3::zero(), |pos| pos.0);

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

            // Pop speech bubbles
            let now = Instant::now();
            self.speech_bubbles
                .retain(|_uid, bubble| bubble.timeout > now);

            // Push speech bubbles
            for msg in self.new_messages.iter() {
                if let Some((bubble, uid)) = msg.to_bubble() {
                    self.speech_bubbles.insert(uid, bubble);
                }
            }

            let mut overhead_walker = self.ids.overheads.walk();
            let mut sct_walker = self.ids.scts.walk();
            let mut sct_bg_walker = self.ids.sct_bgs.walk();

            // Render overhead name tags and health bars
            for (pos, name, stats, energy, height_offset, hpfl, uid) in (
                &entities,
                &pos,
                interpolated.maybe(),
                &stats,
                energy.maybe(),
                players.maybe(),
                scales.maybe(),
                &bodies,
                &hp_floater_lists,
                &uids,
            )
                .join()
                .filter(|(entity, _, _, stats, _, _, _, _, _, _)| *entity != me && !stats.is_dead)
                // Don't show outside a certain range
                .filter(|(_, pos, _, _, _, _, _, _, hpfl, _)| {
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
                .map(|(_, pos, interpolated, stats, energy, player, scale, body, hpfl, uid)| {
                    // TODO: This is temporary
                    // If the player used the default character name display their name instead
                    let name = if stats.name == "Character Name" {
                        player.map_or(&stats.name, |p| &p.alias)
                    } else {
                        &stats.name
                    };
                    (
                        interpolated.map_or(pos.0, |i| i.pos),
                        name,
                        stats,
                        energy,
                        // TODO: when body.height() is more accurate remove the 2.0
                        body.height() * 2.0 * scale.map_or(1.0, |s| s.0),
                        hpfl,
                        uid,
                    )
                })
            {
                let bubble = self.speech_bubbles.get(uid);

                let overhead_id = overhead_walker.next(
                    &mut self.ids.overheads,
                    &mut ui_widgets.widget_id_generator(),
                );
                let ingame_pos = pos + Vec3::unit_z() * height_offset;

                // Speech bubble, name, level, and hp bars
                overhead::Overhead::new(
                    &name,
                    bubble,
                    stats,
                    energy,
                    own_level,
                    &global_state.settings.gameplay,
                    self.pulse,
                    &self.voxygen_i18n,
                    &self.imgs,
                    &self.fonts,
                )
                .x_y(0.0, 100.0)
                .position_ingame(ingame_pos)
                .set(overhead_id, ui_widgets);

                // Enemy SCT
                if global_state.settings.gameplay.sct && !hpfl.floaters.is_empty() {
                    let floaters = &hpfl.floaters;

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
                        let sct_id = sct_walker
                            .next(&mut self.ids.scts, &mut ui_widgets.widget_id_generator());
                        let sct_bg_id = sct_bg_walker
                            .next(&mut self.ids.sct_bgs, &mut ui_widgets.widget_id_generator());
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
                            let sct_id = sct_walker
                                .next(&mut self.ids.scts, &mut ui_widgets.widget_id_generator());
                            let sct_bg_id = sct_bg_walker
                                .next(&mut self.ids.sct_bgs, &mut ui_widgets.widget_id_generator());
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
                                    Color::Rgba(0.0, 0.0, 0.0, 1.0)
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
                                    Color::Rgba(0.1, 1.0, 0.1, 1.0)
                                })
                                .position_ingame(ingame_pos)
                                .set(sct_id, ui_widgets);
                        }
                    }
                }
            }
        }

        // Temporary Example Quest
        if self.show.intro && !self.show.esc_menu {
            match global_state.settings.gameplay.intro_show {
                Intro::Show => {
                    if self.pulse > 20.0 {
                        self.show.want_grab = false;
                        let quest_headline = &self.voxygen_i18n.get("hud.temp_quest_headline");
                        let quest_text = &self.voxygen_i18n.get("hud.temp_quest_text");
                        Image::new(self.imgs.quest_bg)
                            .w_h(404.0, 858.0)
                            .middle_of(ui_widgets.window)
                            .set(self.ids.quest_bg, ui_widgets);

                        Text::new(quest_headline)
                            .mid_top_with_margin_on(self.ids.quest_bg, 310.0)
                            .font_size(self.fonts.cyri.scale(30))
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(TEXT_BG)
                            .set(self.ids.q_headline_bg, ui_widgets);
                        Text::new(quest_headline)
                            .bottom_left_with_margins_on(self.ids.q_headline_bg, 1.0, 1.0)
                            .font_size(self.fonts.cyri.scale(30))
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(TEXT_COLOR)
                            .set(self.ids.q_headline, ui_widgets);

                        Text::new(quest_text)
                            .down_from(self.ids.q_headline_bg, 40.0)
                            .font_size(self.fonts.cyri.scale(17))
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(TEXT_BG)
                            .set(self.ids.q_text_bg, ui_widgets);
                        Text::new(quest_text)
                            .bottom_left_with_margins_on(self.ids.q_text_bg, 1.0, 1.0)
                            .font_size(self.fonts.cyri.scale(17))
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(TEXT_COLOR)
                            .set(self.ids.q_text, ui_widgets);

                        if Button::image(self.imgs.button)
                            .w_h(212.0, 52.0)
                            .hover_image(self.imgs.button_hover)
                            .press_image(self.imgs.button_press)
                            .mid_bottom_with_margin_on(self.ids.q_text_bg, -120.0)
                            .label(&self.voxygen_i18n.get("common.accept"))
                            .label_font_id(self.fonts.cyri.conrod_id)
                            .label_font_size(self.fonts.cyri.scale(22))
                            .label_color(TEXT_COLOR)
                            .label_y(conrod_core::position::Relative::Scalar(2.0))
                            .set(self.ids.accept_button, ui_widgets)
                            .was_clicked()
                        {
                            self.show.intro = !self.show.intro;
                            events.push(Event::Intro(Intro::Never));
                            self.show.want_grab = true;
                        }
                    }
                },
                Intro::Never => {
                    self.show.intro = false;
                },
            }
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
            // Player's orientation vector
            let orientation_text = match debug_info.ori {
                Some(ori) => format!(
                    "Orientation: ({:.1}, {:.1}, {:.1})",
                    ori.0.x, ori.0.y, ori.0.z,
                ),
                None => "Player has no Ori component".to_owned(),
            };
            Text::new(&orientation_text)
                .color(TEXT_COLOR)
                .down_from(self.ids.velocity, 5.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .set(self.ids.orientation, ui_widgets);
            // Loaded distance
            Text::new(&format!(
                "View distance: {:.2} blocks ({:.2} chunks)",
                client.loaded_distance(),
                client.loaded_distance() / TerrainChunk::RECT_SIZE.x as f32,
            ))
            .color(TEXT_COLOR)
            .down_from(self.ids.orientation, 5.0)
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
            if let Some(help_key) = global_state.settings.controls.get_binding(GameInput::Help) {
                Text::new(
                    &self
                        .voxygen_i18n
                        .get("hud.press_key_to_toggle_keybindings_fmt")
                        .replace("{key}", help_key.to_string().as_str()),
                )
                .color(TEXT_COLOR)
                .down_from(self.ids.num_figures, 5.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .set(self.ids.help_info, ui_widgets);
            }
            // Info about Debug Shortcut
            if let Some(toggle_debug_key) = global_state
                .settings
                .controls
                .get_binding(GameInput::ToggleDebug)
            {
                Text::new(
                    &self
                        .voxygen_i18n
                        .get("hud.press_key_to_toggle_debug_info_fmt")
                        .replace("{key}", toggle_debug_key.to_string().as_str()),
                )
                .color(TEXT_COLOR)
                .down_from(self.ids.help_info, 5.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .set(self.ids.debug_info, ui_widgets);
            }
        } else {
            // Help Window
            if let Some(help_key) = global_state.settings.controls.get_binding(GameInput::Help) {
                Text::new(
                    &self
                        .voxygen_i18n
                        .get("hud.press_key_to_show_keybindings_fmt")
                        .replace("{key}", help_key.to_string().as_str()),
                )
                .color(TEXT_COLOR)
                .top_left_with_margins_on(ui_widgets.window, 5.0, 5.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(16))
                .set(self.ids.help_info, ui_widgets);
            }
            // Info about Debug Shortcut
            if let Some(toggle_debug_key) = global_state
                .settings
                .controls
                .get_binding(GameInput::ToggleDebug)
            {
                Text::new(
                    &self
                        .voxygen_i18n
                        .get("hud.press_key_to_show_debug_info_fmt")
                        .replace("{key}", toggle_debug_key.to_string().as_str()),
                )
                .color(TEXT_COLOR)
                .down_from(self.ids.help_info, 5.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(12))
                .set(self.ids.debug_info, ui_widgets);
            }
        }

        // Help Text
        if self.show.help && !self.show.map && !self.show.esc_menu {
            Image::new(self.imgs.help)
                .middle_of(ui_widgets.window)
                .w_h(1260.0, 519.0)
                .set(self.ids.help, ui_widgets);
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
        let ecs = client.state().ecs();
        let stats = ecs.read_storage::<comp::Stats>();
        if let Some(player_stats) = stats.get(client.entity()) {
            match Buttons::new(
                client,
                self.show.bag,
                &self.imgs,
                &self.fonts,
                global_state,
                &self.rot_imgs,
                tooltip_manager,
                &self.voxygen_i18n,
                &player_stats,
            )
            .set(self.ids.buttons, ui_widgets)
            {
                Some(buttons::Event::ToggleBag) => self.show.toggle_bag(),
                Some(buttons::Event::ToggleSettings) => self.show.toggle_settings(),
                Some(buttons::Event::ToggleSocial) => self.show.toggle_social(),
                Some(buttons::Event::ToggleSpell) => self.show.toggle_spell(),
                Some(buttons::Event::ToggleMap) => self.show.toggle_map(),
                None => {},
            }
        }

        // Popup (waypoint saved and similar notifications)
        Popup::new(
            &self.voxygen_i18n,
            client,
            &self.new_notifications,
            &self.fonts,
            &self.show,
        )
        .set(self.ids.popup, ui_widgets);

        // MiniMap
        match MiniMap::new(
            &self.show,
            client,
            &self.imgs,
            &self.rot_imgs,
            &self.world_map,
            &self.fonts,
            camera.get_orientation(),
        )
        .set(self.ids.minimap, ui_widgets)
        {
            Some(minimap::Event::Toggle) => self.show.toggle_mini_map(),
            None => {},
        }

        // Bag contents
        if self.show.bag {
            if let Some(player_stats) = stats.get(client.entity()) {
                match Bag::new(
                    client,
                    &self.imgs,
                    &self.item_imgs,
                    &self.fonts,
                    &self.rot_imgs,
                    tooltip_manager,
                    &mut self.slot_manager,
                    self.pulse,
                    &self.voxygen_i18n,
                    &player_stats,
                    &self.show,
                )
                .set(self.ids.bag, ui_widgets)
                {
                    Some(bag::Event::Stats) => self.show.stats = !self.show.stats,
                    Some(bag::Event::Close) => {
                        self.show.bag(false);
                        self.force_ungrab = true;
                    },
                    None => {},
                }
            }
        }

        // Skillbar
        // Get player stats
        let ecs = client.state().ecs();
        let entity = client.entity();
        let stats = ecs.read_storage::<comp::Stats>();
        let loadouts = ecs.read_storage::<comp::Loadout>();
        let energies = ecs.read_storage::<comp::Energy>();
        let character_states = ecs.read_storage::<comp::CharacterState>();
        let controllers = ecs.read_storage::<comp::Controller>();
        let inventories = ecs.read_storage::<comp::Inventory>();
        if let (
            Some(stats),
            Some(loadout),
            Some(energy),
            Some(character_state),
            Some(controller),
            Some(inventory),
        ) = (
            stats.get(entity),
            loadouts.get(entity),
            energies.get(entity),
            character_states.get(entity),
            controllers.get(entity).map(|c| &c.inputs),
            inventories.get(entity),
        ) {
            Skillbar::new(
                global_state,
                &self.imgs,
                &self.item_imgs,
                &self.fonts,
                &self.rot_imgs,
                &stats,
                &loadout,
                &energy,
                &character_state,
                self.pulse,
                &controller,
                &inventory,
                &self.hotbar,
                tooltip_manager,
                &mut self.slot_manager,
                &self.voxygen_i18n,
                &self.show,
            )
            .set(self.ids.skillbar, ui_widgets);
        }

        // Don't put NPC messages in chat box.
        self.new_messages
            .retain(|m| !matches!(m.chat_type, comp::ChatType::Npc(_, _)));

        // Chat box
        match Chat::new(
            &mut self.new_messages,
            &client,
            global_state,
            &self.imgs,
            &self.fonts,
        )
        .and_then(self.force_chat_input.take(), |c, input| c.input(input))
        .and_then(self.tab_complete.take(), |c, input| {
            c.prepare_tab_completion(input)
        })
        .and_then(self.force_chat_cursor.take(), |c, pos| c.cursor_pos(pos))
        .set(self.ids.chat, ui_widgets)
        {
            Some(chat::Event::TabCompletionStart(input)) => {
                self.tab_complete = Some(input);
            },
            Some(chat::Event::SendMessage(message)) => {
                events.push(Event::SendMessage(message));
            },
            Some(chat::Event::Focus(focus_id)) => {
                self.to_focus = Some(Some(focus_id));
            },
            None => {},
        }

        self.new_messages = VecDeque::new();
        self.new_notifications = VecDeque::new();

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
                    settings_window::Event::SpeechBubbleDarkMode(sbdm) => {
                        events.push(Event::SpeechBubbleDarkMode(sbdm));
                    },
                    settings_window::Event::SpeechBubbleIcon(sbi) => {
                        events.push(Event::SpeechBubbleIcon(sbi));
                    },
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
                    settings_window::Event::ChatCharName(chat_char_name) => {
                        events.push(Event::ChatCharName(chat_char_name));
                    },
                    settings_window::Event::ToggleZoomInvert(zoom_inverted) => {
                        events.push(Event::ToggleZoomInvert(zoom_inverted));
                    },
                    settings_window::Event::ToggleMouseYInvert(mouse_y_inverted) => {
                        events.push(Event::ToggleMouseYInvert(mouse_y_inverted));
                    },
                    settings_window::Event::ToggleSmoothPan(smooth_pan_enabled) => {
                        events.push(Event::ToggleSmoothPan(smooth_pan_enabled));
                    },
                    settings_window::Event::AdjustViewDistance(view_distance) => {
                        events.push(Event::AdjustViewDistance(view_distance));
                    },
                    settings_window::Event::AdjustSpriteRenderDistance(view_distance) => {
                        events.push(Event::AdjustSpriteRenderDistance(view_distance));
                    },
                    settings_window::Event::AdjustFigureLoDRenderDistance(view_distance) => {
                        events.push(Event::AdjustFigureLoDRenderDistance(view_distance));
                    },
                    settings_window::Event::CrosshairTransp(crosshair_transp) => {
                        events.push(Event::CrosshairTransp(crosshair_transp));
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
                    settings_window::Event::ChangeBinding(game_input) => {
                        events.push(Event::ChangeBinding(game_input));
                    },
                    settings_window::Event::ResetBindings => {
                        events.push(Event::ResetBindings);
                    },
                    settings_window::Event::ChangeFreeLookBehavior(behavior) => {
                        events.push(Event::ChangeFreeLookBehavior(behavior));
                    },
                    settings_window::Event::ChangeAutoWalkBehavior(behavior) => {
                        events.push(Event::ChangeAutoWalkBehavior(behavior));
                    },
                    settings_window::Event::ChangeStopAutoWalkOnInput(state) => {
                        events.push(Event::ChangeStopAutoWalkOnInput(state));
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
        // Map
        if self.show.map {
            for event in Map::new(
                &self.show,
                client,
                &self.imgs,
                &self.rot_imgs,
                &self.world_map,
                &self.fonts,
                self.pulse,
                &self.voxygen_i18n,
                &global_state,
            )
            .set(self.ids.map, ui_widgets)
            {
                match event {
                    map::Event::Close => {
                        self.show.map(false);
                        self.force_ungrab = true;
                    },
                    map::Event::MapZoom(map_zoom) => {
                        events.push(Event::MapZoom(map_zoom));
                    },
                }
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

        // Auto walk indicator
        if self.show.auto_walk {
            Text::new(&self.voxygen_i18n.get("hud.auto_walk_indicator"))
                .color(TEXT_BG)
                .mid_top_with_margin_on(
                    ui_widgets.window,
                    if self.show.free_look { 128.0 } else { 100.0 },
                )
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(20))
                .set(self.ids.auto_walk_bg, ui_widgets);
            Text::new(&self.voxygen_i18n.get("hud.auto_walk_indicator"))
                .color(KILL_COLOR)
                .top_left_with_margins_on(self.ids.auto_walk_bg, -1.0, -1.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(20))
                .set(self.ids.auto_walk_txt, ui_widgets);
        }

        // Maintain slot manager
        for event in self.slot_manager.maintain(ui_widgets) {
            use comp::slot::Slot;
            use slots::SlotKind::*;
            let to_slot = |slot_kind| match slot_kind {
                Inventory(i) => Some(Slot::Inventory(i.0)),
                Equip(e) => Some(Slot::Equip(e)),
                Hotbar(_) => None,
            };
            match event {
                slot::Event::Dragged(a, b) => {
                    // Swap between slots
                    if let (Some(a), Some(b)) = (to_slot(a), to_slot(b)) {
                        events.push(Event::SwapSlots(a, b));
                    } else if let (Inventory(i), Hotbar(h)) = (a, b) {
                        self.hotbar.add_inventory_link(h, i.0);
                        events.push(Event::ChangeHotbarState(self.hotbar.to_owned()));
                    } else if let (Hotbar(a), Hotbar(b)) = (a, b) {
                        self.hotbar.swap(a, b);
                        events.push(Event::ChangeHotbarState(self.hotbar.to_owned()));
                    }
                },
                slot::Event::Dropped(from) => {
                    // Drop item
                    if let Some(from) = to_slot(from) {
                        events.push(Event::DropSlot(from));
                    } else if let Hotbar(h) = from {
                        self.hotbar.clear_slot(h);
                        events.push(Event::ChangeHotbarState(self.hotbar.to_owned()));
                    }
                },
                slot::Event::Used(from) => {
                    // Item used (selected and then clicked again)
                    if let Some(from) = to_slot(from) {
                        events.push(Event::UseSlot(from));
                    } else if let Hotbar(h) = from {
                        self.hotbar.get(h).map(|s| {
                            match s {
                                hotbar::SlotContents::Inventory(i) => {
                                    events.push(Event::UseSlot(comp::slot::Slot::Inventory(i)));
                                },
                                hotbar::SlotContents::Ability3 => {}, /* Event::Ability3(true),
                                                                       * sticks */
                            }
                        });
                    }
                },
            }
        }
        self.hotbar.maintain_ability3(client);

        events
    }

    pub fn new_message(&mut self, msg: comp::ChatMsg) { self.new_messages.push_back(msg); }

    pub fn new_notification(&mut self, msg: common::msg::Notification) {
        self.new_notifications.push_back(msg);
    }

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
        // Helper
        fn handle_slot(
            slot: hotbar::Slot,
            state: bool,
            events: &mut Vec<Event>,
            slot_manager: &mut slots::SlotManager,
            hotbar: &mut hotbar::State,
        ) {
            if let Some(slots::SlotKind::Inventory(i)) = slot_manager.selected() {
                hotbar.add_inventory_link(slot, i.0);
                slot_manager.idle();
            } else {
                let just_pressed = hotbar.process_input(slot, state);
                hotbar.get(slot).map(|s| match s {
                    hotbar::SlotContents::Inventory(i) => {
                        if just_pressed {
                            events.push(Event::UseSlot(comp::slot::Slot::Inventory(i)));
                        }
                    },
                    hotbar::SlotContents::Ability3 => events.push(Event::Ability3(state)),
                });
            }
        }

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
            WinEvent::InputUpdate(key, state) if !self.typing() => match key {
                GameInput::Command if state => {
                    self.force_chat_input = Some("/".to_owned());
                    self.force_chat_cursor = Some(Index { line: 0, char: 1 });
                    self.ui.focus_widget(Some(self.ids.chat));
                    true
                },
                GameInput::Map if state => {
                    self.show.toggle_map();
                    true
                },
                GameInput::Bag if state => {
                    self.show.toggle_bag();
                    true
                },
                GameInput::Social if state => {
                    self.show.toggle_social();
                    true
                },
                GameInput::Spellbook if state => {
                    self.show.toggle_spell();
                    true
                },
                GameInput::Settings if state => {
                    self.show.toggle_settings();
                    true
                },
                GameInput::Help if state => {
                    self.show.toggle_help();
                    true
                },
                GameInput::ToggleDebug if state => {
                    global_state.settings.gameplay.toggle_debug =
                        !global_state.settings.gameplay.toggle_debug;
                    true
                },
                GameInput::ToggleIngameUi if state => {
                    self.show.ingame = !self.show.ingame;
                    true
                },
                // Skillbar
                GameInput::Slot1 => {
                    handle_slot(
                        hotbar::Slot::One,
                        state,
                        &mut self.events,
                        &mut self.slot_manager,
                        &mut self.hotbar,
                    );
                    true
                },
                GameInput::Slot2 => {
                    handle_slot(
                        hotbar::Slot::Two,
                        state,
                        &mut self.events,
                        &mut self.slot_manager,
                        &mut self.hotbar,
                    );
                    true
                },
                GameInput::Slot3 => {
                    handle_slot(
                        hotbar::Slot::Three,
                        state,
                        &mut self.events,
                        &mut self.slot_manager,
                        &mut self.hotbar,
                    );
                    true
                },
                GameInput::Slot4 => {
                    handle_slot(
                        hotbar::Slot::Four,
                        state,
                        &mut self.events,
                        &mut self.slot_manager,
                        &mut self.hotbar,
                    );
                    true
                },
                GameInput::Slot5 => {
                    handle_slot(
                        hotbar::Slot::Five,
                        state,
                        &mut self.events,
                        &mut self.slot_manager,
                        &mut self.hotbar,
                    );
                    true
                },
                GameInput::Slot6 => {
                    handle_slot(
                        hotbar::Slot::Six,
                        state,
                        &mut self.events,
                        &mut self.slot_manager,
                        &mut self.hotbar,
                    );
                    true
                },
                GameInput::Slot7 => {
                    handle_slot(
                        hotbar::Slot::Seven,
                        state,
                        &mut self.events,
                        &mut self.slot_manager,
                        &mut self.hotbar,
                    );
                    true
                },
                GameInput::Slot8 => {
                    handle_slot(
                        hotbar::Slot::Eight,
                        state,
                        &mut self.events,
                        &mut self.slot_manager,
                        &mut self.hotbar,
                    );
                    true
                },
                GameInput::Slot9 => {
                    handle_slot(
                        hotbar::Slot::Nine,
                        state,
                        &mut self.events,
                        &mut self.slot_manager,
                        &mut self.hotbar,
                    );
                    true
                },
                GameInput::Slot10 => {
                    handle_slot(
                        hotbar::Slot::Ten,
                        state,
                        &mut self.events,
                        &mut self.slot_manager,
                        &mut self.hotbar,
                    );
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
            WinEvent::Moved(_) => {
                // Prevent the cursor from being grabbed while the window is being moved as this
                // causes the window to move erratically
                self.show.want_grab = false;
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

    #[allow(clippy::blocks_in_if_conditions)] // TODO: Pending review in #587
    pub fn maintain(
        &mut self,
        client: &Client,
        global_state: &mut GlobalState,
        debug_info: DebugInfo,
        camera: &Camera,
        dt: Duration,
        info: HudInfo,
    ) -> Vec<Event> {
        // conrod eats tabs. Un-eat a tabstop so tab completion can work
        if self.ui.ui.global_input().events().any(|event| {
            use conrod_core::{event, input};
            match event {
                //event::Event::Raw(event::Input::Press(input::Button::Keyboard(input::Key::Tab)))
                // => true,
                event::Event::Ui(event::Ui::Press(
                    _,
                    event::Press {
                        button: event::Button::Keyboard(input::Key::Tab),
                        ..
                    },
                )) => true,
                _ => false,
            }
        }) {
            self.ui
                .ui
                .handle_event(conrod_core::event::Input::Text("\t".to_string()));
        }

        if let Some(maybe_id) = self.to_focus.take() {
            self.ui.focus_widget(maybe_id);
        }
        let events = self.update_layout(client, global_state, debug_info, dt, info, camera);
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

    pub fn auto_walk(&mut self, auto_walk: bool) { self.show.auto_walk = auto_walk; }
}
