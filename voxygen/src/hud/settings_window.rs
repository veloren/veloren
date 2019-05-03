use conrod_core::{
    builder_methods, color,
    text::font,
    widget::{self, Button, Image, Rectangle, Text},
    widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};
use super::{
    img_ids::Imgs,
    font_ids::Fonts,
    TEXT_COLOR,
};
use crate::ui::ToggleButton;

widget_ids! {
    struct Ids {
        button_help,
        controls,
        debug_button,
        debug_button_label,
        gameplay,
        interface,
        inventorytest_button,
        inventorytest_button_label,
        rectangle,
        settings_bg,
        settings_close,
        settings_icon,
        settings_title,
        show_help_label,
        sound,
        test,
        video,
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
    imgs: &'a Imgs,
    fonts: &'a Fonts,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    style: (),
}

impl<'a> SettingsWindow<'a> {
    pub fn new(imgs: &'a Imgs, fonts: &'a Fonts) -> Self {
        Self {
            imgs,
            fonts,
            common: widget::CommonBuilder::default(),
            style: (),
        }
    }
}

pub struct State {
    settings_tab: SettingsTab,
    show_debug: bool,
    show_help: bool,
    inventorytest_button: bool,

    ids: Ids,
}

pub enum Event {
    Close,
}

impl<'a> Widget for SettingsWindow<'a> {
    type State = State;
    type Style = ();
    type Event = Option<Event>;

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            settings_tab: SettingsTab::Interface,
            show_debug: false,
            show_help: false,
            inventorytest_button: false,
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {
        ()
    }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs {
            id,
            state,
            ui,
            style,
            ..
        } = args;

        // BG
        Image::new(self.imgs.settings_bg)
            .middle_of(ui.window)
            .w_h(1648.0 / 2.5, 1952.0 / 2.5)
            .set(state.ids.settings_bg, ui);

        // X-Button
        if Button::image(self.imgs.close_button)
            .w_h(244.0 * 0.22 / 2.5, 244.0 * 0.22 / 2.5)
            .hover_image(self.imgs.close_button_hover)
            .press_image(self.imgs.close_button_press)
            .top_right_with_margins_on(state.ids.settings_bg, 4.0, 4.0)
            .set(state.ids.settings_close, ui)
            .was_clicked()
        {
            return Some(Event::Close);
        }

        // Title
        Text::new("Settings")
            .mid_top_with_margin_on(state.ids.settings_bg, 10.0)
            .font_size(30)
            .color(TEXT_COLOR)
            .set(state.ids.settings_title, ui);

        // Icon
        Image::new(self.imgs.settings_icon)
            .w_h(224.0 / 3.0, 224.0 / 3.0)
            .top_left_with_margins_on(state.ids.settings_bg, -10.0, -10.0)
            .set(state.ids.settings_icon, ui);

        // TODO: Find out if we can remove this
        // Alignment Rectangle
        Rectangle::fill_with([1008.0 / 2.5, 1616.0 / 2.5], color::TRANSPARENT)
            .top_left_with_margins_on(state.ids.settings_bg, 77.0, 205.0)
            .set(state.ids.rectangle, ui);

        // Interface
        if Button::image(if let SettingsTab::Interface = state.settings_tab {
            self.imgs.button_blue_mo
        } else {
            self.imgs.blank
        })
            .w_h(304.0 / 2.5, 80.0 / 2.5)
            .hover_image(self.imgs.button_blue_mo)
            .press_image(self.imgs.button_blue_press)
            .top_left_with_margins_on(state.ids.settings_bg, 78.0, 50.0)
            .label("Interface")
            .label_font_size(14)
            .label_color(TEXT_COLOR)
            .set(state.ids.interface, ui)
            .was_clicked()
        {
            state.update(|s| s.settings_tab = SettingsTab::Interface);
        }

        // Toggle Help
        if let SettingsTab::Interface = state.settings_tab {
            let show_debug =
                ToggleButton::new(state.show_help, self.imgs.check, self.imgs.check_checked)
                    .w_h(288.0 / 24.0, 288.0 / 24.0)
                    .top_left_with_margins_on(state.ids.rectangle, 15.0, 15.0)
                    .hover_images(self.imgs.check_checked_mo, self.imgs.check_mo)
                    .press_images(self.imgs.check_press, self.imgs.check_press)
                    .set(state.ids.button_help, ui);
            
            state.update(|s| s.show_debug = show_debug);

            Text::new("Show Help")
                .right_from(state.ids.button_help, 10.0)
                .font_size(12)
                .font_id(self.fonts.opensans)
                .graphics_for(state.ids.button_help)
                .color(TEXT_COLOR)
                .set(state.ids.show_help_label, ui);

            let show_debug = ToggleButton::new(
                state.inventorytest_button,
                self.imgs.check,
                self.imgs.check_checked,
            )
                .w_h(288.0 / 24.0, 288.0 / 24.0)
                .top_left_with_margins_on(state.ids.rectangle, 40.0, 15.0)
                .hover_images(self.imgs.check_checked_mo, self.imgs.check_mo)
                .press_images(self.imgs.check_press, self.imgs.check_press)
                .set(state.ids.inventorytest_button, ui);

            state.update(|s| s.show_debug = show_debug);

            Text::new("Show Inventory Test Button")
                .right_from(state.ids.inventorytest_button, 10.0)
                .font_size(12)
                .font_id(self.fonts.opensans)
                .graphics_for(state.ids.inventorytest_button)
                .color(TEXT_COLOR)
                .set(state.ids.inventorytest_button_label, ui);

            let show_debug = ToggleButton::new(
                state.show_debug,
                self.imgs.check,
                self.imgs.check_checked
            )
                .w_h(288.0 / 24.0, 288.0 / 24.0)
                .top_left_with_margins_on(state.ids.rectangle, 65.0, 15.0)
                .hover_images(self.imgs.check_checked_mo, self.imgs.check_mo)
                .press_images(self.imgs.check_press, self.imgs.check_press)
                .set(state.ids.debug_button, ui);

            state.update(|s| s.show_debug = show_debug);

            Text::new("Show Debug Window")
                .right_from(state.ids.debug_button, 10.0)
                .font_size(12)
                .font_id(self.fonts.opensans)
                .graphics_for(state.ids.debug_button)
                .color(TEXT_COLOR)
                .set(state.ids.debug_button_label, ui);

        }

        // 2 Gameplay////////////////
        if Button::image(if let SettingsTab::Gameplay = state.settings_tab {
                self.imgs.button_blue_mo
            } else {
                self.imgs.blank
            }
        )
            .w_h(304.0 / 2.5, 80.0 / 2.5)
            .hover_image(self.imgs.button_blue_mo)
            .press_image(self.imgs.button_blue_press)
            .down_from(state.ids.interface, 1.0)
            .label("Gameplay")
            .label_font_size(14)
            .label_color(TEXT_COLOR)
            .set(state.ids.gameplay, ui)
            .was_clicked()
        {
            state.update(|s| s.settings_tab = SettingsTab::Gameplay);
        }

        // 3 Controls/////////////////////
        if Button::image(if let SettingsTab::Controls = state.settings_tab {
                self.imgs.button_blue_mo
            } else {
                self.imgs.blank
            }
        )
            .w_h(304.0 / 2.5, 80.0 / 2.5)
            .hover_image(self.imgs.button_blue_mo)
            .press_image(self.imgs.button_blue_press)
            .down_from(state.ids.gameplay, 1.0)
            .label("Controls")
            .label_font_size(14)
            .label_color(TEXT_COLOR)
            .set(state.ids.controls, ui)
            .was_clicked()
        {
            state.update(|s| s.settings_tab = SettingsTab::Controls);
        }

        // 4 Video////////////////////////////////
        if Button::image(if let SettingsTab::Video = state.settings_tab {
                self.imgs.button_blue_mo
            } else {
                self.imgs.blank
            }
        )
            .w_h(304.0 / 2.5, 80.0 / 2.5)
            .hover_image(self.imgs.button_blue_mo)
            .press_image(self.imgs.button_blue_press)
            .down_from(state.ids.controls, 1.0)
            .label("Video")
            .label_font_size(14)
            .label_color(TEXT_COLOR)
            .set(state.ids.video, ui)
            .was_clicked()
        {
            state.update(|s| s.settings_tab = SettingsTab::Video);
        }

        // 5 Sound///////////////////////////////
        if Button::image(if let SettingsTab::Sound = state.settings_tab {
                self.imgs.button_blue_mo
            } else {
                self.imgs.blank
            }
        )
            .w_h(304.0 / 2.5, 80.0 / 2.5)
            .hover_image(self.imgs.button_blue_mo)
            .press_image(self.imgs.button_blue_press)
            .down_from(state.ids.video, 1.0)
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
