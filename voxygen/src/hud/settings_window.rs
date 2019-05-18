use super::{img_ids::Imgs, Fonts, TEXT_COLOR};
use crate::{hud::Show, ui::ToggleButton};
use conrod_core::{
    color,
    widget::{self, Button, Image, Rectangle, Scrollbar, Text},
    widget_ids, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};
use crate::{
    render::Renderer,
    ui::{
        self,
        img_ids::{ImageGraphic, VoxelGraphic},
        ImageSlider, ScaleMode, Ui,
    },
    window::Window,
};
widget_ids! {
    struct Ids {

        settings_content,
        settings_icon,
        settings_button_mo,
        settings_close,
        settings_title,
        settings_r,
        settings_l,
        settings_scrollbar,
        controls_text,
        controls_controls,
        button_help,
        button_help2,
        show_help_label,
        gameplay,
        controls,
        rectangle,
        debug_button,
        debug_button_label,
        interface,
        inventory_test_button,
        inventory_test_button_label,
        settings_bg,
        sound,
        test,
        video,
        vd_slider,
        vd_slider_text,
    }
}

enum SettingsTab {
    Interface,
    Video,
    Sound,
    Gameplay,
    Controls,
}

#[derive(WidgetCommon)]
pub struct SettingsWindow<'a> {
    show: &'a Show,

    imgs: &'a Imgs,
    fonts: &'a Fonts,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> SettingsWindow<'a> {
    pub fn new(show: &'a Show, imgs: &'a Imgs, fonts: &'a Fonts) -> Self {
        Self {
            show,
            imgs,
            fonts,
            common: widget::CommonBuilder::default(),
        }
    }
}

pub struct State {
    settings_tab: SettingsTab,

    ids: Ids,
}

pub enum Event {
    ToggleHelp,
    ToggleInventoryTestButton,
    ToggleDebug,
    Close,
}

impl<'a> Widget for SettingsWindow<'a> {
    type State = State;
    type Style = ();
    type Event = Option<Event>;

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            settings_tab: SettingsTab::Interface,
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {
        ()
    }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;

        // Frame Alignment
        Rectangle::fill_with([824.0, 488.0], color::TRANSPARENT)
            .middle_of(ui.window)
            .set(state.ids.settings_bg, ui);
        // Frame
        Image::new(self.imgs.settings_frame_l)
            .top_left_with_margins_on(state.ids.settings_bg, 0.0, 0.0)
            .w_h(412.0, 488.0)
            .set(state.ids.settings_l, ui);
        Image::new(self.imgs.settings_frame_r)
            .right_from(state.ids.settings_l, 0.0)
            .parent(state.ids.settings_bg)
            .w_h(412.0, 488.0)
            .set(state.ids.settings_r, ui);
        // Content Alignment
        Rectangle::fill_with([198.0 * 4.0, 97.0 * 4.0], color::TRANSPARENT)
            .top_right_with_margins_on(state.ids.settings_r, 21.0 * 4.0, 4.0 * 4.0)
            .scroll_kids()
            .scroll_kids_vertically()
            .set(state.ids.settings_content, ui);
        Scrollbar::y_axis(state.ids.settings_content)
            .thickness(5.0)
            .rgba(0.33, 0.33, 0.33, 1.0)
            .set(state.ids.settings_scrollbar, ui);
        // X-Button
        if Button::image(self.imgs.close_button)
            .w_h(28.0, 28.0)
            .hover_image(self.imgs.close_button_hover)
            .press_image(self.imgs.close_button_press)
            .top_right_with_margins_on(state.ids.settings_r, 0.0, 0.0)
            .set(state.ids.settings_close, ui)
            .was_clicked()
        {
            return Some(Event::Close);
        }

        // Title
        Text::new("Settings")
            .mid_top_with_margin_on(state.ids.settings_bg, 5.0)
            .font_size(14)
            .color(TEXT_COLOR)
            .set(state.ids.settings_title, ui);

        // Interface
        if Button::image(if let SettingsTab::Interface = state.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button
        })
        .w_h(31.0 * 4.0, 12.0 * 4.0)
        .hover_image(if let SettingsTab::Interface = state.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button_hover
        })
        .press_image(if let SettingsTab::Interface = state.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button_press
        })
        .top_left_with_margins_on(state.ids.settings_l, 8.0 * 4.0, 2.0 * 4.0)
        .label("Interface")
        .label_font_size(14)
        .label_color(TEXT_COLOR)
        .set(state.ids.interface, ui)
        .was_clicked()
        {
            state.update(|s| s.settings_tab = SettingsTab::Interface);
        }

        if let SettingsTab::Interface = state.settings_tab {
            // Help
            let show_help =
                ToggleButton::new(self.show.help, self.imgs.check, self.imgs.check_checked)
                    .w_h(288.0 / 24.0, 288.0 / 24.0)
                    .top_left_with_margins_on(state.ids.settings_content, 5.0, 5.0)
                    .hover_images(self.imgs.check_checked_mo, self.imgs.check_mo)
                    .press_images(self.imgs.check_press, self.imgs.check_press)
                    .set(state.ids.button_help, ui);

            if self.show.help != show_help {
                return Some(Event::ToggleHelp);
            }

            Text::new("Show Help")
                .right_from(state.ids.button_help, 10.0)
                .font_size(12)
                .font_id(self.fonts.opensans)
                .graphics_for(state.ids.button_help)
                .color(TEXT_COLOR)
                .set(state.ids.show_help_label, ui);

            // Inventory test
            let inventory_test_button = ToggleButton::new(
                self.show.inventory_test_button,
                self.imgs.check,
                self.imgs.check_checked,
            )
            .w_h(288.0 / 24.0, 288.0 / 24.0)
            .down_from(state.ids.button_help, 7.0)
            .hover_images(self.imgs.check_checked_mo, self.imgs.check_mo)
            .press_images(self.imgs.check_press, self.imgs.check_press)
            .set(state.ids.inventory_test_button, ui);

            if self.show.inventory_test_button != inventory_test_button {
                return Some(Event::ToggleInventoryTestButton);
            }

            Text::new("Show Inventory Test Button")
                .right_from(state.ids.inventory_test_button, 10.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .graphics_for(state.ids.inventory_test_button)
                .color(TEXT_COLOR)
                .set(state.ids.inventory_test_button_label, ui);

            // Debug
            let show_debug =
                ToggleButton::new(self.show.debug, self.imgs.check, self.imgs.check_checked)
                    .w_h(288.0 / 24.0, 288.0 / 24.0)
                    .down_from(state.ids.inventory_test_button, 7.0)
                    .hover_images(self.imgs.check_checked_mo, self.imgs.check_mo)
                    .press_images(self.imgs.check_press, self.imgs.check_press)
                    .set(state.ids.debug_button, ui);

            if self.show.debug != show_debug {
                return Some(Event::ToggleDebug);
            }

            Text::new("Show Debug Window")
                .right_from(state.ids.debug_button, 10.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .graphics_for(state.ids.debug_button)
                .color(TEXT_COLOR)
                .set(state.ids.debug_button_label, ui);
        }

        // 2 Gameplay
        if Button::image(if let SettingsTab::Gameplay = state.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button
        })
        .w_h(31.0 * 4.0, 12.0 * 4.0)
        .hover_image(if let SettingsTab::Gameplay = state.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button_hover
        })
        .press_image(if let SettingsTab::Gameplay = state.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button_press
        })
        .right_from(state.ids.interface, 0.0)
        .label("Gameplay")
        .label_font_size(14)
        .label_color(TEXT_COLOR)
        .set(state.ids.gameplay, ui)
        .was_clicked()
        {
            state.update(|s| s.settings_tab = SettingsTab::Gameplay);
        }

        // 3 Controls
        if Button::image(if let SettingsTab::Controls = state.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button
        })
        .w_h(31.0 * 4.0, 12.0 * 4.0)
        .hover_image(if let SettingsTab::Controls = state.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button_hover
        })
        .press_image(if let SettingsTab::Controls = state.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button_press
        })
        .right_from(state.ids.gameplay, 0.0)
        .label("Controls")
        .label_font_size(14)
        .label_color(TEXT_COLOR)
        .set(state.ids.controls, ui)
        .was_clicked()
        {
            state.update(|s| s.settings_tab = SettingsTab::Controls);
        }
        if let SettingsTab::Controls = state.settings_tab {
            Text::new(
                "Free Cursor\n\
            Toggle Help Window\n\
            Toggle Interface\n\
            Toggle FPS and Debug Info\n\
            \n\
            \n\
            Move Forward\n\
            Move Left\n\
            Move Right\n\
            Move Backwards\n\
            \n\
            Jump\n\
            \n\
            Dodge\n\
            \n\
            Auto Walk\n\
            \n\
            Sheathe/Draw Weapons\n\
            \n\
            Put on/Remove Helmet\n\
            \n\
            \n\
            Basic Attack\n\
            Secondary Attack/Block/Aim\n\
            \n\
            \n\
            Skillbar Slot 1\n\
            Skillbar Slot 2\n\
            Skillbar Slot 3\n\
            Skillbar Slot 4\n\
            Skillbar Slot 5\n\
            Skillbar Slot 6\n\
            Skillbar Slot 7\n\
            Skillbar Slot 8\n\
            Skillbar Slot 9\n\
            Skillbar Slot 10\n\
            \n\
            \n\
            Pause Menu\n\
            Settings\n\
            Social\n\
            Map\n\
            Spellbook\n\
            Character\n\
            Questlog\n\
            Bag\n\
            \n\
            \n\
            \n\
            Send Chat Message\n\
            Scroll Chat\n\
            \n\
            \n\
            Chat commands:  \n\
            \n\
            /alias [Name] - Change your Chat Name   \n\
            /tp [Name] - Teleports you to another player
            ",
            )
            .color(TEXT_COLOR)
            .top_left_with_margins_on(state.ids.settings_content, 5.0, 5.0)
            .font_id(self.fonts.opensans)
            .font_size(18)
            .set(state.ids.controls_text, ui);
            // TODO: Replace with buttons that show actual keybinds and allow the user to change them.
            Text::new(
                "TAB\n\
                 F1\n\
                 F2\n\
                 F3\n\
                 \n\
                 \n\
                 W\n\
                 A\n\
                 S\n\
                 D\n\
                 \n\
                 SPACE\n\
                 \n\
                 ??\n\
                 \n\
                 ??\n\
                 \n\
                 ??\n\
                 \n\
                 ??\n\
                 \n\
                 \n\
                 L-Click\n\
                 R-Click\n\
                 \n\
                 \n\
                 1\n\
                 2\n\
                 3\n\
                 4\n\
                 5\n\
                 6\n\
                 7\n\
                 8\n\
                 9\n\
                 0\n\
                 \n\
                 \n\
                 ESC\n\
                 N\n\
                 O\n\
                 M\n\
                 P\n\
                 C\n\
                 L\n\
                 B\n\
                 \n\
                 \n\
                 \n\
                 ENTER\n\
                 Mousewheel\n\
                 \n\
                 \n\
                 \n\
                 \n\
                 \n\
                 \n\
                 ",
            )
            .color(TEXT_COLOR)
            .right_from(state.ids.controls_text, 0.0)
            .font_id(self.fonts.opensans)
            .font_size(18)
            .set(state.ids.controls_controls, ui);
        }
        // 4 Video
        if Button::image(if let SettingsTab::Video = state.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button
        })
        .w_h(31.0 * 4.0, 12.0 * 4.0)
        .hover_image(if let SettingsTab::Video = state.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button_hover
        })
        .press_image(if let SettingsTab::Video = state.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button_press
        })
        .right_from(state.ids.controls, 0.0)
        .label("Video")
        .parent(state.ids.settings_r)
        .label_font_size(14)
        .label_color(TEXT_COLOR)
        .set(state.ids.video, ui)
        .was_clicked()
        {
            state.update(|s| s.settings_tab = SettingsTab::Video);
        }
        // Contents
        if let SettingsTab::Video = state.settings_tab { 
           Text::new("Viewdistance")
            .top_left_with_margins_on(state.ids.settings_content, 10.0, 10.0)
            .font_size(14)
            .font_id(self.fonts.opensans)
            .color(TEXT_COLOR)
            .set(state.ids.vd_slider_text, ui);

        if let Some(new_val) = ImageSlider::continuous(5.0,
                    5.0,
                    25.0,
                    self.imgs.slider_indicator,
                    self.imgs.slider,)
            .w_h(208.0, 22.0)
            .down_from(state.ids.vd_slider_text, 10.0)
            .track_breadth(12.0)
            .slider_length(10.0)
            .pad_track((5.0, 5.0))
            .set(state.ids.vd_slider, ui)
            {}
        }
        // 5 Sound
        if Button::image(if let SettingsTab::Sound = state.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button
        })
        .w_h(31.0 * 4.0, 12.0 * 4.0)
        .hover_image(if let SettingsTab::Sound = state.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button_hover
        })
        .press_image(if let SettingsTab::Sound = state.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button_press
        })
        .right_from(state.ids.video, 0.0)
        .parent(state.ids.settings_r)
        .label("Sound")
        .label_font_size(14)
        .label_color(TEXT_COLOR)
        .set(state.ids.sound, ui)
        .was_clicked()
        {
            state.update(|s| s.settings_tab = SettingsTab::Sound);
        }

        None
    }
}
