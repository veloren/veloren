mod chat;

use crate::{
    render::Renderer,
    ui::{self, ScaleMode, ToggleButton, Ui},
    window::{Event as WinEvent, Key, Window},
    GlobalState,
};
use common::{assets, figure::Segment};

use conrod_core::{
    color,
    image::Id as ImgId,
    text::font::Id as FontId,
    widget::{Button, Image, Rectangle, Scrollbar, Text},
    widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, Widget,
};

widget_ids! {
    struct Ids {
        // Test
        bag_space_add,
        inventorytest_button,
        inventorytest_button_label,
        // Debug
        debug_bg,
        debug_button,
        debug_button_label,
        fps_counter,
        // Game Version
        version,

        // Bag and Inventory
        bag,
        bag_contents,
        bag_close,
        bag_map_open,
        inv_alignment,
        inv_grid_1,
        inv_grid_2,
        inv_scrollbar,
        inv_slot_0,
        inv_slot[],

        // Buttons
        settings_button,
        social_button,
        map_button,
        spellbook_button,
        character_button,
        qlog_button,
        social_button_bg,
        spellbook_button_bg,
        character_button_bg,
        qlog_button_bg,
        bag_text,
        mmap_button,
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
        xp_bar_progress,
        health_bar_color,
        mana_bar_color,
        // Level Display
        level_text,
        next_level_text,
        //Window Frames
        window_frame_0,
        window_frame_1,
        window_frame_2,
        window_frame_3,
        window_frame_4,
        window_frame_5,
        //0 Settings-Window
        settings_bg,
        settings_content,
        settings_icon,
        settings_button_mo,
        settings_close,
        settings_title,
        settings_r,
        settings_l,
        settings_scrollbar,
        controls_text,
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
        map_frame_l,
        map_frame_r,
        map_frame_bl,
        map_frame_br,
        //3 Spellbook
        spellbook_frame,
        spellbook_bg,
        spellbook_icon,
        spellbook_close,
        spellbook_title,
        //4 Charwindow
        charwindow_frame,
        charwindow,
        charwindow_bg,
        charwindow_icon,
        charwindow_close,
        charwindow_title,
        charwindow_tab_bg,
        charwindow_tab1,
        charwindow_tab1_title,
        charwindow_tab1_level,
        charwindow_tab1_exp,
        charwindow_tab1_stats,
        charwindow_tab1_statnames,
        charwindow_tab1_stats_numbers,
        charwindow_tab1_expbar,
        charwindow_rectangle,
        charwindow_exp_rectangle,
        charwindow_exp_progress_rectangle,
        //5 Quest-Log
        questlog_frame,
        questlog_bg,
        questlog_icon,
        questlog_close,
        questlog_title,
    }
}

// TODO: make macro to mimic widget_ids! for images ids or find another solution to simplify addition of new images.
pub(self) struct Imgs {
    // Bag
    bag: ImgId,
    bag_hover: ImgId,
    bag_press: ImgId,
    bag_open: ImgId,
    bag_open_hover: ImgId,
    bag_open_press: ImgId,
    bag_contents: ImgId,
    inv_grid: ImgId,
    inv_slot: ImgId,

    // Buttons
    mmap_closed: ImgId,
    mmap_closed_hover: ImgId,
    mmap_closed_press: ImgId,
    mmap_open: ImgId,
    mmap_open_hover: ImgId,
    mmap_open_press: ImgId,

    settings: ImgId,
    settings_hover: ImgId,
    settings_press: ImgId,

    social_button: ImgId,
    social_hover: ImgId,
    social_press: ImgId,

    map_button: ImgId,
    map_hover: ImgId,
    map_press: ImgId,

    spellbook_button: ImgId,
    spellbook_hover: ImgId,
    spellbook_press: ImgId,

    character_button: ImgId,
    character_hover: ImgId,
    character_press: ImgId,

    qlog_button: ImgId,
    qlog_hover: ImgId,
    qlog_press: ImgId,

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
    mmap_frame_closed: ImgId,

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
    settings_frame_r: ImgId,
    settings_frame_l: ImgId,
    settings_button: ImgId,
    settings_button_pressed: ImgId,
    settings_button_hover: ImgId,
    settings_button_press: ImgId,
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
    window_bg: ImgId,
    // Social-Window
    social_bg: ImgId,
    social_icon: ImgId,
    // Map-Window
    map_bg: ImgId,
    map_icon: ImgId,
    map_frame_l: ImgId,
    map_frame_r: ImgId,
    map_frame_bl: ImgId,
    map_frame_br: ImgId,
    // Spell Book Window
    spellbook_bg: ImgId,
    spellbook_icon: ImgId,
    // Char Window
    charwindow: ImgId,
    charwindow_icon: ImgId,
    charwindow_tab_bg: ImgId,
    charwindow_tab: ImgId,
    charwindow_expbar: ImgId,
    progress_frame: ImgId,
    progress: ImgId,

    // Buttons
    grid_button: ImgId,
    grid_button_hover: ImgId,
    grid_button_press: ImgId,
    grid_button_open: ImgId,

    // Quest-Log Window
    questlog_bg: ImgId,
    questlog_icon: ImgId,
    //help
    // Chat-Arrow
    chat_arrow: ImgId,
    chat_arrow_mo: ImgId,
    chat_arrow_press: ImgId,
}
impl Imgs {
    fn new(ui: &mut Ui, renderer: &mut Renderer) -> Imgs {
        let mut load_img = |filename, ui: &mut Ui| {
            let fullpath: String = ["/voxygen/", filename].concat();
            let image = image::load_from_memory(
                assets::load(fullpath.as_str())
                    .expect("Error loading Main UI Image")
                    .as_slice(),
            )
            .unwrap();
            ui.new_graphic(ui::Graphic::Image(image))
        };
        let mut load_vox = |filename, ui: &mut Ui| {
            let fullpath: String = ["/voxygen/", filename].concat();
            let dot_vox = dot_vox::load_bytes(
                assets::load(fullpath.as_str())
                    .expect("Error loading Main UI .vox")
                    .as_slice(),
            )
            .unwrap();
            ui.new_graphic(ui::Graphic::Voxel(Segment::from(dot_vox)))
        };
        Imgs {
            // Bag
            bag: load_img("element/buttons/bag/closed.png", ui),
            bag_hover: load_img("element/buttons/bag/closed_hover.png", ui),
            bag_press: load_img("element/buttons/bag/closed_press.png", ui),
            bag_open: load_img("element/buttons/bag/open.png", ui),
            bag_open_hover: load_img("element/buttons/bag/open_hover.png", ui),
            bag_open_press: load_img("element/buttons/bag/open_press.png", ui),
            bag_contents: load_vox("element/frames/bag.vox", ui),
            inv_grid: load_vox("element/frames/inv_grid.vox", ui),
            inv_slot: load_vox("element/buttons/inv_slot.vox", ui),

            // Buttons
            mmap_closed: load_vox("element/buttons/button_mmap_closed.vox", ui),
            mmap_closed_hover: load_vox("element/buttons/button_mmap_closed_hover.vox", ui),
            mmap_closed_press: load_vox("element/buttons/button_mmap_closed_press.vox", ui),
            mmap_open: load_vox("element/buttons/button_mmap_open.vox", ui),
            mmap_open_hover: load_vox("element/buttons/button_mmap_open_hover.vox", ui),
            mmap_open_press: load_vox("element/buttons/button_mmap_open_press.vox", ui),

            settings: load_vox("element/buttons/settings.vox", ui),
            settings_hover: load_vox("element/buttons/settings_hover.vox", ui),
            settings_press: load_vox("element/buttons/settings_press.vox", ui),

            social_button: load_vox("element/buttons/social.vox", ui),
            social_hover: load_vox("element/buttons/social_hover.vox", ui),
            social_press: load_vox("element/buttons/social_press.vox", ui),

            map_button: load_vox("element/buttons/map.vox", ui),
            map_hover: load_vox("element/buttons/map_hover.vox", ui),
            map_press: load_vox("element/buttons/map_press.vox", ui),

            spellbook_button: load_vox("element/buttons/spellbook.vox", ui),
            spellbook_hover: load_vox("element/buttons/spellbook_hover.vox", ui),
            spellbook_press: load_vox("element/buttons/spellbook_press.vox", ui),

            character_button: load_vox("element/buttons/character.vox", ui),
            character_hover: load_vox("element/buttons/character_hover.vox", ui),
            character_press: load_vox("element/buttons/character_press.vox", ui),

            qlog_button: load_vox("element/buttons/qlog.vox", ui),
            qlog_hover: load_vox("element/buttons/qlog_hover.vox", ui),
            qlog_press: load_vox("element/buttons/qlog_press.vox", ui),

            grid_button: load_img("element/buttons/border.png", ui),
            grid_button_hover: load_img("element/buttons/border_mo.png", ui),
            grid_button_press: load_img("element/buttons/border_press.png", ui),
            grid_button_open: load_img("element/buttons/border_pressed.png", ui),

            // Close button
            close_button: load_vox("element/buttons/x.vox", ui),
            close_button_hover: load_vox("element/buttons/x_hover.vox", ui),
            close_button_press: load_vox("element/buttons/x_press.vox", ui),

            // Esc-Menu
            esc_bg: load_img("element/frames/menu.png", ui),
            fireplace: load_vox("element/misc_bg/fireplace.vox", ui),
            button_dark: load_vox("element/buttons/button_dark.vox", ui),
            button_dark_hover: load_img("element/buttons/button_dark_hover.png", ui),
            button_dark_press: load_img("element/buttons/button_dark_press.png", ui),

            // MiniMap
            mmap_frame: load_vox("element/frames/mmap.vox", ui),
            mmap_frame_closed: load_vox("element/frames/mmap_closed.vox", ui),

            // Skillbar Module
            sb_grid: load_img("element/skill_bar/sbar_grid.png", ui),
            sb_grid_bg: load_img("element/skill_bar/sbar_grid_bg.png", ui),
            l_click: load_img("element/skill_bar/l.png", ui),
            r_click: load_img("element/skill_bar/r.png", ui),
            mana_bar: load_img("element/skill_bar/mana_bar.png", ui),
            health_bar: load_img("element/skill_bar/health_bar.png", ui),
            xp_bar: load_img("element/skill_bar/xp_bar.png", ui),

            // Missing: Buff Frame Animation (.gif ?!) (we could do animation in ui.maintain(), or in shader?)
            window_frame: load_vox("element/frames/window2.vox", ui),
            window_frame_2: load_img("element/frames/window_2.png", ui),

            // Settings Window
            settings_frame_r: load_vox("element/frames/settings_r.vox", ui),
            settings_frame_l: load_vox("element/frames/settings_l.vox", ui),
            settings_button: load_vox("element/buttons/settings_button.vox", ui),
            settings_button_pressed: load_vox("element/buttons/settings_button_pressed.vox", ui),
            settings_button_hover: load_vox("element/buttons/settings_button_hover.vox", ui),
            settings_button_press: load_vox("element/buttons/settings_button_press.vox", ui),
            settings_bg: load_img("element/frames/settings.png", ui),
            settings_icon: load_img("element/icons/settings.png", ui),
            settings_button_mo: load_img("element/buttons/blue_mo.png", ui),
            check: load_img("element/buttons/check/no.png", ui),
            check_mo: load_img("element/buttons/check/no_mo.png", ui),
            check_press: load_img("element/buttons/check/press.png", ui),
            check_checked: load_img("element/buttons/check/yes.png", ui),
            check_checked_mo: load_img("element/buttons/check/yes_mo.png", ui),
            slider: load_img("element/slider/track.png", ui),
            slider_indicator: load_img("element/slider/indicator.png", ui),
            button_blank: ui.new_graphic(ui::Graphic::Blank),
            button_blue_mo: load_img("element/buttons/blue_mo.png", ui),
            button_blue_press: load_img("element/buttons/blue_press.png", ui),

            // Window BG
            window_bg: load_img("element/misc_bg/window_bg.png", ui),

            // Social Window
            social_bg: load_img("element/misc_bg/small_bg.png", ui),
            social_icon: load_img("element/icons/social.png", ui),

            // Map Window
            map_bg: load_img("element/misc_bg/small_bg.png", ui),
            map_icon: load_img("element/icons/map.png", ui),
            map_frame_l: load_vox("element/frames/map_l.vox", ui),
            map_frame_r: load_vox("element/frames/map_r.vox", ui),
            map_frame_bl: load_vox("element/frames/map_bl.vox", ui),
            map_frame_br: load_vox("element/frames/map_br.vox", ui),

            // Spell Book Window
            spellbook_bg: load_img("element/misc_bg/small_bg.png", ui),
            spellbook_icon: load_img("element/icons/spellbook.png", ui),

            // Char Window
            charwindow: load_img("element/misc_bg/charwindow.png", ui),
            charwindow_icon: load_img("element/icons/charwindow.png", ui),
            charwindow_tab_bg: load_img("element/frames/tab.png", ui),
            charwindow_tab: load_img("element/buttons/tab.png", ui),
            charwindow_expbar: load_img("element/misc_bg/small_bg.png", ui),
            progress_frame: load_img("element/frames/progress_bar.png", ui),
            progress: load_img("element/misc_bg/progress.png", ui),

            // Quest-Log Window
            questlog_bg: load_img("element/misc_bg/small_bg.png", ui),
            questlog_icon: load_img("element/icons/questlog.png", ui),

            // Chat-Arrows
            chat_arrow: load_img("element/buttons/arrow/chat_arrow.png", ui),
            chat_arrow_mo: load_img("element/buttons/arrow/chat_arrow_mo.png", ui),
            chat_arrow_press: load_img("element/buttons/arrow/chat_arrow_press.png", ui),
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
    Settings,                    // display settings window
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
    font_metamorph: FontId,
    font_opensans: FontId,
    show_help: bool,
    show_debug: bool,
    bag_open: bool,
    menu_open: bool,
    open_windows: Windows,
    map_open: bool,
    mmap_open: bool,
    show_ui: bool,
    inventory_space: u32,
    xp_percentage: f64,
    hp_percentage: f64,
    mana_percentage: f64,
    inventorytest_button: bool,
    settings_tab: SettingsTab,
}

//#[inline]
//pub fn rgba_bytes(r: u8, g: u8, b: u8, a: f32) -> Color {
//Color::Rgba(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a)
//}

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
        let load_font = |filename, ui: &mut Ui| {
            let fullpath: String = ["/voxygen/font", filename].concat();
            ui.new_font(
                conrod_core::text::Font::from_bytes(
                    assets::load(fullpath.as_str()).expect("Error loading file"),
                )
                .unwrap(),
            )
        };
        let font_opensans = load_font("/OpenSans-Regular.ttf", &mut ui);
        let font_metamorph = load_font("/Metamorphous-Regular.ttf", &mut ui);
        // Chat box
        let chat = chat::Chat::new(&mut ui);

        Self {
            ui,
            imgs,
            ids,
            chat,
            settings_tab: SettingsTab::Interface,
            show_help: false,
            show_debug: true,
            bag_open: false,
            menu_open: false,
            map_open: false,
            mmap_open: false,
            show_ui: true,
            inventorytest_button: false,
            inventory_space: 0,
            open_windows: Windows::None,
            font_metamorph,
            font_opensans,
            xp_percentage: 0.4,
            hp_percentage: 1.0,
            mana_percentage: 1.0,
        }
    }

    fn update_layout(&mut self, tps: f64) -> Vec<Event> {
        let mut events = Vec::new();
        let ref mut ui_widgets = self.ui.set_widgets();
        let version = env!("CARGO_PKG_VERSION");

        const TEXT_COLOR: Color = Color::Rgba(1.0, 1.0, 1.0, 1.0);
        const HP_COLOR: Color = Color::Rgba(0.33, 0.63, 0.0, 1.0);
        const MANA_COLOR: Color = Color::Rgba(0.42, 0.41, 0.66, 1.0);
        const XP_COLOR: Color = Color::Rgba(0.59, 0.41, 0.67, 1.0);

        // Don't show anything if the ui is toggled off
        if !self.show_ui {
            return events;
        }

        // Display debug window
        if self.show_debug {
            // Alpha Version
            Text::new(version)
                .top_left_with_margins_on(ui_widgets.window, 5.0, 5.0)
                .font_size(14)
                .font_id(self.font_opensans)
                .color(TEXT_COLOR)
                .set(self.ids.version, ui_widgets);
            Text::new(&format!("FPS: {:.1}", tps))
                .color(TEXT_COLOR)
                .down_from(self.ids.version, 5.0)
                .font_id(self.font_opensans)
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
                self.inventory_space = self.inventory_space + 1;
            };
        }

        // Chat box
        if let Some(msg) = self
            .chat
            .update_layout(ui_widgets, self.font_opensans, &self.imgs)
        {
            events.push(Event::SendMessage(msg));
        }

        // Help Text
        if self.show_help {
            Image::new(self.imgs.window_frame_2)
                .top_left_with_margins_on(ui_widgets.window, 3.0, 3.0)
                .w_h(300.0, 190.0)
                .set(self.ids.help_bg, ui_widgets);

            Text::new(
                "Tab = Free Cursor       \n\
                 Esc = Open/Close Menus  \n\
                 \n\
                 F1 = Toggle this Window \n\
                 F2 = Toggle Interface   \n\
                 \n\
                 Enter = Open Chat       \n\
                 Mouse Wheel = Scroll Chat",
            )
            .color(TEXT_COLOR)
            .top_left_with_margins_on(self.ids.help_bg, 20.0, 20.0)
            .font_id(self.font_opensans)
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

        // Buttons at Bag

        // 0 Settings
        if Button::image(self.imgs.settings)
            .w_h(29.0, 25.0)
            .bottom_right_with_margins_on(ui_widgets.window, 5.0, 57.0)
            .hover_image(self.imgs.settings_hover)
            .press_image(self.imgs.settings_press)
            .label("N")
            .label_font_size(10)
            .label_font_id(self.font_metamorph)
            .color(TEXT_COLOR)
            .label_color(TEXT_COLOR)
            .label_y(conrod_core::position::Relative::Scalar(-7.0))
            .label_x(conrod_core::position::Relative::Scalar(10.0))
            .set(self.ids.settings_button, ui_widgets)
            .was_clicked()
        {
            self.open_windows = match self.open_windows {
                Windows::Settings => Windows::None,
                _ => Windows::Settings,
            };
            self.bag_open = false;
        };

        // 2 Map
        if Button::image(self.imgs.map_button)
            .w_h(22.0, 25.0)
            .left_from(self.ids.social_button, 10.0)
            .hover_image(self.imgs.map_hover)
            .press_image(self.imgs.map_press)
            .label("M")
            .label_font_size(10)
            .label_font_id(self.font_metamorph)
            .label_color(TEXT_COLOR)
            .label_y(conrod_core::position::Relative::Scalar(-7.0))
            .label_x(conrod_core::position::Relative::Scalar(10.0))
            .set(self.ids.map_button, ui_widgets)
            .was_clicked()
        {
            self.map_open = !self.map_open;
            self.bag_open = false;
        };

        // Other Windows can only be accessed, when Settings are closed.
        // Opening Settings will close all other Windows including the Bag.
        // Opening the Map won't close the windows displayed before.
        Image::new(self.imgs.social_button)
            .w_h(25.0, 25.0)
            .left_from(self.ids.settings_button, 10.0)
            .set(self.ids.social_button_bg, ui_widgets);
        Image::new(self.imgs.spellbook_button)
            .w_h(28.0, 25.0)
            .left_from(self.ids.map_button, 10.0)
            .set(self.ids.spellbook_button_bg, ui_widgets);
        Image::new(self.imgs.character_button)
            .w_h(27.0, 25.0)
            .left_from(self.ids.spellbook_button, 10.0)
            .set(self.ids.character_button_bg, ui_widgets);
        Image::new(self.imgs.qlog_button)
            .w_h(23.0, 25.0)
            .left_from(self.ids.character_button, 10.0)
            .set(self.ids.qlog_button_bg, ui_widgets);

        if match self.open_windows {
            Windows::Settings => false,
            _ => true,
        } && self.map_open == false
        {
            // 1 Social
            if Button::image(self.imgs.social_button)
                .w_h(25.0, 25.0)
                .left_from(self.ids.settings_button, 10.0)
                .hover_image(self.imgs.social_hover)
                .press_image(self.imgs.social_press)
                .label("O")
                .label_font_size(10)
                .label_font_id(self.font_metamorph)
                .label_color(TEXT_COLOR)
                .label_y(conrod_core::position::Relative::Scalar(-7.0))
                .label_x(conrod_core::position::Relative::Scalar(10.0))
                .set(self.ids.social_button, ui_widgets)
                .was_clicked()
            {
                self.open_windows = match self.open_windows {
                    Windows::Small(Small::Social) => Windows::None,
                    Windows::None | Windows::Small(_) => Windows::Small(Small::Social),
                    Windows::CharacterAnd(small) => match small {
                        Some(Small::Social) => Windows::CharacterAnd(None),
                        _ => Windows::CharacterAnd(Some(Small::Social)),
                    },
                    Windows::Settings => Windows::Settings,
                };
            }

            // 3 Spellbook
            if Button::image(self.imgs.spellbook_button)
                .w_h(28.0, 25.0)
                .left_from(self.ids.map_button, 10.0)
                .hover_image(self.imgs.spellbook_hover)
                .press_image(self.imgs.spellbook_press)
                .label("P")
                .label_font_size(10)
                .label_font_id(self.font_metamorph)
                .label_color(TEXT_COLOR)
                .label_y(conrod_core::position::Relative::Scalar(-7.0))
                .label_x(conrod_core::position::Relative::Scalar(10.0))
                .set(self.ids.spellbook_button, ui_widgets)
                .was_clicked()
            {
                self.open_windows = match self.open_windows {
                    Windows::Small(Small::Spellbook) => Windows::None,
                    Windows::None | Windows::Small(_) => Windows::Small(Small::Spellbook),
                    Windows::CharacterAnd(small) => match small {
                        Some(Small::Spellbook) => Windows::CharacterAnd(None),
                        _ => Windows::CharacterAnd(Some(Small::Spellbook)),
                    },
                    Windows::Settings => Windows::Settings,
                };
            }

            // 4 Char-Window
            if Button::image(self.imgs.character_button)
                .w_h(27.0, 25.0)
                .left_from(self.ids.spellbook_button, 10.0)
                .hover_image(self.imgs.character_hover)
                .press_image(self.imgs.character_press)
                .label("C")
                .label_font_size(10)
                .label_font_id(self.font_metamorph)
                .label_color(TEXT_COLOR)
                .label_y(conrod_core::position::Relative::Scalar(-7.0))
                .label_x(conrod_core::position::Relative::Scalar(10.0))
                .set(self.ids.character_button, ui_widgets)
                .was_clicked()
            {
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

            // 5 Quest-Log
            if Button::image(self.imgs.qlog_button)
                .w_h(23.0, 25.0)
                .left_from(self.ids.character_button, 10.0)
                .hover_image(self.imgs.qlog_hover)
                .press_image(self.imgs.qlog_press)
                .label("L")
                .label_font_size(10)
                .label_font_id(self.font_metamorph)
                .label_color(TEXT_COLOR)
                .label_y(conrod_core::position::Relative::Scalar(-7.0))
                .label_x(conrod_core::position::Relative::Scalar(10.0))
                .set(self.ids.qlog_button, ui_widgets)
                .was_clicked()
            {
                self.open_windows = match self.open_windows {
                    Windows::Small(Small::Questlog) => Windows::None,
                    Windows::None | Windows::Small(_) => Windows::Small(Small::Questlog),
                    Windows::CharacterAnd(small) => match small {
                        Some(Small::Questlog) => Windows::CharacterAnd(None),
                        _ => Windows::CharacterAnd(Some(Small::Questlog)),
                    },
                    Windows::Settings => Windows::Settings,
                };
            }
        }

        // Skillbar Module

        // Experience-Bar
        Image::new(self.imgs.xp_bar)
            .w_h(2688.0 / 6.0, 116.0 / 6.0)
            .mid_bottom_of(ui_widgets.window)
            .set(self.ids.xp_bar, ui_widgets);

        Rectangle::fill_with([406.0 * (self.xp_percentage), 5.0], XP_COLOR) // "W=406*[Exp. %]"
            .top_left_with_margins_on(self.ids.xp_bar, 5.0, 21.0)
            .set(self.ids.xp_bar_progress, ui_widgets);

        // Left Grid
        Image::new(self.imgs.sb_grid)
            .w_h(2240.0 / 12.0, 448.0 / 12.0)
            .up_from(self.ids.xp_bar, 0.0)
            .align_left_of(self.ids.xp_bar)
            .set(self.ids.sb_grid_l, ui_widgets);

        Image::new(self.imgs.sb_grid_bg)
            .w_h(2240.0 / 12.0, 448.0 / 12.0)
            .middle_of(self.ids.sb_grid_l)
            .set(self.ids.sb_grid_bg_l, ui_widgets);

        // Right Grid
        Image::new(self.imgs.sb_grid)
            .w_h(2240.0 / 12.0, 448.0 / 12.0)
            .up_from(self.ids.xp_bar, 0.0)
            .align_right_of(self.ids.xp_bar)
            .set(self.ids.sb_grid_r, ui_widgets);

        Image::new(self.imgs.sb_grid_bg)
            .w_h(2240.0 / 12.0, 448.0 / 12.0)
            .middle_of(self.ids.sb_grid_r)
            .set(self.ids.sb_grid_bg_r, ui_widgets);

        // Right and Left Click
        Image::new(self.imgs.l_click)
            .w_h(224.0 / 6.0, 320.0 / 6.0)
            .right_from(self.ids.sb_grid_bg_l, 0.0)
            .align_bottom_of(self.ids.sb_grid_bg_l)
            .set(self.ids.l_click, ui_widgets);

        Image::new(self.imgs.r_click)
            .w_h(224.0 / 6.0, 320.0 / 6.0)
            .left_from(self.ids.sb_grid_bg_r, 0.0)
            .align_bottom_of(self.ids.sb_grid_bg_r)
            .set(self.ids.r_click, ui_widgets);

        // Health Bar
        Image::new(self.imgs.health_bar)
            .w_h(1120.0 / 6.0, 96.0 / 6.0)
            .left_from(self.ids.l_click, 0.0)
            .align_top_of(self.ids.l_click)
            .set(self.ids.health_bar, ui_widgets);

        // Filling
        Rectangle::fill_with([182.0 * (self.hp_percentage), 6.0], HP_COLOR) // "W=182.0 * [Health. %]"
            .top_right_with_margins_on(self.ids.health_bar, 5.0, 0.0)
            .set(self.ids.health_bar_color, ui_widgets);

        // Mana Bar
        Image::new(self.imgs.mana_bar)
            .w_h(1120.0 / 6.0, 96.0 / 6.0)
            .right_from(self.ids.r_click, 0.0)
            .align_top_of(self.ids.r_click)
            .set(self.ids.mana_bar, ui_widgets);

        // Filling
        Rectangle::fill_with([182.0 * (self.mana_percentage), 6.0], MANA_COLOR) // "W=182.0 * [Mana. %]"
            .top_left_with_margins_on(self.ids.mana_bar, 5.0, 0.0)
            .set(self.ids.mana_bar_color, ui_widgets);

        // Buffs/Debuffs

        // Buffs

        // Debuffs

        // Level Display

        // Insert actual Level here
        Text::new("1")
            .left_from(self.ids.xp_bar, -15.0)
            .font_size(10)
            .color(TEXT_COLOR)
            .set(self.ids.level_text, ui_widgets);

        // Insert next Level here
        Text::new("2")
            .right_from(self.ids.xp_bar, -15.0)
            .font_size(10)
            .color(TEXT_COLOR)
            .set(self.ids.next_level_text, ui_widgets);

        // Bag contents
        if self.bag_open {
            // Contents
            Image::new(self.imgs.bag_contents)
                .w_h(68.0 * 4.0, 123.0 * 4.0)
                .bottom_right_with_margins_on(ui_widgets.window, 60.0, 5.0)
                .set(self.ids.bag_contents, ui_widgets);

            // Alignment for Grid
            Rectangle::fill_with([58.0 * 4.0 - 5.0, 100.0 * 4.0], color::TRANSPARENT)
                .top_left_with_margins_on(self.ids.bag_contents, 11.0 * 4.0, 5.0 * 4.0)
                .scroll_kids()
                .scroll_kids_vertically()
                .set(self.ids.inv_alignment, ui_widgets);
            // Grid
            Image::new(self.imgs.inv_grid)
                .w_h(58.0 * 4.0, 111.0 * 4.0)
                .mid_top_with_margin_on(self.ids.inv_alignment, 0.0)
                .set(self.ids.inv_grid_1, ui_widgets);
            Image::new(self.imgs.inv_grid)
                .w_h(58.0 * 4.0, 111.0 * 4.0)
                .mid_top_with_margin_on(self.ids.inv_alignment, 110.0 * 4.0)
                .set(self.ids.inv_grid_2, ui_widgets);
            Scrollbar::y_axis(self.ids.inv_alignment)
                .thickness(5.0)
                .rgba(0.33, 0.33, 0.33, 1.0)
                .set(self.ids.inv_scrollbar, ui_widgets);

            // X-button
            if Button::image(self.imgs.close_button)
                .w_h(28.0, 28.0)
                .hover_image(self.imgs.close_button_hover)
                .press_image(self.imgs.close_button_press)
                .top_right_with_margins_on(self.ids.bag_contents, 0.0, 0.0)
                .set(self.ids.bag_close, ui_widgets)
                .was_clicked()
            {
                self.bag_open = false;
            }

            if self.inventory_space > 0 {
                // First Slot
                Button::image(self.imgs.inv_slot)
                    .top_left_with_margins_on(self.ids.inv_grid_1, 4.0, 4.0)
                    .w_h(10.0 * 4.0, 10.0 * 4.0)
                    .set(self.ids.inv_slot_0, ui_widgets);
            }
        }

        // Bag
        if !self.map_open && self.show_ui {
            self.bag_open = ToggleButton::new(self.bag_open, self.imgs.bag, self.imgs.bag_open)
                .bottom_right_with_margins_on(ui_widgets.window, 5.0, 5.0)
                .hover_images(self.imgs.bag_hover, self.imgs.bag_open_hover)
                .press_images(self.imgs.bag_press, self.imgs.bag_open_press)
                .w_h(420.0 / 10.0, 480.0 / 10.0)
                .set(self.ids.bag, ui_widgets);
            Text::new("B")
                .bottom_right_with_margins_on(self.ids.bag, 0.0, 0.0)
                .font_size(10)
                .font_id(self.font_metamorph)
                .color(TEXT_COLOR)
                .set(self.ids.bag_text, ui_widgets);
        } else {
            Image::new(self.imgs.bag)
                .bottom_right_with_margins_on(ui_widgets.window, 5.0, 5.0)
                .w_h(420.0 / 10.0, 480.0 / 10.0)
                .set(self.ids.bag_map_open, ui_widgets);
            Text::new("B")
                .bottom_right_with_margins_on(self.ids.bag, 0.0, 0.0)
                .font_size(10)
                .font_id(self.font_metamorph)
                .set(self.ids.bag_text, ui_widgets);
        }

        //Windows

        //Char Window will always appear at the left side. Other Windows either appear at the left side,
        //or when the Char Window is opened they will appear right from it.

        // 0 Settings

        if let Windows::Settings = self.open_windows {
            // Frame Alignment
            Rectangle::fill_with([824.0, 488.0], color::TRANSPARENT)
                .middle_of(ui_widgets.window)
                .set(self.ids.settings_bg, ui_widgets);
            // Frame
            Image::new(self.imgs.settings_frame_l)
                .top_left_with_margins_on(self.ids.settings_bg, 0.0, 0.0)
                .w_h(412.0, 488.0)
                .set(self.ids.settings_l, ui_widgets);
            Image::new(self.imgs.settings_frame_r)
                .right_from(self.ids.settings_l, 0.0)
                .parent(self.ids.settings_bg)
                .w_h(412.0, 488.0)
                .set(self.ids.settings_r, ui_widgets);
            // Content Alignment
            Rectangle::fill_with([198.0 * 4.0, 97.0 * 4.0], color::TRANSPARENT)
                .top_right_with_margins_on(self.ids.settings_r, 21.0 * 4.0, 4.0 * 4.0)
                .scroll_kids()
                .scroll_kids_vertically()
                .set(self.ids.settings_content, ui_widgets);
            Scrollbar::y_axis(self.ids.settings_content)
                .thickness(5.0)
                .rgba(0.33, 0.33, 0.33, 1.0)                
                .set(self.ids.settings_scrollbar, ui_widgets);
            // X-Button
            if Button::image(self.imgs.close_button)
                .w_h(28.0, 28.0)
                .hover_image(self.imgs.close_button_hover)
                .press_image(self.imgs.close_button_press)
                .top_right_with_margins_on(self.ids.settings_r, 0.0, 0.0)
                .set(self.ids.settings_close, ui_widgets)
                .was_clicked()
            {
                self.open_windows = Windows::None;
                self.settings_tab = SettingsTab::Interface;
            }

            // Title
            Text::new("Settings")
                .mid_top_with_margin_on(self.ids.settings_bg, 5.0)
                .font_size(14)
                .color(TEXT_COLOR)
                .set(self.ids.settings_title, ui_widgets);
            // Icon
            //Image::new(self.imgs.settings_icon)
            //.w_h(224.0 / 3.0, 224.0 / 3.0)
            //.top_left_with_margins_on(self.ids.settings_bg, -10.0, -10.0)
            //.set(self.ids.settings_icon, ui_widgets);
            // TODO: Find out if we can remove this

            // 1 Interface////////////////////////////
            if Button::image(if let SettingsTab::Interface = self.settings_tab {
                self.imgs.settings_button_pressed
            } else {
                self.imgs.settings_button
            })
            .w_h(31.0 * 4.0, 12.0 * 4.0)
            .hover_image(if let SettingsTab::Interface = self.settings_tab {
                self.imgs.settings_button_pressed
            } else {
                self.imgs.settings_button_hover
            })
            .press_image(if let SettingsTab::Interface = self.settings_tab {
                self.imgs.settings_button_pressed
            } else {
                self.imgs.settings_button_press
            })
            .top_left_with_margins_on(self.ids.settings_l, 8.0 * 4.0, 2.0 * 4.0)
            .label("Interface")
            .label_font_size(14)
            .label_color(TEXT_COLOR)
            .set(self.ids.interface, ui_widgets)
            .was_clicked()
            {
                self.settings_tab = SettingsTab::Interface;
            }
            // Toggle Help
            if let SettingsTab::Interface = self.settings_tab {
                self.show_help =
                    ToggleButton::new(self.show_help, self.imgs.check, self.imgs.check_checked)
                        .w_h(288.0 / 24.0, 288.0 / 24.0)
                        .top_left_with_margins_on(self.ids.settings_content, 5.0, 5.0)
                        .hover_images(self.imgs.check_checked_mo, self.imgs.check_mo)
                        .press_images(self.imgs.check_press, self.imgs.check_press)
                        .set(self.ids.button_help, ui_widgets);
                Text::new("Show Help")
                    .right_from(self.ids.button_help, 10.0)
                    .font_size(14)
                    .font_id(self.font_opensans)
                    .graphics_for(self.ids.button_help)
                    .color(TEXT_COLOR)
                    .set(self.ids.show_help_label, ui_widgets);

                self.inventorytest_button = ToggleButton::new(
                    self.inventorytest_button,
                    self.imgs.check,
                    self.imgs.check_checked,
                )
                .w_h(288.0 / 24.0, 288.0 / 24.0)
                .down_from(self.ids.button_help, 7.0)
                .hover_images(self.imgs.check_checked_mo, self.imgs.check_mo)
                .press_images(self.imgs.check_press, self.imgs.check_press)
                .set(self.ids.inventorytest_button, ui_widgets);

                Text::new("Show Inventory Test Button")
                    .right_from(self.ids.inventorytest_button, 10.0)
                    .font_size(14)
                    .font_id(self.font_opensans)
                    .graphics_for(self.ids.inventorytest_button)
                    .color(TEXT_COLOR)
                    .set(self.ids.inventorytest_button_label, ui_widgets);

                self.show_debug =
                    ToggleButton::new(self.show_debug, self.imgs.check, self.imgs.check_checked)
                        .w_h(288.0 / 24.0, 288.0 / 24.0)
                        .down_from(self.ids.inventorytest_button, 7.0)
                        .hover_images(self.imgs.check_checked_mo, self.imgs.check_mo)
                        .press_images(self.imgs.check_press, self.imgs.check_press)
                        .set(self.ids.debug_button, ui_widgets);

                Text::new("Show Debug Window")
                    .right_from(self.ids.debug_button, 10.0)
                    .font_size(14)
                    .font_id(self.font_opensans)
                    .graphics_for(self.ids.debug_button)
                    .color(TEXT_COLOR)
                    .set(self.ids.debug_button_label, ui_widgets);
            }

            // 2 Gameplay////////////////
            if Button::image(if let SettingsTab::Gameplay = self.settings_tab {
                self.imgs.settings_button_pressed
            } else {
                self.imgs.settings_button
            })
            .w_h(31.0 * 4.0, 12.0 * 4.0)
            .hover_image(if let SettingsTab::Gameplay = self.settings_tab {
                self.imgs.settings_button_pressed
            } else {
                self.imgs.settings_button_hover
            })
            .press_image(if let SettingsTab::Gameplay = self.settings_tab {
                self.imgs.settings_button_pressed
            } else {
                self.imgs.settings_button_press
            })
            .right_from(self.ids.interface, 0.0)
            .label("Gameplay")
            .label_font_size(14)
            .label_color(TEXT_COLOR)
            .set(self.ids.gameplay, ui_widgets)
            .was_clicked()
            {
                self.settings_tab = SettingsTab::Gameplay;
            }

            // 3 Controls/////////////////////
            if Button::image(if let SettingsTab::Controls = self.settings_tab {
                self.imgs.settings_button_pressed
            } else {
                self.imgs.settings_button
            })
            .w_h(31.0 * 4.0, 12.0 * 4.0)
            .hover_image(if let SettingsTab::Controls = self.settings_tab {
                self.imgs.settings_button_pressed
            } else {
                self.imgs.settings_button_hover
            })
            .press_image(if let SettingsTab::Controls = self.settings_tab {
                self.imgs.settings_button_pressed
            } else {
                self.imgs.settings_button_press
            })
            .right_from(self.ids.gameplay, 0.0)
            .label("Controls")
            .label_font_size(14)
            .label_color(TEXT_COLOR)
            .set(self.ids.controls, ui_widgets)
            .was_clicked()
            {
                self.settings_tab = SettingsTab::Controls;
            }
            if let SettingsTab::Controls = self.settings_tab {

            Text::new(
            "Free Cursor: TAB \n\
            Toggle Help Window: F1     \n\
            Toggle Interface: F2  \n\
            Toggle FPS and Debug Info: F3 \n\
            \n\
            \n\
            Move Forward: W     \n\
            Move Left : A       \n\
            Move Right: S       \n\
            Move Backwards: D   \n\
            \n\
            Jump: Space         \n\
            \n\
            Dodge: ??           \n\
            \n\
            Auto Walk: ??       \n\
            \n\
            Sheathe/Draw Weapons: Y \n\
            \n\
            Put on/Remove Helmet: ?? [Has a Cast time of 0,5s]  \n\
            \n\
            \n\
            Basic Attack: L-Click       \n\
            Secondary Attack/Block/Aim: R-Click \n\
            \n\
            \n\
            Skillbar Slot 1: 1  \n\
            Skillbar Slot 2: 2  \n\
            Skillbar Slot 3: 3  \n\
            Skillbar Slot 4: 4  \n\
            Skillbar Slot 5: 5  \n\
            Skillbar Slot 6: 6  \n\
            Skillbar Slot 7: 7  \n\
            Skillbar Slot 8: 8  \n\
            Skillbar Slot 9: 9  \n\
            Skillbar Slot 10: 0 \n\
            \n\
            \n\
            Pause Menu: ESC \n\
            Settings: N \n\
            Social: O   \n\
            Map: M  \n\
            Spellbook: P    \n\
            Character: C    \n\
            Questlog  L \n\
            Bag: B  \n\
            \n\
            \n\
            \n\
            Activate Chat & Input/Send Message: Enter \n\
            Scroll Chat: Mousewheel on Chat-Window  \n\
            \n\
            \n\
            Chat commands:  \n\
            \n\
            /alias [Name] - Change your Chat Name   \n\
            /tp [Name] - Teleports you to another player
            ")
            .color(TEXT_COLOR)
            .top_left_with_margins_on(self.ids.settings_content, 5.0, 5.0)
            .font_id(self.font_opensans)
            .font_size(18)
            .set(self.ids.controls_text, ui_widgets);
   

            }
            // 4 Video////////////////////////////////
            if Button::image(if let SettingsTab::Video = self.settings_tab {
                self.imgs.settings_button_pressed
            } else {
                self.imgs.settings_button
            })
            .w_h(31.0 * 4.0, 12.0 * 4.0)
            .hover_image(if let SettingsTab::Video = self.settings_tab {
                self.imgs.settings_button_pressed
            } else {
                self.imgs.settings_button_hover
            })
            .press_image(if let SettingsTab::Video = self.settings_tab {
                self.imgs.settings_button_pressed
            } else {
                self.imgs.settings_button_press
            })
            .right_from(self.ids.controls, 0.0)
            .label("Video")
            .parent(self.ids.settings_r)
            .label_font_size(14)
            .label_color(TEXT_COLOR)
            .set(self.ids.video, ui_widgets)
            .was_clicked()
            {
                self.settings_tab = SettingsTab::Video;
            }

            // 5 Sound///////////////////////////////
            if Button::image(if let SettingsTab::Sound = self.settings_tab {
                self.imgs.settings_button_pressed
            } else {
                self.imgs.settings_button
            })
            .w_h(31.0 * 4.0, 12.0 * 4.0)
            .hover_image(if let SettingsTab::Sound = self.settings_tab {
                self.imgs.settings_button_pressed
            } else {
                self.imgs.settings_button_hover
            })
            .press_image(if let SettingsTab::Sound = self.settings_tab {
                self.imgs.settings_button_pressed
            } else {
                self.imgs.settings_button_press
            })
            .right_from(self.ids.video, 0.0)
            .parent(self.ids.settings_r)
            .label("Sound")
            .label_font_size(14)
            .label_color(TEXT_COLOR)
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
                            .w_h(107.0 * 4.0, 125.0 * 4.0)
                            .set(self.ids.social_frame, ui_widgets);
                    } else {
                        Image::new(self.imgs.window_frame)
                            .top_left_with_margins_on(ui_widgets.window, 200.0, 10.0)
                            .w_h(107.0 * 4.0, 125.0 * 4.0)
                            .set(self.ids.social_frame, ui_widgets);
                    }

                    // Icon
                    Image::new(self.imgs.social_icon)
                        .w_h(40.0, 40.0)
                        .top_left_with_margins_on(self.ids.social_frame, 4.0, 4.0)
                        .set(self.ids.social_icon, ui_widgets);

                    // Content alignment
                    Rectangle::fill_with([362.0, 418.0], color::TRANSPARENT)
                        .bottom_right_with_margins_on(self.ids.social_frame, 17.0, 17.0)
                        .scroll_kids()
                        .scroll_kids_vertically()
                        .set(self.ids.social_bg, ui_widgets);

                    // X-Button
                    if Button::image(self.imgs.close_button)
                        .w_h(28.0, 28.0)
                        .hover_image(self.imgs.close_button_hover)
                        .press_image(self.imgs.close_button_press)
                        .top_right_with_margins_on(self.ids.social_frame, 12.0, 0.0)
                        .set(self.ids.social_close, ui_widgets)
                        .was_clicked()
                    {
                        self.open_windows = match self.open_windows {
                            Windows::Small(_) => Windows::None,
                            Windows::CharacterAnd(_) => Windows::CharacterAnd(None),
                            _ => Windows::Settings,
                        }
                    }
                    // Title
                    Text::new("Social")
                        .mid_top_with_margin_on(self.ids.social_frame, 17.0)
                        .font_id(self.font_metamorph)
                        .font_size(14)
                        .color(TEXT_COLOR)
                        .set(self.ids.social_title, ui_widgets);
                }
                Small::Spellbook => {
                    // Frame
                    if char_window_open {
                        Image::new(self.imgs.window_frame)
                            .right_from(self.ids.charwindow_frame, 20.0)
                            .w_h(107.0 * 4.0, 125.0 * 4.0)
                            .set(self.ids.spellbook_frame, ui_widgets);
                    } else {
                        Image::new(self.imgs.window_frame)
                            .top_left_with_margins_on(ui_widgets.window, 200.0, 10.0)
                            .w_h(107.0 * 4.0, 125.0 * 4.0)
                            .set(self.ids.spellbook_frame, ui_widgets);
                    }

                    // Icon
                    Image::new(self.imgs.spellbook_icon)
                        .w_h(40.0, 40.0)
                        .top_left_with_margins_on(self.ids.spellbook_frame, 4.0, 4.0)
                        .set(self.ids.spellbook_icon, ui_widgets);

                    // Content alignment
                    Rectangle::fill_with([362.0, 418.0], color::TRANSPARENT)
                        .bottom_right_with_margins_on(self.ids.spellbook_frame, 17.0, 17.0)
                        .scroll_kids()
                        .scroll_kids_vertically()
                        .set(self.ids.spellbook_bg, ui_widgets);

                    // X-Button
                    if Button::image(self.imgs.close_button)
                        .w_h(14.0, 14.0)
                        .hover_image(self.imgs.close_button_hover)
                        .press_image(self.imgs.close_button_press)
                        .top_right_with_margins_on(self.ids.spellbook_frame, 12.0, 0.0)
                        .set(self.ids.spellbook_close, ui_widgets)
                        .was_clicked()
                    {
                        self.open_windows = match self.open_windows {
                            Windows::Small(_) => Windows::None,
                            Windows::CharacterAnd(_) => Windows::CharacterAnd(None),
                            _ => Windows::Settings,
                        }
                    }
                    // Title
                    Text::new("Spellbook")
                        .mid_top_with_margin_on(self.ids.spellbook_frame, 17.0)
                        .font_size(14)
                        .color(TEXT_COLOR)
                        .set(self.ids.spellbook_title, ui_widgets);
                }
                Small::Questlog => {
                    // Frame
                    if char_window_open {
                        Image::new(self.imgs.window_frame)
                            .right_from(self.ids.charwindow_frame, 20.0)
                            .w_h(107.0 * 4.0, 125.0 * 4.0)
                            .set(self.ids.questlog_frame, ui_widgets);
                    } else {
                        Image::new(self.imgs.window_frame)
                            .top_left_with_margins_on(ui_widgets.window, 200.0, 10.0)
                            .w_h(107.0 * 4.0, 125.0 * 4.0)
                            .set(self.ids.questlog_frame, ui_widgets);
                    }

                    // Icon
                    Image::new(self.imgs.questlog_icon)
                        .w_h(40.0, 40.0)
                        .top_left_with_margins_on(self.ids.questlog_frame, 4.0, 4.0)
                        .set(self.ids.questlog_icon, ui_widgets);

                    // Content alignment
                    Rectangle::fill_with([362.0, 418.0], color::TRANSPARENT)
                        .bottom_right_with_margins_on(self.ids.questlog_frame, 17.0, 17.0)
                        .scroll_kids()
                        .scroll_kids_vertically()
                        .set(self.ids.questlog_bg, ui_widgets);

                    // X-Button
                    if Button::image(self.imgs.close_button)
                        .w_h(20.0, 20.0)
                        .press_image(self.imgs.close_button_press)
                        .top_right_with_margins_on(self.ids.questlog_frame, 17.0, 5.0)
                        .set(self.ids.questlog_close, ui_widgets)
                        .was_clicked()
                    {
                        self.open_windows = match self.open_windows {
                            Windows::Small(_) => Windows::None,
                            Windows::CharacterAnd(_) => Windows::CharacterAnd(None),
                            _ => Windows::Settings,
                        }
                    }
                    // Title
                    Text::new("Quest-Log")
                        .mid_top_with_margin_on(self.ids.questlog_frame, 17.0)
                        .color(TEXT_COLOR)
                        .font_size(14)
                        .set(self.ids.questlog_title, ui_widgets);
                }
            }
        }

        // 4 Char-Window
        if let Windows::CharacterAnd(small) = self.open_windows {
            // Frame
            Image::new(self.imgs.window_frame)
                .top_left_with_margins_on(ui_widgets.window, 200.0, 215.0)
                .w_h(1648.0 / 4.0, 1952.0 / 4.0)
                .set(self.ids.charwindow_frame, ui_widgets);

            // BG
            //Image::new(self.imgs.window_bg)
            //.w_h(348.0, 404.0)
            //.mid_top_with_margin_on(self.ids.charwindow_frame, 48.0)
            //.set(self.ids.charwindow_bg, ui_widgets);

            // Overlay
            //Image::new(self.imgs.charwindow)
            //.middle_of(self.ids.charwindow_bg)
            //.set(self.ids.charwindow, ui_widgets);

            // Icon
            //Image::new(self.imgs.charwindow_icon)
            //.w_h(224.0 / 3.0, 224.0 / 3.0)
            //.top_left_with_margins_on(self.ids.charwindow_frame, -10.0, -10.0)
            //.set(self.ids.charwindow_icon, ui_widgets);

            // X-Button
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
                .color(TEXT_COLOR)
                .set(self.ids.charwindow_title, ui_widgets);
            // Tab BG
            Image::new(self.imgs.charwindow_tab_bg)
                .w_h(205.0, 412.0)
                .mid_left_with_margin_on(self.ids.charwindow_frame, -205.0)
                .set(self.ids.charwindow_tab_bg, ui_widgets);
            // Tab Rectangle
            Rectangle::fill_with([192.0, 371.0], color::rgba(0.0, 0.0, 0.0, 0.8))
                .top_right_with_margins_on(self.ids.charwindow_tab_bg, 20.0, 0.0)
                .set(self.ids.charwindow_rectangle, ui_widgets);
            // Tab Button
            Button::image(self.imgs.charwindow_tab)
                .w_h(65.0, 23.0)
                .top_left_with_margins_on(self.ids.charwindow_tab_bg, -18.0, 2.0)
                .label("Stats")
                .label_color(TEXT_COLOR)
                .label_font_id(self.font_opensans)
                .label_font_size(14)
                .set(self.ids.charwindow_tab1, ui_widgets);
            Text::new("1") //Add in actual Character Level
                .mid_top_with_margin_on(self.ids.charwindow_rectangle, 10.0)
                .font_id(self.font_opensans)
                .font_size(30)
                .color(TEXT_COLOR)
                .set(self.ids.charwindow_tab1_level, ui_widgets);
            // Exp-Bar Background
            Rectangle::fill_with([170.0, 10.0], color::BLACK)
                .mid_top_with_margin_on(self.ids.charwindow_rectangle, 50.0)
                .set(self.ids.charwindow_exp_rectangle, ui_widgets);
            // Exp-Bar Progress
            Rectangle::fill_with([170.0 * (self.xp_percentage), 6.0], XP_COLOR) // 0.8 = Experience percantage
                .mid_left_with_margin_on(self.ids.charwindow_tab1_expbar, 1.0)
                .set(self.ids.charwindow_exp_progress_rectangle, ui_widgets);
            // Exp-Bar Foreground Frame
            Image::new(self.imgs.progress_frame)
                .w_h(170.0, 10.0)
                .middle_of(self.ids.charwindow_exp_rectangle)
                .set(self.ids.charwindow_tab1_expbar, ui_widgets);
            // Exp-Text
            Text::new("120/170") // Shows the Exp / Exp to reach the next level
                .mid_top_with_margin_on(self.ids.charwindow_tab1_expbar, 10.0)
                .font_id(self.font_opensans)
                .font_size(15)
                .color(TEXT_COLOR)
                .set(self.ids.charwindow_tab1_exp, ui_widgets);

            // Stats
            Text::new(
                "Stamina\n\
                 \n\
                 Strength\n\
                 \n\
                 Dexterity\n\
                 \n\
                 Intelligence",
            )
            .top_left_with_margins_on(self.ids.charwindow_rectangle, 100.0, 20.0)
            .font_id(self.font_opensans)
            .font_size(16)
            .color(TEXT_COLOR)
            .set(self.ids.charwindow_tab1_statnames, ui_widgets);

            Text::new(
                "1234\n\
                 \n\
                 12312\n\
                 \n\
                 12414\n\
                 \n\
                 124124",
            )
            .right_from(self.ids.charwindow_tab1_statnames, 10.0)
            .font_id(self.font_opensans)
            .font_size(16)
            .color(TEXT_COLOR)
            .set(self.ids.charwindow_tab1_stats, ui_widgets);
        }

        // 2 Map
        if self.map_open {
            // BG
            Rectangle::fill_with([824.0, 976.0], color::TRANSPARENT)
                .mid_top_with_margin_on(ui_widgets.window, 15.0)
                .scroll_kids()
                .scroll_kids_vertically()
                .set(self.ids.map_bg, ui_widgets);
            // Frame
            Image::new(self.imgs.map_frame_l)
                .top_left_with_margins_on(self.ids.map_bg, 0.0, 0.0)
                .w_h(412.0, 488.0)
                .set(self.ids.map_frame_l, ui_widgets);
            Image::new(self.imgs.map_frame_r)
                .right_from(self.ids.map_frame_l, 0.0)
                .w_h(412.0, 488.0)
                .set(self.ids.map_frame_r, ui_widgets);
            Image::new(self.imgs.map_frame_br)
                .down_from(self.ids.map_frame_r, 0.0)
                .w_h(412.0, 488.0)
                .set(self.ids.map_frame_br, ui_widgets);
            Image::new(self.imgs.map_frame_bl)
                .down_from(self.ids.map_frame_l, 0.0)
                .w_h(412.0, 488.0)
                .set(self.ids.map_frame_bl, ui_widgets);

            // Icon
            Image::new(self.imgs.map_icon)
                .w_h(224.0 / 3.0, 224.0 / 3.0)
                .top_left_with_margins_on(self.ids.map_frame, -10.0, -10.0)
                .set(self.ids.map_icon, ui_widgets);

            // X-Button
            if Button::image(self.imgs.close_button)
                .w_h(28.0, 28.0)
                .hover_image(self.imgs.close_button_hover)
                .press_image(self.imgs.close_button_press)
                .top_right_with_margins_on(self.ids.map_frame_r, 0.0, 0.0)
                .set(self.ids.map_close, ui_widgets)
                .was_clicked()
            {
                self.map_open = false;
            }
            // Title
            //Text::new("Map")
            //.mid_top_with_margin_on(self.ids.map_bg, -7.0)
            //.font_size(14)
            //.color(TEXT_COLOR)
            //.set(self.ids.map_title, ui_widgets);
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
                .label_y(conrod_core::position::Relative::Scalar(2.0))
                .label_color(TEXT_COLOR)
                .label_font_size(17)
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
                .label_y(conrod_core::position::Relative::Scalar(2.0))
                .label_color(TEXT_COLOR)
                .label_font_size(17)
                .set(self.ids.menu_button_2, ui_widgets)
                .was_clicked()
            {
                self.menu_open = false;
                self.settings_tab = SettingsTab::Controls;
                self.open_windows = Windows::Settings;
            };
            // Servers
            if Button::image(self.imgs.button_dark)
                .mid_top_with_margin_on(self.ids.esc_bg, 235.0)
                .w_h(170.0, 50.0)
                .hover_image(self.imgs.button_dark_hover)
                .press_image(self.imgs.button_dark_press)
                .label("Servers")
                .label_y(conrod_core::position::Relative::Scalar(2.0))
                .label_color(TEXT_COLOR)
                .label_font_size(17)
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
                .label_y(conrod_core::position::Relative::Scalar(2.0))
                .label_color(TEXT_COLOR)
                .label_font_size(17)
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
                .label_y(conrod_core::position::Relative::Scalar(2.0))
                .label_color(TEXT_COLOR)
                .label_font_size(17)
                .set(self.ids.menu_button_5, ui_widgets)
                .was_clicked()
            {
                events.push(Event::Quit);
            };
        }

        events
    }

    pub fn new_message(&mut self, msg: String) {
        self.chat.new_message(msg);
    }

    fn toggle_menu(&mut self) {
        self.menu_open = !self.menu_open;
    }
    fn toggle_bag(&mut self) {
        self.bag_open = !self.bag_open
    }
    fn toggle_map(&mut self) {
        self.map_open = !self.map_open;
        self.bag_open = false;
    }
    fn toggle_questlog(&mut self) {
        self.open_windows = match self.open_windows {
            Windows::Small(Small::Questlog) => Windows::None,
            Windows::None | Windows::Small(_) => Windows::Small(Small::Questlog),
            Windows::CharacterAnd(small) => match small {
                Some(Small::Questlog) => Windows::CharacterAnd(None),
                _ => Windows::CharacterAnd(Some(Small::Questlog)),
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
    fn toggle_social(&mut self) {
        self.open_windows = match self.open_windows {
            Windows::Small(Small::Social) => Windows::None,
            Windows::None | Windows::Small(_) => Windows::Small(Small::Social),
            Windows::CharacterAnd(small) => match small {
                Some(Small::Social) => Windows::CharacterAnd(None),
                _ => Windows::CharacterAnd(Some(Small::Social)),
            },
            Windows::Settings => Windows::Settings,
        };
    }
    fn toggle_spellbook(&mut self) {
        self.open_windows = match self.open_windows {
            Windows::Small(Small::Spellbook) => Windows::None,
            Windows::None | Windows::Small(_) => Windows::Small(Small::Spellbook),
            Windows::CharacterAnd(small) => match small {
                Some(Small::Spellbook) => Windows::CharacterAnd(None),
                _ => Windows::CharacterAnd(Some(Small::Spellbook)),
            },
            Windows::Settings => Windows::Settings,
        };
    }
    fn toggle_settings(&mut self) {
        self.open_windows = match self.open_windows {
            Windows::Settings => Windows::None,
            _ => Windows::Settings,
        };
        self.bag_open = false;
    }
    fn toggle_help(&mut self) {
        self.show_help = !self.show_help
    }
    fn toggle_ui(&mut self) {
        self.show_ui = !self.show_ui;
    }

    fn toggle_windows(&mut self, global_state: &mut GlobalState) {
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
            global_state.window.grab_cursor(true);
        } else {
            self.menu_open = true;
            global_state.window.grab_cursor(false);
        }
    }

    fn typing(&self) -> bool {
        match self.ui.widget_capturing_keyboard() {
            Some(id) if id == self.chat.input_box_id() => true,
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
                self.ui.focus_widget(if self.typing() {
                    None
                } else {
                    Some(self.chat.input_box_id())
                });
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
                    self.toggle_questlog();
                    true
                }
                Key::CharacterWindow => {
                    self.toggle_charwindow();
                    true
                }
                Key::Social => {
                    self.toggle_social();
                    true
                }
                Key::Spellbook => {
                    self.toggle_spellbook();
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
