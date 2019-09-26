mod bag;
mod buttons;
mod character_window;
mod chat;
mod esc_menu;
mod img_ids;
mod map;
mod minimap;
mod quest;
mod settings_window;
mod skillbar;
mod social;
mod spell;

use crate::hud::img_ids::ImgsRot;
pub use settings_window::ScaleChange;

use bag::Bag;
use buttons::Buttons;
use character_window::CharacterWindow;
use chat::Chat;
use chrono::NaiveTime;
use esc_menu::EscMenu;
use img_ids::Imgs;
use map::Map;
use minimap::MiniMap;
use quest::Quest;
use serde::{Deserialize, Serialize};
use settings_window::{SettingsTab, SettingsWindow};
use skillbar::Skillbar;
use social::{Social, SocialTab};
use spell::Spell;

use crate::{
    render::{AaMode, Consts, Globals, Renderer},
    scene::camera::Camera,
    settings::ControlSettings,
    ui::{Ingameable, ScaleMode, Ui},
    window::{Event as WinEvent, GameInput},
    GlobalState,
};
use client::{Client, Event as ClientEvent};
use common::{comp, terrain::TerrainChunk, vol::RectRasterableVol};
use conrod_core::{
    text::cursor::Index,
    widget::{self, Button, Image, Rectangle, Text},
    widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, Widget,
};
use specs::Join;
use std::collections::VecDeque;
use vek::*;

#[cfg(feature = "discord")]
use crate::{discord, discord::DiscordUpdate};

const XP_COLOR: Color = Color::Rgba(0.59, 0.41, 0.67, 1.0);
const TEXT_COLOR: Color = Color::Rgba(1.0, 1.0, 1.0, 1.0);
const TEXT_COLOR_2: Color = Color::Rgba(0.0, 0.0, 0.0, 1.0);
const TEXT_COLOR_3: Color = Color::Rgba(1.0, 1.0, 1.0, 0.1);
//const BG_COLOR: Color = Color::Rgba(1.0, 1.0, 1.0, 0.8);
const HP_COLOR: Color = Color::Rgba(0.33, 0.63, 0.0, 1.0);
const LOW_HP_COLOR: Color = Color::Rgba(0.93, 0.59, 0.03, 1.0);
const CRITICAL_HP_COLOR: Color = Color::Rgba(1.0, 0.0, 0.0, 1.0);
const MANA_COLOR: Color = Color::Rgba(0.47, 0.55, 1.0, 0.9);
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

widget_ids! {
    struct Ids {
        // Crosshair
        crosshair_inner,
        crosshair_outer,

        // Character Names
        name_tags[],
        // Health Bars
        health_bars[],
        health_bar_backs[],

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

        // Game Version
        version,

        // Help
        help,
        help_bg,

        // Window Frames
        window_frame_0,
        window_frame_1,
        window_frame_2,
        window_frame_3,
        window_frame_4,
        window_frame_5,

        // Contents
        button_help2,

        // External
        chat,
        map,
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
    }
}

font_ids! {
    pub struct Fonts {
        opensans: "voxygen.font.OpenSans-Regular",
        metamorph: "voxygen.font.Metamorphous-Regular",
    }
}

pub struct DebugInfo {
    pub tps: f64,
    pub ping_ms: f64,
    pub coordinates: Option<comp::Pos>,
    pub velocity: Option<comp::Vel>,
}

pub enum Event {
    SendMessage(String),
    AdjustMousePan(u32),
    AdjustMouseZoom(u32),
    AdjustViewDistance(u32),
    AdjustMusicVolume(f32),
    AdjustSfxVolume(f32),
    ChangeAudioDevice(String),
    ChangeMaxFPS(u32),
    ChangeFOV(u16),
    ChangeAaMode(AaMode),
    CrosshairTransp(f32),
    CrosshairType(CrosshairType),
    ToggleXpBar(XpBar),
    ToggleBarNumbers(BarNumbers),
    ToggleShortcutNumbers(ShortcutNumbers),
    UiScale(ScaleChange),
    CharacterSelection,
    UseInventorySlot(usize),
    SwapInventorySlots(usize, usize),
    DropInventorySlot(usize),
    Logout,
    Quit,
}

// TODO: Are these the possible layouts we want?
// TODO: Maybe replace this with bitflags.
// `map` is not here because it currently is displayed over the top of other open windows.
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

pub struct Show {
    ui: bool,
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
    inventory_test_button: bool,
    mini_map: bool,
    ingame: bool,
    settings_tab: SettingsTab,
    social_tab: SocialTab,

    want_grab: bool,
}
impl Show {
    fn bag(&mut self, open: bool) {
        self.bag = open;
        self.want_grab = !open;
    }
    fn toggle_bag(&mut self) {
        self.bag(!self.bag);
    }
    fn map(&mut self, open: bool) {
        self.map = open;
        self.bag = false;
        self.want_grab = !open;
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
    fn toggle_map(&mut self) {
        self.map(!self.map)
    }

    fn toggle_mini_map(&mut self) {
        self.mini_map = !self.mini_map;
    }

    fn toggle_char_window(&mut self) {
        self.character_window = !self.character_window
    }

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

    fn toggle_help(&mut self) {
        self.help = !self.help
    }

    fn toggle_ui(&mut self) {
        self.ui = !self.ui;
    }

    fn toggle_windows(&mut self) {
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
        } else {
            self.esc_menu = true;
            self.want_grab = false;
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
    imgs: Imgs,
    fonts: Fonts,
    rot_imgs: ImgsRot,
    new_messages: VecDeque<ClientEvent>,
    inventory_space: usize,
    show: Show,
    to_focus: Option<Option<widget::Id>>,
    force_ungrab: bool,
    force_chat_input: Option<String>,
    force_chat_cursor: Option<Index>,
}

impl Hud {
    pub fn new(global_state: &mut GlobalState) -> Self {
        let window = &mut global_state.window;
        let settings = &global_state.settings;

        let mut ui = Ui::new(window).unwrap();
        ui.set_scaling_mode(settings.gameplay.ui_scale);
        // Generate ids.
        let ids = Ids::new(ui.id_generator());
        // Load images.
        let imgs = Imgs::load(&mut ui).expect("Failed to load images!");
        // Load rotation images.
        let rot_imgs = ImgsRot::load(&mut ui).expect("Failed to load rot images!");
        // Load fonts.
        let fonts = Fonts::load(&mut ui).expect("Failed to load fonts!");

        Self {
            ui,
            imgs,
            rot_imgs,
            fonts,
            ids,
            new_messages: VecDeque::new(),
            inventory_space: 8,
            show: Show {
                help: false,
                debug: true,
                bag: false,
                esc_menu: false,
                open_windows: Windows::None,
                map: false,
                ui: true,
                social: false,
                quest: false,
                spell: false,
                character_window: false,
                inventory_test_button: false,
                mini_map: false,
                settings_tab: SettingsTab::Interface,
                social_tab: SocialTab::Online,
                want_grab: true,
                ingame: true,
            },
            to_focus: None,
            force_ungrab: false,
            force_chat_input: None,
            force_chat_cursor: None,
        }
    }

    fn update_layout(
        &mut self,
        client: &Client,
        global_state: &GlobalState,
        debug_info: DebugInfo,
    ) -> Vec<Event> {
        let mut events = Vec::new();
        let (ref mut ui_widgets, ref mut tooltip_manager) = self.ui.set_widgets();

        let version = format!("{}-{}", env!("CARGO_PKG_VERSION"), common::util::GIT_HASH);

        if self.show.ingame {
            // Crosshair
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

            // Nametags and healthbars
            let ecs = client.state().ecs();
            let pos = ecs.read_storage::<comp::Pos>();
            let stats = ecs.read_storage::<comp::Stats>();
            let players = ecs.read_storage::<comp::Player>();
            let scales = ecs.read_storage::<comp::Scale>();
            let entities = ecs.entities();
            let me = client.entity();
            let view_distance = client.view_distance().unwrap_or(1);
            // Get player position.
            let player_pos = client
                .state()
                .ecs()
                .read_storage::<comp::Pos>()
                .get(client.entity())
                .map_or(Vec3::zero(), |pos| pos.0);
            let mut name_id_walker = self.ids.name_tags.walk();
            let mut health_id_walker = self.ids.health_bars.walk();
            let mut health_back_id_walker = self.ids.health_bar_backs.walk();

            // Render Name Tags
            for (pos, name, scale) in (&entities, &pos, &stats, players.maybe(), scales.maybe())
                .join()
                .filter(|(entity, _, stats, _, _)| *entity != me && !stats.is_dead)
                // Don't process nametags outside the vd (visibility further limited by ui backend)
                .filter(|(_, pos, _, _, _)| {
                    Vec2::from(pos.0 - player_pos)
                        .map2(TerrainChunk::RECT_SIZE, |d: f32, sz| {
                            d.abs() as f32 / sz as f32
                        })
                        .magnitude()
                        < view_distance as f32
                })
                .map(|(_, pos, stats, player, scale)| {
                    // TODO: This is temporary
                    // If the player used the default character name display their name instead
                    let name = if stats.name == "Character Name" {
                        player.map_or(&stats.name, |p| &p.alias)
                    } else {
                        &stats.name
                    };
                    (pos.0, name, scale)
                })
            {
                let scale = scale.map(|s| s.0).unwrap_or(1.0);

                let id = name_id_walker.next(
                    &mut self.ids.name_tags,
                    &mut ui_widgets.widget_id_generator(),
                );
                Text::new(&name)
                    .font_size(20)
                    .color(Color::Rgba(0.61, 0.61, 0.89, 1.0))
                    .x_y(0.0, 0.0)
                    .position_ingame(pos + Vec3::new(0.0, 0.0, 1.5 * scale + 1.5))
                    .resolution(100.0)
                    .set(id, ui_widgets);
            }

            // Render Health Bars
            for (_entity, pos, stats, scale) in (&entities, &pos, &stats, scales.maybe())
                .join()
                .filter(|(entity, _, stats, _)| {
                    *entity != me
                        && !stats.is_dead
                        && stats.health.current() != stats.health.maximum()
                })
                // Don't process health bars outside the vd (visibility further limited by ui backend)
                .filter(|(_, pos, _, _)| {
                    Vec2::from(pos.0 - player_pos)
                        .map2(TerrainChunk::RECT_SIZE, |d: f32, sz| {
                            d.abs() as f32 / sz as f32
                        })
                        .magnitude()
                        < view_distance as f32
                })
            {
                let scale = scale.map(|s| s.0).unwrap_or(1.0);

                let back_id = health_back_id_walker.next(
                    &mut self.ids.health_bar_backs,
                    &mut ui_widgets.widget_id_generator(),
                );
                let bar_id = health_id_walker.next(
                    &mut self.ids.health_bars,
                    &mut ui_widgets.widget_id_generator(),
                );
                // Background
                Rectangle::fill_with([120.0, 8.0], Color::Rgba(0.3, 0.3, 0.3, 0.5))
                    .x_y(0.0, -25.0)
                    .position_ingame(pos.0 + Vec3::new(0.0, 0.0, 1.5 * scale + 1.5))
                    .resolution(100.0)
                    .set(back_id, ui_widgets);

                // % HP Filling
                Rectangle::fill_with(
                    [
                        120.0 * (stats.health.current() as f64 / stats.health.maximum() as f64),
                        8.0,
                    ],
                    HP_COLOR,
                )
                .x_y(0.0, -25.0)
                .position_ingame(pos.0 + Vec3::new(0.0, 0.0, 1.5 * scale + 1.5))
                .resolution(100.0)
                .set(bar_id, ui_widgets);
            }
        }

        // Display debug window.
        if self.show.debug {
            // Alpha Version
            Text::new(&version)
                .top_left_with_margins_on(ui_widgets.window, 5.0, 5.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(self.ids.version, ui_widgets);
            // Ticks per second
            Text::new(&format!("FPS: {:.0}", debug_info.tps))
                .color(TEXT_COLOR)
                .down_from(self.ids.version, 5.0)
                .font_id(self.fonts.opensans)
                .font_size(14)
                .set(self.ids.fps_counter, ui_widgets);
            // Ping
            Text::new(&format!("Ping: {:.0}ms", debug_info.ping_ms))
                .color(TEXT_COLOR)
                .down_from(self.ids.fps_counter, 5.0)
                .font_id(self.fonts.opensans)
                .font_size(14)
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
                .font_id(self.fonts.opensans)
                .font_size(14)
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
                .font_id(self.fonts.opensans)
                .font_size(14)
                .set(self.ids.velocity, ui_widgets);
            // Loaded distance
            Text::new(&format!(
                "View distance: {} chunks",
                client.loaded_distance().unwrap_or(0)
            ))
            .color(TEXT_COLOR)
            .down_from(self.ids.velocity, 5.0)
            .font_id(self.fonts.opensans)
            .font_size(14)
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
            .font_id(self.fonts.opensans)
            .font_size(14)
            .set(self.ids.time, ui_widgets);
        }

        // Add Bag-Space Button.
        if self.show.inventory_test_button {
            if Button::image(self.imgs.button)
                .w_h(100.0, 100.0)
                .middle_of(ui_widgets.window)
                .label("Add 1 Space")
                .label_font_size(20)
                .label_color(TEXT_COLOR)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .set(self.ids.bag_space_add, ui_widgets)
                .was_clicked()
            {
                if self.inventory_space < 100 {
                    self.inventory_space += 1;
                } else {
                }
            };
        }

        // Help Text
        if self.show.help {
            Image::new(self.imgs.window_frame_2)
                .top_left_with_margins_on(ui_widgets.window, 3.0, 3.0)
                .w_h(300.0, 190.0)
                .set(self.ids.help_bg, ui_widgets);
            Text::new(get_help_text(&global_state.settings.controls).as_str())
                .color(TEXT_COLOR)
                .top_left_with_margins_on(self.ids.help_bg, 20.0, 20.0)
                .font_id(self.fonts.opensans)
                .font_size(18)
                .set(self.ids.help, ui_widgets);
            // X-button
            if Button::image(self.imgs.close_button)
                .w_h(100.0 * 0.2, 100.0 * 0.2)
                .hover_image(self.imgs.close_button_hover)
                .press_image(self.imgs.close_button_press)
                .top_right_with_margins_on(self.ids.help_bg, 4.0, 4.0)
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
            None => {}
        }

        // MiniMap
        match MiniMap::new(&self.show, client, &self.imgs, &self.fonts)
            .set(self.ids.minimap, ui_widgets)
        {
            Some(minimap::Event::Toggle) => self.show.toggle_mini_map(),
            None => {}
        }

        // Bag contents
        if self.show.bag {
            match Bag::new(
                client,
                &self.imgs,
                &self.fonts,
                &self.rot_imgs,
                tooltip_manager,
            )
            .set(self.ids.bag, ui_widgets)
            {
                Some(bag::Event::HudEvent(event)) => events.push(event),
                Some(bag::Event::Close) => {
                    self.show.bag(false);
                    self.force_ungrab = true;
                }
                None => {}
            }
        }

        // Skillbar
        // Get player stats
        if let Some(stats) = client
            .state()
            .ecs()
            .read_storage::<comp::Stats>()
            .get(client.entity())
        {
            Skillbar::new(global_state, &self.imgs, &self.fonts, stats)
                .set(self.ids.skillbar, ui_widgets);
        }

        // Chat box
        match Chat::new(&mut self.new_messages, &self.imgs, &self.fonts)
            .and_then(self.force_chat_input.take(), |c, input| c.input(input))
            .and_then(self.force_chat_cursor.take(), |c, pos| c.cursor_pos(pos))
            .set(self.ids.chat, ui_widgets)
        {
            Some(chat::Event::SendMessage(message)) => {
                events.push(Event::SendMessage(message));
            }
            Some(chat::Event::Focus(focus_id)) => {
                self.to_focus = Some(Some(focus_id));
            }
            None => {}
        }

        self.new_messages = VecDeque::new();

        // Windows

        // Char Window will always appear at the left side. Other Windows default to the
        // left side, but when the Char Window is opened they will appear to the right of it.

        // Settings
        if let Windows::Settings = self.show.open_windows {
            for event in SettingsWindow::new(&global_state, &self.show, &self.imgs, &self.fonts)
                .set(self.ids.settings_window, ui_widgets)
            {
                match event {
                    settings_window::Event::ToggleHelp => self.show.help = !self.show.help,
                    settings_window::Event::ToggleDebug => self.show.debug = !self.show.debug,
                    settings_window::Event::ChangeTab(tab) => self.show.open_setting_tab(tab),
                    settings_window::Event::Close => self.show.settings(false),
                    settings_window::Event::AdjustMousePan(sensitivity) => {
                        events.push(Event::AdjustMousePan(sensitivity));
                    }
                    settings_window::Event::AdjustMouseZoom(sensitivity) => {
                        events.push(Event::AdjustMouseZoom(sensitivity));
                    }
                    settings_window::Event::AdjustViewDistance(view_distance) => {
                        events.push(Event::AdjustViewDistance(view_distance));
                    }
                    settings_window::Event::CrosshairTransp(crosshair_transp) => {
                        events.push(Event::CrosshairTransp(crosshair_transp));
                    }
                    settings_window::Event::AdjustMusicVolume(music_volume) => {
                        events.push(Event::AdjustMusicVolume(music_volume));
                    }
                    settings_window::Event::AdjustSfxVolume(sfx_volume) => {
                        events.push(Event::AdjustSfxVolume(sfx_volume));
                    }
                    settings_window::Event::MaximumFPS(max_fps) => {
                        events.push(Event::ChangeMaxFPS(max_fps));
                    }
                    settings_window::Event::ChangeAudioDevice(name) => {
                        events.push(Event::ChangeAudioDevice(name));
                    }
                    settings_window::Event::CrosshairType(crosshair_type) => {
                        events.push(Event::CrosshairType(crosshair_type));
                    }
                    settings_window::Event::ToggleXpBar(xp_bar) => {
                        events.push(Event::ToggleXpBar(xp_bar));
                    }
                    settings_window::Event::ToggleBarNumbers(bar_numbers) => {
                        events.push(Event::ToggleBarNumbers(bar_numbers));
                    }
                    settings_window::Event::ToggleShortcutNumbers(shortcut_numbers) => {
                        events.push(Event::ToggleShortcutNumbers(shortcut_numbers));
                    }
                    settings_window::Event::UiScale(scale_change) => {
                        events.push(Event::UiScale(scale_change));
                    }
                    settings_window::Event::AdjustFOV(new_fov) => {
                        events.push(Event::ChangeFOV(new_fov));
                    }
                    settings_window::Event::ChangeAaMode(new_aa_mode) => {
                        events.push(Event::ChangeAaMode(new_aa_mode));
                    }
                }
            }
        }

        // Social Window
        if self.show.social {
            for event in Social::new(
                /*&global_state,*/ &self.show,
                client,
                &self.imgs,
                &self.fonts,
            )
            .set(self.ids.social_window, ui_widgets)
            {
                match event {
                    social::Event::Close => self.show.social(false),
                    social::Event::ChangeSocialTab(social_tab) => {
                        self.show.open_social_tab(social_tab)
                    }
                }
            }
        }

        // Character Window
        if self.show.character_window {
            let ecs = client.state().ecs();
            let stats = ecs.read_storage::<comp::Stats>();
            let player_stats = stats.get(client.entity()).unwrap();
            match CharacterWindow::new(&self.show, &player_stats, &self.imgs, &self.fonts)
                .set(self.ids.character_window, ui_widgets)
            {
                Some(character_window::Event::Close) => {
                    self.show.character_window(false);
                    self.force_ungrab = true;
                }
                None => {}
            }
        }

        // Spellbook
        if self.show.spell {
            match Spell::new(&self.show, client, &self.imgs, &self.fonts)
                .set(self.ids.spell, ui_widgets)
            {
                Some(spell::Event::Close) => {
                    self.show.spell(false);
                    self.force_ungrab = true;
                }
                None => {}
            }
        }

        // Quest Log
        if self.show.quest {
            match Quest::new(&self.show, client, &self.imgs, &self.fonts)
                .set(self.ids.quest, ui_widgets)
            {
                Some(quest::Event::Close) => {
                    self.show.quest(false);
                    self.force_ungrab = true;
                }
                None => {}
            }
        }
        // Map
        if self.show.map {
            match Map::new(&self.show, client, &self.imgs, &self.fonts)
                .set(self.ids.map, ui_widgets)
            {
                Some(map::Event::Close) => {
                    self.show.map(false);
                    self.force_ungrab = true;
                }
                None => {}
            }
        }

        if self.show.esc_menu {
            match EscMenu::new(&self.imgs, &self.fonts).set(self.ids.esc_menu, ui_widgets) {
                Some(esc_menu::Event::OpenSettings(tab)) => {
                    self.show.open_setting_tab(tab);
                }
                Some(esc_menu::Event::Close) => {
                    self.show.esc_menu = false;
                    self.show.want_grab = false;
                    self.force_ungrab = true;
                }
                Some(esc_menu::Event::Logout) => {
                    events.push(Event::Logout);

                    #[cfg(feature = "discord")]
                    {
                        discord::send_all(vec![
                            DiscordUpdate::Details("Menu".into()),
                            DiscordUpdate::State("Idling".into()),
                            DiscordUpdate::LargeImg("bg_main".into()),
                        ]);
                    }
                }
                Some(esc_menu::Event::Quit) => events.push(Event::Quit),
                Some(esc_menu::Event::CharacterSelection) => events.push(Event::CharacterSelection),
                None => {}
            }
        }

        events
    }

    pub fn new_message(&mut self, msg: ClientEvent) {
        self.new_messages.push_back(msg);
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
        let cursor_grabbed = global_state.window.is_cursor_grabbed();
        let handled = match event {
            WinEvent::Ui(event) => {
                if (self.typing() && event.is_keyboard() && self.show.ui)
                    || !(cursor_grabbed && event.is_keyboard_or_mouse())
                {
                    self.ui.handle_event(event);
                }
                true
            }
            WinEvent::InputUpdate(GameInput::ToggleInterface, true) if !self.typing() => {
                self.show.toggle_ui();
                true
            }
            WinEvent::InputUpdate(GameInput::ToggleCursor, true) if !self.typing() => {
                self.force_ungrab = !self.force_ungrab;
                true
            }
            _ if !self.show.ui => false,
            WinEvent::Zoom(_) => !cursor_grabbed && !self.ui.no_widget_capturing_mouse(),

            WinEvent::InputUpdate(GameInput::Enter, true) => {
                self.ui.focus_widget(if self.typing() {
                    None
                } else {
                    Some(self.ids.chat)
                });
                true
            }
            WinEvent::InputUpdate(GameInput::Escape, true) => {
                if self.typing() {
                    self.ui.focus_widget(None);
                } else {
                    // Close windows on esc
                    self.show.toggle_windows();
                }
                true
            }

            // Press key while not typing
            WinEvent::InputUpdate(key, true) if !self.typing() => match key {
                GameInput::Command => {
                    self.force_chat_input = Some("/".to_owned());
                    self.force_chat_cursor = Some(Index { line: 0, char: 1 });
                    self.ui.focus_widget(Some(self.ids.chat));
                    true
                }
                GameInput::Map => {
                    self.show.toggle_map();
                    true
                }
                GameInput::Bag => {
                    self.show.toggle_bag();
                    true
                }
                GameInput::QuestLog => {
                    self.show.toggle_quest();
                    true
                }
                GameInput::CharacterWindow => {
                    self.show.toggle_char_window();
                    true
                }
                GameInput::Social => {
                    self.show.toggle_social();
                    true
                }
                GameInput::Spellbook => {
                    self.show.toggle_spell();
                    true
                }
                GameInput::Settings => {
                    self.show.toggle_settings();
                    true
                }
                GameInput::Help => {
                    self.show.toggle_help();
                    true
                }
                GameInput::ToggleDebug => {
                    self.show.debug = !self.show.debug;
                    true
                }
                GameInput::ToggleIngameUi => {
                    self.show.ingame = !self.show.ingame;
                    true
                }
                _ => false,
            },
            // Else the player is typing in chat
            WinEvent::InputUpdate(_key, _) => self.typing(),
            WinEvent::Char(_) => self.typing(),
            WinEvent::Focused(state) => {
                self.force_ungrab = !state;
                true
            }

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
    ) -> Vec<Event> {
        if let Some(maybe_id) = self.to_focus.take() {
            self.ui.focus_widget(maybe_id);
        }
        let events = self.update_layout(client, global_state, debug_info);
        let (view_mat, _, _) = camera.compute_dependents(client);
        let fov = camera.get_fov();
        self.ui.maintain(
            &mut global_state.window.renderer_mut(),
            Some((view_mat, fov)),
        );
        events
    }

    pub fn render(&self, renderer: &mut Renderer, globals: &Consts<Globals>) {
        // Don't show anything if the UI is toggled off.
        if self.show.ui {
            self.ui.render(renderer, Some(globals));
        }
    }
}

// Get the text to show in the help window and use the
// length of the longest line to resize the window.
fn get_help_text(cs: &ControlSettings) -> String {
    format!(
        "{free_cursor:?} = Free cursor\n\
         {escape:?} = Open/close menus\n\
         \n\
         {help:?} = Toggle this window\n\
         {toggle_interface:?} = Toggle interface\n\
         \n\
         {chat:?} = Open chat\n\
         Mouse Wheel = Scroll chat/zoom",
        free_cursor = cs.toggle_cursor,
        escape = cs.escape,
        help = cs.help,
        toggle_interface = cs.toggle_interface,
        chat = cs.enter
    )
}
