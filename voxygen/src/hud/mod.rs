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

use bag::Bag;
use buttons::Buttons;
use character_window::CharacterWindow;
use chat::Chat;
use esc_menu::EscMenu;
use img_ids::Imgs;
use map::Map;
use minimap::MiniMap;
use settings_window::SettingsWindow;
use skillbar::Skillbar;
use small_window::{SmallWindow, SmallWindowType};

use crate::{
    render::{Consts, Globals, Renderer},
    scene::camera::Camera,
    settings::{ControlSettings, Settings},
    ui::{Ingame, Ingameable, ScaleMode, Ui},
    window::{Event as WinEvent, Key, Window},
    GlobalState,
};
use client::Client;
use common::comp;
use conrod_core::{
    color, graph,
    widget::{self, Button, Image, Rectangle, Text},
    widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, Widget,
};
use specs::Join;
use std::collections::VecDeque;
use vek::*;

const XP_COLOR: Color = Color::Rgba(0.59, 0.41, 0.67, 1.0);
const TEXT_COLOR: Color = Color::Rgba(1.0, 1.0, 1.0, 1.0);
const HP_COLOR: Color = Color::Rgba(0.33, 0.63, 0.0, 1.0);
const MANA_COLOR: Color = Color::Rgba(0.42, 0.41, 0.66, 1.0);

widget_ids! {
    struct Ids {
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
}

pub enum Event {
    SendMessage(String),
    AdjustViewDistance(u32),
    AdjustVolume(f32),
    ChangeAudioDevice(String),
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

pub struct Show {
    ui: bool,
    help: bool,
    debug: bool,
    bag: bool,
    esc_menu: bool,
    open_windows: Windows,
    map: bool,
    inventory_test_button: bool,
    mini_map: bool,

    want_grab: bool,
}
impl Show {
    fn toggle_bag(&mut self) {
        self.bag = !self.bag;
        self.want_grab = !self.bag;
    }

    fn toggle_map(&mut self) {
        self.map = !self.map;
        self.bag = false;
        self.want_grab = !self.map;
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

    fn toggle_settings(&mut self) {
        self.open_windows = match self.open_windows {
            Windows::Settings => Windows::None,
            _ => Windows::Settings,
        };
        self.bag = false;
        self.want_grab = self.open_windows != Windows::Settings;
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
}

pub struct Hud {
    ui: Ui,
    ids: Ids,
    imgs: Imgs,
    fonts: Fonts,
    new_messages: VecDeque<String>,
    inventory_space: u32,
    show: Show,
    to_focus: Option<Option<widget::Id>>,
    force_ungrab: bool,
}

impl Hud {
    pub fn new(window: &mut Window) -> Self {
        let mut ui = Ui::new(window).unwrap();
        // TODO: Adjust/remove this, right now it is used to demonstrate window scaling functionality.
        ui.scaling_mode(ScaleMode::RelativeToWindow([1920.0, 1080.0].into()));
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
            inventory_space: 0,
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
                want_grab: true,
            },
            to_focus: None,
            force_ungrab: false,
        }
    }

    fn update_layout(&mut self, client: &Client, global_state: &GlobalState, debug_info: DebugInfo) -> Vec<Event> {
        let mut events = Vec::new();
        let ref mut ui_widgets = self.ui.set_widgets();
        let version = env!("CARGO_PKG_VERSION");

        // Don't show anything if the UI is toggled off.
        if !self.show.ui {
            return events;
        }

        // Nametags and healthbars
        {
            let ecs = client.state().ecs();
            let actor = ecs.read_storage::<comp::Actor>();
            let pos = ecs.read_storage::<comp::phys::Pos>();
            let stats = ecs.read_storage::<comp::Stats>();
            let entities = ecs.entities();
            let player = client.entity();
            let mut name_id_walker = self.ids.name_tags.walk();
            let mut health_id_walker = self.ids.health_bars.walk();
            let mut health_back_id_walker = self.ids.health_bar_backs.walk();
            for (pos, name) in
                (&entities, &pos, &actor)
                    .join()
                    .filter_map(|(entity, pos, actor)| match actor {
                        comp::Actor::Character { name, .. } if entity != player => {
                            Some((pos.0, name))
                        }
                        _ => None,
                    })
            {
                let id = name_id_walker.next(
                    &mut self.ids.name_tags,
                    &mut ui_widgets.widget_id_generator(),
                );
                Text::new(&name)
                    .font_size(20)
                    .color(Color::Rgba(1.0, 1.0, 1.0, 1.0))
                    .x_y(0.0, 0.0)
                    .position_ingame(pos + Vec3::new(0.0, 0.0, 3.0))
                    .resolution(100.0)
                    .set(id, ui_widgets);
            }
            for (pos, hp) in (&entities, &pos, &stats)
                .join()
                .filter_map(|(entity, pos, stats)| {
                    if entity != player {
                        Some((pos.0, stats.hp))
                    } else {
                        None
                    }
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
                // Healh Bar
                Rectangle::fill_with([120.0, 8.0], Color::Rgba(0.3, 0.3, 0.3, 0.5))
                    .x_y(0.0, -25.0)
                    .position_ingame(pos + Vec3::new(0.0, 0.0, 3.0))
                    .resolution(100.0)
                    .set(back_id, ui_widgets);

                // Filling
                Rectangle::fill_with(
                    [120.0 * (hp.current as f64 / hp.maximum as f64), 8.0],
                    HP_COLOR,
                )
                .x_y(0.0, -25.0)
                .position_ingame(pos + Vec3::new(0.0, 0.0, 3.0))
                .resolution(100.0)
                .set(bar_id, ui_widgets);
            }
        }

        // Display debug window.
        if self.show.debug {
            // Alpha Version
            Text::new(version)
                .top_left_with_margins_on(ui_widgets.window, 5.0, 5.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(self.ids.version, ui_widgets);
            Text::new(&format!("FPS: {:.1}", debug_info.tps))
                .color(TEXT_COLOR)
                .down_from(self.ids.version, 5.0)
                .font_id(self.fonts.opensans)
                .font_size(14)
                .set(self.ids.fps_counter, ui_widgets);
            Text::new(&format!("Ping: {:.1}ms", debug_info.ping_ms))
                .color(TEXT_COLOR)
                .down_from(self.ids.fps_counter, 5.0)
                .font_id(self.fonts.opensans)
                .font_size(14)
                .set(self.ids.ping, ui_widgets);
        }

        // Add Bag-Space Button.
        if self.show.inventory_test_button {
            if Button::image(self.imgs.grid_button)
                .w_h(100.0, 100.0)
                .middle_of(ui_widgets.window)
                .label("1 Up!")
                .label_font_size(20)
                .hover_image(self.imgs.grid_button_hover)
                .press_image(self.imgs.grid_button_press)
                .set(self.ids.bag_space_add, ui_widgets)
                .was_clicked()
            {
                self.inventory_space += 1;
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
        match MiniMap::new(&self.show, &self.imgs, &self.fonts).set(self.ids.minimap, ui_widgets) {
            Some(minimap::Event::Toggle) => self.show.toggle_mini_map(),
            None => {}
        }

        // Bag contents
        if self.show.bag {
            match Bag::new(self.inventory_space, &self.imgs, &self.fonts)
                .set(self.ids.bag, ui_widgets)
            {
                Some(bag::Event::Close) => self.show.bag = false,
                None => {}
            }
        }

        // Skillbar
        // Get player stats
        let stats = client
            .state()
            .ecs()
            .read_storage::<comp::Stats>()
            .get(client.entity())
            .map(|&s| s)
            .unwrap_or_default();
        Skillbar::new(&self.imgs, &self.fonts, stats).set(self.ids.skillbar, ui_widgets);

        // Chat box
        match Chat::new(&mut self.new_messages, &self.imgs, &self.fonts)
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
            for event in SettingsWindow::new(&self.show, &self.imgs, &self.fonts, &global_state)
                .set(self.ids.settings_window, ui_widgets)
            {
                match event {
                    settings_window::Event::ToggleHelp => self.show.toggle_help(),
                    settings_window::Event::ToggleInventoryTestButton => {
                        self.show.inventory_test_button = !self.show.inventory_test_button
                    }
                    settings_window::Event::ToggleDebug => self.show.debug = !self.show.debug,
                    settings_window::Event::Close => self.show.open_windows = Windows::None,
                    settings_window::Event::AdjustViewDistance(view_distance) => {
                        events.push(Event::AdjustViewDistance(view_distance));
                    }
                    settings_window::Event::AdjustVolume(volume) => {
                        events.push(Event::AdjustVolume(volume));
                    }
                    settings_window::Event::ChangeAudioDevice(name) => {
                        events.push(Event::ChangeAudioDevice(name));
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
            match CharacterWindow::new(&self.imgs, &self.fonts)
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
            match Map::new(&self.imgs, &self.fonts).set(self.ids.map, ui_widgets) {
                Some(map::Event::Close) => self.show.map = false,
                None => {}
            }
        }

        // Esc-menu
        if self.show.esc_menu {
            match EscMenu::new(&self.imgs, &self.fonts).set(self.ids.esc_menu, ui_widgets) {
                Some(esc_menu::Event::OpenSettings) => {
                    self.show.esc_menu = false;
                    self.show.open_windows = Windows::Settings;
                }
                Some(esc_menu::Event::Close) => self.show.esc_menu = false,
                Some(esc_menu::Event::Logout) => events.push(Event::Logout),
                Some(esc_menu::Event::Quit) => events.push(Event::Quit),
                None => {}
            }
        }

        events
    }

    pub fn new_message(&mut self, msg: String) {
        self.new_messages.push_back(msg);
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
            WinEvent::KeyDown(Key::ToggleInterface) => {
                self.show.toggle_ui();
                true
            }
            WinEvent::KeyDown(Key::ToggleCursor) => {
                self.force_ungrab = !self.force_ungrab;
                if self.force_ungrab {
                    global_state.window.grab_cursor(false);
                }
                true
            }
            _ if !self.show.ui => false,
            WinEvent::Zoom(_) => !cursor_grabbed && !self.ui.no_widget_capturing_mouse(),
            WinEvent::KeyDown(Key::Enter) => {
                self.ui.focus_widget(if self.typing() {
                    None
                } else {
                    Some(self.ids.chat)
                });
                true
            }
            WinEvent::KeyDown(Key::Escape) => {
                if self.typing() {
                    self.ui.focus_widget(None);
                } else {
                    // Close windows on esc
                    self.show.toggle_windows();
                }
                true
            }
            WinEvent::KeyDown(key) if !self.typing() => match key {
                Key::Map => {
                    self.show.toggle_map();
                    true
                }
                Key::Bag => {
                    self.show.toggle_bag();
                    true
                }
                Key::QuestLog => {
                    self.show.toggle_small(SmallWindowType::QuestLog);
                    true
                }
                Key::CharacterWindow => {
                    self.show.toggle_char_window();
                    true
                }
                Key::Social => {
                    self.show.toggle_small(SmallWindowType::Social);
                    true
                }
                Key::Spellbook => {
                    self.show.toggle_small(SmallWindowType::Spellbook);
                    true
                }
                Key::Settings => {
                    self.show.toggle_settings();
                    true
                }
                Key::Help => {
                    self.show.toggle_help();
                    true
                }
                Key::ToggleDebug => {
                    self.show.debug = !self.show.debug;
                    true
                }
                _ => false,
            },
            WinEvent::KeyDown(key) | WinEvent::KeyUp(key) => match key {
                Key::ToggleCursor => false,
                _ => self.typing(),
            },
            WinEvent::Char(_) => self.typing(),
            _ => false,
        };
        // Handle cursor grab.
        if !self.force_ungrab {
            if cursor_grabbed != self.show.want_grab {
                global_state.window.grab_cursor(self.show.want_grab);
            }
        }

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
        self.ui.maintain(&mut global_state.window.renderer_mut(), Some((view_mat, fov)));
        events
    }

    pub fn render(&self, renderer: &mut Renderer, globals: &Consts<Globals>) {
        self.ui.render(renderer, Some(globals));
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
