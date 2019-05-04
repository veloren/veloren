mod chat;
mod character_window;
mod skillbar;
mod buttons;
mod map;
mod bag;
mod esc_menu;
mod small_window;
mod settings_window;
mod img_ids;
mod font_ids;

use chat::Chat;
use character_window::CharacterWindow;
use map::Map;
use bag::Bag;
use skillbar::Skillbar;
use buttons::Buttons;
use esc_menu::EscMenu;
use small_window::{SmallWindow, SmallWindowType};
use settings_window::SettingsWindow;
use img_ids::Imgs;
use font_ids::Fonts;

use crate::{
    render::Renderer,
    settings::{ControlSettings, Settings},
    ui::{ScaleMode, ToggleButton, Ui, Graphic},
    window::{Event as WinEvent, Key, Window},
    GlobalState,
};
use conrod_core::{
    color,
    text::{Font, font::Id as FontId},
    widget::{Button, Image, Rectangle, Scrollbar, Text},
    WidgetStyle, widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, Widget,
};
use common::assets;
use std::collections::VecDeque;

const XP_COLOR: Color = Color::Rgba(0.59, 0.41, 0.67, 1.0);
const TEXT_COLOR: Color = Color::Rgba(1.0, 1.0, 1.0, 1.0);
const HP_COLOR: Color = Color::Rgba(0.33, 0.63, 0.0, 1.0);
const MANA_COLOR: Color = Color::Rgba(0.42, 0.41, 0.66, 1.0);

widget_ids! {
    struct Ids {
        // Test
        bag_space_add,
        // Debug
        debug_bg,
        fps_counter,
        
        // Game Version
        version,

        // Help
        help,
        help_bg,

        // Mini-Map
        mmap_frame,
        mmap_frame_bg,
        mmap_location,
        mmap_button,

    

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
        bag,
        skillbar,
        buttons,
        esc_menu,
        small_window,
        settings_window,
    }
}

pub enum Event {
    SendMessage(String),
    Logout,
    Quit,
}

// TODO: are these the possible layouts we want?
// TODO: maybe replace this with bitflags
// map not here because it currently is displayed over the top of other open windows
#[derive(PartialEq)]
pub enum Windows {
    Settings,                    // display settings window
    CharacterAnd(Option<SmallWindowType>), // show character window + optionally another
    Small(SmallWindowType),
    None,
}

pub struct Hud {
    ui: Ui,
    ids: Ids,
    imgs: Imgs,
    fonts: Fonts,
    new_messages: VecDeque<String>,
    show_help: bool,
    show_debug: bool,
    show_bag: bool,
    esc_menu_open: bool,
    open_windows: Windows,
    mmap_open: bool,
    show_map: bool,
    show_ui: bool,
    mmap_open: bool,
    inventory_space: u32,
    inventorytest_button: bool,
}

impl Hud {
    pub fn new(window: &mut Window, settings: Settings) -> Self {
        let mut ui = Ui::new(window).unwrap();
        // TODO: adjust/remove this, right now it is used to demonstrate window scaling functionality
        ui.scaling_mode(ScaleMode::RelativeToWindow([1920.0, 1080.0].into()));
        // Generate ids
        let ids = Ids::new(ui.id_generator());
        // Load images
        let imgs = Imgs::load(&mut ui).expect("Failed to load images");
        // Load fonts
        let fonts = Fonts::load(&mut ui).expect("Failed to load fonts");

        Self {
            ui,
            imgs,
            fonts,
            ids,
            new_messages: VecDeque::new(),
            show_help: false,
            show_debug: true,
            show_bag: false,
            esc_menu_open: false,
            open_windows: Windows::None,
            show_map: false,
            show_ui: true,
            mmap_open: false,
            inventorytest_button: false,
            inventory_space: 0,
        }
    }

    fn update_layout(&mut self, tps: f64) -> Vec<Event> {
        let mut events = Vec::new();
        let ref mut ui_widgets = self.ui.set_widgets();
        let version = env!("CARGO_PKG_VERSION");

        // Don't show anything if the UI is toggled off
        if !self.show_ui {
            return events;
        }

        // Display debug window
        if self.show_debug {
            // Alpha Version
            Text::new(version)
                .top_left_with_margins_on(ui_widgets.window, 5.0, 5.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(self.ids.version, ui_widgets);
            Text::new(&format!("FPS: {:.1}", tps))
                .color(TEXT_COLOR)
                .down_from(self.ids.version, 5.0)
                .font_id(self.fonts.opensans)
                .font_size(14)
                .set(self.ids.fps_counter, ui_widgets);
        }
        // Add Bag-Space Button
        if self.inventorytest_button {
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
        if self.show_help {
            Image::new(self.imgs.window_frame_2)
                .top_left_with_margins_on(ui_widgets.window, 3.0, 3.0)
                .w_h(300.0, 190.0)
                .set(self.ids.help_bg, ui_widgets);
            Text::new(get_help_text(&self.settings.controls).as_str())
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
                .top_right_with_margins_on(self.ids.help_bg, 8.0, 3.0)
                .set(self.ids.button_help2, ui_widgets)
                .was_clicked()
            {
                self.show_help = false;
            };
        }

        match Buttons::new(&self.open_windows, self.show_map, self.show_bag, &self.imgs, &self.fonts)
            .set(self.ids.buttons, ui_widgets) 
        {
            //Some(buttons::Event::ToggleBag) => self.toggle_bag(),
            //Some(buttons::Event::ToggleSettings) => self.toggle_settings(),
            //Some(buttons::Event::ToggleCharacter) => self.toggle_charwindow(),
            //Some(buttons::Event::ToggleSmall(small)) => self.toggle_small(small),
            //Some(buttons::Event::ToggleMap) => self.toggle_map(),
            //None => {}
            _ => {}
        }

        // Minimap

        if self.mmap_open {
            Image::new(self.imgs.mmap_frame)
                .w_h(100.0 * 2.0, 100.0 * 2.0)
                .top_right_with_margins_on(ui_widgets.window, 5.0, 5.0)
                .set(self.ids.mmap_frame, ui_widgets);

            Rectangle::fill_with([92.0 * 2.0, 82.0 * 2.0], color::TRANSPARENT)
                .mid_top_with_margin_on(self.ids.mmap_frame, 13.0 * 2.0 + 2.0)
                .set(self.ids.mmap_frame_bg, ui_widgets);
        } else {
            Image::new(self.imgs.mmap_frame_closed)
                .w_h(100.0 * 2.0, 11.0 * 2.0)
                .top_right_with_margins_on(ui_widgets.window, 5.0, 5.0)
                .set(self.ids.mmap_frame, ui_widgets);
        };

        if Button::image(if self.mmap_open {
            self.imgs.mmap_open
        } else {
            self.imgs.mmap_closed
        })
        .w_h(100.0 * 0.2, 100.0 * 0.2)
        .hover_image(if self.mmap_open {
            self.imgs.mmap_open_hover
        } else {
            self.imgs.mmap_closed_hover
        })
        .press_image(if self.mmap_open {
            self.imgs.mmap_open_press
        } else {
            self.imgs.mmap_closed_press
        })
        .top_right_with_margins_on(self.ids.mmap_frame, 0.0, 0.0)
        .set(self.ids.mmap_button, ui_widgets)
        .was_clicked()
        {
            self.mmap_open = !self.mmap_open;
        };

        // Title
        // Make it display the actual location
        Text::new("Uncanny Valley")
            .mid_top_with_margin_on(self.ids.mmap_frame, 3.0)
            .font_size(14)
            .color(TEXT_COLOR)
            .set(self.ids.mmap_location, ui_widgets);

        // Bag contents
        if self.show_bag {
            match Bag::new(self.inventory_space, &self.imgs, &self.fonts)
                .set(self.ids.bag, ui_widgets) 
            {
                Some(bag::Event::Close) => self.show_bag = false,
                None => {}
            }
        }

        Skillbar::new(&self.imgs, &self.fonts)
            .set(self.ids.skillbar, ui_widgets);

        // Chat box
        match Chat::new(&mut self.new_messages, &self.imgs, &self.fonts)
            .set(self.ids.chat, ui_widgets) 
        {
            Some(chat::Event::SendMessage(message)) => {
                events.push(Event::SendMessage(message));
            }
            None => {}
        }
        self.new_messages = VecDeque::new();

        //Windows

        //Char Window will always appear at the left side. Other Windows either appear at the left side,
        //or when the Char Window is opened they will appear right from it.

        // Settings

        if let Windows::Settings = self.open_windows {
            match SettingsWindow::new(&self.imgs, &self.fonts)
                .set(self.ids.settings_window, ui_widgets) 
            {
                Some(settings_window::Event::Close) => {
                    self.open_windows = Windows::None;
                }
                None => {}
            }
        }

        // Small Window
        if let Some((small, char_window_open)) = match self.open_windows {
            Windows::Small(small) => Some((small, false)),
            Windows::CharacterAnd(Some(small)) => Some((small, true)),
            _ => None,
        } {
            match SmallWindow::new(small, &self.imgs, &self.fonts)
                .set(self.ids.small_window, ui_widgets) 
            {
                Some(small_window::Event::Close) => self.open_windows = match self.open_windows {
                    Windows::Small(_) => Windows::None,
                    Windows::CharacterAnd(_) => Windows::CharacterAnd(None),
                    _ => Windows::Settings,
                },
                None => {}
            }
        }

        // Character Window
        if let Windows::CharacterAnd(small) = self.open_windows {
            match CharacterWindow::new(&self.imgs, &self.fonts)
                .set(self.ids.character_window, ui_widgets) 
            {
                Some(character_window::Event::Close) => self.open_windows = match small {
                    Some(small) => Windows::Small(small),
                    None => Windows::None,
                },
                None => {}
            }
        }

        // Map
        if self.show_map {
            match Map::new(&self.imgs, &self.fonts)
                .set(self.ids.map, ui_widgets) 
            {
                Some(map::Event::Close) => self.show_map = false,
                None => {}
            }
        }


        // Esc-menu
        if self.esc_menu_open {
            match EscMenu::new(&self.imgs, &self.fonts)
                .set(self.ids.esc_menu, ui_widgets) 
            {
                Some(esc_menu::Event::OpenSettings) => {
                    self.esc_menu_open = false;
                    self.open_windows = Windows::Settings;
                }
                Some(esc_menu::Event::Close) => self.esc_menu_open = false,
                Some(esc_menu::Event::Logout) => events.push(Event::Logout),
                Some(esc_menu::Event::Quit) => events.push(Event::Quit),
                None => {},
            }
        }
        
        events
    }

    pub fn new_message(&mut self, msg: String) {
        self.new_messages.push_back(msg);
    }

    fn toggle_menu(&mut self) {
        self.esc_menu_open = !self.esc_menu_open;
    }
    fn toggle_bag(&mut self) {
        self.show_bag = !self.show_bag
    }
    fn toggle_map(&mut self) {
        self.show_map = !self.show_map;
        self.show_bag = false;
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
    fn toggle_charwindow(&mut self) {
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
        self.show_bag = false;
    }
    fn toggle_help(&mut self) {
        self.show_help = !self.show_help
    }
    fn toggle_ui(&mut self) {
        self.show_ui = !self.show_ui;
    }

    fn toggle_windows(&mut self, global_state: &mut GlobalState) {
        if self.show_bag
            || self.esc_menu_open
            || self.show_map
            || match self.open_windows {
                Windows::None => false,
                _ => true,
            }
        {
            self.show_bag = false;
            self.esc_menu_open = false;
            self.show_map = false;
            self.open_windows = Windows::None;
            global_state.window.grab_cursor(true);
        } else {
            self.esc_menu_open = true;
            global_state.window.grab_cursor(false);
        }
    }

    fn typing(&self) -> bool {
        match self.ui.widget_capturing_keyboard() {
            Some(id) if id == self.ids.chat => true,
            _ => false,
        }
    }

    pub fn handle_event(&mut self, event: WinEvent, global_state: &mut GlobalState) -> bool {
        let cursor_grabbed = global_state.window.is_cursor_grabbed();
        match event {
            WinEvent::Ui(event) => {
                if (self.typing() && event.is_keyboard() && self.show_ui)
                    || !(cursor_grabbed && event.is_keyboard_or_mouse())
                {
                    self.ui.handle_event(event);
                }
                true
            }
            WinEvent::KeyDown(Key::ToggleInterface) => {
                self.toggle_ui();
                true
            }
            _ if !self.show_ui => false,
            WinEvent::Zoom(_) => !cursor_grabbed && !self.ui.no_widget_capturing_mouse(),
            WinEvent::KeyDown(Key::Enter) => {
                self.ui.focus_widget(Some(self.ids.chat));
                true
            }
            WinEvent::KeyDown(Key::Escape) => {
                if self.typing() {
                    self.ui.focus_widget(None);
                } else {
                    // Close windows on esc
                    self.toggle_windows(global_state);
                }
                true
            }
            WinEvent::KeyDown(key) if !self.typing() => match key {
                Key::Map => {
                    self.toggle_map();
                    true
                }
                Key::Bag => {
                    self.toggle_bag();
                    true
                }
                Key::QuestLog => {
                    self.toggle_small(SmallWindowType::QuestLog);
                    true
                }
                Key::CharacterWindow => {
                    self.toggle_charwindow();
                    true
                }
                Key::Social => {
                    self.toggle_small(SmallWindowType::Social);
                    true
                }
                Key::Spellbook => {
                    self.toggle_small(SmallWindowType::Spellbook);
                    true
                }
                Key::Settings => {
                    self.toggle_settings();
                    true
                }
                Key::Help => {
                    self.toggle_help();
                    true
                }
                _ => false,
            },
            WinEvent::KeyDown(key) | WinEvent::KeyUp(key) => match key {
                Key::ToggleCursor => false,
                _ => self.typing(),
            },
            WinEvent::Char(_) => self.typing(),
            WinEvent::SettingsChanged => {
                self.settings = global_state.settings.clone();
                true
            }
            _ => false,
        }
    }

    pub fn maintain(&mut self, renderer: &mut Renderer, tps: f64) -> Vec<Event> {
        let events = self.update_layout(tps);
        self.ui.maintain(renderer);
        events
    }

    pub fn render(&self, renderer: &mut Renderer) {
        self.ui.render(renderer);
    }
}

//Get the text to show in the help window, along with the
//length of the longest line in order to resize the window
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
