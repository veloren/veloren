use crate::{
    settings::ControlSettings,
    ui::{fonts::Fonts, Ingameable},
    window::GameInput,
};
use conrod_core::{
    widget::{self, RoundedRectangle, Text},
    widget_ids, Color, Colorable, Positionable, Widget, WidgetCommon,
};
use std::borrow::Cow;

pub const TEXT_COLOR: Color = Color::Rgba(0.61, 0.61, 0.89, 1.0);
use keyboard_keynames::key_layout::KeyLayout;

widget_ids! {
    struct Ids {
        // Name
        name_bg,
        name,
        // Key
        btn_bg,
        btn,
    }
}

/// UI widget containing everything that goes over a item
/// (Item, DistanceFromPlayer, Rarity, etc.)
#[derive(WidgetCommon)]
pub struct Overitem<'a> {
    name: Cow<'a, str>,
    quality: Color,
    distance_from_player_sqr: f32,
    fonts: &'a Fonts,
    controls: &'a ControlSettings,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    active: bool,
    key_layout: &'a Option<KeyLayout>,
}

impl<'a> Overitem<'a> {
    pub fn new(
        name: Cow<'a, str>,
        quality: Color,
        distance_from_player_sqr: f32,
        fonts: &'a Fonts,
        controls: &'a ControlSettings,
        active: bool,
        key_layout: &'a Option<KeyLayout>,
    ) -> Self {
        Self {
            name,
            quality,
            distance_from_player_sqr,
            fonts,
            controls,
            common: widget::CommonBuilder::default(),
            active,
            key_layout,
        }
    }
}

pub struct State {
    ids: Ids,
}

impl<'a> Ingameable for Overitem<'a> {
    fn prim_count(&self) -> usize {
        // Number of conrod primitives contained in the overitem display.
        // TODO maybe this could be done automatically?
        // - 2 Text for name
        // - 0 or 2 Rectangle and Text for button
        2 + match self
            .controls
            .get_binding(GameInput::Interact)
            .filter(|_| self.active)
        {
            Some(_) => 2,
            None => 0,
        }
    }
}

impl<'a> Widget for Overitem<'a> {
    type Event = ();
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { id, state, ui, .. } = args;

        let btn_color = Color::Rgba(0.0, 0.0, 0.0, 0.4);

        // Example:
        //            MUSHROOM
        //              ___
        //             | E |
        //              ———

        // Scale at max distance is 10, and at min distance is 30. Disabled since the
        // scaling ruins glyph caching, causing performance issues near lootbags
        // let scale: f64 = ((1.5
        //     - (self.distance_from_player_sqr /
        //       common::consts::MAX_PICKUP_RANGE.powi(2)))
        //     * 20.0)
        //     .into();
        let scale = 30.0;
        let text_font_size = scale * 1.0;
        let text_pos_y = scale * 1.2;
        let btn_rect_size = scale * 0.8;
        let btn_font_size = scale * 0.6;
        let btn_rect_pos_y = 0.0;
        let btn_text_pos_y = btn_rect_pos_y + ((btn_rect_size - btn_font_size) * 0.5);
        let btn_radius = btn_rect_size / 5.0;

        // Item Name
        Text::new(&self.name)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(text_font_size as u32)
            .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
            .x_y(-1.0, text_pos_y - 2.0)
            .parent(id)
            .depth(self.distance_from_player_sqr + 4.0)
            .set(state.ids.name_bg, ui);
        Text::new(&self.name)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(text_font_size as u32)
            .color(self.quality)
            .x_y(0.0, text_pos_y)
            .depth(self.distance_from_player_sqr + 3.0)
            .parent(id)
            .set(state.ids.name, ui);

        // Pickup Button
        if let Some(key_button) = self
            .controls
            .get_binding(GameInput::Interact)
            .filter(|_| self.active)
        {
            RoundedRectangle::fill_with([btn_rect_size, btn_rect_size], btn_radius, btn_color)
                .x_y(0.0, btn_rect_pos_y)
                .depth(self.distance_from_player_sqr + 1.0)
                .parent(id)
                .set(state.ids.btn_bg, ui);
            Text::new(key_button.display_string(self.key_layout).as_str())
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(btn_font_size as u32)
                .color(TEXT_COLOR)
                .x_y(0.0, btn_text_pos_y)
                .depth(self.distance_from_player_sqr + 2.0)
                .parent(id)
                .set(state.ids.btn, ui);
        }
    }
}
