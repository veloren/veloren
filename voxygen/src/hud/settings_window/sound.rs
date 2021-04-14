use super::{RESET_BUTTONS_HEIGHT, RESET_BUTTONS_WIDTH};

use crate::{
    hud::{img_ids::Imgs, TEXT_COLOR},
    i18n::Localization,
    session::settings_change::Audio as AudioChange,
    ui::{fonts::Fonts, ImageSlider},
    GlobalState,
};
use conrod_core::{
    color,
    position::Relative,
    widget::{self, Button, Rectangle, Scrollbar, Text},
    widget_ids, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};

widget_ids! {
    struct Ids {
        window,
        window_r,
        window_scrollbar,
        reset_sound_button,
        audio_volume_slider,
        audio_volume_text,
        sfx_volume_slider,
        sfx_volume_text,
        audio_device_list,
        audio_device_text,
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

    #[allow(clippy::unused_unit)] // TODO: Pending review in #587
    fn style(&self) -> Self::Style { () }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
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

        // Music Volume -----------------------------------------------------
        Text::new(&self.localized_strings.get("hud.settings.music_volume"))
            .top_left_with_margins_on(state.ids.window, 10.0, 10.0)
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
            events.push(AudioChange::AdjustMusicVolume(new_val));
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
            events.push(AudioChange::AdjustSfxVolume(new_val));
        }

        // Audio Device Selector
        // --------------------------------------------
        // let device = &self.global_state.audio.device;
        //let device_list = &self.global_state.audio.device_list;
        //Text::new(&self.localized_strings.get("hud.settings.audio_device"
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
        //    events.push(AudioChange::ChangeAudioDevice(new_val));
        //}

        // Reset the sound settings to the default settings
        if Button::image(self.imgs.button)
            .w_h(RESET_BUTTONS_WIDTH, RESET_BUTTONS_HEIGHT)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .down_from(state.ids.sfx_volume_slider, 12.0)
            .label(&self.localized_strings.get("hud.settings.reset_sound"))
            .label_font_size(self.fonts.cyri.scale(14))
            .label_color(TEXT_COLOR)
            .label_font_id(self.fonts.cyri.conrod_id)
            .label_y(Relative::Scalar(2.0))
            .set(state.ids.reset_sound_button, ui)
            .was_clicked()
        {
            events.push(AudioChange::ResetAudioSettings);
        }

        events
    }
}
