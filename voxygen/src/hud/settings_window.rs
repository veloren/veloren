use super::{
    img_ids::Imgs, BarNumbers, CrosshairType, PressBehavior, ShortcutNumbers, Show, XpBar,
    CRITICAL_HP_COLOR, ERROR_COLOR, HP_COLOR, LOW_HP_COLOR, MANA_COLOR, MENU_BG,
    TEXT_BIND_CONFLICT_COLOR, TEXT_COLOR, UI_HIGHLIGHT_0, UI_MAIN,
};
use crate::{
    i18n::{list_localizations, LanguageMetadata, VoxygenLocalization},
    render::{AaMode, CloudMode, FluidMode, LightingMode, RenderMode, ShadowMapMode, ShadowMode},
    ui::{fonts::ConrodVoxygenFonts, ImageSlider, ScaleMode, ToggleButton},
    window::{FullScreenSettings, FullscreenMode, GameInput},
    GlobalState,
};
use conrod_core::{
    color,
    position::{Align, Relative},
    widget::{self, Button, DropDownList, Image, Rectangle, Scrollbar, Text},
    widget_ids, Borderable, Color, Colorable, Labelable, Positionable, Sizeable, Widget,
    WidgetCommon,
};
use core::convert::TryFrom;

use inline_tweak::*;
use itertools::Itertools;
use std::iter::once;
use winit::monitor::VideoMode;

const FPS_CHOICES: [u32; 11] = [15, 30, 40, 50, 60, 90, 120, 144, 240, 300, 500];

widget_ids! {
    struct Ids {
        frame,
        tabs_align,
        icon,
        settings_content,
        settings_content_r,
        settings_icon,
        settings_button_mo,
        settings_close,
        settings_title,
        settings_r,
        settings_l,
        settings_scrollbar,
        controls_texts[],
        controls_buttons[],
        reset_controls_button,
        controls_alignment_rectangle,
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
        languages,
        language_list[],
        languages_list,
        rectangle,
        general_txt,
        load_tips_button,
        load_tips_button_label,
        debug_button,
        debug_button_label,
        interface,
        language_text,
        mouse_pan_slider,
        mouse_pan_label,
        mouse_pan_value,
        mouse_zoom_slider,
        mouse_zoom_label,
        mouse_zoom_value,
        mouse_zoom_invert_button,
        mouse_zoom_invert_label,
        mouse_y_invert_button,
        mouse_y_invert_label,
        smooth_pan_toggle_button,
        smooth_pan_toggle_label,
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
        language,
        fps_counter,
        vd_slider,
        vd_text,
        vd_value,
        lod_detail_slider,
        lod_detail_text,
        lod_detail_value,
        sprite_dist_slider,
        sprite_dist_text,
        sprite_dist_value,
        figure_dist_slider,
        figure_dist_text,
        figure_dist_value,
        max_fps_slider,
        max_fps_text,
        max_fps_value,
        fov_slider,
        fov_text,
        fov_value,
        gamma_slider,
        gamma_text,
        gamma_value,
        ambiance_slider,
        ambiance_text,
        ambiance_value,
        aa_mode_text,
        aa_mode_list,
        cloud_mode_text,
        cloud_mode_list,
        fluid_mode_text,
        fluid_mode_list,
        fullscreen_mode_text,
        fullscreen_mode_list,
        //
        resolution,
        resolution_label,
        bit_depth,
        bit_depth_label,
        refresh_rate,
        refresh_rate_label,
        //
        particles_button,
        particles_label,
        //
        fullscreen_button,
        fullscreen_label,
        lighting_mode_text,
        lighting_mode_list,
        shadow_mode_text,
        shadow_mode_list,
        shadow_mode_map_resolution_text,
        shadow_mode_map_resolution_slider,
        shadow_mode_map_resolution_value,
        save_window_size_button,
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
        chat_transp_title,
        chat_transp_text,
        chat_transp_slider,
        chat_char_name_text,
        chat_char_name_button,
        sct_title,
        sct_show_text,
        sct_show_radio,
        sct_single_dmg_text,
        sct_single_dmg_radio,
        sct_show_batch_text,
        sct_show_batch_radio,
        sct_batched_dmg_text,
        sct_batched_dmg_radio,
        sct_inc_dmg_text,
        sct_inc_dmg_radio,
        sct_batch_inc_text,
        sct_batch_inc_radio,
        sct_num_dur_text,
        sct_num_dur_slider,
        sct_num_dur_value,
        speech_bubble_text,
        speech_bubble_dark_mode_text,
        speech_bubble_dark_mode_button,
        speech_bubble_icon_text,
        speech_bubble_icon_button,
        free_look_behavior_text,
        free_look_behavior_list,
        auto_walk_behavior_text,
        auto_walk_behavior_list,
        stop_auto_walk_on_input_button,
        stop_auto_walk_on_input_label,
    }
}

pub enum SettingsTab {
    Interface,
    Video,
    Sound,
    Gameplay,
    Controls,
    Lang,
}

#[derive(WidgetCommon)]
pub struct SettingsWindow<'a> {
    global_state: &'a GlobalState,
    show: &'a Show,
    imgs: &'a Imgs,
    fonts: &'a ConrodVoxygenFonts,
    localized_strings: &'a std::sync::Arc<VoxygenLocalization>,
    fps: f32,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> SettingsWindow<'a> {
    pub fn new(
        global_state: &'a GlobalState,
        show: &'a Show,
        imgs: &'a Imgs,
        fonts: &'a ConrodVoxygenFonts,
        localized_strings: &'a std::sync::Arc<VoxygenLocalization>,
        fps: f32,
    ) -> Self {
        Self {
            global_state,
            show,
            imgs,
            fonts,
            localized_strings,
            fps,
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
    ToggleTips(bool),
    ToggleXpBar(XpBar),
    ToggleBarNumbers(BarNumbers),
    ToggleShortcutNumbers(ShortcutNumbers),
    ChangeTab(SettingsTab),
    Close,
    AdjustMousePan(u32),
    AdjustMouseZoom(u32),
    ToggleZoomInvert(bool),
    ToggleMouseYInvert(bool),
    ToggleSmoothPan(bool),
    AdjustViewDistance(u32),
    AdjustSpriteRenderDistance(u32),
    AdjustFigureLoDRenderDistance(u32),
    AdjustFOV(u16),
    AdjustLodDetail(u32),
    AdjustGamma(f32),
    AdjustAmbiance(f32),
    AdjustWindowSize([u16; 2]),
    ChangeFullscreenMode(FullScreenSettings),
    ToggleParticlesEnabled(bool),
    ChangeRenderMode(Box<RenderMode>),
    AdjustMusicVolume(f32),
    AdjustSfxVolume(f32),
    ChangeAudioDevice(String),
    MaximumFPS(u32),
    CrosshairTransp(f32),
    CrosshairType(CrosshairType),
    UiScale(ScaleChange),
    ChatTransp(f32),
    ChatCharName(bool),
    Sct(bool),
    SctPlayerBatch(bool),
    SctDamageBatch(bool),
    SpeechBubbleDarkMode(bool),
    SpeechBubbleIcon(bool),
    ChangeLanguage(Box<LanguageMetadata>),
    ChangeBinding(GameInput),
    ResetBindings,
    ChangeFreeLookBehavior(PressBehavior),
    ChangeAutoWalkBehavior(PressBehavior),
    ChangeStopAutoWalkOnInput(bool),
}

pub enum ScaleChange {
    ToAbsolute,
    ToRelative,
    Adjust(f64),
}

impl<'a> Widget for SettingsWindow<'a> {
    type Event = Vec<Event>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    #[allow(clippy::unused_unit)] // TODO: Pending review in #587
    fn style(&self) -> Self::Style { () }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;

        let mut events = Vec::new();
        let bar_values = self.global_state.settings.gameplay.bar_numbers;
        let tab_font_scale = 18;

        //let mut xp_bar = self.global_state.settings.gameplay.xp_bar;

        // Frame
        Image::new(self.imgs.settings_bg)
            .w_h(1052.0, 886.0)
            .mid_top_with_margin_on(ui.window, 5.0)
            .color(Some(UI_MAIN))
            .set(state.ids.settings_bg, ui);

        Image::new(self.imgs.settings_frame)
            .w_h(1052.0, 886.0)
            .middle_of(state.ids.settings_bg)
            .color(Some(UI_HIGHLIGHT_0))
            .set(state.ids.frame, ui);

        // Content Alignment
        Rectangle::fill_with([814.0, 834.0], color::TRANSPARENT)
            .top_right_with_margins_on(state.ids.frame, 46.0, 2.0)
            .scroll_kids()
            .scroll_kids_vertically()
            .set(state.ids.settings_content, ui);
        Rectangle::fill_with([814.0 / 2.0, 834.0], color::TRANSPARENT)
            .top_right_with_margins_on(state.ids.settings_content, 0.0, 0.0)
            .set(state.ids.settings_content_r, ui);
        Scrollbar::y_axis(state.ids.settings_content)
            .thickness(5.0)
            .rgba(0.33, 0.33, 0.33, 1.0)
            .set(state.ids.settings_scrollbar, ui);

        // Tabs Content Alignment
        Rectangle::fill_with([232.0, 814.0], color::TRANSPARENT)
            .top_left_with_margins_on(state.ids.frame, 44.0, 2.0)
            .scroll_kids()
            .scroll_kids_vertically()
            .set(state.ids.tabs_align, ui);

        // Icon
        Image::new(self.imgs.settings)
            .w_h(29.0 * 1.5, 25.0 * 1.5)
            .top_left_with_margins_on(state.ids.frame, 2.0, 1.0)
            .set(state.ids.icon, ui);
        // Title
        Text::new(match self.show.settings_tab {
            SettingsTab::Interface => self.localized_strings.get("common.interface_settings"),
            SettingsTab::Gameplay => self.localized_strings.get("common.gameplay_settings"),
            SettingsTab::Controls => self.localized_strings.get("common.controls_settings"),
            SettingsTab::Video => self.localized_strings.get("common.video_settings"),
            SettingsTab::Sound => self.localized_strings.get("common.sound_settings"),
            SettingsTab::Lang => self.localized_strings.get("common.language_settings"),
        })
        .mid_top_with_margin_on(state.ids.frame, 3.0)
        .font_id(self.fonts.cyri.conrod_id)
        .font_size(self.fonts.cyri.scale(29))
        .color(TEXT_COLOR)
        .set(state.ids.settings_title, ui);

        // X-Button
        if Button::image(self.imgs.close_button)
            .w_h(24.0, 25.0)
            .hover_image(self.imgs.close_btn_hover)
            .press_image(self.imgs.close_btn_press)
            .top_right_with_margins_on(state.ids.frame, 0.0, 0.0)
            .set(state.ids.settings_close, ui)
            .was_clicked()
        {
            events.push(Event::Close);
        }

        // 1) Interface Tab -------------------------------
        if Button::image(if let SettingsTab::Interface = self.show.settings_tab {
            self.imgs.selection
        } else {
            self.imgs.nothing
        })
        .w_h(230.0, 48.0)
        .hover_image(self.imgs.selection_hover)
        .press_image(self.imgs.selection_press)
        .mid_top_with_margin_on(state.ids.tabs_align, 28.0)
        .label(&self.localized_strings.get("common.interface"))
        .label_font_size(self.fonts.cyri.scale(tab_font_scale))
        .label_font_id(self.fonts.cyri.conrod_id)
        .label_color(TEXT_COLOR)
        .set(state.ids.interface, ui)
        .was_clicked()
        {
            events.push(Event::ChangeTab(SettingsTab::Interface));
        }

        // Contents Left Side
        if let SettingsTab::Interface = self.show.settings_tab {
            let crosshair_transp = self.global_state.settings.gameplay.crosshair_transp;
            let crosshair_type = self.global_state.settings.gameplay.crosshair_type;
            let ui_scale = self.global_state.settings.gameplay.ui_scale;
            let chat_transp = self.global_state.settings.gameplay.chat_transp;

            Text::new(&self.localized_strings.get("hud.settings.general"))
                .top_left_with_margins_on(state.ids.settings_content, 5.0, 5.0)
                .font_size(self.fonts.cyri.scale(18))
                .font_id(self.fonts.cyri.conrod_id)
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

            Text::new(&self.localized_strings.get("hud.settings.help_window"))
                .right_from(state.ids.button_help, 10.0)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .graphics_for(state.ids.button_help)
                .color(TEXT_COLOR)
                .set(state.ids.show_help_label, ui);

            // Loading Screen Tips
            let show_tips = ToggleButton::new(
                self.global_state.settings.gameplay.loading_tips,
                self.imgs.checkbox,
                self.imgs.checkbox_checked,
            )
            .w_h(18.0, 18.0)
            .down_from(state.ids.button_help, 8.0)
            .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
            .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
            .set(state.ids.load_tips_button, ui);

            if self.global_state.settings.gameplay.loading_tips != show_tips {
                events.push(Event::ToggleTips(
                    !self.global_state.settings.gameplay.loading_tips,
                ));
            }

            Text::new(&self.localized_strings.get("hud.settings.loading_tips"))
                .right_from(state.ids.load_tips_button, 10.0)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .graphics_for(state.ids.load_tips_button)
                .color(TEXT_COLOR)
                .set(state.ids.load_tips_button_label, ui);
            // Debug
            let show_debug = ToggleButton::new(
                self.show.debug,
                self.imgs.checkbox,
                self.imgs.checkbox_checked,
            )
            .w_h(18.0, 18.0)
            .down_from(state.ids.load_tips_button, 8.0)
            .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
            .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
            .set(state.ids.debug_button, ui);

            if self.show.debug != show_debug {
                events.push(Event::ToggleDebug);
            }

            Text::new(&self.localized_strings.get("hud.settings.debug_info"))
                .right_from(state.ids.debug_button, 10.0)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .graphics_for(state.ids.debug_button)
                .color(TEXT_COLOR)
                .set(state.ids.debug_button_label, ui);

            // Ui Scale
            Text::new(&self.localized_strings.get("hud.settings.ui_scale"))
                .down_from(state.ids.debug_button, 20.0)
                .font_size(self.fonts.cyri.scale(18))
                .font_id(self.fonts.cyri.conrod_id)
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

            Text::new(self.localized_strings.get("hud.settings.relative_scaling"))
                .right_from(state.ids.relative_to_win_button, 10.0)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
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

            Text::new(self.localized_strings.get("hud.settings.custom_scaling"))
                .right_from(state.ids.absolute_scale_button, 10.0)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .graphics_for(state.ids.absolute_scale_button)
                .color(TEXT_COLOR)
                .set(state.ids.absolute_scale_text, ui);

            // Slider -> Inactive when "Relative to window" is selected
            if let ScaleMode::Absolute(scale) = ui_scale {
                if let Some(new_val) = ImageSlider::continuous(
                    scale.log(2.0),
                    0.5f64.log(2.0),
                    1.0f64.log(2.0),
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
                    .font_size(self.fonts.cyri.scale(14))
                    .font_id(self.fonts.cyri.conrod_id)
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
            Text::new(&self.localized_strings.get("hud.settings.crosshair"))
                .down_from(state.ids.absolute_scale_button, 20.0)
                .font_size(self.fonts.cyri.scale(18))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.ch_title, ui);
            Text::new(&self.localized_strings.get("hud.settings.transparency"))
                .right_from(state.ids.ch_3_bg, 20.0)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
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
                .font_size(self.fonts.cyri.scale(14))
                .graphics_for(state.ids.ch_transp_slider)
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.ch_transp_value, ui);

            // Hotbar text
            Text::new(&self.localized_strings.get("hud.settings.hotbar"))
                .down_from(state.ids.ch_1_bg, 20.0)
                .font_size(self.fonts.cyri.scale(18))
                .font_id(self.fonts.cyri.conrod_id)
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
            Text::new(
                &self
                    .localized_strings
                    .get("hud.settings.toggle_bar_experience"),
            )
            .right_from(state.ids.show_xpbar_button, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
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
                    },
                    ShortcutNumbers::Off => {
                        events.push(Event::ToggleShortcutNumbers(ShortcutNumbers::On))
                    },
                }
            }
            Text::new(&self.localized_strings.get("hud.settings.toggle_shortcuts"))
                .right_from(state.ids.show_shortcuts_button, 10.0)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .graphics_for(state.ids.show_shortcuts_button)
                .color(TEXT_COLOR)
                .set(state.ids.show_shortcuts_text, ui);

            Rectangle::fill_with([60.0 * 4.0, 1.0 * 4.0], color::TRANSPARENT)
                .down_from(state.ids.show_shortcuts_text, 30.0)
                .set(state.ids.placeholder, ui);

            // Content Right Side

            /*Scrolling Combat text

            O Show Damage Numbers
                O Show single Damage Numbers
                O Show batched dealt Damage
                O Show incoming Damage
                    O Batch incoming Numbers

            Number Display Duration: 1s ----I----5s
             */
            // SCT/ Scrolling Combat Text
            Text::new(
                &self
                    .localized_strings
                    .get("hud.settings.scrolling_combat_text"),
            )
            .top_left_with_margins_on(state.ids.settings_content_r, 5.0, 5.0)
            .font_size(self.fonts.cyri.scale(18))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.sct_title, ui);
            // Generally toggle the SCT
            let show_sct = ToggleButton::new(
                self.global_state.settings.gameplay.sct,
                self.imgs.checkbox,
                self.imgs.checkbox_checked,
            )
            .w_h(18.0, 18.0)
            .down_from(state.ids.sct_title, 20.0)
            .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
            .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
            .set(state.ids.sct_show_radio, ui);

            if self.global_state.settings.gameplay.sct != show_sct {
                events.push(Event::Sct(!self.global_state.settings.gameplay.sct))
            }
            Text::new(
                &self
                    .localized_strings
                    .get("hud.settings.scrolling_combat_text"),
            )
            .right_from(state.ids.sct_show_radio, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .graphics_for(state.ids.sct_show_radio)
            .color(TEXT_COLOR)
            .set(state.ids.sct_show_text, ui);
            if self.global_state.settings.gameplay.sct {
                // Toggle single damage numbers
                let show_sct_damage_batch = !ToggleButton::new(
                    !self.global_state.settings.gameplay.sct_damage_batch,
                    self.imgs.checkbox,
                    self.imgs.checkbox_checked,
                )
                .w_h(18.0, 18.0)
                .down_from(state.ids.sct_show_text, 8.0)
                .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
                .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
                .set(state.ids.sct_single_dmg_radio, ui);

                Text::new(
                    &self
                        .localized_strings
                        .get("hud.settings.single_damage_number"),
                )
                .right_from(state.ids.sct_single_dmg_radio, 10.0)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .graphics_for(state.ids.sct_single_dmg_radio)
                .color(TEXT_COLOR)
                .set(state.ids.sct_single_dmg_text, ui);
                // Toggle Batched Damage
                let show_sct_damage_batch = ToggleButton::new(
                    show_sct_damage_batch,
                    self.imgs.checkbox,
                    self.imgs.checkbox_checked,
                )
                .w_h(18.0, 18.0)
                .down_from(state.ids.sct_single_dmg_radio, 8.0)
                .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
                .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
                .set(state.ids.sct_show_batch_radio, ui);

                if self.global_state.settings.gameplay.sct_damage_batch != show_sct_damage_batch {
                    events.push(Event::SctDamageBatch(
                        !self.global_state.settings.gameplay.sct_damage_batch,
                    ))
                }
                Text::new(&self.localized_strings.get("hud.settings.cumulated_damage"))
                    .right_from(state.ids.sct_show_batch_radio, 10.0)
                    .font_size(self.fonts.cyri.scale(14))
                    .font_id(self.fonts.cyri.conrod_id)
                    .graphics_for(state.ids.sct_batched_dmg_radio)
                    .color(TEXT_COLOR)
                    .set(state.ids.sct_show_batch_text, ui);
                // Toggle Incoming Damage
                let show_sct_player_batch = !ToggleButton::new(
                    !self.global_state.settings.gameplay.sct_player_batch,
                    self.imgs.checkbox,
                    self.imgs.checkbox_checked,
                )
                .w_h(18.0, 18.0)
                .down_from(state.ids.sct_show_batch_radio, 8.0)
                .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
                .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
                .set(state.ids.sct_inc_dmg_radio, ui);

                Text::new(&self.localized_strings.get("hud.settings.incoming_damage"))
                    .right_from(state.ids.sct_inc_dmg_radio, 10.0)
                    .font_size(self.fonts.cyri.scale(14))
                    .font_id(self.fonts.cyri.conrod_id)
                    .graphics_for(state.ids.sct_inc_dmg_radio)
                    .color(TEXT_COLOR)
                    .set(state.ids.sct_inc_dmg_text, ui);
                // Toggle Batched Incoming Damage
                let show_sct_player_batch = ToggleButton::new(
                    show_sct_player_batch,
                    self.imgs.checkbox,
                    self.imgs.checkbox_checked,
                )
                .w_h(18.0, 18.0)
                .down_from(state.ids.sct_inc_dmg_radio, 8.0)
                .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
                .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
                .set(state.ids.sct_batch_inc_radio, ui);

                if self.global_state.settings.gameplay.sct_player_batch != show_sct_player_batch {
                    events.push(Event::SctPlayerBatch(
                        !self.global_state.settings.gameplay.sct_player_batch,
                    ))
                }
                Text::new(
                    &self
                        .localized_strings
                        .get("hud.settings.cumulated_incoming_damage"),
                )
                .right_from(state.ids.sct_batch_inc_radio, 10.0)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .graphics_for(state.ids.sct_batch_inc_radio)
                .color(TEXT_COLOR)
                .set(state.ids.sct_batch_inc_text, ui);
            }

            // Speech bubble dark mode
            Text::new(&self.localized_strings.get("hud.settings.speech_bubble"))
                .down_from(
                    if self.global_state.settings.gameplay.sct {
                        state.ids.sct_batch_inc_radio
                    } else {
                        state.ids.sct_show_radio
                    },
                    20.0,
                )
                .x_align(Align::Start)
                .x_relative_to(state.ids.sct_show_text, -40.0)
                .font_size(self.fonts.cyri.scale(18))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.speech_bubble_text, ui);
            let speech_bubble_dark_mode = ToggleButton::new(
                self.global_state.settings.gameplay.speech_bubble_dark_mode,
                self.imgs.checkbox,
                self.imgs.checkbox_checked,
            )
            .down_from(state.ids.speech_bubble_text, 10.0)
            .w_h(18.0, 18.0)
            .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
            .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
            .set(state.ids.speech_bubble_dark_mode_button, ui);
            if self.global_state.settings.gameplay.speech_bubble_dark_mode
                != speech_bubble_dark_mode
            {
                events.push(Event::SpeechBubbleDarkMode(speech_bubble_dark_mode));
            }
            Text::new(
                &self
                    .localized_strings
                    .get("hud.settings.speech_bubble_dark_mode"),
            )
            .right_from(state.ids.speech_bubble_dark_mode_button, 10.0)
            .font_size(self.fonts.cyri.scale(15))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.speech_bubble_dark_mode_text, ui);
            // Speech bubble icon
            let speech_bubble_icon = ToggleButton::new(
                self.global_state.settings.gameplay.speech_bubble_icon,
                self.imgs.checkbox,
                self.imgs.checkbox_checked,
            )
            .down_from(state.ids.speech_bubble_dark_mode_button, 10.0)
            .w_h(18.0, 18.0)
            .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
            .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
            .set(state.ids.speech_bubble_icon_button, ui);
            if self.global_state.settings.gameplay.speech_bubble_icon != speech_bubble_icon {
                events.push(Event::SpeechBubbleIcon(speech_bubble_icon));
            }
            Text::new(
                &self
                    .localized_strings
                    .get("hud.settings.speech_bubble_icon"),
            )
            .right_from(state.ids.speech_bubble_icon_button, 10.0)
            .font_size(self.fonts.cyri.scale(15))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.speech_bubble_icon_text, ui);

            // Energybars Numbers
            // Hotbar text
            Text::new(&self.localized_strings.get("hud.settings.energybar_numbers"))
                .down_from(state.ids.speech_bubble_icon_button, 20.0)
                .font_size(self.fonts.cyri.scale(18))
                .font_id(self.fonts.cyri.conrod_id)
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
            Text::new(&self.localized_strings.get("hud.settings.none"))
                .right_from(state.ids.show_bar_numbers_none_button, 10.0)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
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
            Text::new(&self.localized_strings.get("hud.settings.values"))
                .right_from(state.ids.show_bar_numbers_values_button, 10.0)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
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
            Text::new(&self.localized_strings.get("hud.settings.percentages"))
                .right_from(state.ids.show_bar_numbers_percentage_button, 10.0)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .graphics_for(state.ids.show_bar_numbers_percentage_button)
                .color(TEXT_COLOR)
                .set(state.ids.show_bar_numbers_percentage_text, ui);

            // Chat Transp
            Text::new(&self.localized_strings.get("hud.settings.chat"))
                .down_from(state.ids.show_bar_numbers_percentage_button, 20.0)
                .font_size(self.fonts.cyri.scale(18))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.chat_transp_title, ui);
            Text::new(
                &self
                    .localized_strings
                    .get("hud.settings.background_transparency"),
            )
            .right_from(state.ids.chat_transp_slider, 20.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.chat_transp_text, ui);

            if let Some(new_val) = ImageSlider::continuous(
                chat_transp,
                0.0,
                0.9,
                self.imgs.slider_indicator,
                self.imgs.slider,
            )
            .w_h(104.0, 22.0)
            .down_from(state.ids.chat_transp_title, 8.0)
            .track_breadth(12.0)
            .slider_length(10.0)
            .pad_track((5.0, 5.0))
            .set(state.ids.chat_transp_slider, ui)
            {
                events.push(Event::ChatTransp(new_val));
            }

            // "Show character names in chat" toggle button
            let chat_char_name = ToggleButton::new(
                self.global_state.settings.gameplay.chat_character_name,
                self.imgs.checkbox,
                self.imgs.checkbox_checked,
            )
            .w_h(18.0, 18.0)
            .down_from(state.ids.chat_transp_slider, 20.0)
            .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
            .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
            .set(state.ids.chat_char_name_button, ui);
            if self.global_state.settings.gameplay.chat_character_name != chat_char_name {
                events.push(Event::ChatCharName(
                    !self.global_state.settings.gameplay.chat_character_name,
                ));
            }
            Text::new(
                &self
                    .localized_strings
                    .get("hud.settings.chat_character_name"),
            )
            .right_from(state.ids.chat_char_name_button, 20.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.chat_char_name_text, ui);

            // TODO Show account name in chat
        }

        // 2) Gameplay Tab --------------------------------
        if Button::image(if let SettingsTab::Gameplay = self.show.settings_tab {
            self.imgs.selection
        } else {
            self.imgs.nothing
        })
        .w_h(230.0, 48.0)
        .hover_image(self.imgs.selection_hover)
        .press_image(self.imgs.selection_press)
        .down_from(state.ids.interface, 0.0)
        .parent(state.ids.tabs_align)
        .label(&self.localized_strings.get("common.gameplay"))
        .label_font_size(self.fonts.cyri.scale(tab_font_scale))
        .label_font_id(self.fonts.cyri.conrod_id)
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
            Text::new(&self.localized_strings.get("hud.settings.pan_sensitivity"))
                .top_left_with_margins_on(state.ids.settings_content, 10.0, 10.0)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
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
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.mouse_pan_value, ui);

            // Mouse Zoom Sensitivity
            Text::new(&self.localized_strings.get("hud.settings.zoom_sensitivity"))
                .down_from(state.ids.mouse_pan_slider, 10.0)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.mouse_zoom_label, ui);

            if let Some(new_val) = ImageSlider::discrete(
                display_zoom,
                1,
                300,
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
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
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

            Text::new(
                &self
                    .localized_strings
                    .get("hud.settings.invert_scroll_zoom"),
            )
            .right_from(state.ids.mouse_zoom_invert_button, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .graphics_for(state.ids.mouse_zoom_invert_button)
            .color(TEXT_COLOR)
            .set(state.ids.mouse_zoom_invert_label, ui);

            // Mouse Y Inversion
            let mouse_y_inverted = ToggleButton::new(
                self.global_state.settings.gameplay.mouse_y_inversion,
                self.imgs.checkbox,
                self.imgs.checkbox_checked,
            )
            .w_h(18.0, 18.0)
            .right_from(state.ids.mouse_zoom_invert_label, 10.0)
            .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
            .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
            .set(state.ids.mouse_y_invert_button, ui);

            if self.global_state.settings.gameplay.mouse_y_inversion != mouse_y_inverted {
                events.push(Event::ToggleMouseYInvert(
                    !self.global_state.settings.gameplay.mouse_y_inversion,
                ));
            }

            Text::new(
                &self
                    .localized_strings
                    .get("hud.settings.invert_mouse_y_axis"),
            )
            .right_from(state.ids.mouse_y_invert_button, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .graphics_for(state.ids.mouse_y_invert_button)
            .color(TEXT_COLOR)
            .set(state.ids.mouse_y_invert_label, ui);

            // Mouse Smoothing Toggle
            let smooth_pan_enabled = ToggleButton::new(
                self.global_state.settings.gameplay.smooth_pan_enable,
                self.imgs.checkbox,
                self.imgs.checkbox_checked,
            )
            .w_h(18.0, 18.0)
            .right_from(state.ids.mouse_y_invert_label, 10.0)
            .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
            .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
            .set(state.ids.smooth_pan_toggle_button, ui);

            if self.global_state.settings.gameplay.smooth_pan_enable != smooth_pan_enabled {
                events.push(Event::ToggleSmoothPan(
                    !self.global_state.settings.gameplay.smooth_pan_enable,
                ));
            }

            Text::new(
                &self
                    .localized_strings
                    .get("hud.settings.enable_mouse_smoothing"),
            )
            .right_from(state.ids.smooth_pan_toggle_button, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .graphics_for(state.ids.smooth_pan_toggle_button)
            .color(TEXT_COLOR)
            .set(state.ids.smooth_pan_toggle_label, ui);

            // Free look behaviour
            Text::new(
                &self
                    .localized_strings
                    .get("hud.settings.free_look_behavior"),
            )
            .down_from(state.ids.mouse_zoom_invert_button, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.free_look_behavior_text, ui);

            let mode_label_list = [
                &self
                    .localized_strings
                    .get("hud.settings.press_behavior.toggle"),
                &self
                    .localized_strings
                    .get("hud.settings.press_behavior.hold"),
            ];

            // Get which free look behavior is currently active
            let selected = self.global_state.settings.gameplay.free_look_behavior as usize;

            if let Some(clicked) = DropDownList::new(&mode_label_list, Some(selected))
                .w_h(200.0, 30.0)
                .color(MENU_BG)
                .label_color(TEXT_COLOR)
                .label_font_id(self.fonts.cyri.conrod_id)
                .down_from(state.ids.free_look_behavior_text, 8.0)
                .set(state.ids.free_look_behavior_list, ui)
            {
                match clicked {
                    0 => events.push(Event::ChangeFreeLookBehavior(PressBehavior::Toggle)),
                    1 => events.push(Event::ChangeFreeLookBehavior(PressBehavior::Hold)),
                    _ => unreachable!(),
                }
            }

            // Auto walk behavior
            Text::new(
                &self
                    .localized_strings
                    .get("hud.settings.auto_walk_behavior"),
            )
            .down_from(state.ids.mouse_zoom_invert_button, 10.0)
            .right_from(state.ids.free_look_behavior_text, 150.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.auto_walk_behavior_text, ui);

            let auto_walk_selected =
                self.global_state.settings.gameplay.auto_walk_behavior as usize;

            if let Some(clicked) = DropDownList::new(&mode_label_list, Some(auto_walk_selected))
                .w_h(200.0, 30.0)
                .color(MENU_BG)
                .label_color(TEXT_COLOR)
                .label_font_id(self.fonts.cyri.conrod_id)
                .down_from(state.ids.auto_walk_behavior_text, 8.0)
                .set(state.ids.auto_walk_behavior_list, ui)
            {
                match clicked {
                    0 => events.push(Event::ChangeAutoWalkBehavior(PressBehavior::Toggle)),
                    1 => events.push(Event::ChangeAutoWalkBehavior(PressBehavior::Hold)),
                    _ => unreachable!(),
                }
            }

            // Stop autowalk on input toggle
            let stop_auto_walk_on_input_toggle = ToggleButton::new(
                self.global_state.settings.gameplay.stop_auto_walk_on_input,
                self.imgs.checkbox,
                self.imgs.checkbox_checked,
            )
            .w_h(18.0, 18.0)
            .right_from(state.ids.auto_walk_behavior_text, 80.0)
            .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
            .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
            .set(state.ids.stop_auto_walk_on_input_button, ui);

            if self.global_state.settings.gameplay.stop_auto_walk_on_input
                != stop_auto_walk_on_input_toggle
            {
                events.push(Event::ChangeStopAutoWalkOnInput(
                    !self.global_state.settings.gameplay.stop_auto_walk_on_input,
                ));
            }

            Text::new(
                &self
                    .localized_strings
                    .get("hud.settings.stop_auto_walk_on_input"),
            )
            .right_from(state.ids.stop_auto_walk_on_input_button, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .graphics_for(state.ids.stop_auto_walk_on_input_button)
            .color(TEXT_COLOR)
            .set(state.ids.stop_auto_walk_on_input_label, ui);
        }

        // 3) Controls Tab --------------------------------
        if Button::image(if let SettingsTab::Controls = self.show.settings_tab {
            self.imgs.selection
        } else {
            self.imgs.nothing
        })
        .w_h(230.0, 48.0)
        .hover_image(self.imgs.selection_hover)
        .press_image(self.imgs.selection_press)
        .down_from(state.ids.gameplay, 0.0)
        .parent(state.ids.tabs_align)
        .label(&self.localized_strings.get("common.controls"))
        .label_font_size(self.fonts.cyri.scale(tab_font_scale))
        .label_font_id(self.fonts.cyri.conrod_id)
        .label_color(TEXT_COLOR)
        .set(state.ids.controls, ui)
        .was_clicked()
        {
            events.push(Event::ChangeTab(SettingsTab::Controls));
        }

        // Contents
        if let SettingsTab::Controls = self.show.settings_tab {
            // Used for sequential placement in a flow-down pattern
            let mut previous_element_id = None;
            let mut keybindings_vec: Vec<GameInput> = GameInput::iterator().collect();
            keybindings_vec.sort();

            let controls = &self.global_state.settings.controls;
            if keybindings_vec.len() > state.ids.controls_texts.len()
                || keybindings_vec.len() > state.ids.controls_buttons.len()
            {
                state.update(|s| {
                    s.ids
                        .controls_texts
                        .resize(keybindings_vec.len(), &mut ui.widget_id_generator());
                    s.ids
                        .controls_buttons
                        .resize(keybindings_vec.len(), &mut ui.widget_id_generator());
                });
            }

            // Loop all existing keybindings and the ids for text and button widgets
            for (game_input, (&text_id, &button_id)) in keybindings_vec.into_iter().zip(
                state
                    .ids
                    .controls_texts
                    .iter()
                    .zip(state.ids.controls_buttons.iter()),
            ) {
                let (key_string, key_color) =
                    if self.global_state.window.remapping_keybindings == Some(game_input) {
                        (
                            String::from(self.localized_strings.get("hud.settings.awaitingkey")),
                            TEXT_COLOR,
                        )
                    } else if let Some(key) = controls.get_binding(game_input) {
                        (
                            key.to_string(),
                            if controls.has_conflicting_bindings(key) {
                                TEXT_BIND_CONFLICT_COLOR
                            } else {
                                TEXT_COLOR
                            },
                        )
                    } else {
                        (
                            String::from(self.localized_strings.get("hud.settings.unbound")),
                            ERROR_COLOR,
                        )
                    };
                let loc_key = self
                    .localized_strings
                    .get(game_input.get_localization_key());
                let text_widget = Text::new(loc_key)
                    .color(TEXT_COLOR)
                    .font_id(self.fonts.cyri.conrod_id)
                    .font_size(self.fonts.cyri.scale(18));
                let button_widget = Button::new()
                    .label(&key_string)
                    .label_color(key_color)
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .label_font_size(self.fonts.cyri.scale(15))
                    .w(150.0)
                    .rgba(0.0, 0.0, 0.0, 0.0)
                    .border_rgba(0.0, 0.0, 0.0, 255.0)
                    .label_y(Relative::Scalar(3.0));
                // Place top-left if it's the first text, else under the previous one
                let text_widget = match previous_element_id {
                    None => {
                        text_widget.top_left_with_margins_on(state.ids.settings_content, 10.0, 5.0)
                    },
                    Some(prev_id) => text_widget.down_from(prev_id, 10.0),
                };
                let text_width = text_widget.get_w(ui).unwrap_or(0.0);
                text_widget.set(text_id, ui);
                if button_widget
                    .right_from(text_id, 350.0 - text_width)
                    .set(button_id, ui)
                    .was_clicked()
                {
                    events.push(Event::ChangeBinding(game_input));
                }
                // Set the previous id to the current one for the next cycle
                previous_element_id = Some(text_id);
            }
            if let Some(prev_id) = previous_element_id {
                let key_string = self.localized_strings.get("hud.settings.reset_keybinds");
                let button_widget = Button::new()
                    .label(&key_string)
                    .label_color(TEXT_COLOR)
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .label_font_size(self.fonts.cyri.scale(18))
                    .down_from(prev_id, 20.0)
                    .w(200.0)
                    .rgba(0.0, 0.0, 0.0, 0.0)
                    .border_rgba(0.0, 0.0, 0.0, 255.0)
                    .label_y(Relative::Scalar(3.0))
                    .set(state.ids.reset_controls_button, ui);
                if button_widget.was_clicked() {
                    events.push(Event::ResetBindings);
                }
                previous_element_id = Some(state.ids.reset_controls_button)
            }
            // Add an empty text widget to simulate some bottom margin, because conrod sucks
            if let Some(prev_id) = previous_element_id {
                Rectangle::fill_with([1.0, 1.0], color::TRANSPARENT)
                    .down_from(prev_id, 10.0)
                    .set(state.ids.controls_alignment_rectangle, ui);
            }
        }

        // 4) Video Tab -----------------------------------
        if Button::image(if let SettingsTab::Video = self.show.settings_tab {
            self.imgs.selection
        } else {
            self.imgs.nothing
        })
        .w_h(230.0, 48.0)
        .hover_image(self.imgs.selection_hover)
        .press_image(self.imgs.selection_press)
        .down_from(state.ids.controls, 0.0)
        .parent(state.ids.tabs_align)
        .label(&self.localized_strings.get("common.video"))
        .label_font_size(self.fonts.cyri.scale(tab_font_scale))
        .label_font_id(self.fonts.cyri.conrod_id)
        .label_color(TEXT_COLOR)
        .set(state.ids.video, ui)
        .was_clicked()
        {
            events.push(Event::ChangeTab(SettingsTab::Video));
        }

        // Contents
        if let SettingsTab::Video = self.show.settings_tab {
            // FPS/TPS Counter
            //let text_col = match
            let fps_col = match self.fps as i32 {
                0..=14 => CRITICAL_HP_COLOR,
                15..=29 => LOW_HP_COLOR,
                30..=50 => HP_COLOR,
                _ => MANA_COLOR,
            };
            Text::new(&format!("FPS: {:.0}", self.fps))
                .color(fps_col)
                .top_right_with_margins_on(state.ids.settings_content_r, 10.0, 10.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(18))
                .set(state.ids.fps_counter, ui);
            // View Distance
            Text::new(&self.localized_strings.get("hud.settings.view_distance"))
                .top_left_with_margins_on(state.ids.settings_content, 10.0, 10.0)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.vd_text, ui);

            if let Some(new_val) = ImageSlider::discrete(
                self.global_state.settings.graphics.view_distance,
                1,
                // FIXME: Move back to 64 once we support multiple texture atlases, or figure out a
                // way to increase the size of the terrain atlas.
                25,
                // 65,
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
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.vd_value, ui);

            // Max FPS
            Text::new(&self.localized_strings.get("hud.settings.maximum_fps"))
                .down_from(state.ids.vd_slider, 10.0)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
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
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.max_fps_value, ui);

            // FOV
            Text::new(&self.localized_strings.get("hud.settings.fov"))
                .down_from(state.ids.max_fps_slider, 10.0)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
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
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.fov_value, ui);

            // LoD detail
            Text::new(&self.localized_strings.get("hud.settings.lod_detail"))
                .down_from(state.ids.fov_slider, 10.0)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.lod_detail_text, ui);

            if let Some(new_val) = ImageSlider::discrete(
                ((self.global_state.settings.graphics.lod_detail as f32 / 100.0).log(5.0) * 10.0)
                    .round() as i32,
                0,
                20,
                self.imgs.slider_indicator,
                self.imgs.slider,
            )
            .w_h(104.0, 22.0)
            .down_from(state.ids.lod_detail_text, 8.0)
            .track_breadth(12.0)
            .slider_length(10.0)
            .pad_track((5.0, 5.0))
            .set(state.ids.lod_detail_slider, ui)
            {
                events.push(Event::AdjustLodDetail(
                    (5.0f32.powf(new_val as f32 / 10.0) * 100.0) as u32,
                ));
            }

            Text::new(&format!(
                "{}",
                self.global_state.settings.graphics.lod_detail
            ))
            .right_from(state.ids.lod_detail_slider, 8.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.lod_detail_value, ui);

            // Gamma
            Text::new(&self.localized_strings.get("hud.settings.gamma"))
                .down_from(state.ids.lod_detail_slider, 10.0)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.gamma_text, ui);

            if let Some(new_val) = ImageSlider::discrete(
                (self.global_state.settings.graphics.gamma.log2() * 8.0).round() as i32,
                8,
                -8,
                self.imgs.slider_indicator,
                self.imgs.slider,
            )
            .w_h(104.0, 22.0)
            .down_from(state.ids.gamma_text, 8.0)
            .track_breadth(12.0)
            .slider_length(10.0)
            .pad_track((5.0, 5.0))
            .set(state.ids.gamma_slider, ui)
            {
                events.push(Event::AdjustGamma(2.0f32.powf(new_val as f32 / 8.0)));
            }

            Text::new(&format!("{:.2}", self.global_state.settings.graphics.gamma))
                .right_from(state.ids.gamma_slider, 8.0)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.gamma_value, ui);

            //Ambiance Brightness
            // 320.0 = maximum brightness in shaders
            let min_ambiance = 10.0;
            let max_ambiance = 80.0;
            if let Some(new_val) = ImageSlider::discrete(
                self.global_state.settings.graphics.ambiance.round() as i32,
                min_ambiance as i32,
                max_ambiance as i32,
                self.imgs.slider_indicator,
                self.imgs.slider,
            )
            .w_h(104.0, 22.0)
            .right_from(state.ids.gamma_slider, 50.0)
            .track_breadth(12.0)
            .slider_length(10.0)
            .pad_track((5.0, 5.0))
            .set(state.ids.ambiance_slider, ui)
            {
                events.push(Event::AdjustAmbiance(new_val as f32));
            }
            Text::new(&self.localized_strings.get("hud.settings.ambiance"))
                .up_from(state.ids.ambiance_slider, 8.0)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.ambiance_text, ui);
            Text::new(&format!(
                "{:.0}%",
                ((self.global_state.settings.graphics.ambiance - min_ambiance)
                    / (max_ambiance - min_ambiance)
                    * 100.0)
                    .round()
            ))
            .right_from(state.ids.ambiance_slider, 8.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.ambiance_value, ui);

            // Sprites VD
            if let Some(new_val) = ImageSlider::discrete(
                self.global_state.settings.graphics.sprite_render_distance,
                50,
                500,
                self.imgs.slider_indicator,
                self.imgs.slider,
            )
            .w_h(104.0, 22.0)
            .right_from(state.ids.vd_slider, 50.0)
            .track_breadth(12.0)
            .slider_length(10.0)
            .pad_track((5.0, 5.0))
            .set(state.ids.sprite_dist_slider, ui)
            {
                events.push(Event::AdjustSpriteRenderDistance(new_val));
            }
            Text::new(
                &self
                    .localized_strings
                    .get("hud.settings.sprites_view_distance"),
            )
            .up_from(state.ids.sprite_dist_slider, 8.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.sprite_dist_text, ui);

            Text::new(&format!(
                "{}",
                self.global_state.settings.graphics.sprite_render_distance
            ))
            .right_from(state.ids.sprite_dist_slider, 8.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.sprite_dist_value, ui);
            // Figure VD
            if let Some(new_val) = ImageSlider::discrete(
                self.global_state
                    .settings
                    .graphics
                    .figure_lod_render_distance,
                50,
                500,
                self.imgs.slider_indicator,
                self.imgs.slider,
            )
            .w_h(104.0, 22.0)
            .right_from(state.ids.sprite_dist_slider, 50.0)
            .track_breadth(12.0)
            .slider_length(10.0)
            .pad_track((5.0, 5.0))
            .set(state.ids.figure_dist_slider, ui)
            {
                events.push(Event::AdjustFigureLoDRenderDistance(new_val));
            }
            Text::new(
                &self
                    .localized_strings
                    .get("hud.settings.figures_view_distance"),
            )
            .up_from(state.ids.figure_dist_slider, 8.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.figure_dist_text, ui);

            Text::new(&format!(
                "{}",
                self.global_state
                    .settings
                    .graphics
                    .figure_lod_render_distance
            ))
            .right_from(state.ids.figure_dist_slider, 8.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.figure_dist_value, ui);

            let render_mode = &self.global_state.settings.graphics.render_mode;

            // AaMode
            Text::new(&self.localized_strings.get("hud.settings.antialiasing_mode"))
                .down_from(state.ids.gamma_slider, 8.0)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.aa_mode_text, ui);

            // NOTE: MSAA modes are currently disabled from the UI due to poor
            // interaction with greedy meshing, and may eventually be removed.
            let mode_list = [
                AaMode::None,
                AaMode::Fxaa,
                /* AaMode::MsaaX4,
                AaMode::MsaaX8,
                AaMode::MsaaX16, */
                AaMode::SsaaX4,
            ];
            let mode_label_list = [
                "No AA", "FXAA",
                /* "MSAA x4",
                "MSAA x8",
                "MSAA x16 (experimental)", */
                "SSAA x4",
            ];

            // Get which AA mode is currently active
            let selected = mode_list.iter().position(|x| *x == render_mode.aa);

            if let Some(clicked) = DropDownList::new(&mode_label_list, selected)
                .w_h(400.0, 22.0)
                .color(MENU_BG)
                .label_color(TEXT_COLOR)
                .label_font_id(self.fonts.cyri.conrod_id)
                .down_from(state.ids.aa_mode_text, 8.0)
                .set(state.ids.aa_mode_list, ui)
            {
                events.push(Event::ChangeRenderMode(Box::new(RenderMode {
                    aa: mode_list[clicked],
                    ..render_mode.clone()
                })));
            }

            // CloudMode
            Text::new(
                &self
                    .localized_strings
                    .get("hud.settings.cloud_rendering_mode"),
            )
            .down_from(state.ids.aa_mode_list, 8.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.cloud_mode_text, ui);

            let mode_list = [CloudMode::None, CloudMode::Regular];
            let mode_label_list = [
                &self.localized_strings.get("common.none"),
                &self
                    .localized_strings
                    .get("hud.settings.cloud_rendering_mode.regular"),
            ];

            // Get which cloud rendering mode is currently active
            let selected = mode_list.iter().position(|x| *x == render_mode.cloud);

            if let Some(clicked) = DropDownList::new(&mode_label_list, selected)
                .w_h(400.0, 22.0)
                .color(MENU_BG)
                .label_color(TEXT_COLOR)
                .label_font_id(self.fonts.cyri.conrod_id)
                .down_from(state.ids.cloud_mode_text, 8.0)
                .set(state.ids.cloud_mode_list, ui)
            {
                events.push(Event::ChangeRenderMode(Box::new(RenderMode {
                    cloud: mode_list[clicked],
                    ..render_mode.clone()
                })));
            }

            // FluidMode
            Text::new(
                &self
                    .localized_strings
                    .get("hud.settings.fluid_rendering_mode"),
            )
            .down_from(state.ids.cloud_mode_list, 8.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.fluid_mode_text, ui);

            let mode_list = [FluidMode::Cheap, FluidMode::Shiny];
            let mode_label_list = [
                &self
                    .localized_strings
                    .get("hud.settings.fluid_rendering_mode.cheap"),
                &self
                    .localized_strings
                    .get("hud.settings.fluid_rendering_mode.shiny"),
            ];

            // Get which fluid rendering mode is currently active
            let selected = mode_list.iter().position(|x| *x == render_mode.fluid);

            if let Some(clicked) = DropDownList::new(&mode_label_list, selected)
                .w_h(400.0, 22.0)
                .color(MENU_BG)
                .label_color(TEXT_COLOR)
                .label_font_id(self.fonts.cyri.conrod_id)
                .down_from(state.ids.fluid_mode_text, 8.0)
                .set(state.ids.fluid_mode_list, ui)
            {
                events.push(Event::ChangeRenderMode(Box::new(RenderMode {
                    fluid: mode_list[clicked],
                    ..render_mode.clone()
                })));
            }

            // LightingMode
            Text::new(
                &self
                    .localized_strings
                    .get("hud.settings.lighting_rendering_mode"),
            )
            .down_from(state.ids.fluid_mode_list, 8.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.lighting_mode_text, ui);

            let mode_list = [
                LightingMode::Ashikhmin,
                LightingMode::BlinnPhong,
                LightingMode::Lambertian,
            ];
            let mode_label_list = [
                &self
                    .localized_strings
                    .get("hud.settings.lighting_rendering_mode.ashikhmin"),
                &self
                    .localized_strings
                    .get("hud.settings.lighting_rendering_mode.blinnphong"),
                &self
                    .localized_strings
                    .get("hud.settings.lighting_rendering_mode.lambertian"),
            ];

            // Get which lighting rendering mode is currently active
            let selected = mode_list.iter().position(|x| *x == render_mode.lighting);

            if let Some(clicked) = DropDownList::new(&mode_label_list, selected)
                .w_h(400.0, 22.0)
                .color(MENU_BG)
                .label_color(TEXT_COLOR)
                .label_font_id(self.fonts.cyri.conrod_id)
                .down_from(state.ids.lighting_mode_text, 8.0)
                .set(state.ids.lighting_mode_list, ui)
            {
                events.push(Event::ChangeRenderMode(Box::new(RenderMode {
                    lighting: mode_list[clicked],
                    ..render_mode.clone()
                })));
            }

            // ShadowMode
            Text::new(
                &self
                    .localized_strings
                    .get("hud.settings.shadow_rendering_mode"),
            )
            .down_from(state.ids.lighting_mode_list, 8.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.shadow_mode_text, ui);

            let shadow_map_mode = ShadowMapMode::try_from(render_mode.shadow).ok();
            let mode_list = [
                ShadowMode::None,
                ShadowMode::Cheap,
                ShadowMode::Map(shadow_map_mode.unwrap_or_default()),
            ];
            let mode_label_list = [
                &self
                    .localized_strings
                    .get("hud.settings.shadow_rendering_mode.none"),
                &self
                    .localized_strings
                    .get("hud.settings.shadow_rendering_mode.cheap"),
                &self
                    .localized_strings
                    .get("hud.settings.shadow_rendering_mode.map"),
            ];

            // Get which shadow rendering mode is currently active
            let selected = mode_list.iter().position(|x| *x == render_mode.shadow);

            if let Some(clicked) = DropDownList::new(&mode_label_list, selected)
                .w_h(400.0, 22.0)
                .color(MENU_BG)
                .label_color(TEXT_COLOR)
                .label_font_id(self.fonts.cyri.conrod_id)
                .down_from(state.ids.shadow_mode_text, 8.0)
                .set(state.ids.shadow_mode_list, ui)
            {
                events.push(Event::ChangeRenderMode(Box::new(RenderMode {
                    shadow: mode_list[clicked],
                    ..render_mode.clone()
                })));
            }

            if let Some(shadow_map_mode) = shadow_map_mode {
                // Display the shadow map mode if selected.
                Text::new(
                    &self
                        .localized_strings
                        .get("hud.settings.shadow_rendering_mode.map.resolution"),
                )
                .right_from(state.ids.shadow_mode_list, 10.0)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.shadow_mode_map_resolution_text, ui);

                if let Some(new_val) = ImageSlider::discrete(
                    (shadow_map_mode.resolution.log2() * 4.0).round() as i8,
                    -8,
                    8,
                    self.imgs.slider_indicator,
                    self.imgs.slider,
                )
                .w_h(104.0, 22.0)
                .right_from(state.ids.shadow_mode_map_resolution_text, 8.0)
                .track_breadth(12.0)
                .slider_length(10.0)
                .pad_track((5.0, 5.0))
                .set(state.ids.shadow_mode_map_resolution_slider, ui)
                {
                    events.push(Event::ChangeRenderMode(Box::new(RenderMode {
                        shadow: ShadowMode::Map(ShadowMapMode {
                            resolution: 2.0f32.powf(f32::from(new_val) / 4.0),
                        }),
                        ..render_mode.clone()
                    })));
                }

                // TODO: Consider fixing to avoid allocation (it's probably not a bottleneck but
                // there's no reason to allocate for numbers).
                Text::new(&format!("{}", shadow_map_mode.resolution))
                    .right_from(state.ids.shadow_mode_map_resolution_slider, 8.0)
                    .font_size(self.fonts.cyri.scale(14))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(TEXT_COLOR)
                    .set(state.ids.shadow_mode_map_resolution_value, ui);
            }

            // Particles
            Text::new(&self.localized_strings.get("hud.settings.particles"))
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .down_from(state.ids.shadow_mode_list, 8.0)
                .color(TEXT_COLOR)
                .set(state.ids.particles_label, ui);

            let particles_enabled = ToggleButton::new(
                self.global_state.settings.graphics.particles_enabled,
                self.imgs.checkbox,
                self.imgs.checkbox_checked,
            )
            .w_h(18.0, 18.0)
            .right_from(state.ids.particles_label, 10.0)
            .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
            .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
            .set(state.ids.particles_button, ui);

            if self.global_state.settings.graphics.particles_enabled != particles_enabled {
                events.push(Event::ToggleParticlesEnabled(particles_enabled));
            }

            // Resolution, Bit Depth and Refresh Rate
            let video_modes: Vec<VideoMode> = self
                .global_state
                .window
                .window()
                .window()
                .current_monitor()
                .video_modes()
                .collect();

            // Resolution
            let resolutions: Vec<[u16; 2]> = video_modes
                .iter()
                .sorted_by_key(|mode| mode.size().height)
                .sorted_by_key(|mode| mode.size().width)
                .map(|mode| [mode.size().width as u16, mode.size().height as u16])
                .dedup()
                .collect();

            Text::new(&self.localized_strings.get("hud.settings.resolution"))
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .down_from(state.ids.particles_label, 8.0)
                .color(TEXT_COLOR)
                .set(state.ids.resolution_label, ui);

            if let Some(clicked) = DropDownList::new(
                resolutions
                    .iter()
                    .map(|res| format!("{}x{}", res[0], res[1]))
                    .collect::<Vec<String>>()
                    .as_slice(),
                resolutions.iter().position(|res| {
                    res == &self.global_state.settings.graphics.fullscreen.resolution
                }),
            )
            .w_h(128.0, 22.0)
            .color(MENU_BG)
            .label_color(TEXT_COLOR)
            .label_font_id(self.fonts.opensans.conrod_id)
            .down_from(state.ids.resolution_label, 10.0)
            .set(state.ids.resolution, ui)
            {
                events.push(Event::ChangeFullscreenMode(FullScreenSettings {
                    resolution: resolutions[clicked],
                    ..self.global_state.settings.graphics.fullscreen
                }));
            }

            // Bit Depth and Refresh Rate
            let correct_res: Vec<VideoMode> = video_modes
                .into_iter()
                .filter(|mode| {
                    mode.size().width
                        == self.global_state.settings.graphics.fullscreen.resolution[0] as u32
                })
                .filter(|mode| {
                    mode.size().height
                        == self.global_state.settings.graphics.fullscreen.resolution[1] as u32
                })
                .collect();

            // Bit Depth
            let bit_depths: Vec<u16> = correct_res
                .iter()
                .filter(
                    |mode| match self.global_state.settings.graphics.fullscreen.refresh_rate {
                        Some(refresh_rate) => mode.refresh_rate() == refresh_rate,
                        None => true,
                    },
                )
                .sorted_by_key(|mode| mode.bit_depth())
                .map(|mode| mode.bit_depth())
                .rev()
                .dedup()
                .collect();

            Text::new(&self.localized_strings.get("hud.settings.bit_depth"))
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .down_from(state.ids.particles_label, 8.0)
                .right_from(state.ids.resolution, 8.0)
                .color(TEXT_COLOR)
                .set(state.ids.bit_depth_label, ui);

            if let Some(clicked) = DropDownList::new(
                once(String::from(self.localized_strings.get("common.automatic")))
                    .chain(bit_depths.iter().map(|depth| format!("{}", depth)))
                    .collect::<Vec<String>>()
                    .as_slice(),
                match self.global_state.settings.graphics.fullscreen.bit_depth {
                    Some(bit_depth) => bit_depths
                        .iter()
                        .position(|depth| depth == &bit_depth)
                        .map(|index| index + 1),
                    None => Some(0),
                },
            )
            .w_h(128.0, 22.0)
            .color(MENU_BG)
            .label_color(TEXT_COLOR)
            .label_font_id(self.fonts.opensans.conrod_id)
            .down_from(state.ids.bit_depth_label, 10.0)
            .right_from(state.ids.resolution, 8.0)
            .set(state.ids.bit_depth, ui)
            {
                events.push(Event::ChangeFullscreenMode(FullScreenSettings {
                    bit_depth: if clicked == 0 {
                        None
                    } else {
                        Some(bit_depths[clicked - 1])
                    },
                    ..self.global_state.settings.graphics.fullscreen
                }));
            }

            // Refresh Rate
            let refresh_rates: Vec<u16> = correct_res
                .into_iter()
                .filter(
                    |mode| match self.global_state.settings.graphics.fullscreen.bit_depth {
                        Some(bit_depth) => mode.bit_depth() == bit_depth,
                        None => true,
                    },
                )
                .sorted_by_key(|mode| mode.refresh_rate())
                .map(|mode| mode.refresh_rate())
                .rev()
                .dedup()
                .collect();

            Text::new(&self.localized_strings.get("hud.settings.refresh_rate"))
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .down_from(state.ids.particles_label, 8.0)
                .right_from(state.ids.bit_depth, 8.0)
                .color(TEXT_COLOR)
                .set(state.ids.refresh_rate_label, ui);

            if let Some(clicked) = DropDownList::new(
                once(String::from(self.localized_strings.get("common.automatic")))
                    .chain(refresh_rates.iter().map(|rate| format!("{}", rate)))
                    .collect::<Vec<String>>()
                    .as_slice(),
                match self.global_state.settings.graphics.fullscreen.refresh_rate {
                    Some(refresh_rate) => refresh_rates
                        .iter()
                        .position(|rate| rate == &refresh_rate)
                        .map(|index| index + 1),
                    None => Some(0),
                },
            )
            .w_h(128.0, 22.0)
            .color(MENU_BG)
            .label_color(TEXT_COLOR)
            .label_font_id(self.fonts.opensans.conrod_id)
            .down_from(state.ids.refresh_rate_label, 10.0)
            .right_from(state.ids.bit_depth, 8.0)
            .set(state.ids.refresh_rate, ui)
            {
                events.push(Event::ChangeFullscreenMode(FullScreenSettings {
                    refresh_rate: if clicked == 0 {
                        None
                    } else {
                        Some(refresh_rates[clicked - 1])
                    },
                    ..self.global_state.settings.graphics.fullscreen
                }));
            }

            // Fullscreen
            Text::new(&self.localized_strings.get("hud.settings.fullscreen"))
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .down_from(state.ids.resolution, 8.0)
                .color(TEXT_COLOR)
                .set(state.ids.fullscreen_label, ui);

            let enabled = ToggleButton::new(
                self.global_state.settings.graphics.fullscreen.enabled,
                self.imgs.checkbox,
                self.imgs.checkbox_checked,
            )
            .w_h(18.0, 18.0)
            .right_from(state.ids.fullscreen_label, 10.0)
            .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
            .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
            .set(state.ids.fullscreen_button, ui);

            if self.global_state.settings.graphics.fullscreen.enabled != enabled {
                events.push(Event::ChangeFullscreenMode(FullScreenSettings {
                    enabled,
                    ..self.global_state.settings.graphics.fullscreen
                }));
            }

            // Fullscreen Mode
            Text::new(&self.localized_strings.get("hud.settings.fullscreen_mode"))
                .down_from(state.ids.fullscreen_label, 8.0)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.fullscreen_mode_text, ui);

            let mode_list = [FullscreenMode::Exclusive, FullscreenMode::Borderless];
            let mode_label_list = [
                &self
                    .localized_strings
                    .get("hud.settings.fullscreen_mode.exclusive"),
                &self
                    .localized_strings
                    .get("hud.settings.fullscreen_mode.borderless"),
            ];

            // Get which fullscreen mode is currently active
            let selected = mode_list
                .iter()
                .position(|x| *x == self.global_state.settings.graphics.fullscreen.mode);

            if let Some(clicked) = DropDownList::new(&mode_label_list, selected)
                .w_h(400.0, 22.0)
                .color(MENU_BG)
                .label_color(TEXT_COLOR)
                .label_font_id(self.fonts.cyri.conrod_id)
                .down_from(state.ids.fullscreen_mode_text, 8.0)
                .set(state.ids.fullscreen_mode_list, ui)
            {
                events.push(Event::ChangeFullscreenMode(FullScreenSettings {
                    mode: mode_list[clicked],
                    ..self.global_state.settings.graphics.fullscreen
                }));
            }

            // Save current screen size
            if Button::image(self.imgs.button)
                .w_h(31.0 * 5.0, 12.0 * 2.0)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .down_from(state.ids.fullscreen_mode_list, 12.0)
                .label(&self.localized_strings.get("hud.settings.save_window_size"))
                .label_font_size(self.fonts.cyri.scale(14))
                .label_color(TEXT_COLOR)
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_y(Relative::Scalar(2.0))
                .set(state.ids.save_window_size_button, ui)
                .was_clicked()
            {
                events.push(Event::AdjustWindowSize(
                    self.global_state
                        .window
                        .logical_size()
                        .map(|e| e as u16)
                        .into_array(),
                ));
            }
        }

        // 5) Sound Tab -----------------------------------
        if Button::image(if let SettingsTab::Sound = self.show.settings_tab {
            self.imgs.selection
        } else {
            self.imgs.nothing
        })
        .w_h(230.0, 48.0)
        .hover_image(self.imgs.selection_hover)
        .press_image(self.imgs.selection_press)
        .down_from(state.ids.video, 0.0)
        .parent(state.ids.tabs_align)
        .label(&self.localized_strings.get("common.sound"))
        .label_font_size(self.fonts.cyri.scale(tab_font_scale))
        .label_font_id(self.fonts.cyri.conrod_id)
        .label_color(TEXT_COLOR)
        .set(state.ids.sound, ui)
        .was_clicked()
        {
            events.push(Event::ChangeTab(SettingsTab::Sound));
        }

        // Contents
        if let SettingsTab::Sound = self.show.settings_tab {
            // Music Volume -----------------------------------------------------
            Text::new(&self.localized_strings.get("hud.settings.music_volume"))
                .top_left_with_margins_on(state.ids.settings_content, 10.0, 10.0)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
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
            Text::new(
                &self
                    .localized_strings
                    .get("hud.settings.sound_effect_volume"),
            )
            .down_from(state.ids.audio_volume_slider, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
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
            Text::new(&self.localized_strings.get("hud.settings.audio_device"))
                .down_from(state.ids.sfx_volume_slider, 10.0)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.audio_device_text, ui);

            // Get which device is currently selected
            let selected = device_list.iter().position(|x| x.contains(device));

            if let Some(clicked) = DropDownList::new(&device_list, selected)
                .w_h(400.0, 22.0)
                .color(MENU_BG)
                .label_color(TEXT_COLOR)
                .label_font_id(self.fonts.opensans.conrod_id)
                .down_from(state.ids.audio_device_text, 10.0)
                .set(state.ids.audio_device_list, ui)
            {
                let new_val = device_list[clicked].clone();
                events.push(Event::ChangeAudioDevice(new_val));
            }
        }

        // 5) Languages Tab -----------------------------------
        if Button::image(if let SettingsTab::Lang = self.show.settings_tab {
            self.imgs.selection
        } else {
            self.imgs.nothing
        })
        .w_h(230.0, 48.0)
        .hover_image(self.imgs.selection_hover)
        .press_image(self.imgs.selection_press)
        .down_from(state.ids.sound, 0.0)
        .parent(state.ids.tabs_align)
        .label(&self.localized_strings.get("common.languages"))
        .label_font_size(self.fonts.cyri.scale(tab_font_scale))
        .label_font_id(self.fonts.cyri.conrod_id)
        .label_color(TEXT_COLOR)
        .set(state.ids.language, ui)
        .was_clicked()
        {
            events.push(Event::ChangeTab(SettingsTab::Lang));
        }

        // Contents
        if let SettingsTab::Lang = self.show.settings_tab {
            // List available languages
            let selected_language = &self.global_state.settings.language.selected_language;
            let language_list = list_localizations();
            if state.ids.language_list.len() < language_list.len() {
                state.update(|state| {
                    state
                        .ids
                        .language_list
                        .resize(language_list.len(), &mut ui.widget_id_generator())
                });
            };
            for (i, language) in language_list.iter().enumerate() {
                let button_w = tweak!(400.0);
                let button_h = tweak!(50.0);
                let button = Button::image(if selected_language == &language.language_identifier {
                    self.imgs.selection
                } else {
                    self.imgs.nothing
                });
                let button = if i == 0 {
                    button.mid_top_with_margin_on(state.ids.settings_content, 20.0)
                } else {
                    button.mid_bottom_with_margin_on(state.ids.language_list[i - 1], -button_h)
                };
                if button
                    .label(&language.language_name)
                    .w_h(button_w, button_h)
                    .hover_image(self.imgs.selection_hover)
                    .press_image(self.imgs.selection_press)
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(tweak!(22)))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .label_y(conrod_core::position::Relative::Scalar(2.0))
                    .set(state.ids.language_list[i], ui)
                    .was_clicked()
                {
                    events.push(Event::ChangeLanguage(Box::new(language.to_owned())));
                }
            }
        };

        events
    }
}
