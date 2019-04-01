mod chat;

use crate::{
    render::Renderer,
    ui::{ScaleMode, Ui, ToggleButton},
    window::{Event as WinEvent, Window, Key},
};
use conrod_core::{
    color,
    event::Input,
    image::Id as ImgId,
    text::font::Id as FontId,
    widget::{Button, Image, Text, Rectangle},
    widget_ids, Colorable, Labelable, Positionable, Sizeable, Widget,
};

widget_ids! {
    struct Ids {
        //Bag and Inventory
        bag,
        bag_contents,
        bag_close,
        bag_map_open,
        //help
        help,
        help_bg,
        //ESC-Menu
        esc_bg,
        fireplace,
        menu_button_1,
        menu_button_2,
        menu_button_3,
        menu_button_4,
        menu_button_5,
        //Mini-Map
        mmap_frame,
        mmap_frame_bg,
        mmap_button_0,
        mmap_button_1,
        mmap_button_2,
        mmap_button_3,
        mmap_button_4,
        mmap_button_5,
        mmap_icons,
        mmap_location,
        //Action-Bar
        xp_bar,
        l_click,
        r_click,
        health_bar,
        mana_bar,
        sb_grid_l,
        sb_grid_r,
        sb_grid_bg_l,
        sb_grid_bg_r,
        //Window Frames
        window_frame_0,
        window_frame_1,
        window_frame_2,
        window_frame_3,
        window_frame_4,
        window_frame_5,
        //0 Settings-Window
        settings_bg,
        settings_icon,
        settings_button_mo,
        settings_close,
        settings_title,
        //Contents
        button_help,
        button_help2,
        show_help_label,
        interface,
        video,
        sound,
        gameplay,
        controls,
        rectangle,
        //1 Social
        social_frame,
        social_bg,
        social_icon,
        social_close,
        social_title,
        //2 Map
        map_frame,
        map_bg,
        map_icon,
        map_close,
        map_title,
        //3 Spellbook
        spellbook_frame,
        spellbook_bg,
        spellbook_icon,
        spellbook_close,
        spellbook_title,
        //4 Charwindow
        charwindow_frame,
        charwindow_bg,
        charwindow_icon,
        charwindow_close,
        charwindow_title,
        //5 Quest-Log
        questlog_frame,
        questlog_bg,
        questlog_icon,
        questlog_close,
        questlog_title,
    }
}

// TODO: make macro to mimic widget_ids! for images ids or find another solution to simplify addition of new images.
struct Imgs {
    //Missing: ActionBar, Health/Mana/Energy Bar & Char Window BG/Frame
    // Bag
    bag: ImgId,
    bag_hover: ImgId,
    bag_press: ImgId,
    bag_open: ImgId,
    bag_open_hover: ImgId,
    bag_open_press: ImgId,
    bag_contents: ImgId,

    // Close button
    close_button: ImgId,
    close_button_hover: ImgId,
    close_button_press: ImgId,

    // Menu
    esc_bg: ImgId,
    fireplace: ImgId,
    button_dark: ImgId,
    button_dark_hover: ImgId,
    button_dark_press: ImgId,

    // MiniMap
    mmap_frame: ImgId,
    mmap_frame_bg: ImgId,
    mmap_icons: ImgId,

    // Settings at Mini-Map
    mmap_button: ImgId,
    mmap_button_hover: ImgId,
    mmap_button_press: ImgId,
    mmap_button_open: ImgId,

    // SkillBar Module
    sb_grid: ImgId,
    sb_grid_bg: ImgId,
    l_click: ImgId,
    r_click: ImgId,
    mana_bar: ImgId,
    health_bar: ImgId,
    xp_bar: ImgId,

    //Buff Frame(s)
    //buff_frame: ImgId,
    //buff_frame_bg: ImgId,
    //buff_frame_red: ImgId,
    //buff_frame_green: ImgId,

    //Missing: Buff Frame Animation
    window_frame: ImgId,
    window_frame_2: ImgId,
    //Settings-Window
    settings_bg: ImgId,
    settings_icon: ImgId,
    settings_button_mo: ImgId,
    check: ImgId,
    check_mo: ImgId,
    check_press: ImgId,
    check_checked: ImgId,
    check_checked_mo: ImgId,
    slider: ImgId,
    slider_indicator: ImgId,
    button_blank: ImgId,
    button_blue_mo: ImgId,
    button_blue_press: ImgId,
    // Social-Window
    social_bg: ImgId,
    social_icon: ImgId,
    // Map-Window
    map_bg: ImgId,
    map_icon: ImgId,
    map_frame: ImgId,
    // Spell Book Window
    spellbook_bg: ImgId,
    spellbook_icon: ImgId,
    // Char Window
    charwindow_bg: ImgId,
    charwindow_icon: ImgId,
    // Quest-Log Window
    questlog_bg: ImgId,
    questlog_icon: ImgId,
    //help
    //help: ImgId,
    // Chat-Arrow
    chat_arrow: ImgId,
    chat_arrow_mo: ImgId,
    chat_arrow_press: ImgId,
}
impl Imgs {
    fn new(ui: &mut Ui, renderer: &mut Renderer) -> Imgs {
        let mut load = |filename| {
            let image = image::open(
                &[env!("CARGO_MANIFEST_DIR"), "/../assets/voxygen/", filename].concat(),
            )
            .unwrap();
            ui.new_image(renderer, &image).unwrap()
        };
        Imgs {
            // Bag
            bag: load("element/buttons/bag/closed.png"),
            bag_hover: load("element/buttons/bag/closed_hover.png"),
            bag_press: load("element/buttons/bag/closed_press.png"),
            bag_open: load("element/buttons/bag/open.png"),
            bag_open_hover: load("element/buttons/bag/open_hover.png"),
            bag_open_press: load("element/buttons/bag/open_press.png"),
            bag_contents: load("element/frames/bag.png"),

            // Close button
            close_button: load("element/buttons/x.png"),
            close_button_hover: load("element/buttons/x_hover.png"),
            close_button_press: load("element/buttons/x_press.png"),

            // Esc-Menu
            esc_bg: load("element/frames/menu.png"),
            fireplace: load("element/misc_backgrounds/fireplace.png"),
            button_dark: load("element/buttons/button_dark.png"),
            button_dark_hover: load("element/buttons/button_dark_hover.png"),
            button_dark_press: load("element/buttons/button_dark_press.png"),

            // MiniMap
            mmap_frame: load("element/frames/mmap.png"),
            mmap_frame_bg: load("element/misc_backgrounds/mmap_bg.png"),
            mmap_icons: load("element/buttons/mmap_icons.png"),

            // Settings at Mini-Map
            mmap_button: load("element/buttons/border.png"),
            mmap_button_hover: load("element/buttons/border_mo.png"),
            mmap_button_press: load("element/buttons/border_press.png"),
            mmap_button_open: load("element/buttons/border_pressed.png"),

            // Skillbar Module
            sb_grid: load("element/skill_bar/sbar_grid.png"),
            sb_grid_bg: load("element/skill_bar/sbar_grid_bg.png"),
            l_click: load("element/skill_bar/l.png"),
            r_click: load("element/skill_bar/r.png"),
            mana_bar: load("element/skill_bar/mana_bar.png"),
            health_bar: load("element/skill_bar/health_bar.png"),
            xp_bar: load("element/skill_bar/xp_bar.png"),

            //Buff Frame(s)
            //buff_frame: load("element/skill_bar/buff_frame.png"),
            //buff_frame_bg: load("element/skill_bar/buff_frame_bg.png"),
            //buff_frame_red: load("element/skill_bar/buff_frame_red.png"),
            //buff_frame_green: load("element/skill_bar/buff_frame_green.png"),

            //Missing: Buff Frame Animation (.gif ?!) (we could do animation in ui.maintain(), or in shader?)
            window_frame: load("element/frames/window.png"),
            window_frame_2: load("element/frames/window_2.png"),

            //Settings Window
            settings_bg: load("element/frames/settings.png"),
            settings_icon: load("element/icons/settings.png"),
            settings_button_mo: load("element/buttons/blue_mo.png"),
            check: load("element/buttons/check/no.png"),
            check_mo: load("element/buttons/check/no_mo.png"),
            check_press: load("element/buttons/check/press.png"),
            check_checked: load("element/buttons/check/yes.png"),
            check_checked_mo: load("element/buttons/check/yes_mo.png"),
            slider: load("element/slider/track.png"),
            slider_indicator: load("element/slider/indicator.png"),
            button_blank: load("element/nothing.png"),
            button_blue_mo: load("element/buttons/blue_mo.png"),
            button_blue_press: load("element/buttons/blue_press.png"),

            //Social Window
            social_bg: load("element/misc_backgrounds/small_bg.png"),
            social_icon: load("element/icons/social.png"),

            //Map Window
            map_bg: load("element/misc_backgrounds/small_bg.png"),
            map_icon: load("element/icons/map.png"),
            map_frame: load("element/frames/window_map.png"),

            // Spell Book Window
            spellbook_bg: load("element/misc_backgrounds/small_bg.png"),
            spellbook_icon: load("element/icons/spellbook.png"),

            //Char Window
            charwindow_bg: load("element/misc_backgrounds/small_bg.png"),
            charwindow_icon: load("element/icons/charwindow.png"),

            //Quest-Log Window
            questlog_bg: load("element/misc_backgrounds/small_bg.png"),
            questlog_icon: load("element/icons/questlog.png"),

            // Chat-Arrows
            chat_arrow: load("element/buttons/arrow/chat_arrow.png"),
            chat_arrow_mo: load("element/buttons/arrow/chat_arrow_mo.png"),
            chat_arrow_press: load("element/buttons/arrow/chat_arrow_press.png"),
        }
    }
}

enum SettingsTab {
    Interface,
    Video,
    Sound,
    Gameplay,
    Controls,
}

pub enum Event {
    SendMessage(String),
    Logout,
    Quit,
}

// TODO: are these the possible layouts we want?
// TODO: maybe replace this with bitflags
// map not here because it currently is displayed over the top of other open windows
enum Windows {
    Settings, // display settings window
    CharacterAnd(Option<Small>), // show character window + optionally another
    Small(Small),
    None,
}
#[derive(Clone, Copy)]
enum Small {
    Spellbook,
    Social,
    Questlog,
}


pub struct Hud {
    ui: Ui,
    ids: Ids,
    imgs: Imgs,
    chat: chat::Chat,
    typing: bool,
    cursor_grabbed: bool,
    font_metamorph: FontId,
    font_opensans: FontId,
    show_help: bool,
    bag_open: bool,
    menu_open: bool,
    open_windows: Windows,
    map_open: bool,
    settings_tab: SettingsTab
}

impl Hud {
    pub fn new(window: &mut Window) -> Self {
        let mut ui = Ui::new(window).unwrap();
        // TODO: adjust/remove this, right now it is used to demonstrate window scaling functionality
        ui.scaling_mode(ScaleMode::RelativeToWindow([1920.0, 1080.0].into()));
        // Generate ids
        let ids = Ids::new(ui.id_generator());
        // Load images
        let imgs = Imgs::new(&mut ui, window.renderer_mut());
        // Load fonts
        let font_opensans = ui.new_font(
            conrod_core::text::font::from_file(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../assets/voxygen/font/OpenSans-Regular.ttf"
            ))
            .unwrap(),
        );
        let font_metamorph = ui.new_font(
            conrod_core::text::font::from_file(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../assets/voxygen/font/Metamorphous-Regular.ttf"
            ))
            .unwrap(),
        );
        // Chat box
        let chat = chat::Chat::new(&mut ui);
        Self {
            ui,
            imgs,
            ids,
            chat,
            typing: false,
            cursor_grabbed: true,
            settings_tab: SettingsTab::Interface,
            show_help: true,
            bag_open: false,
            menu_open: false,
            map_open: false,
            open_windows: Windows::None,
            font_metamorph,
            font_opensans,
        }
    }

    fn update_layout(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        let ref mut ui_widgets = self.ui.set_widgets();

        // Chat box
        if let Some(msg) = self.chat.update_layout(ui_widgets, self.font_opensans, &self.imgs) {
            events.push(Event::SendMessage(msg));
        }
        // Help Text
        if self.show_help {
            Image::new(self.imgs.window_frame_2)
                .top_left_with_margins_on(ui_widgets.window, 5.0, 5.0)
                .w_h(300.0, 300.0)
                .set(self.ids.help_bg, ui_widgets);

            Text::new(
               "Tab = Free Cursor       \n\
                Esc = Open/Close Menus  \n\
                Q = Back to Login       \n\
                                        \n\
                F1 = Toggle this Window \n\
                F2 = Toggle Interface   \n\
                                        \n\
                M = Map                 \n\
                I = Inventory           \n\
                L = Quest-Log           \n\
                C = Character Window    \n\
                O = Social              \n\
                P = Spellbook           \n\
                N = Settings")
            .rgba(220.0, 220.0, 220.0, 0.8)
            .top_left_with_margins_on(self.ids.help_bg, 20.0,20.0)
            .font_id(self.font_opensans)
            .font_size(18)
            .set(self.ids.help, ui_widgets);
            if Button::image(self.imgs.button_dark)
                .w_h(50.0, 30.0)
                .bottom_right_with_margins_on(self.ids.help_bg, 10.0, 10.0)
                .hover_image(self.imgs.button_dark_hover)
                .press_image(self.imgs.button_dark_press)
                .label("Close")
                .label_font_size(10)
                .label_rgba(220.0, 220.0, 220.0, 0.8)
                .set(self.ids.button_help2, ui_widgets)
                .was_clicked()
            {
                self.show_help = false;
            };
        }

        // Minimap frame and bg
        Image::new(self.imgs.mmap_frame_bg)
            .w_h(1750.0 / 8.0, 1650.0 / 8.0)
            .top_right_with_margins_on(ui_widgets.window, 20.0, 30.0)
            .set(self.ids.mmap_frame_bg, ui_widgets);

        Image::new(self.imgs.mmap_frame)
            .w_h(1750.0 / 8.0, 1650.0 / 8.0)
            .top_right_with_margins_on(ui_widgets.window, 20.0, 30.0)
            .set(self.ids.mmap_frame, ui_widgets);

        Image::new(self.imgs.mmap_icons)
            .w_h(448.0 / 14.93, 2688.0 / 14.93)
            .right_from(self.ids.mmap_frame, 0.0)
            .align_bottom_of(self.ids.mmap_frame)
            .set(self.ids.mmap_icons, ui_widgets);
        // Title
        // TODO Make it display the actual Location
        Text::new("Unknown Location")
            .mid_top_with_margin_on(self.ids.mmap_frame, 5.0)
            .font_size(14)
            .rgba(220.0, 220.0, 220.0, 0.8)
            .set(self.ids.mmap_location, ui_widgets);

        // Minimap Buttons

        //0 Settings
        if Button::image(self.imgs.mmap_button)
            .w_h(448.0 / 15.0, 448.0 / 15.0)
            .top_right_with_margins_on(self.ids.mmap_icons, 0.0, 0.0)
            .hover_image(self.imgs.mmap_button_hover)
            .press_image(self.imgs.mmap_button_press)
            .set(self.ids.mmap_button_0, ui_widgets)
            .was_clicked()
        {
            self.open_windows = match self.open_windows {
                Windows::Settings => Windows::None,
                _ => Windows::Settings,
            };
            self.bag_open = false;
        };
        // 2 Map
        if Button::image(self.imgs.mmap_button)
            .w_h(448.0 / 15.0, 448.0 / 15.0)
            .down_from(self.ids.mmap_button_1, 0.0)
            .hover_image(self.imgs.mmap_button_hover)
            .press_image(self.imgs.mmap_button_press)
            .set(self.ids.mmap_button_2, ui_widgets)
            .was_clicked()
        {
            self.map_open = !self.map_open;
            self.bag_open = false;
        };

        // Other Windows can only be accessed, when Settings are closed. Opening Settings will close all other Windows including the Bag.
        // Opening the Map won't close the windows displayed before.


        if match self.open_windows {
            Windows::Settings => false,
            _ => true,
        } && self.map_open == false {
            //1 Social
            if Button::image(self.imgs.mmap_button)
                .w_h(448.0 / 15.0, 448.0 / 15.0)
                .down_from(self.ids.mmap_button_0, 0.0)
                .hover_image(self.imgs.mmap_button_hover)
                .press_image(self.imgs.mmap_button_press)
                .set(self.ids.mmap_button_1, ui_widgets)
                .was_clicked()
            {
                self.open_windows = match self.open_windows {
                    Windows::Small(Small::Social) => Windows::None,
                    Windows::None | Windows::Small(_) => Windows::Small(Small::Social),
                    Windows::CharacterAnd(small) => match small {
                        Some(Small::Social) => Windows::CharacterAnd(None),
                        _ => Windows::CharacterAnd(Some(Small::Social)),
                    }
                    Windows::Settings => unreachable!(),
                };
            }

            //3 Spellbook
            if Button::image(self.imgs.mmap_button)
                .w_h(448.0 / 15.0, 448.0 / 15.0)
                .down_from(self.ids.mmap_button_2, 0.0)
                .hover_image(self.imgs.mmap_button_hover)
                .press_image(self.imgs.mmap_button_press)
                .set(self.ids.mmap_button_3, ui_widgets)
                .was_clicked()
            {
                self.open_windows = match self.open_windows {
                    Windows::Small(Small::Spellbook) => Windows::None,
                    Windows::None | Windows::Small(_) => Windows::Small(Small::Spellbook),
                    Windows::CharacterAnd(small) => match small {
                        Some(Small::Spellbook) => Windows::CharacterAnd(None),
                        _ => Windows::CharacterAnd(Some(Small::Spellbook)),
                    }
                    Windows::Settings => unreachable!(),
                };
            }
            //4 Char-Window
            if Button::image(self.imgs.mmap_button)
                .w_h(448.0 / 15.0, 448.0 / 15.0)
                .down_from(self.ids.mmap_button_3, 0.0)
                .hover_image(self.imgs.mmap_button_hover)
                .press_image(self.imgs.mmap_button_press)
                .set(self.ids.mmap_button_4, ui_widgets)
                .was_clicked()
            {
                self.open_windows = match self.open_windows {
                    Windows::CharacterAnd(small) => match small {
                        Some(small) => Windows::Small(small),
                        None => Windows::None,
                    },
                    Windows::Small(small) => Windows::CharacterAnd(Some(small)),
                    Windows::None => Windows::CharacterAnd(None),
                    Windows::Settings => unreachable!()
                }
            }
            //5 Quest-Log
            if Button::image(self.imgs.mmap_button)
                .w_h(448.0 / 15.0, 448.0 / 15.0)
                .down_from(self.ids.mmap_button_4, 0.0)
                .hover_image(self.imgs.mmap_button_hover)
                .press_image(self.imgs.mmap_button_press)
                .set(self.ids.mmap_button_5, ui_widgets)
                .was_clicked()
            {
                self.open_windows = match self.open_windows {
                    Windows::Small(Small::Questlog) => Windows::None,
                    Windows::None | Windows::Small(_) => Windows::Small(Small::Questlog),
                    Windows::CharacterAnd(small) => match small {
                        Some(Small::Questlog) => Windows::CharacterAnd(None),
                        _ => Windows::CharacterAnd(Some(Small::Questlog)),
                    }
                    Windows::Settings => unreachable!(),
                };
            }
        }

        // Skillbar Module

        // Experience-Bar
        Image::new(self.imgs.xp_bar)
            .w_h(2688.0 / 4.0, 116.0 / 4.0)
            .mid_bottom_of(ui_widgets.window)
            .set(self.ids.xp_bar, ui_widgets);

        // Left Grid
        Image::new(self.imgs.sb_grid)
            .w_h(2240.0 / 8.0, 448.0 / 8.0)
            .up_from(self.ids.xp_bar, 0.0)
            .align_left_of(self.ids.xp_bar)
            .set(self.ids.sb_grid_l, ui_widgets);

        Image::new(self.imgs.sb_grid_bg)
            .w_h(2240.0 / 8.0, 448.0 / 8.0)
            .middle_of(self.ids.sb_grid_l)
            .set(self.ids.sb_grid_bg_l, ui_widgets);

        // Right Grid
        Image::new(self.imgs.sb_grid)
            .w_h(2240.0 / 8.0, 448.0 / 8.0)
            .up_from(self.ids.xp_bar, 0.0)
            .align_right_of(self.ids.xp_bar)
            .set(self.ids.sb_grid_r, ui_widgets);

        Image::new(self.imgs.sb_grid_bg)
            .w_h(2240.0 / 8.0, 448.0 / 8.0)
            .middle_of(self.ids.sb_grid_r)
            .set(self.ids.sb_grid_bg_r, ui_widgets);

        // Right and Left Click
        Image::new(self.imgs.l_click)
            .w_h(224.0 / 4.0, 320.0 / 4.0)
            .right_from(self.ids.sb_grid_bg_l, 0.0)
            .align_bottom_of(self.ids.sb_grid_bg_l)
            .set(self.ids.l_click, ui_widgets);

        Image::new(self.imgs.r_click)
            .w_h(224.0 / 4.0, 320.0 / 4.0)
            .left_from(self.ids.sb_grid_bg_r, 0.0)
            .align_bottom_of(self.ids.sb_grid_bg_r)
            .set(self.ids.r_click, ui_widgets);

        // Health and mana bars
        Image::new(self.imgs.health_bar)
            .w_h(1120.0 / 4.0, 96.0 / 4.0)
            .left_from(self.ids.l_click, 0.0)
            .align_top_of(self.ids.l_click)
            .set(self.ids.health_bar, ui_widgets);

        Image::new(self.imgs.mana_bar)
            .w_h(1120.0 / 4.0, 96.0 / 4.0)
            .right_from(self.ids.r_click, 0.0)
            .align_top_of(self.ids.r_click)
            .set(self.ids.mana_bar, ui_widgets);

        // Buffs/Debuffs

        // Buffs

        // Debuffs

        // Bag contents
        if self.bag_open {
            // Contents
            Image::new(self.imgs.bag_contents)
            .w_h(1504.0 / 4.0, 1760.0 / 4.0)
            .bottom_right_with_margins_on(ui_widgets.window, 88.0, 68.0)
            .set(self.ids.bag_contents, ui_widgets);

            // X-button
            if Button::image(self.imgs.close_button)
            .w_h(244.0 * 0.22 / 3.0, 244.0 * 0.22 / 3.0)
            .hover_image(self.imgs.close_button_hover)
            .press_image(self.imgs.close_button_press)
            .top_right_with_margins_on(self.ids.bag_contents, 5.0, 17.0)
            .set(self.ids.bag_close, ui_widgets)
            .was_clicked()
            {
                self.bag_open = false;
            }
        }
        // Bag
        if !self.map_open {
            self.bag_open =
                ToggleButton::new(self.bag_open, self.imgs.bag, self.imgs.bag_open)
                    .bottom_right_with_margin_on(ui_widgets.window, 20.0)
                    .hover_images(self.imgs.bag_hover, self.imgs.bag_open_hover)
                    .press_images(self.imgs.bag_press, self.imgs.bag_open_press)
                    .w_h(420.0 / 4.0, 480.0 / 4.0)
                    .set(self.ids.bag, ui_widgets);

        } else {
            Image::new(self.imgs.bag)
                .bottom_right_with_margin_on(ui_widgets.window, 20.0)
                .w_h(420.0 / 4.0, 480.0 / 4.0)
                .set(self.ids.bag_map_open, ui_widgets);
        }

        //Windows

        //Char Window will always appear at the left side. Other Windows either appear at the left side,
        //or when the Char Window is opened they will appear right from it.

        //0 Settings

        if let Windows::Settings = self.open_windows {
            //BG
            Image::new(self.imgs.settings_bg)
                .middle_of(ui_widgets.window)
                .w_h(1648.0 / 2.5, 1952.0 / 2.5)
                .set(self.ids.settings_bg, ui_widgets);
            //X-Button
            if Button::image(self.imgs.close_button)
                .w_h(244.0 * 0.22 / 2.5, 244.0 * 0.22 / 2.5)
                .hover_image(self.imgs.close_button_hover)
                .press_image(self.imgs.close_button_press)
                .top_right_with_margins_on(self.ids.settings_bg, 4.0, 4.0)
                .set(self.ids.settings_close, ui_widgets)
                .was_clicked()
            {
                self.open_windows = Windows::None;
                self.settings_tab = SettingsTab::Interface;
            }

            // Title
            Text::new("Settings")
                .mid_top_with_margin_on(self.ids.settings_bg, 10.0)
                .font_size(30)
                .rgba(220.0, 220.0, 220.0, 0.8)
                .set(self.ids.settings_title, ui_widgets);
            // Icon
            Image::new(self.imgs.settings_icon)
                .w_h(224.0 / 3.0, 224.0 / 3.0)
                .top_left_with_margins_on(self.ids.settings_bg, -10.0, -10.0)
                .set(self.ids.settings_icon, ui_widgets);
            // TODO: Find out if we can remove this
            // Alignment Rectangle
            Rectangle::fill_with([1008.0/ 2.5, 1616.0 / 2.5], color::TRANSPARENT)
                    .top_left_with_margins_on(self.ids.settings_bg, 77.0, 205.0)
                    .set(self.ids.rectangle, ui_widgets);

            //1 Interface////////////////////////////
            if Button::image(if let SettingsTab::Interface = self.settings_tab  {self.imgs.button_blue_mo} else {self.imgs.button_blank})
                .w_h(304.0 / 2.5, 80.0 / 2.5)
                .hover_image(self.imgs.button_blue_mo)
                .press_image(self.imgs.button_blue_press)
                .top_left_with_margins_on(self.ids.settings_bg, 78.0, 50.0)
                .label("Interface")
                .label_font_size(14)
                .label_rgba(220.0, 220.0, 220.0, 0.8)
                .set(self.ids.interface, ui_widgets)
                .was_clicked()
            {
                self.settings_tab = SettingsTab::Interface;
            }
            //Toggle Help
            if let SettingsTab::Interface = self.settings_tab {
                self.show_help = ToggleButton::new(self.show_help, self.imgs.check, self.imgs.check_checked)
                    .w_h(288.0 / 24.0, 288.0 / 24.0)
                    .top_left_with_margins_on(self.ids.rectangle, 15.0, 15.0)
                    .hover_images(self.imgs.check_checked_mo, self.imgs.check_mo)
                    .press_images(self.imgs.check_press, self.imgs.check_press)
                    .set(self.ids.button_help, ui_widgets);
                Text::new("Show Help")
                    .x_relative_to(self.ids.button_help, 55.0)
                    .font_size(12)
                    .graphics_for(self.ids.button_help)
                    .rgba(220.0, 220.0, 220.0, 0.8)
                    .set(self.ids.show_help_label, ui_widgets);
            }
            //2 Gameplay////////////////
            if Button::image(if let SettingsTab::Gameplay = self.settings_tab  {self.imgs.button_blue_mo} else {self.imgs.button_blank})
                .w_h(304.0 / 2.5, 80.0 / 2.5)
                .hover_image(self.imgs.button_blue_mo)
                .press_image(self.imgs.button_blue_press)
                .down_from(self.ids.interface, 1.0)
                .label("Gameplay")
                .label_font_size(14)
                .label_rgba(220.0, 220.0, 220.0, 0.8)
                .set(self.ids.gameplay, ui_widgets)
                .was_clicked()
            {
                self.settings_tab = SettingsTab::Gameplay;
            }

            //3 Controls/////////////////////
            if Button::image(if let SettingsTab::Controls = self.settings_tab  {self.imgs.button_blue_mo} else {self.imgs.button_blank})
                .w_h(304.0 / 2.5, 80.0 / 2.5)
                .hover_image(self.imgs.button_blue_mo)
                .press_image(self.imgs.button_blue_press)
                .down_from(self.ids.gameplay, 1.0)
                .label("Controls")
                .label_font_size(14)
                .label_rgba(220.0, 220.0, 220.0, 0.8)
                .set(self.ids.controls, ui_widgets)
                .was_clicked()
            {
                self.settings_tab = SettingsTab::Controls;
            }

            //4 Video////////////////////////////////
            if Button::image(if let SettingsTab::Video = self.settings_tab  {self.imgs.button_blue_mo} else {self.imgs.button_blank})
                .w_h(304.0 / 2.5, 80.0 / 2.5)
                .hover_image(self.imgs.button_blue_mo)
                .press_image(self.imgs.button_blue_press)
                .down_from(self.ids.controls, 1.0)
                .label("Video")
                .label_font_size(14)
                .label_rgba(220.0, 220.0, 220.0, 0.8)
                .set(self.ids.video, ui_widgets)
                .was_clicked()
            {
                self.settings_tab = SettingsTab::Video;
            }

            //5 Sound///////////////////////////////
            if Button::image(if let SettingsTab::Sound = self.settings_tab  {self.imgs.button_blue_mo} else {self.imgs.button_blank})
                .w_h(304.0 / 2.5, 80.0 / 2.5)
                .hover_image(self.imgs.button_blue_mo)
                .press_image(self.imgs.button_blue_press)
                .down_from(self.ids.video, 1.0)
                .label("Sound")
                .label_font_size(14)
                .label_rgba(220.0, 220.0, 220.0, 0.8)
                .set(self.ids.sound, ui_widgets)
                .was_clicked()
            {
                self.settings_tab = SettingsTab::Sound;
            }
        }
        if let Some((small, char_window_open)) = match self.open_windows {
            Windows::Small(small) => Some((small, false)),
            Windows::CharacterAnd(Some(small)) => Some((small, true)),
            _ => None,
        } {
            // TODO: there is common code in each match arm, might be able to combine this
            match small {
                Small::Social => {
                    //Frame
                    if char_window_open {
                        Image::new(self.imgs.window_frame)
                            .right_from(self.ids.charwindow_frame, 20.0)
                            .w_h(1648.0 / 4.0, 1952.0 / 4.0)
                            .set(self.ids.social_frame, ui_widgets);
                    } else {
                        Image::new(self.imgs.window_frame)
                            .top_left_with_margins_on(ui_widgets.window, 200.0, 90.0)
                            .w_h(1648.0 / 4.0, 1952.0 / 4.0)
                            .set(self.ids.social_frame, ui_widgets);
                    }

                    //BG
                    Image::new(self.imgs.social_bg)
                        .w_h(1648.0 / 4.0, 1952.0 / 4.0)
                        .middle_of(self.ids.social_frame)
                        .set(self.ids.social_bg, ui_widgets);

                    //Icon
                    Image::new(self.imgs.social_icon)
                        .w_h(224.0 / 3.0, 224.0 / 3.0)
                        .top_left_with_margins_on(self.ids.social_frame, -10.0, -10.0)
                        .set(self.ids.social_icon, ui_widgets);

                    //X-Button
                    if Button::image(self.imgs.close_button)
                        .w_h(244.0 * 0.22 / 4.0, 244.0 * 0.22 / 4.0)
                        .hover_image(self.imgs.close_button_hover)
                        .press_image(self.imgs.close_button_press)
                        .top_right_with_margins_on(self.ids.social_frame, 4.0, 4.0)
                        .set(self.ids.social_close, ui_widgets)
                        .was_clicked()
                    {
                        self.open_windows = match self.open_windows {
                            Windows::Small(_) => Windows::None,
                            Windows::CharacterAnd(_) => Windows::CharacterAnd(None),
                            _ => unreachable!(),
                        }
                    }
                    // Title
                    Text::new("Social")
                        .mid_top_with_margin_on(self.ids.social_frame, 7.0)
                        .rgba(220.0, 220.0, 220.0, 0.8)
                        .set(self.ids.social_title, ui_widgets);
                }
                Small::Spellbook => {
                    //Frame
                    if char_window_open {
                        Image::new(self.imgs.window_frame)
                            .right_from(self.ids.charwindow_frame, 20.0)
                            .w_h(1648.0 / 4.0, 1952.0 / 4.0)
                            .set(self.ids.spellbook_frame, ui_widgets);
                    } else {
                        Image::new(self.imgs.window_frame)
                            .top_left_with_margins_on(ui_widgets.window, 200.0, 90.0)
                            .w_h(1648.0 / 4.0, 1952.0 / 4.0)
                            .set(self.ids.spellbook_frame, ui_widgets);
                    }

                    //BG
                    Image::new(self.imgs.spellbook_bg)
                        .w_h(1648.0 / 4.0, 1952.0 / 4.0)
                        .middle_of(self.ids.spellbook_frame)
                        .set(self.ids.spellbook_bg, ui_widgets);

                    //Icon
                    Image::new(self.imgs.spellbook_icon)
                        .w_h(224.0 / 3.0, 224.0 / 3.0)
                        .top_left_with_margins_on(self.ids.spellbook_frame, -10.0, -10.0)
                        .set(self.ids.spellbook_icon, ui_widgets);

                    //X-Button
                    if Button::image(self.imgs.close_button)
                        .w_h(244.0 * 0.22 / 4.0, 244.0 * 0.22 / 4.0)
                        .hover_image(self.imgs.close_button_hover)
                        .press_image(self.imgs.close_button_press)
                        .top_right_with_margins_on(self.ids.spellbook_frame, 4.0, 4.0)
                        .set(self.ids.spellbook_close, ui_widgets)
                        .was_clicked()
                    {
                        self.open_windows = match self.open_windows {
                            Windows::Small(_) => Windows::None,
                            Windows::CharacterAnd(_) => Windows::CharacterAnd(None),
                            _ => unreachable!(),
                        }
                    }
                    // Title
                    Text::new("Spellbook")
                        .mid_top_with_margin_on(self.ids.spellbook_frame, 7.0)
                        .rgba(220.0, 220.0, 220.0, 0.8)
                        .set(self.ids.spellbook_title, ui_widgets);
                }
                Small::Questlog => {
                    //Frame
                    if char_window_open {
                        Image::new(self.imgs.window_frame)
                            .right_from(self.ids.charwindow_frame, 20.0)
                            .w_h(1648.0 / 4.0, 1952.0 / 4.0)
                            .set(self.ids.questlog_frame, ui_widgets);
                    } else {
                        Image::new(self.imgs.window_frame)
                            .top_left_with_margins_on(ui_widgets.window, 200.0, 90.0)
                            .w_h(1648.0 / 4.0, 1952.0 / 4.0)
                            .set(self.ids.questlog_frame, ui_widgets);
                    }

                    //BG
                    Image::new(self.imgs.questlog_bg)
                        .w_h(1648.0 / 4.0, 1952.0 / 4.0)
                        .middle_of(self.ids.questlog_frame)
                        .set(self.ids.questlog_bg, ui_widgets);

                    //Icon
                    Image::new(self.imgs.questlog_icon)
                        .w_h(224.0 / 3.0, 224.0 / 3.0)
                        .top_left_with_margins_on(self.ids.questlog_frame, -10.0, -10.0)
                        .set(self.ids.questlog_icon, ui_widgets);

                    //X-Button
                    if Button::image(self.imgs.close_button)
                        .w_h(244.0 * 0.22 / 4.0, 244.0 * 0.22 / 4.0)
                        .hover_image(self.imgs.close_button_hover)
                        .press_image(self.imgs.close_button_press)
                        .top_right_with_margins_on(self.ids.questlog_frame, 4.0, 4.0)
                        .set(self.ids.questlog_close, ui_widgets)
                        .was_clicked()
                    {
                        self.open_windows = match self.open_windows {
                            Windows::Small(_) => Windows::None,
                            Windows::CharacterAnd(_) => Windows::CharacterAnd(None),
                            _ => unreachable!(),
                        }
                    }
                    // Title
                    Text::new("Quest-Log")
                        .mid_top_with_margin_on(self.ids.questlog_frame, 7.0)
                        .rgba(220.0, 220.0, 220.0, 0.8)
                        .set(self.ids.questlog_title, ui_widgets);
                }
            }


        }

        //4 Char-Window
        if let Windows::CharacterAnd(small) = self.open_windows {
            //Frame
            Image::new(self.imgs.window_frame)
                .top_left_with_margins_on(ui_widgets.window, 200.0, 90.0)
                .w_h(1648.0 / 4.0, 1952.0 / 4.0)
                .set(self.ids.charwindow_frame, ui_widgets);

            //BG
            Image::new(self.imgs.charwindow_bg)
                .w_h(1648.0 / 4.0, 1952.0 / 4.0)
                .middle_of(self.ids.charwindow_frame)
                .set(self.ids.charwindow_bg, ui_widgets);

            //Icon
            Image::new(self.imgs.charwindow_icon)
                .w_h(224.0 / 3.0, 224.0 / 3.0)
                .top_left_with_margins_on(self.ids.charwindow_frame, -10.0, -10.0)
                .set(self.ids.charwindow_icon, ui_widgets);

            //X-Button
            if Button::image(self.imgs.close_button)
                .w_h(244.0 * 0.22 / 4.0, 244.0 * 0.22 / 4.0)
                .hover_image(self.imgs.close_button_hover)
                .press_image(self.imgs.close_button_press)
                .top_right_with_margins_on(self.ids.charwindow_frame, 4.0, 4.0)
                .set(self.ids.charwindow_close, ui_widgets)
                .was_clicked()
            {
                self.open_windows = match small {
                    Some(small) => Windows::Small(small),
                    None => Windows::None,
                }
            }
            // Title
            Text::new("Character Name") //Add in actual Character Name
                .mid_top_with_margin_on(self.ids.charwindow_frame, 7.0)
                .rgba(220.0, 220.0, 220.0, 0.8)
                .set(self.ids.charwindow_title, ui_widgets);
        }

        //2 Map
        if self.map_open {
            //Frame
            Image::new(self.imgs.map_frame)
                .middle_of(ui_widgets.window)
                .w_h(5000.0 / 4.0, 3000.0 / 4.0)
                .set(self.ids.map_frame, ui_widgets);

            //BG
            Image::new(self.imgs.map_bg)
                .w_h(5000.0 / 4.0, 3000.0 / 4.0)
                .middle_of(self.ids.map_frame)
                .set(self.ids.map_bg, ui_widgets);

            //Icon
            Image::new(self.imgs.map_icon)
                .w_h(224.0 / 3.0, 224.0 / 3.0)
                .top_left_with_margins_on(self.ids.map_frame, -10.0, -10.0)
                .set(self.ids.map_icon, ui_widgets);

            //X-Button
            if Button::image(self.imgs.close_button)
                .w_h(244.0 * 0.22 / 1.0, 244.0 * 0.22 / 1.0)
                .hover_image(self.imgs.close_button_hover)
                .press_image(self.imgs.close_button_press)
                .top_right_with_margins_on(self.ids.map_frame, 1.0, 1.0)
                .set(self.ids.map_close, ui_widgets)
                .was_clicked()
            {
                self.map_open = false;
            }
            // Title
            Text::new("Map")
                .mid_top_with_margin_on(self.ids.map_frame, -7.0)
                .font_size(50)
                .rgba(220.0, 220.0, 220.0, 0.8)
                .set(self.ids.map_title, ui_widgets);
        }

        // ESC-MENU
        // Background
        if self.menu_open {
            Image::new(self.imgs.esc_bg)
                .w_h(228.0, 450.0)
                .middle_of(ui_widgets.window)
                .set(self.ids.esc_bg, ui_widgets);

            Image::new(self.imgs.fireplace)
                .w_h(180.0, 60.0)
                .mid_top_with_margin_on(self.ids.esc_bg, 50.0)
                .set(self.ids.fireplace, ui_widgets);

            // Settings
            if Button::image(self.imgs.button_dark)
                .mid_top_with_margin_on(self.ids.esc_bg, 115.0)
                .w_h(170.0, 50.0)
                .hover_image(self.imgs.button_dark_hover)
                .press_image(self.imgs.button_dark_press)
                .label("Settings")
                .label_rgba(220.0, 220.0, 220.0, 0.8)
                .label_font_size(20)
                .set(self.ids.menu_button_1, ui_widgets)
                .was_clicked()
            {
                self.menu_open = false;
                self.open_windows = Windows::Settings;
            };
            // Controls
            if Button::image(self.imgs.button_dark)
                .mid_top_with_margin_on(self.ids.esc_bg, 175.0)
                .w_h(170.0, 50.0)
                .hover_image(self.imgs.button_dark_hover)
                .press_image(self.imgs.button_dark_press)
                .label("Controls")
                .label_rgba(220.0, 220.0, 220.0, 0.8)
                .label_font_size(20)
                .set(self.ids.menu_button_2, ui_widgets)
                .was_clicked()
            {
                //self.menu_open = false;
            };
            // Servers
            if Button::image(self.imgs.button_dark)
                .mid_top_with_margin_on(self.ids.esc_bg, 235.0)
                .w_h(170.0, 50.0)
                .hover_image(self.imgs.button_dark_hover)
                .press_image(self.imgs.button_dark_press)
                .label("Servers")
                .label_rgba(220.0, 220.0, 220.0, 0.8)
                .label_font_size(20)
                .set(self.ids.menu_button_3, ui_widgets)
                .was_clicked()
            {
                //self.menu_open = false;
            };
            // Logout
            if Button::image(self.imgs.button_dark)
                .mid_top_with_margin_on(self.ids.esc_bg, 295.0)
                .w_h(170.0, 50.0)
                .hover_image(self.imgs.button_dark_hover)
                .press_image(self.imgs.button_dark_press)
                .label("Logout")
                .label_rgba(220.0, 220.0, 220.0, 0.8)
                .label_font_size(20)
                .set(self.ids.menu_button_4, ui_widgets)
                .was_clicked()
            {
                events.push(Event::Logout);
            };
            // Quit
            if Button::image(self.imgs.button_dark)
                .mid_top_with_margin_on(self.ids.esc_bg, 355.0)
                .w_h(170.0, 50.0)
                .hover_image(self.imgs.button_dark_hover)
                .press_image(self.imgs.button_dark_press)
                .label("Quit")
                .label_rgba(220.0, 220.0, 220.0, 0.8)
                .label_font_size(20)
                .set(self.ids.menu_button_5, ui_widgets)
                .was_clicked()
            {
                events.push(Event::Quit);
            };
        }
        // update whether keyboard is captured
        self.typing = if let Some(widget_id) = ui_widgets.global_input().current.widget_capturing_keyboard {
            widget_id == self.chat.input_box_id()
        } else {
            false
        };

        events
    }

    pub fn new_message(&mut self, msg: String) {
        self.chat.new_message(msg);
    }

    pub fn toggle_menu(&mut self) {
        self.menu_open = !self.menu_open;
    }

    pub fn update_grab(&mut self, cursor_grabbed: bool) {
        self.cursor_grabbed = cursor_grabbed;
    }

    pub fn handle_event(&mut self, event: WinEvent) -> bool {
        match event {
            WinEvent::Ui(event) => {
                if (self.typing && event.is_keyboard()) || !(self.cursor_grabbed && event.is_keyboard_or_mouse()) {
                    self.ui.handle_event(event);
                }
                true
            },
            WinEvent::KeyDown(Key::Enter) => {
                if self.cursor_grabbed && self.typing {
                    self.ui.focus_widget(None);
                    self.typing = false;
                } else if !self.typing {
                    self.ui.focus_widget(Some(self.chat.input_box_id()));
                    self.typing = true;
                };
                true
            }
            WinEvent::KeyDown(Key::Escape) if self.typing => {
                if self.typing {
                    self.typing = false;
                    self.ui.focus_widget(None);
                    true
                } else {
                    false
                }
            }
            WinEvent::KeyDown(key) | WinEvent::KeyUp(key) => match key {
                Key::ToggleCursor => false,
                _ => self.typing,
            },
            WinEvent::Char(_) => self.typing,
            _ => false,
        }
    }

    pub fn maintain(&mut self, renderer: &mut Renderer) -> Vec<Event> {
        let events = self.update_layout();
        self.ui.maintain(renderer);
        events
    }

    pub fn render(&self, renderer: &mut Renderer) {
        self.ui.render(renderer);
    }
    pub fn toggle_windows(&mut self) {
        if self.bag_open
            || self.menu_open
            || self.map_open
            || match self.open_windows {
                Windows::None => false,
                _ => true,
            }
        {
            self.bag_open = false;
            self.menu_open = false;
            self.map_open = false;
            self.open_windows = Windows::None;
        } else {
            self.menu_open = true;
        }
    }
}
