use super::{RESET_BUTTONS_HEIGHT, RESET_BUTTONS_WIDTH};

use crate::{
    hud::{img_ids::Imgs, TEXT_COLOR, TEXT_COLOR_GREY},
    session::settings_change::{Audio as AudioChange, Audio::*},
    ui::{fonts::Fonts, ImageSlider, ToggleButton},
    GlobalState,
};
use conrod_core::{
    color,
    position::{Align, Relative},
    widget::{self, Button, Rectangle, Scrollbar, Text},
    widget_ids, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};
use i18n::Localization;

widget_ids! {
    struct Ids {
        window,
        window_r,
        window_scrollbar,
        master_volume_text,
        master_volume_slider,
        master_volume_number,
        master_volume_muted,
        inactive_master_volume_text,
        inactive_master_volume_slider,
        inactive_master_volume_number,
        inactive_master_volume_muted,
        music_volume_text,
        music_volume_slider,
        music_volume_number,
        music_volume_muted,
        sfx_volume_text,
        sfx_volume_slider,
        sfx_volume_number,
        sfx_volume_muted,
        ambience_volume_text,
        ambience_volume_slider,
        ambience_volume_number,
        ambience_volume_muted,
        music_spacing_text,
        music_spacing_slider,
        music_spacing_number,
        //audio_device_list,
        //audio_device_text,
        reset_sound_button,
    }
}

#[derive(WidgetCommon)]
pub struct Sound<'a> {
    global_state: &'a GlobalState,
    imgs: &'a Imgs,
    fonts: &'a Fonts,
    localized_strings: &'a Localization,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}
impl<'a> Sound<'a> {
    pub fn new(
        global_state: &'a GlobalState,
        imgs: &'a Imgs,
        fonts: &'a Fonts,
        localized_strings: &'a Localization,
    ) -> Self {
        Self {
            global_state,
            imgs,
            fonts,
            localized_strings,
            common: widget::CommonBuilder::default(),
        }
    }
}

pub struct State {
    ids: Ids,
}

impl<'a> Widget for Sound<'a> {
    type Event = Vec<AudioChange>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        common_base::prof_span!("Sound::update");
        let widget::UpdateArgs { state, ui, .. } = args;

        let mut events = Vec::new();

        Rectangle::fill_with(args.rect.dim(), color::TRANSPARENT)
            .xy(args.rect.xy())
            .graphics_for(args.id)
            .scroll_kids()
            .scroll_kids_vertically()
            .set(state.ids.window, ui);
        Rectangle::fill_with([args.rect.w() / 2.0, args.rect.h()], color::TRANSPARENT)
            .top_right()
            .parent(state.ids.window)
            .set(state.ids.window_r, ui);
        Scrollbar::y_axis(state.ids.window)
            .thickness(5.0)
            .rgba(0.33, 0.33, 0.33, 1.0)
            .set(state.ids.window_scrollbar, ui);

        let non_master_volume_text_color =
            match self.global_state.settings.audio.master_volume.muted {
                true => TEXT_COLOR_GREY,
                false => TEXT_COLOR,
            };

        // Master Volume
        Text::new(&self.localized_strings.get_msg("hud-settings-master_volume"))
            .top_left_with_margins_on(state.ids.window, 10.0, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.master_volume_text, ui);
        // Master Volume Muted Indicator
        let master_muted = ToggleButton::new(
            self.global_state.settings.audio.master_volume.muted,
            self.imgs.button_mute,
            self.imgs.button_muted,
        )
        .w_h(24.0, 25.0)
        .down_from(state.ids.master_volume_text, 10.0)
        .hover_images(self.imgs.button_mute_hover, self.imgs.button_muted_hover)
        .press_images(self.imgs.button_mute_press, self.imgs.button_muted_press)
        .set(state.ids.master_volume_muted, ui);
        if master_muted != self.global_state.settings.audio.master_volume.muted {
            events.push(MuteMasterVolume(master_muted));
        }
        // Master Volume Slider
        if let Some(new_val) = ImageSlider::continuous(
            self.global_state.settings.audio.master_volume.volume,
            0.0,
            1.0,
            self.imgs.slider_indicator,
            self.imgs.slider,
        )
        .w_h(104.0, 22.0)
        .right_from(state.ids.master_volume_muted, 8.0)
        .track_breadth(12.0)
        .slider_length(10.0)
        .pad_track((5.0, 5.0))
        .set(state.ids.master_volume_slider, ui)
        {
            events.push(AdjustMasterVolume(new_val));
        }
        // Master Volume Number
        Text::new(&format!(
            "{:2.0}%",
            self.global_state.settings.audio.master_volume.volume * 100.0
        ))
        .right_from(state.ids.master_volume_slider, 8.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.master_volume_number, ui);

        // Master Volume (inactive window)
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-inactive_master_volume_perc"),
        )
        .down_from(state.ids.master_volume_muted, 10.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.inactive_master_volume_text, ui);
        // Master Volume (inactive window) Muted Indicator
        let inactive_master_muted = ToggleButton::new(
            self.global_state
                .settings
                .audio
                .inactive_master_volume_perc
                .muted,
            self.imgs.button_mute,
            self.imgs.button_muted,
        )
        .w_h(24.0, 25.0)
        .down_from(state.ids.inactive_master_volume_text, 10.0)
        .hover_images(self.imgs.button_mute_hover, self.imgs.button_muted_hover)
        .press_images(self.imgs.button_mute_press, self.imgs.button_muted_press)
        .set(state.ids.inactive_master_volume_muted, ui);
        if inactive_master_muted
            != self
                .global_state
                .settings
                .audio
                .inactive_master_volume_perc
                .muted
        {
            events.push(MuteInactiveMasterVolume(inactive_master_muted));
        }
        // Master Volume (inactive window) Slider
        if let Some(new_val) = ImageSlider::continuous(
            self.global_state
                .settings
                .audio
                .inactive_master_volume_perc
                .volume,
            0.0,
            1.0,
            self.imgs.slider_indicator,
            self.imgs.slider,
        )
        .w_h(104.0, 22.0)
        .right_from(state.ids.inactive_master_volume_muted, 8.0)
        .track_breadth(12.0)
        .slider_length(10.0)
        .pad_track((5.0, 5.0))
        .set(state.ids.inactive_master_volume_slider, ui)
        {
            events.push(AdjustInactiveMasterVolume(new_val));
        }
        // Master Volume (inactive window) Number
        Text::new(&format!(
            "{:2.0}%",
            self.global_state
                .settings
                .audio
                .inactive_master_volume_perc
                .volume
                * 100.0
        ))
        .right_from(state.ids.inactive_master_volume_slider, 8.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(non_master_volume_text_color)
        .set(state.ids.inactive_master_volume_number, ui);

        // Music Volume
        Text::new(&self.localized_strings.get_msg("hud-settings-music_volume"))
            .down_from(state.ids.inactive_master_volume_muted, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.music_volume_text, ui);
        // Music Muted Indicator
        let music_muted = ToggleButton::new(
            self.global_state.settings.audio.music_volume.muted,
            self.imgs.button_mute,
            self.imgs.button_muted,
        )
        .w_h(24.0, 25.0)
        .down_from(state.ids.music_volume_text, 10.0)
        .hover_images(self.imgs.button_mute_hover, self.imgs.button_muted_hover)
        .press_images(self.imgs.button_mute_press, self.imgs.button_muted_press)
        .set(state.ids.music_volume_muted, ui);
        if music_muted != self.global_state.settings.audio.music_volume.muted {
            events.push(MuteMusicVolume(music_muted));
        }
        // Music Volume Slider
        if let Some(new_val) = ImageSlider::continuous(
            self.global_state.settings.audio.music_volume.volume,
            0.0,
            1.0,
            self.imgs.slider_indicator,
            self.imgs.slider,
        )
        .w_h(104.0, 22.0)
        .right_from(state.ids.music_volume_muted, 8.0)
        .track_breadth(12.0)
        .slider_length(10.0)
        .pad_track((5.0, 5.0))
        .set(state.ids.music_volume_slider, ui)
        {
            events.push(AdjustMusicVolume(new_val));
        }
        // Music Volume Number
        Text::new(&format!(
            "{:2.0}%",
            self.global_state.settings.audio.music_volume.volume * 100.0
        ))
        .right_from(state.ids.music_volume_slider, 8.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(non_master_volume_text_color)
        .set(state.ids.music_volume_number, ui);

        // SFX Volume
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-sound_effect_volume"),
        )
        .down_from(state.ids.music_volume_muted, 10.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.sfx_volume_text, ui);
        // SFX Volume Muted Indicator
        let sfx_muted = ToggleButton::new(
            self.global_state.settings.audio.sfx_volume.muted,
            self.imgs.button_mute,
            self.imgs.button_muted,
        )
        .w_h(24.0, 25.0)
        .down_from(state.ids.sfx_volume_text, 10.0)
        .hover_images(self.imgs.button_mute_hover, self.imgs.button_muted_hover)
        .press_images(self.imgs.button_mute_press, self.imgs.button_muted_press)
        .set(state.ids.sfx_volume_muted, ui);
        if sfx_muted != self.global_state.settings.audio.sfx_volume.muted {
            events.push(MuteSfxVolume(sfx_muted));
        }
        // SFX Volume Slider
        if let Some(new_val) = ImageSlider::continuous(
            self.global_state.settings.audio.sfx_volume.volume,
            0.0,
            1.0,
            self.imgs.slider_indicator,
            self.imgs.slider,
        )
        .w_h(104.0, 22.0)
        .right_from(state.ids.sfx_volume_muted, 8.0)
        .track_breadth(12.0)
        .slider_length(10.0)
        .pad_track((5.0, 5.0))
        .set(state.ids.sfx_volume_slider, ui)
        {
            events.push(AdjustSfxVolume(new_val));
        }
        // SFX Volume Number
        Text::new(&format!(
            "{:2.0}%",
            self.global_state.settings.audio.sfx_volume.volume * 100.0
        ))
        .right_from(state.ids.sfx_volume_slider, 8.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(non_master_volume_text_color)
        .set(state.ids.sfx_volume_number, ui);

        // Ambience Volume
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-ambience_volume"),
        )
        .down_from(state.ids.sfx_volume_muted, 10.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.ambience_volume_text, ui);
        // Ambience Volume Muted Indicator
        let ambience_muted = ToggleButton::new(
            self.global_state.settings.audio.ambience_volume.muted,
            self.imgs.button_mute,
            self.imgs.button_muted,
        )
        .w_h(24.0, 25.0)
        .down_from(state.ids.ambience_volume_text, 10.0)
        .hover_images(self.imgs.button_mute_hover, self.imgs.button_muted_hover)
        .press_images(self.imgs.button_mute_press, self.imgs.button_muted_press)
        .set(state.ids.ambience_volume_muted, ui);
        if ambience_muted != self.global_state.settings.audio.ambience_volume.muted {
            events.push(MuteAmbienceVolume(ambience_muted));
        }
        // Ambience Volume Slider
        if let Some(new_val) = ImageSlider::continuous(
            self.global_state.settings.audio.ambience_volume.volume,
            0.0,
            1.0,
            self.imgs.slider_indicator,
            self.imgs.slider,
        )
        .w_h(104.0, 22.0)
        .right_from(state.ids.ambience_volume_muted, 8.0)
        .track_breadth(12.0)
        .slider_length(10.0)
        .pad_track((5.0, 5.0))
        .set(state.ids.ambience_volume_slider, ui)
        {
            events.push(AdjustAmbienceVolume(new_val));
        }
        // Ambience Volume Number
        Text::new(&format!(
            "{:2.0}%",
            self.global_state.settings.audio.ambience_volume.volume * 100.0
        ))
        .right_from(state.ids.ambience_volume_slider, 8.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(non_master_volume_text_color)
        .set(state.ids.ambience_volume_number, ui);

        // Music spacing
        Text::new(&self.localized_strings.get_msg("hud-settings-music_spacing"))
            .down_from(state.ids.ambience_volume_muted, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.music_spacing_text, ui);
        // Music spacing Slider
        if let Some(new_val) = ImageSlider::continuous(
            self.global_state.settings.audio.music_spacing,
            0.0,
            2.0,
            self.imgs.slider_indicator,
            self.imgs.slider,
        )
        .w_h(104.0, 22.0)
        .down_from(state.ids.music_spacing_text, 10.0)
        .x_align_to(state.ids.ambience_volume_slider, Align::Start)
        .track_breadth(12.0)
        .slider_length(10.0)
        .pad_track((5.0, 5.0))
        .set(state.ids.music_spacing_slider, ui)
        {
            events.push(AdjustMusicSpacing(new_val));
        }
        // Music spacing Number
        Text::new(&format!(
            "{:1.2}x",
            self.global_state.settings.audio.music_spacing
        ))
        .right_from(state.ids.music_spacing_slider, 8.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.music_spacing_number, ui);

        // Audio Device Selector
        // --------------------------------------------
        // let device = &self.global_state.audio.device;
        //let device_list = &self.global_state.audio.device_list;
        //Text::new(self.localized_strings.get("hud.settings.audio_device"
        // ))    .down_from(state.ids.sfx_volume_slider, 10.0)
        //    .font_size(self.fonts.cyri.scale(14))
        //    .font_id(self.fonts.cyri.conrod_id)
        //    .color(TEXT_COLOR)
        //    .set(state.ids.audio_device_text, ui);

        //// Get which device is currently selected
        //let selected = device_list.iter().position(|x|
        // x.contains(device));

        //if let Some(clicked) = DropDownList::new(&device_list, selected)
        //    .w_h(400.0, 22.0)
        //    .color(MENU_BG)
        //    .label_color(TEXT_COLOR)
        //    .label_font_id(self.fonts.opensans.conrod_id)
        //    .down_from(state.ids.audio_device_text, 10.0)
        //    .set(state.ids.audio_device_list, ui)
        //{
        //    let new_val = device_list[clicked].clone();
        //    events.push(ChangeAudioDevice(new_val));
        //}

        // Reset the sound settings to the default settings
        if Button::image(self.imgs.button)
            .w_h(RESET_BUTTONS_WIDTH, RESET_BUTTONS_HEIGHT)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .down_from(state.ids.music_spacing_slider, 12.0)
            .x_align_to(state.ids.ambience_volume_text, Align::Start)
            .label(&self.localized_strings.get_msg("hud-settings-reset_sound"))
            .label_font_size(self.fonts.cyri.scale(14))
            .label_color(TEXT_COLOR)
            .label_font_id(self.fonts.cyri.conrod_id)
            .label_y(Relative::Scalar(2.0))
            .set(state.ids.reset_sound_button, ui)
            .was_clicked()
        {
            events.push(ResetAudioSettings);
        }

        events
    }
}
