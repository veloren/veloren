use super::{
    img_ids::Imgs, BarNumbers, CrosshairType, Fonts, ShortcutNumbers, Show, XpBar, TEXT_COLOR,
};
use crate::{
    render::AaMode,
    ui::{ImageSlider, RadioList, ScaleMode, ToggleButton},
    GlobalState,
};
use conrod_core::{
    color,
    widget::{self, Button, DropDownList, Image, Rectangle, Scrollbar, Text},
    widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};

const FPS_CHOICES: [u32; 11] = [15, 30, 40, 50, 60, 90, 120, 144, 240, 300, 500];

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
        ui_scale_label,
        ui_scale_slider,
        ui_scale_button,
        ui_scale_value,
        relative_to_win_button,
        relative_to_win_text,
        absolute_scale_button,
        absolute_scale_text,
        gameplay,
        controls,
        rectangle,
        general_txt,
        debug_button,
        debug_button_label,
        interface,
        mouse_pan_slider,
        mouse_pan_label,
        mouse_pan_value,
        mouse_zoom_slider,
        mouse_zoom_label,
        mouse_zoom_value,
        mouse_zoom_invert_button,
        mouse_zoom_invert_label,
        ch_title,
        ch_transp_slider,
        ch_transp_label,
        ch_transp_value,
        ch_transp_text,
        ch_1_bg,
        ch_2_bg,
        ch_3_bg,
        crosshair_outer_1,
        crosshair_inner_1,
        crosshair_outer_2,
        crosshair_inner_2,
        crosshair_outer_3,
        crosshair_inner_3,
        settings_bg,
        sound,
        test,
        video,
        vd_slider,
        vd_text,
        vd_value,
        max_fps_slider,
        max_fps_text,
        max_fps_value,
        fov_slider,
        fov_text,
        fov_value,
        aa_radio_buttons,
        aa_mode_text,
        audio_volume_slider,
        audio_volume_text,
        sfx_volume_slider,
        sfx_volume_text,
        audio_device_list,
        audio_device_text,
        hotbar_title,
        bar_numbers_title,
        show_bar_numbers_none_button,
        show_bar_numbers_none_text,
        show_bar_numbers_values_button,
        show_bar_numbers_values_text,
        show_bar_numbers_percentage_button,
        show_bar_numbers_percentage_text,
        show_shortcuts_button,
        show_shortcuts_text,
        show_xpbar_button,
        show_xpbar_text,
        show_bars_button,
        show_bars_text,
        placeholder,
    }
}

pub enum SettingsTab {
    Interface,
    Video,
    Sound,
    Gameplay,
    Controls,
}

#[derive(WidgetCommon)]
pub struct SettingsWindow<'a> {
    global_state: &'a GlobalState,

    show: &'a Show,

    imgs: &'a Imgs,
    fonts: &'a Fonts,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> SettingsWindow<'a> {
    pub fn new(
        global_state: &'a GlobalState,
        show: &'a Show,
        imgs: &'a Imgs,
        fonts: &'a Fonts,
    ) -> Self {
        Self {
            global_state,
            show,
            imgs,
            fonts,
            common: widget::CommonBuilder::default(),
        }
    }
}

pub struct State {
    ids: Ids,
}

pub enum Event {
    ToggleHelp,
    ToggleDebug,
    ToggleXpBar(XpBar),
    ToggleBarNumbers(BarNumbers),
    ToggleShortcutNumbers(ShortcutNumbers),
    ChangeTab(SettingsTab),
    Close,
    AdjustMousePan(u32),
    AdjustMouseZoom(u32),
    ToggleZoomInvert(bool),
    AdjustViewDistance(u32),
    AdjustFOV(u16),
    ChangeAaMode(AaMode),
    AdjustMusicVolume(f32),
    AdjustSfxVolume(f32),
    ChangeAudioDevice(String),
    MaximumFPS(u32),
    CrosshairTransp(f32),
    CrosshairType(CrosshairType),
    UiScale(ScaleChange),
}

pub enum ScaleChange {
    ToAbsolute,
    ToRelative,
    Adjust(f64),
}

impl<'a> Widget for SettingsWindow<'a> {
    type State = State;
    type Style = ();
    type Event = Vec<Event>;

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {
        ()
    }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;

        let mut events = Vec::new();
        let bar_values = self.global_state.settings.gameplay.bar_numbers;

        //let mut xp_bar = self.global_state.settings.gameplay.xp_bar;

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
            events.push(Event::Close);
        }

        // Title
        Text::new("Settings")
            .mid_top_with_margin_on(state.ids.settings_bg, 5.0)
            .font_size(14)
            .color(TEXT_COLOR)
            .set(state.ids.settings_title, ui);

        // 1) Interface Tab -------------------------------
        if Button::image(if let SettingsTab::Interface = self.show.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button
        })
        .w_h(31.0 * 4.0, 12.0 * 4.0)
        .hover_image(if let SettingsTab::Interface = self.show.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button_hover
        })
        .press_image(if let SettingsTab::Interface = self.show.settings_tab {
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
            events.push(Event::ChangeTab(SettingsTab::Interface));
        }

        // Contents
        if let SettingsTab::Interface = self.show.settings_tab {
            let crosshair_transp = self.global_state.settings.gameplay.crosshair_transp;
            let crosshair_type = self.global_state.settings.gameplay.crosshair_type;
            let ui_scale = self.global_state.settings.gameplay.ui_scale;

            Text::new("General")
                .top_left_with_margins_on(state.ids.settings_content, 5.0, 5.0)
                .font_size(18)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(state.ids.general_txt, ui);

            // Help
            let show_help = ToggleButton::new(
                self.show.help,
                self.imgs.checkbox,
                self.imgs.checkbox_checked,
            )
            .w_h(18.0, 18.0)
            .down_from(state.ids.general_txt, 20.0)
            .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
            .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
            .set(state.ids.button_help, ui);

            if self.show.help != show_help {
                events.push(Event::ToggleHelp);
            }

            Text::new("Show Help Window")
                .right_from(state.ids.button_help, 10.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .graphics_for(state.ids.button_help)
                .color(TEXT_COLOR)
                .set(state.ids.show_help_label, ui);

            // Debug
            let show_debug = ToggleButton::new(
                self.show.debug,
                self.imgs.checkbox,
                self.imgs.checkbox_checked,
            )
            .w_h(18.0, 18.0)
            .down_from(state.ids.button_help, 8.0)
            .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
            .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
            .set(state.ids.debug_button, ui);

            if self.show.debug != show_debug {
                events.push(Event::ToggleDebug);
            }

            Text::new("Show Debug Info")
                .right_from(state.ids.debug_button, 10.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .graphics_for(state.ids.debug_button)
                .color(TEXT_COLOR)
                .set(state.ids.debug_button_label, ui);

            // Ui Scale
            Text::new("UI-Scale")
                .down_from(state.ids.debug_button, 20.0)
                .font_size(18)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(state.ids.ui_scale_label, ui);

            // Relative Scaling Button
            let (check_img, check_mo_img, check_press_img, relative_selected) = match ui_scale {
                ScaleMode::RelativeToWindow(_) => (
                    self.imgs.check_checked,
                    self.imgs.check_checked_mo,
                    self.imgs.check_checked,
                    true,
                ),
                ScaleMode::Absolute(_) | ScaleMode::DpiFactor => (
                    self.imgs.check,
                    self.imgs.check_mo,
                    self.imgs.check_press,
                    false,
                ),
            };
            if Button::image(check_img)
                .w_h(288.0 / 24.0, 288.0 / 24.0)
                .down_from(state.ids.ui_scale_label, 20.0)
                .hover_image(check_mo_img)
                .press_image(check_press_img)
                .set(state.ids.relative_to_win_button, ui)
                .was_clicked()
                && !relative_selected
            {
                events.push(Event::UiScale(ScaleChange::ToRelative));
            }

            Text::new("Relative Scaling")
                .right_from(state.ids.relative_to_win_button, 10.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .graphics_for(state.ids.relative_to_win_button)
                .color(TEXT_COLOR)
                .set(state.ids.relative_to_win_text, ui);

            // Absolute Scaling Button
            let (check_img, check_mo_img, check_press_img, absolute_selected) = match ui_scale {
                ScaleMode::Absolute(_) => (
                    self.imgs.check_checked,
                    self.imgs.check_checked_mo,
                    self.imgs.check_checked,
                    true,
                ),
                ScaleMode::RelativeToWindow(_) | ScaleMode::DpiFactor => (
                    self.imgs.check,
                    self.imgs.check_mo,
                    self.imgs.check_press,
                    false,
                ),
            };
            if Button::image(check_img)
                .w_h(288.0 / 24.0, 288.0 / 24.0)
                .down_from(state.ids.relative_to_win_button, 8.0)
                .hover_image(check_mo_img)
                .press_image(check_press_img)
                .set(state.ids.absolute_scale_button, ui)
                .was_clicked()
                && !absolute_selected
            {
                events.push(Event::UiScale(ScaleChange::ToAbsolute));
            }

            Text::new("Custom Scaling")
                .right_from(state.ids.absolute_scale_button, 10.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .graphics_for(state.ids.absolute_scale_button)
                .color(TEXT_COLOR)
                .set(state.ids.absolute_scale_text, ui);

            // Slider -> Inactive when "Relative to window" is selected
            if let ScaleMode::Absolute(scale) = ui_scale {
                if let Some(new_val) = ImageSlider::continuous(
                    scale.log(2.0),
                    0.5f64.log(2.0),
                    1.2f64.log(2.0),
                    self.imgs.slider_indicator,
                    self.imgs.slider,
                )
                .w_h(208.0, 22.0)
                .right_from(state.ids.absolute_scale_text, 12.0)
                .track_breadth(30.0)
                .slider_length(10.0)
                .pad_track((5.0, 5.0))
                .set(state.ids.ui_scale_slider, ui)
                {
                    events.push(Event::UiScale(ScaleChange::Adjust(2.0f64.powf(new_val))));
                }
                // Custom Scaling Text
                Text::new(&format!("{:.2}", scale))
                    .right_from(state.ids.ui_scale_slider, 10.0)
                    .font_size(14)
                    .font_id(self.fonts.opensans)
                    .color(TEXT_COLOR)
                    .set(state.ids.ui_scale_value, ui);
            } else {
                // Grey and unfunctional slider when Relative is selected
                ImageSlider::continuous(0.0, 0.0, 1.0, self.imgs.nothing, self.imgs.slider)
                    .w_h(208.0, 22.0)
                    .right_from(state.ids.absolute_scale_text, 10.0)
                    .track_breadth(12.0)
                    .slider_length(10.0)
                    .track_color(Color::Rgba(1.0, 1.0, 1.0, 0.2))
                    .slider_color(Color::Rgba(1.0, 1.0, 1.0, 0.2))
                    .pad_track((5.0, 5.0))
                    .set(state.ids.ui_scale_slider, ui);
            }

            // Crosshair Options
            // Crosshair Types
            // Round
            if Button::image(if let CrosshairType::Round = crosshair_type {
                self.imgs.crosshair_bg_pressed
            } else {
                self.imgs.crosshair_bg
            })
            .w_h(15.0 * 4.0, 15.0 * 4.0)
            .hover_image(if let CrosshairType::Round = crosshair_type {
                self.imgs.crosshair_bg_pressed
            } else {
                self.imgs.crosshair_bg_hover
            })
            .press_image(if let CrosshairType::Round = crosshair_type {
                self.imgs.crosshair_bg_pressed
            } else {
                self.imgs.crosshair_bg_press
            })
            .down_from(state.ids.ch_title, 20.0)
            .set(state.ids.ch_1_bg, ui)
            .was_clicked()
            {
                events.push(Event::CrosshairType(CrosshairType::Round));
            }

            // Crosshair
            Image::new(self.imgs.crosshair_outer_round)
                .w_h(20.0 * 1.5, 20.0 * 1.5)
                .middle_of(state.ids.ch_1_bg)
                .color(Some(Color::Rgba(
                    1.0,
                    1.0,
                    1.0,
                    self.global_state.settings.gameplay.crosshair_transp,
                )))
                .graphics_for(state.ids.ch_1_bg)
                .set(state.ids.crosshair_outer_1, ui);
            Image::new(self.imgs.crosshair_inner)
                .w_h(21.0 * 2.0, 21.0 * 2.0)
                .middle_of(state.ids.crosshair_outer_1)
                .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.6)))
                .graphics_for(state.ids.ch_1_bg)
                .set(state.ids.crosshair_inner_1, ui);

            // Rounded Edges
            if Button::image(if let CrosshairType::RoundEdges = crosshair_type {
                self.imgs.crosshair_bg_pressed
            } else {
                self.imgs.crosshair_bg
            })
            .w_h(15.0 * 4.0, 15.0 * 4.0)
            .hover_image(if let CrosshairType::RoundEdges = crosshair_type {
                self.imgs.crosshair_bg_pressed
            } else {
                self.imgs.crosshair_bg_hover
            })
            .press_image(if let CrosshairType::RoundEdges = crosshair_type {
                self.imgs.crosshair_bg_pressed
            } else {
                self.imgs.crosshair_bg_press
            })
            .right_from(state.ids.ch_1_bg, 20.0)
            .set(state.ids.ch_2_bg, ui)
            .was_clicked()
            {
                events.push(Event::CrosshairType(CrosshairType::RoundEdges));
            }

            // Crosshair
            Image::new(self.imgs.crosshair_outer_round_edges)
                .w_h(21.0 * 1.5, 21.0 * 1.5)
                .middle_of(state.ids.ch_2_bg)
                .color(Some(Color::Rgba(
                    1.0,
                    1.0,
                    1.0,
                    self.global_state.settings.gameplay.crosshair_transp,
                )))
                .graphics_for(state.ids.ch_2_bg)
                .set(state.ids.crosshair_outer_2, ui);
            Image::new(self.imgs.crosshair_inner)
                .w_h(21.0 * 2.0, 21.0 * 2.0)
                .middle_of(state.ids.crosshair_outer_2)
                .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.6)))
                .graphics_for(state.ids.ch_2_bg)
                .set(state.ids.crosshair_inner_2, ui);

            // Edges
            if Button::image(if let CrosshairType::Edges = crosshair_type {
                self.imgs.crosshair_bg_pressed
            } else {
                self.imgs.crosshair_bg
            })
            .w_h(15.0 * 4.0, 15.0 * 4.0)
            .hover_image(if let CrosshairType::Edges = crosshair_type {
                self.imgs.crosshair_bg_pressed
            } else {
                self.imgs.crosshair_bg_hover
            })
            .press_image(if let CrosshairType::Edges = crosshair_type {
                self.imgs.crosshair_bg_pressed
            } else {
                self.imgs.crosshair_bg_press
            })
            .right_from(state.ids.ch_2_bg, 20.0)
            .set(state.ids.ch_3_bg, ui)
            .was_clicked()
            {
                events.push(Event::CrosshairType(CrosshairType::Edges));
            }

            // Crosshair
            Image::new(self.imgs.crosshair_outer_edges)
                .w_h(21.0 * 1.5, 21.0 * 1.5)
                .middle_of(state.ids.ch_3_bg)
                .color(Some(Color::Rgba(
                    1.0,
                    1.0,
                    1.0,
                    self.global_state.settings.gameplay.crosshair_transp,
                )))
                .graphics_for(state.ids.ch_3_bg)
                .set(state.ids.crosshair_outer_3, ui);
            Image::new(self.imgs.crosshair_inner)
                .w_h(21.0 * 2.0, 21.0 * 2.0)
                .middle_of(state.ids.crosshair_outer_3)
                .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.6)))
                .graphics_for(state.ids.ch_3_bg)
                .set(state.ids.crosshair_inner_3, ui);
            // Crosshair Transparency Text and Slider
            Text::new("Crosshair")
                .down_from(state.ids.absolute_scale_button, 20.0)
                .font_size(18)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(state.ids.ch_title, ui);
            Text::new("Transparency")
                .right_from(state.ids.ch_3_bg, 20.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(state.ids.ch_transp_text, ui);

            if let Some(new_val) = ImageSlider::continuous(
                crosshair_transp,
                0.0,
                1.0,
                self.imgs.slider_indicator,
                self.imgs.slider,
            )
            .w_h(104.0, 22.0)
            .down_from(state.ids.ch_transp_text, 8.0)
            .track_breadth(12.0)
            .slider_length(10.0)
            .pad_track((5.0, 5.0))
            .set(state.ids.ch_transp_slider, ui)
            {
                events.push(Event::CrosshairTransp(new_val));
            }

            Text::new(&format!("{:.2}", crosshair_transp,))
                .right_from(state.ids.ch_transp_slider, 8.0)
                .font_size(14)
                .graphics_for(state.ids.ch_transp_slider)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(state.ids.ch_transp_value, ui);

            // Hotbar text
            Text::new("Hotbar")
                .down_from(state.ids.ch_1_bg, 20.0)
                .font_size(18)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(state.ids.hotbar_title, ui);
            // Show xp bar
            if Button::image(match self.global_state.settings.gameplay.xp_bar {
                XpBar::Always => self.imgs.checkbox_checked,
                XpBar::OnGain => self.imgs.checkbox,
            })
            .w_h(18.0, 18.0)
            .hover_image(match self.global_state.settings.gameplay.xp_bar {
                XpBar::Always => self.imgs.checkbox_checked_mo,
                XpBar::OnGain => self.imgs.checkbox_mo,
            })
            .press_image(match self.global_state.settings.gameplay.xp_bar {
                XpBar::Always => self.imgs.checkbox_checked,
                XpBar::OnGain => self.imgs.checkbox_press,
            })
            .down_from(state.ids.hotbar_title, 8.0)
            .set(state.ids.show_xpbar_button, ui)
            .was_clicked()
            {
                match self.global_state.settings.gameplay.xp_bar {
                    XpBar::Always => events.push(Event::ToggleXpBar(XpBar::OnGain)),
                    XpBar::OnGain => events.push(Event::ToggleXpBar(XpBar::Always)),
                }
            }
            Text::new("Always show Experience Bar")
                .right_from(state.ids.show_xpbar_button, 10.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .graphics_for(state.ids.show_xpbar_button)
                .color(TEXT_COLOR)
                .set(state.ids.show_xpbar_text, ui);
            // Show Shortcut Numbers
            if Button::image(match self.global_state.settings.gameplay.shortcut_numbers {
                ShortcutNumbers::On => self.imgs.checkbox_checked,
                ShortcutNumbers::Off => self.imgs.checkbox,
            })
            .w_h(18.0, 18.0)
            .hover_image(match self.global_state.settings.gameplay.shortcut_numbers {
                ShortcutNumbers::On => self.imgs.checkbox_checked_mo,
                ShortcutNumbers::Off => self.imgs.checkbox_mo,
            })
            .press_image(match self.global_state.settings.gameplay.shortcut_numbers {
                ShortcutNumbers::On => self.imgs.checkbox_checked,
                ShortcutNumbers::Off => self.imgs.checkbox_press,
            })
            .down_from(state.ids.show_xpbar_button, 8.0)
            .set(state.ids.show_shortcuts_button, ui)
            .was_clicked()
            {
                match self.global_state.settings.gameplay.shortcut_numbers {
                    ShortcutNumbers::On => {
                        events.push(Event::ToggleShortcutNumbers(ShortcutNumbers::Off))
                    }
                    ShortcutNumbers::Off => {
                        events.push(Event::ToggleShortcutNumbers(ShortcutNumbers::On))
                    }
                }
            }
            Text::new("Always show Shortcuts")
                .right_from(state.ids.show_shortcuts_button, 10.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .graphics_for(state.ids.show_shortcuts_button)
                .color(TEXT_COLOR)
                .set(state.ids.show_shortcuts_text, ui);

            // Energybars Numbers
            // Hotbar text
            Text::new("Energybar Numbers")
                .down_from(state.ids.show_shortcuts_button, 20.0)
                .font_size(18)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(state.ids.bar_numbers_title, ui);

            // None
            if Button::image(if let BarNumbers::Off = bar_values {
                self.imgs.check_checked
            } else {
                self.imgs.check
            })
            .w_h(288.0 / 24.0, 288.0 / 24.0)
            .hover_image(if let BarNumbers::Off = bar_values {
                self.imgs.check_checked_mo
            } else {
                self.imgs.check_mo
            })
            .press_image(if let BarNumbers::Off = bar_values {
                self.imgs.check_checked
            } else {
                self.imgs.check_press
            })
            .down_from(state.ids.bar_numbers_title, 8.0)
            .set(state.ids.show_bar_numbers_none_button, ui)
            .was_clicked()
            {
                events.push(Event::ToggleBarNumbers(BarNumbers::Off))
            }
            Text::new("None")
                .right_from(state.ids.show_bar_numbers_none_button, 10.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .graphics_for(state.ids.show_bar_numbers_none_button)
                .color(TEXT_COLOR)
                .set(state.ids.show_bar_numbers_none_text, ui);

            // Values
            if Button::image(if let BarNumbers::Values = bar_values {
                self.imgs.check_checked
            } else {
                self.imgs.check
            })
            .w_h(288.0 / 24.0, 288.0 / 24.0)
            .hover_image(if let BarNumbers::Values = bar_values {
                self.imgs.check_checked_mo
            } else {
                self.imgs.check_mo
            })
            .press_image(if let BarNumbers::Values = bar_values {
                self.imgs.check_checked
            } else {
                self.imgs.check_press
            })
            .down_from(state.ids.show_bar_numbers_none_button, 8.0)
            .set(state.ids.show_bar_numbers_values_button, ui)
            .was_clicked()
            {
                events.push(Event::ToggleBarNumbers(BarNumbers::Values))
            }
            Text::new("Values")
                .right_from(state.ids.show_bar_numbers_values_button, 10.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .graphics_for(state.ids.show_bar_numbers_values_button)
                .color(TEXT_COLOR)
                .set(state.ids.show_bar_numbers_values_text, ui);

            // Percentages
            if Button::image(if let BarNumbers::Percent = bar_values {
                self.imgs.check_checked
            } else {
                self.imgs.check
            })
            .w_h(288.0 / 24.0, 288.0 / 24.0)
            .hover_image(if let BarNumbers::Percent = bar_values {
                self.imgs.check_checked_mo
            } else {
                self.imgs.check_mo
            })
            .press_image(if let BarNumbers::Percent = bar_values {
                self.imgs.check_checked
            } else {
                self.imgs.check_press
            })
            .down_from(state.ids.show_bar_numbers_values_button, 8.0)
            .set(state.ids.show_bar_numbers_percentage_button, ui)
            .was_clicked()
            {
                events.push(Event::ToggleBarNumbers(BarNumbers::Percent))
            }
            Text::new("Percentages")
                .right_from(state.ids.show_bar_numbers_percentage_button, 10.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .graphics_for(state.ids.show_bar_numbers_percentage_button)
                .color(TEXT_COLOR)
                .set(state.ids.show_bar_numbers_percentage_text, ui);

            Rectangle::fill_with([20.0 * 4.0, 1.0 * 4.0], color::TRANSPARENT)
                .down_from(state.ids.show_bar_numbers_percentage_button, 8.0)
                .set(state.ids.placeholder, ui);
        }

        // 2) Gameplay Tab --------------------------------
        if Button::image(if let SettingsTab::Gameplay = self.show.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button
        })
        .w_h(31.0 * 4.0, 12.0 * 4.0)
        .hover_image(if let SettingsTab::Gameplay = self.show.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button_hover
        })
        .press_image(if let SettingsTab::Gameplay = self.show.settings_tab {
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
            events.push(Event::ChangeTab(SettingsTab::Gameplay));
        }

        // Contents
        if let SettingsTab::Gameplay = self.show.settings_tab {
            let display_pan = self.global_state.settings.gameplay.pan_sensitivity;
            let display_zoom = self.global_state.settings.gameplay.zoom_sensitivity;

            // Mouse Pan Sensitivity
            Text::new("Pan Sensitivity")
                .top_left_with_margins_on(state.ids.settings_content, 10.0, 10.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(state.ids.mouse_pan_label, ui);

            if let Some(new_val) = ImageSlider::discrete(
                display_pan,
                1,
                200,
                self.imgs.slider_indicator,
                self.imgs.slider,
            )
            .w_h(550.0, 22.0)
            .down_from(state.ids.mouse_pan_label, 10.0)
            .track_breadth(30.0)
            .slider_length(10.0)
            .pad_track((5.0, 5.0))
            .set(state.ids.mouse_pan_slider, ui)
            {
                events.push(Event::AdjustMousePan(new_val));
            }

            Text::new(&format!("{}", display_pan))
                .right_from(state.ids.mouse_pan_slider, 8.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(state.ids.mouse_pan_value, ui);

            // Mouse Zoom Sensitivity
            Text::new("Zoom Sensitivity")
                .down_from(state.ids.mouse_pan_slider, 10.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(state.ids.mouse_zoom_label, ui);

            if let Some(new_val) = ImageSlider::discrete(
                display_zoom,
                1,
                200,
                self.imgs.slider_indicator,
                self.imgs.slider,
            )
            .w_h(550.0, 22.0)
            .down_from(state.ids.mouse_zoom_label, 10.0)
            .track_breadth(30.0)
            .slider_length(10.0)
            .pad_track((5.0, 5.0))
            .set(state.ids.mouse_zoom_slider, ui)
            {
                events.push(Event::AdjustMouseZoom(new_val));
            }

            Text::new(&format!("{}", display_zoom))
                .right_from(state.ids.mouse_zoom_slider, 8.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(state.ids.mouse_zoom_value, ui);

            // Zoom Inversion
            let zoom_inverted = ToggleButton::new(
                self.global_state.settings.gameplay.zoom_inversion,
                self.imgs.checkbox,
                self.imgs.checkbox_checked,
            )
            .w_h(18.0, 18.0)
            .down_from(state.ids.mouse_zoom_slider, 20.0)
            .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
            .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
            .set(state.ids.mouse_zoom_invert_button, ui);

            if self.global_state.settings.gameplay.zoom_inversion != zoom_inverted {
                events.push(Event::ToggleZoomInvert(
                    !self.global_state.settings.gameplay.zoom_inversion,
                ));
            }

            Text::new("Invert Scroll Zoom")
                .right_from(state.ids.mouse_zoom_invert_button, 10.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .graphics_for(state.ids.button_help)
                .color(TEXT_COLOR)
                .set(state.ids.mouse_zoom_invert_label, ui);
        }

        // 3) Controls Tab --------------------------------
        if Button::image(if let SettingsTab::Controls = self.show.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button
        })
        .w_h(31.0 * 4.0, 12.0 * 4.0)
        .hover_image(if let SettingsTab::Controls = self.show.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button_hover
        })
        .press_image(if let SettingsTab::Controls = self.show.settings_tab {
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
            events.push(Event::ChangeTab(SettingsTab::Controls));
        }

        // Contents
        if let SettingsTab::Controls = self.show.settings_tab {
            Text::new(
                "Free Cursor\n\
            Toggle Help Window\n\
            Toggle Interface\n\
            Toggle FPS and Debug Info\n\
            Take Screenshot\n\
            Toggle Nametags\n\
            Toggle Fullscreen\n\
            \n\
            \n\
            Move Forward\n\
            Move Left\n\
            Move Right\n\
            Move Backwards\n\
            \n\
            Jump\n\
            \n\
            Glider
            \n\
            Dodge\n\
            \n\
            Auto Walk\n\
            \n\
            Sheathe/Draw Weapons\n\
            \n\
            Put on/Remove Helmet\n\
            \n\
            Sit\n\
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
            /tp [Name] - Teleports you to another player    \n\
            /jump <dx> <dy> <dz> - Offset your position \n\
            /goto <x> <y> <z> - Teleport to a position  \n\
            /kill - Kill yourself   \n\
            /pig - Spawn pig NPC    \n\
            /wolf - Spawn wolf NPC  \n\
            /help - Display chat commands
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
                 F4\n\
                 F6\n\
                 F11\n\
                 \n\
                 \n\
                 W\n\
                 A\n\
                 S\n\
                 D\n\
                 \n\
                 SPACE\n\
                 \n\
                 L-Shift\n\
                 \n\
                 ??\n\
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

        // 4) Video Tab -----------------------------------
        if Button::image(if let SettingsTab::Video = self.show.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button
        })
        .w_h(31.0 * 4.0, 12.0 * 4.0)
        .hover_image(if let SettingsTab::Video = self.show.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button_hover
        })
        .press_image(if let SettingsTab::Video = self.show.settings_tab {
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
            events.push(Event::ChangeTab(SettingsTab::Video));
        }

        // Contents
        if let SettingsTab::Video = self.show.settings_tab {
            // View Distance
            Text::new("View Distance")
                .top_left_with_margins_on(state.ids.settings_content, 10.0, 10.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(state.ids.vd_text, ui);

            if let Some(new_val) = ImageSlider::discrete(
                self.global_state.settings.graphics.view_distance,
                1,
                65,
                self.imgs.slider_indicator,
                self.imgs.slider,
            )
            .w_h(104.0, 22.0)
            .down_from(state.ids.vd_text, 8.0)
            .track_breadth(12.0)
            .slider_length(10.0)
            .pad_track((5.0, 5.0))
            .set(state.ids.vd_slider, ui)
            {
                events.push(Event::AdjustViewDistance(new_val));
            }

            Text::new(&format!(
                "{}",
                self.global_state.settings.graphics.view_distance
            ))
            .right_from(state.ids.vd_slider, 8.0)
            .font_size(14)
            .font_id(self.fonts.opensans)
            .color(TEXT_COLOR)
            .set(state.ids.vd_value, ui);

            // Max FPS
            Text::new("Maximum FPS")
                .down_from(state.ids.vd_slider, 10.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(state.ids.max_fps_text, ui);

            if let Some(which) = ImageSlider::discrete(
                FPS_CHOICES
                    .iter()
                    .position(|&x| x == self.global_state.settings.graphics.max_fps)
                    .unwrap_or(5),
                0,
                10,
                self.imgs.slider_indicator,
                self.imgs.slider,
            )
            .w_h(104.0, 22.0)
            .down_from(state.ids.max_fps_text, 8.0)
            .track_breadth(12.0)
            .slider_length(10.0)
            .pad_track((5.0, 5.0))
            .set(state.ids.max_fps_slider, ui)
            {
                events.push(Event::MaximumFPS(FPS_CHOICES[which]));
            }

            Text::new(&format!("{}", self.global_state.settings.graphics.max_fps))
                .right_from(state.ids.max_fps_slider, 8.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(state.ids.max_fps_value, ui);

            // FOV
            Text::new("Field of View (deg)")
                .down_from(state.ids.max_fps_slider, 10.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(state.ids.fov_text, ui);

            if let Some(new_val) = ImageSlider::discrete(
                self.global_state.settings.graphics.fov,
                30,
                120,
                self.imgs.slider_indicator,
                self.imgs.slider,
            )
            .w_h(104.0, 22.0)
            .down_from(state.ids.fov_text, 8.0)
            .track_breadth(12.0)
            .slider_length(10.0)
            .pad_track((5.0, 5.0))
            .set(state.ids.fov_slider, ui)
            {
                events.push(Event::AdjustFOV(new_val));
            }

            Text::new(&format!("{}", self.global_state.settings.graphics.fov))
                .right_from(state.ids.fov_slider, 8.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(state.ids.fov_value, ui);

            // AaMode
            Text::new("AntiAliasing Mode")
                .down_from(state.ids.fov_slider, 8.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(state.ids.aa_mode_text, ui);
            let mode_label_list = [
                (&AaMode::None, "No AA"),
                (&AaMode::Fxaa, "FXAA"),
                (&AaMode::MsaaX4, "MSAA x4"),
                (&AaMode::MsaaX8, "MSAA x8"),
                (&AaMode::MsaaX16, "MSAA x16 (experimental)"),
                (&AaMode::SsaaX4, "SSAA x4"),
            ];
            if let Some((_, mode)) = RadioList::new(
                (0..mode_label_list.len())
                    .find(|i| *mode_label_list[*i].0 == self.global_state.settings.graphics.aa_mode)
                    .unwrap_or(0),
                self.imgs.check,
                self.imgs.check_checked,
                &mode_label_list,
            )
            .hover_images(self.imgs.check_mo, self.imgs.check_checked_mo)
            .press_images(self.imgs.check_press, self.imgs.check_press)
            .down_from(state.ids.aa_mode_text, 8.0)
            .text_color(TEXT_COLOR)
            .font_size(12)
            .set(state.ids.aa_radio_buttons, ui)
            {
                events.push(Event::ChangeAaMode(*mode))
            }
        }

        // 5) Sound Tab -----------------------------------
        if Button::image(if let SettingsTab::Sound = self.show.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button
        })
        .w_h(31.0 * 4.0, 12.0 * 4.0)
        .hover_image(if let SettingsTab::Sound = self.show.settings_tab {
            self.imgs.settings_button_pressed
        } else {
            self.imgs.settings_button_hover
        })
        .press_image(if let SettingsTab::Sound = self.show.settings_tab {
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
            events.push(Event::ChangeTab(SettingsTab::Sound));
        }

        // Contents
        if let SettingsTab::Sound = self.show.settings_tab {
            // Music Volume -----------------------------------------------------
            Text::new("Music Volume")
                .top_left_with_margins_on(state.ids.settings_content, 10.0, 10.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(state.ids.audio_volume_text, ui);

            if let Some(new_val) = ImageSlider::continuous(
                self.global_state.settings.audio.music_volume,
                0.0,
                1.0,
                self.imgs.slider_indicator,
                self.imgs.slider,
            )
            .w_h(104.0, 22.0)
            .down_from(state.ids.audio_volume_text, 10.0)
            .track_breadth(12.0)
            .slider_length(10.0)
            .pad_track((5.0, 5.0))
            .set(state.ids.audio_volume_slider, ui)
            {
                events.push(Event::AdjustMusicVolume(new_val));
            }

            // SFX Volume -------------------------------------------------------
            Text::new("Sound Effects Volume")
                .down_from(state.ids.audio_volume_slider, 10.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(state.ids.sfx_volume_text, ui);

            if let Some(new_val) = ImageSlider::continuous(
                self.global_state.settings.audio.sfx_volume,
                0.0,
                1.0,
                self.imgs.slider_indicator,
                self.imgs.slider,
            )
            .w_h(104.0, 22.0)
            .down_from(state.ids.sfx_volume_text, 10.0)
            .track_breadth(12.0)
            .slider_length(10.0)
            .pad_track((5.0, 5.0))
            .set(state.ids.sfx_volume_slider, ui)
            {
                events.push(Event::AdjustSfxVolume(new_val));
            }

            // Audio Device Selector --------------------------------------------
            let device = &self.global_state.audio.device;
            let device_list = &self.global_state.audio.device_list;
            Text::new("Audio Device")
                .down_from(state.ids.sfx_volume_slider, 10.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(state.ids.audio_device_text, ui);

            // Get which device is currently selected
            let selected = device_list.iter().position(|x| x.contains(device));

            if let Some(clicked) = DropDownList::new(&device_list, selected)
                .w_h(400.0, 22.0)
                .down_from(state.ids.audio_device_text, 10.0)
                .label_font_id(self.fonts.opensans)
                .set(state.ids.audio_device_list, ui)
            {
                let new_val = device_list[clicked].clone();
                events.push(Event::ChangeAudioDevice(new_val));
            }
        }

        events
    }
}
