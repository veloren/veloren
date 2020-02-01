use super::{img_ids::Imgs, Fonts, Show, TEXT_COLOR};
use conrod_core::{
    color,
    widget::{self, Button, Image, Rectangle, Text},
    widget_ids, Colorable, Positionable, Sizeable, Widget, WidgetCommon,
};

use client::{self, Client};

use crate::i18n::VoxygenLocalization;

widget_ids! {
    pub struct Ids {
        spell_frame,
        spell_close,
        spell_title,
        frame,
        content_align,
    }
}

#[derive(WidgetCommon)]
pub struct Spell<'a> {
    _show: &'a Show,
    _client: &'a Client,

    imgs: &'a Imgs,
    fonts: &'a Fonts,
    localized_strings: &'a std::sync::Arc<VoxygenLocalization>,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> Spell<'a> {
    pub fn new(
        show: &'a Show,
        _client: &'a Client,
        imgs: &'a Imgs,
        fonts: &'a Fonts,
        localized_strings: &'a std::sync::Arc<VoxygenLocalization>,
    ) -> Self {
        Self {
            _show: show,
            _client,
            imgs,
            fonts,
            localized_strings,
            common: widget::CommonBuilder::default(),
        }
    }
}

/*pub struct State {
    ids: Ids,
}*/

pub enum Event {
    Close,
}

impl<'a> Widget for Spell<'a> {
    type Event = Option<Event>;
    type State = Ids;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State { Ids::new(id_gen) }

    fn style(&self) -> Self::Style { () }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs {
            id: _, state, ui, ..
        } = args;

        if self._show.character_window {
            Image::new(self.imgs.window_3)
                .top_left_with_margins_on(ui.window, 200.0, 658.0)
                .w_h(103.0 * 4.0, 122.0 * 4.0)
                .set(state.spell_frame, ui);
        } else {
            Image::new(self.imgs.window_3)
                .top_left_with_margins_on(ui.window, 200.0, 25.0)
                .w_h(103.0 * 4.0, 122.0 * 4.0)
                .set(state.spell_frame, ui);
        }

        // X-Button
        if Button::image(self.imgs.close_button)
            .w_h(28.0, 28.0)
            .hover_image(self.imgs.close_button_hover)
            .press_image(self.imgs.close_button_press)
            .top_right_with_margins_on(state.spell_frame, 0.0, 0.0)
            .set(state.spell_close, ui)
            .was_clicked()
        {
            return Some(Event::Close);
        }

        // Title
        // TODO: Use an actual character name.
        Text::new(&self.localized_strings.get("hud.spell"))
            .mid_top_with_margin_on(state.spell_frame, 6.0)
            .font_id(self.fonts.cyri)
            .font_size(14)
            .color(TEXT_COLOR)
            .set(state.spell_title, ui);

        // Content Alignment
        Rectangle::fill_with([95.0 * 4.0, 108.0 * 4.0], color::TRANSPARENT)
            .mid_top_with_margin_on(state.spell_frame, 40.0)
            .set(state.content_align, ui);

        // Contents

        // Frame

        None
    }
}
