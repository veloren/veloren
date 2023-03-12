use crate::{
    game_input::GameInput,
    settings::ControlSettings,
    ui::{fonts::Fonts, Ingameable},
};
use conrod_core::{
    color,
    widget::{self, RoundedRectangle, Text},
    widget_ids, Color, Colorable, Positionable, Sizeable, Widget, WidgetCommon,
};
use i18n::Localization;
use std::borrow::Cow;

use crate::hud::{CollectFailedData, HudCollectFailedReason, HudLootOwner};
use keyboard_keynames::key_layout::KeyLayout;

pub const TEXT_COLOR: Color = Color::Rgba(0.61, 0.61, 0.89, 1.0);
pub const PICKUP_FAILED_FADE_OUT_TIME: f32 = 1.5;

widget_ids! {
    struct Ids {
        // Name
        name_bg,
        name,
        // Interaction hints
        btn_bg,
        btn,
        // Inventory full
        inv_full_bg,
        inv_full,
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
    localized_strings: &'a Localization,
    controls: &'a ControlSettings,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    properties: OveritemProperties,
    pulse: f32,
    key_layout: &'a Option<KeyLayout>,
    // GameInput optional so we can just show stuff like "needs pickaxe"
    interaction_options: Vec<(Option<GameInput>, String)>,
}

impl<'a> Overitem<'a> {
    pub fn new(
        name: Cow<'a, str>,
        quality: Color,
        distance_from_player_sqr: f32,
        fonts: &'a Fonts,
        localized_strings: &'a Localization,
        controls: &'a ControlSettings,
        properties: OveritemProperties,
        pulse: f32,
        key_layout: &'a Option<KeyLayout>,
        interaction_options: Vec<(Option<GameInput>, String)>,
    ) -> Self {
        Self {
            name,
            quality,
            distance_from_player_sqr,
            fonts,
            localized_strings,
            controls,
            common: widget::CommonBuilder::default(),
            properties,
            pulse,
            key_layout,
            interaction_options,
        }
    }
}

pub struct OveritemProperties {
    pub active: bool,
    pub pickup_failed_pulse: Option<CollectFailedData>,
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
            .filter(|_| self.properties.active)
        {
            Some(_) => 2,
            None => 0,
        } + if self.properties.pickup_failed_pulse.is_some() {
            2
        } else {
            0
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

        let btn_color = Color::Rgba(0.0, 0.0, 0.0, 0.8);

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

        let inv_full_font_size = scale * 1.0;
        let inv_full_pos_y = scale * 2.4;

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

        // Interaction hints
        if !self.interaction_options.is_empty() && self.properties.active {
            let text = self
                .interaction_options
                .iter()
                .filter_map(|(input, action)| {
                    let binding = if let Some(input) = input {
                        Some(self.controls.get_binding(*input)?)
                    } else {
                        None
                    };
                    Some((binding, action))
                })
                .map(|(input, action)| {
                    if let Some(input) = input {
                        let input = input.display_string(self.key_layout);
                        format!("{}  {action}", input.as_str())
                    } else {
                        action.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");

            let hints_text = Text::new(&text)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(btn_font_size as u32)
                .color(TEXT_COLOR)
                .x_y(0.0, btn_text_pos_y)
                .depth(self.distance_from_player_sqr + 1.0)
                .parent(id);

            let [w, h] = hints_text.get_wh(ui).unwrap_or([btn_rect_size; 2]);

            hints_text.set(state.ids.btn, ui);

            RoundedRectangle::fill_with(
                [w + btn_radius * 2.0, h + btn_radius * 2.0],
                btn_radius,
                btn_color,
            )
            .x_y(0.0, btn_rect_pos_y)
            .depth(self.distance_from_player_sqr + 2.0)
            .parent(id)
            .set(state.ids.btn_bg, ui);
        }
        if let Some(collect_failed_data) = self.properties.pickup_failed_pulse {
            //should never exceed 1.0, but just in case
            let age = ((self.pulse - collect_failed_data.pulse) / PICKUP_FAILED_FADE_OUT_TIME)
                .clamp(0.0, 1.0);

            let alpha = 1.0 - age.powi(4);
            let brightness = 1.0 / (age / 0.07 - 1.0).abs().clamp(0.01, 1.0);
            let shade_color = |color: Color| {
                let color::Hsla(hue, sat, lum, alp) = color.to_hsl();
                color::hsla(hue, sat / brightness, lum * brightness.sqrt(), alp * alpha)
            };

            let text = match collect_failed_data.reason {
                HudCollectFailedReason::InventoryFull => {
                    self.localized_strings.get_msg("hud-inventory_full")
                },
                HudCollectFailedReason::LootOwned { owner, expiry_secs } => {
                    let owner_name = match owner {
                        HudLootOwner::Name(name) => Cow::Owned(name),
                        HudLootOwner::Group => self.localized_strings.get_msg("hud-another_group"),
                        HudLootOwner::Unknown => self.localized_strings.get_msg("hud-someone_else"),
                    };
                    self.localized_strings.get_msg_ctx(
                        "hud-owned_by_for_secs",
                        &i18n::fluent_args! {
                            "name" => owner_name,
                            "secs" => expiry_secs,
                        },
                    )
                },
            };

            Text::new(&text)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(inv_full_font_size as u32)
                .color(shade_color(Color::Rgba(0.0, 0.0, 0.0, 1.0)))
                .x_y(-1.0, inv_full_pos_y - 2.0)
                .parent(id)
                .depth(self.distance_from_player_sqr + 6.0)
                .set(state.ids.inv_full_bg, ui);

            Text::new(&text)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(inv_full_font_size as u32)
                .color(shade_color(Color::Rgba(1.0, 0.0, 0.0, 1.0)))
                .x_y(0.0, inv_full_pos_y)
                .parent(id)
                .depth(self.distance_from_player_sqr + 5.0)
                .set(state.ids.inv_full, ui);
        }
    }
}
