mod bag;
mod buttons;
mod character_window;
mod chat;
mod esc_menu;
mod img_ids;
mod map;
mod minimap;
mod settings_window;
mod skillbar;
mod small_window;

pub use settings_window::ScaleChange;

use bag::Bag;
use buttons::Buttons;
use character_window::CharacterWindow;
use chat::Chat;
use esc_menu::EscMenu;
use img_ids::Imgs;
use map::Map;
use minimap::MiniMap;
use serde::{Deserialize, Serialize};
use settings_window::{SettingsTab, SettingsWindow};
use skillbar::Skillbar;
use small_window::{SmallWindow, SmallWindowType};

use crate::{
    render::{Consts, Globals, Renderer},
    scene::camera::Camera,
    settings::ControlSettings,
    ui::{Ingameable, ScaleMode, Ui},
    window::{Event as WinEvent, GameInput},
    GlobalState,
};
use client::{Client, Event as ClientEvent};
use common::{comp, terrain::TerrainChunkSize, vol::VolSize};
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
const HP_COLOR: Color = Color::Rgba(0.33, 0.63, 0.0, 1.0);
const MANA_COLOR: Color = Color::Rgba(0.42, 0.41, 0.66, 1.0);
const TELL_COLOR: Color = Color::Rgba(1.0, 1.0, 0.0, 1.0);
const PRIVATE_COLOR: Color = Color::Rgba(1.0, 1.0, 0.0, 1.0);
const BROADCAST_COLOR: Color = Color::Rgba(0.0, 1.0, 0.0, 1.0);
const GAME_UPDATE_COLOR: Color = Color::Rgba(1.0, 1.0, 0.0, 1.0);

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
        loaded_distance,

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
        skillbar,
        buttons,
        esc_menu,
        small_window,
        settings_window,
    }
}

font_ids! {
    pub struct Fonts {
        opensans: "voxygen/font/OpenSans-Regular.ttf",
        metamorph: "voxygen/font/Metamorphous-Regular.ttf",
    }
}

pub struct DebugInfo {
    pub tps: f64,
    pub ping_ms: f64,
    pub coordinates: Option<comp::Pos>,
}
//#[derive(Serialize, Deserialize)]
pub enum Event {
    SendMessage(String),
    AdjustMousePan(u32),
    AdjustMouseZoom(u32),
    AdjustViewDistance(u32),
    AdjustVolume(f32),
    ChangeAudioDevice(String),
    ChangeMaxFPS(u32),
    CrosshairTransp(f32),
    CrosshairType(CrosshairType),
    UiScale(ScaleChange),
    CharacterSelection,
    Logout,
    Quit,
}

// TODO: Are these the possible layouts we want?
// TODO: Maybe replace this with bitflags.
// `map` is not here because it currently is displayed over the top of other open windows.
#[derive(PartialEq)]
pub enum Windows {
    Settings,                              // Display settings window.
    CharacterAnd(Option<SmallWindowType>), // Show character window + optionally another.
    Small(SmallWindowType),
    None,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum CrosshairType {
    Round,
    RoundEdges,
    Edges,
}

pub struct Show {
    ui: bool,
    help: bool,
    debug: bool,
    bag: bool,
    esc_menu: bool,
    open_windows: Windows,
    map: bool,
    inventory_test_button: bool,
    rel_to_win: bool,
    absolute: bool,
    mini_map: bool,
    ingame: bool,
    settings_tab: SettingsTab,
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
    fn toggle_map(&mut self) {
        self.map(!self.map)
    }

    fn toggle_mini_map(&mut self) {
        self.mini_map = !self.mini_map;
    }

    fn toggle_small(&mut self, target: SmallWindowType) {
        self.open_windows = match self.open_windows {
            Windows::Small(small) if small == target => Windows::None,
            Windows::None | Windows::Small(_) => Windows::Small(target),
            Windows::CharacterAnd(small) => match small {
                Some(small) if small == target => Windows::CharacterAnd(None),
                _ => Windows::CharacterAnd(Some(target)),
            },
            Windows::Settings => Windows::Settings,
        };
    }

    fn toggle_char_window(&mut self) {
        self.open_windows = match self.open_windows {
            Windows::CharacterAnd(small) => match small {
                Some(small) => Windows::Small(small),
                None => Windows::None,
            },
            Windows::Small(small) => Windows::CharacterAnd(Some(small)),
            Windows::None => Windows::CharacterAnd(None),
            Windows::Settings => Windows::Settings,
        }
    }

    fn settings(&mut self, open: bool) {
        self.open_windows = if open {
            Windows::Settings
        } else {
            Windows::None
        };
        self.bag = false;
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
            || match self.open_windows {
                Windows::None => false,
                _ => true,
            }
        {
            self.bag = false;
            self.esc_menu = false;
            self.map = false;
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
}

pub struct Hud {
    ui: Ui,
    ids: Ids,
    imgs: Imgs,
    fonts: Fonts,
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
        // Load fonts.
        let fonts = Fonts::load(&mut ui).expect("Failed to load fonts!");

        Self {
            ui,
            imgs,
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
                inventory_test_button: false,
                mini_map: false,
                settings_tab: SettingsTab::Interface,
                want_grab: true,
                ingame: true,
                rel_to_win: true,
                absolute: false,
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
        let ref mut ui_widgets = self.ui.set_widgets();

        let version = format!("{}-{}", env!("CARGO_PKG_VERSION"), common::util::GIT_HASH);

        // Nametags and healthbars
        if self.show.ingame {
            let ecs = client.state().ecs();
            let pos = ecs.read_storage::<comp::Pos>();
            let stats = ecs.read_storage::<comp::Stats>();
            let player = ecs.read_storage::<comp::Player>();
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

            // Render Name Tags
            for (pos, name) in (&entities, &pos, &stats, player.maybe())
                .join()
                .filter(|(entity, _, stats, _)| *entity != me && !stats.is_dead)
                // Don't process nametags outside the vd (visibility further limited by ui backend)
                .filter(|(_, pos, _, _)| {
                    (pos.0 - player_pos)
                        .map2(TerrainChunkSize::SIZE, |d, sz| d.abs() as f32 / sz as f32)
                        .magnitude()
                        < view_distance as f32
                })
                .map(|(_, pos, stats, player)| {
                    // TODO: This is temporary
                    // If the player used the default character name display their name instead
                    let name = if stats.name == "Character Name" {
                        player.map_or(&stats.name, |p| &p.alias)
                    } else {
                        &stats.name
                    };
                    (pos.0, name)
                })
            {
                let id = name_id_walker.next(
                    &mut self.ids.name_tags,
                    &mut ui_widgets.widget_id_generator(),
                );
                Text::new(&name)
                    .font_size(20)
                    .color(Color::Rgba(0.61, 0.61, 0.89, 1.0))
                    .x_y(0.0, 0.0)
                    .position_ingame(pos + Vec3::new(0.0, 0.0, 3.0))
                    .resolution(100.0)
                    .set(id, ui_widgets);
            }

            // Render Health Bars
            for (_entity, pos, stats) in (&entities, &pos, &stats)
                .join()
                .filter(|(entity, _, stats)| {
                    *entity != me
                        && !stats.is_dead
                        && stats.health.get_current() != stats.health.get_maximum()
                })
                // Don't process health bars outside the vd (visibility further limited by ui backend)
                .filter(|(_, pos, _)| {
                    (pos.0 - player_pos)
                        .map2(TerrainChunkSize::SIZE, |d, sz| d.abs() as f32 / sz as f32)
                        .magnitude()
                        < view_distance as f32
                })
            {
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
                    .position_ingame(pos.0 + Vec3::new(0.0, 0.0, 3.0))
                    .resolution(100.0)
                    .set(back_id, ui_widgets);

                // % HP Filling
                Rectangle::fill_with(
                    [
                        120.0
                            * (stats.health.get_current() as f64
                                / stats.health.get_maximum() as f64),
                        8.0,
                    ],
                    HP_COLOR,
                )
                .x_y(0.0, -25.0)
                .position_ingame(pos.0 + Vec3::new(0.0, 0.0, 3.0))
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
            Text::new(&format!("FPS: {:.1}", debug_info.tps))
                .color(TEXT_COLOR)
                .down_from(self.ids.version, 5.0)
                .font_id(self.fonts.opensans)
                .font_size(14)
                .set(self.ids.fps_counter, ui_widgets);
            // Ping
            Text::new(&format!("Ping: {:.1}ms", debug_info.ping_ms))
                .color(TEXT_COLOR)
                .down_from(self.ids.fps_counter, 5.0)
                .font_id(self.fonts.opensans)
                .font_size(14)
                .set(self.ids.ping, ui_widgets);
            // Players position
            let coordinates_text = match debug_info.coordinates {
                Some(coordinates) => format!("Coordinates: {:.1}", coordinates.0),
                None => "Player has no Pos component".to_owned(),
            };
            Text::new(&coordinates_text)
                .color(TEXT_COLOR)
                .down_from(self.ids.ping, 5.0)
                .font_id(self.fonts.opensans)
                .font_size(14)
                .set(self.ids.coordinates, ui_widgets);
            // Loaded distance
            Text::new(&format!(
                "View distance: {} chunks",
                client.loaded_distance().unwrap_or(0)
            ))
            .color(TEXT_COLOR)
            .down_from(self.ids.coordinates, 5.0)
            .font_id(self.fonts.opensans)
            .font_size(14)
            .set(self.ids.loaded_distance, ui_widgets);
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
            Some(buttons::Event::ToggleSmall(small)) => self.show.toggle_small(small),
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
            match Bag::new(self.inventory_space, &self.imgs, &self.fonts)
                .set(self.ids.bag, ui_widgets)
            {
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
            Skillbar::new(&self.imgs, &self.fonts, stats).set(self.ids.skillbar, ui_widgets);
        }

        // Chat box
        let mut chat = Chat::new(&mut self.new_messages, &self.imgs, &self.fonts);

        if let Some(input) = self.force_chat_input.take() {
            chat = chat.input(input);
        }

        if let Some(pos) = self.force_chat_cursor.take() {
            chat = chat.cursor_pos(pos);
        }

        match chat.set(self.ids.chat, ui_widgets) {
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
            for event in SettingsWindow::new(&global_state, &self.show, &self.imgs, &self.fonts, self.show.rel_to_win, self.show.absolute)
                .set(self.ids.settings_window, ui_widgets)
            {
                match event {
                    settings_window::Event::ToggleHelp => self.show.toggle_help(),
                    settings_window::Event::ToggleInventoryTestButton => {
                        self.show.inventory_test_button = !self.show.inventory_test_button
                    }
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
                    settings_window::Event::AdjustVolume(volume) => {
                        events.push(Event::AdjustVolume(volume));
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
                    settings_window::Event::UiScale(scale_change) => {
                        events.push(Event::UiScale(scale_change));
                    }
                }
            }
        }

        // Small Window
        if let Windows::Small(small) | Windows::CharacterAnd(Some(small)) = self.show.open_windows {
            match SmallWindow::new(small, &self.show, &self.imgs, &self.fonts)
                .set(self.ids.small_window, ui_widgets)
            {
                Some(small_window::Event::Close) => {
                    self.show.open_windows = match self.show.open_windows {
                        Windows::Small(_) => Windows::None,
                        Windows::CharacterAnd(_) => Windows::CharacterAnd(None),
                        _ => Windows::Settings,
                    }
                }
                None => {}
            }
        }

        // Character Window
        if let Windows::CharacterAnd(small) = self.show.open_windows {
            let ecs = client.state().ecs();
            let stats = ecs.read_storage::<comp::Stats>();
            let player_stats = stats.get(client.entity()).unwrap();
            match CharacterWindow::new(&self.imgs, &self.fonts, &player_stats)
                .set(self.ids.character_window, ui_widgets)
            {
                Some(character_window::Event::Close) => {
                    self.show.open_windows = match small {
                        Some(small) => Windows::Small(small),
                        None => Windows::None,
                    }
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
                    self.show.toggle_small(SmallWindowType::QuestLog);
                    true
                }
                GameInput::CharacterWindow => {
                    self.show.toggle_char_window();
                    true
                }
                GameInput::Social => {
                    self.show.toggle_small(SmallWindowType::Social);
                    true
                }
                GameInput::Spellbook => {
                    self.show.toggle_small(SmallWindowType::Spellbook);
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
