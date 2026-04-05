use super::img_ids::Imgs;

use crate::{
    GlobalState,
    game_input::GameInput,
    hud::{
        CollectFailedData, HudCollectFailedReason, HudLootOwner, IconHandler,
        controller_icons::LayerIconIds,
    },
    ui::{Ingameable, fonts::Fonts},
    window::LastInput,
};
use conrod_core::{
    Color, Colorable, Positionable, Sizeable, Widget, WidgetCommon, color,
    widget::{self, RoundedRectangle, Text},
    widget_ids,
};
use i18n::Localization;
use std::borrow::Cow;

pub const TEXT_COLOR: Color = Color::Rgba(0.61, 0.61, 0.89, 1.0);
pub const NEGATIVE_TEXT_COLOR: Color = Color::Rgba(0.91, 0.15, 0.17, 1.0);
pub const PICKUP_FAILED_FADE_OUT_TIME: f32 = 1.5;

widget_ids! {
    struct Ids {
        // Name
        name_bg,
        name,
        // Interaction hints
        btn_bg,
        btns[],
        icns[], // controller icons
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
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    properties: OveritemProperties,
    pulse: f32,
    // GameInput optional so we can just show stuff like "needs pickaxe"
    interaction_options: Vec<(Option<GameInput>, String, Color)>,
    imgs: &'a Imgs,
    global_state: &'a GlobalState,
}

impl<'a> Overitem<'a> {
    pub fn new(
        name: Cow<'a, str>,
        quality: Color,
        distance_from_player_sqr: f32,
        fonts: &'a Fonts,
        localized_strings: &'a Localization,
        properties: OveritemProperties,
        pulse: f32,
        interaction_options: Vec<(Option<GameInput>, String, Color)>,
        imgs: &'a Imgs,
        global_state: &'a GlobalState,
    ) -> Self {
        Self {
            name,
            quality,
            distance_from_player_sqr,
            fonts,
            localized_strings,
            common: widget::CommonBuilder::default(),
            properties,
            pulse,
            interaction_options,
            imgs,
            global_state,
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

impl Ingameable for Overitem<'_> {
    fn prim_count(&self) -> usize {
        // Number of conrod primitives contained in the overitem display.
        // TODO maybe this could be done automatically?

        // + 2 Text for name
        let base = 2;

        // + 0 or 2 Rectangle and Text for button
        let interaction_ids = match self.global_state.window.last_input() {
            LastInput::KeyboardMouse => self
                .global_state
                .settings
                .controls
                .get_binding(GameInput::Interact)
                .filter(|_| self.properties.active)
                .map_or(0, |_| 2),
            LastInput::Controller => {
                // + 3 more than keyboard for icon ids (main icon, mod1, mod2)
                self.global_state
                    .settings
                    .controller
                    .get_game_button_binding(GameInput::Interact)
                    .filter(|_| self.properties.active)
                    .map_or(3, |_| 5)
            },
        };

        // + 0 or 2 for pickup failed pulse
        let pulse = if self.properties.pickup_failed_pulse.is_some() {
            2
        } else {
            0
        };

        base + interaction_ids + pulse
    }
}

impl Widget for Overitem<'_> {
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
            let mut max_w = btn_rect_size;
            let mut max_h = 0.0;
            let mut box_offset = 0.0;

            match self.global_state.window.last_input() {
                LastInput::KeyboardMouse => {
                    let texts = self
                        .interaction_options
                        .iter()
                        .filter_map(|(input, action, color)| {
                            let binding = if let Some(input) = input {
                                Some(self.global_state.settings.controls.get_binding(*input)?)
                            } else {
                                None
                            };
                            Some((binding, action, color))
                        })
                        .map(|(input, action, color)| {
                            if let Some(input) = input {
                                let input = input.display_string();
                                (format!("{}  {action}", input.as_str()), color)
                            } else {
                                (action.to_string(), color)
                            }
                        })
                        .collect::<Vec<_>>();
                    if state.ids.btns.len() < texts.len() {
                        state.update(|state| {
                            state
                                .ids
                                .btns
                                .resize(texts.len(), &mut ui.widget_id_generator());
                        })
                    }

                    for (idx, (text, color)) in texts.iter().enumerate() {
                        let hints_text = Text::new(text)
                            .font_id(self.fonts.cyri.conrod_id)
                            .font_size(btn_font_size as u32)
                            .color(**color)
                            .x_y(0.0, btn_text_pos_y + max_h)
                            .depth(self.distance_from_player_sqr + 1.0)
                            .parent(id);
                        let [w, h] = hints_text.get_wh(ui).unwrap_or([btn_rect_size; 2]);
                        max_w = max_w.max(w);
                        max_h += h;
                        hints_text.set(state.ids.btns[idx], ui);
                    }

                    max_h = max_h.max(btn_rect_size);
                },
                LastInput::Controller => {
                    // because in-line images are not easily supported, the controller icons are
                    // manually rendered left of the text

                    let controller_texts = self.interaction_options.iter().collect::<Vec<_>>(); // &Option<GameInput>, &String, &Color
                    if state.ids.btns.len() < controller_texts.len() {
                        state.update(|state| {
                            state
                                .ids
                                .btns
                                .resize(controller_texts.len(), &mut ui.widget_id_generator());
                        })
                    }
                    let icns_size = controller_texts.len() * 3; // main icon + 2 modifier buttons
                    if state.ids.icns.len() < icns_size {
                        state.update(|state| {
                            state
                                .ids
                                .icns
                                .resize(icns_size, &mut ui.widget_id_generator());
                        })
                    }

                    let icon_handler = IconHandler::new(self.global_state, self.imgs);
                    let mut icons_w: u8 = 0;

                    // render text here, call button next to it
                    for (idx, (inputs, action, color)) in controller_texts.iter().enumerate() {
                        // render text widget first
                        let text_widget_id = state.ids.btns[idx];
                        let hints_text = Text::new(action)
                            .font_id(self.fonts.cyri.conrod_id)
                            .font_size(btn_font_size as u32)
                            .color(*color)
                            .x_y(0.0, btn_text_pos_y + max_h)
                            .depth(self.distance_from_player_sqr + 1.0)
                            .parent(id);
                        let [w, h] = hints_text.get_wh(ui).unwrap_or([btn_rect_size; 2]);
                        max_w = max_w.max(w);
                        max_h += h;
                        hints_text.set(text_widget_id, ui);

                        // render controller icon left of the text
                        let idx_icns = idx * 3;
                        let icon_ids = LayerIconIds {
                            main: state.ids.icns[idx_icns],
                            modifier1: state.ids.icns[idx_icns + 1],
                            modifier2: state.ids.icns[idx_icns + 2],
                        };
                        if let Some(input) = inputs {
                            let count = icon_handler.set_controller_icons_left(
                                *input,
                                17.0,
                                text_widget_id,
                                &icon_ids,
                                ui,
                            );
                            icons_w = icons_w.max(count);
                        } else {
                            // render transparant widgets to keep conrod from freaking out
                            icon_handler.set_controller_icons_left_none(
                                17.0,
                                text_widget_id,
                                &icon_ids,
                                ui,
                            );
                        }
                    }

                    let icon_largest_width = icons_w as f64 * 21.0;
                    box_offset = icon_largest_width / 2.0;
                    max_w += icon_largest_width;
                    max_h = max_h.max(btn_rect_size);
                },
            }

            RoundedRectangle::fill_with(
                [max_w + btn_radius * 2.0, max_h + btn_radius * 2.0],
                btn_radius,
                btn_color,
            )
            .x_y(0.0 - box_offset, btn_rect_pos_y)
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
                        HudLootOwner::Name(name) => {
                            Cow::Owned(self.localized_strings.get_content(&name))
                        },
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
