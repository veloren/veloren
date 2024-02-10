use super::{
    animate_by_pulse, get_quality_col,
    img_ids::{Imgs, ImgsRot},
    item_imgs::ItemImgs,
    util, HudInfo, Show, Windows, TEXT_COLOR,
};
use crate::ui::{fonts::Fonts, ImageFrame, ItemTooltip, ItemTooltipManager, ItemTooltipable};
use client::Client;
use common::{
    comp::inventory::item::{Item, ItemDesc, ItemI18n, MaterialStatManifest, Quality},
    uid::Uid,
};
use common_net::sync::WorldSyncExt;
use conrod_core::{
    color,
    position::Dimension,
    widget::{self, Image, List, Rectangle, Scrollbar, Text},
    widget_ids, Color, Colorable, Positionable, Sizeable, Widget, WidgetCommon,
};
use i18n::Localization;
use std::collections::VecDeque;

widget_ids! {
    struct Ids{
        frame,
        message_box,
        scrollbar,
        message_icons[],
        message_icon_bgs[],
        message_icon_frames[],
        message_texts[],
        message_text_shadows[],
    }
}

const MAX_MESSAGES: usize = 50;

const BOX_WIDTH: f64 = 300.0;
const BOX_HEIGHT: f64 = 350.0;

const ICON_BG_SIZE: f64 = 33.0;
const ICON_SIZE: f64 = 30.0;
const ICON_LABEL_SPACER: f64 = 7.0;

const MESSAGE_VERTICAL_PADDING: f64 = 1.0;

const HOVER_FADE_OUT_TIME: f32 = 2.0;
const MESSAGE_FADE_OUT_TIME: f32 = 4.5;
const AUTO_SHOW_FADE_OUT_TIME: f32 = 1.0;

const MAX_MERGE_TIME: f32 = MESSAGE_FADE_OUT_TIME;

#[derive(WidgetCommon)]
pub struct LootScroller<'a> {
    new_messages: &'a mut VecDeque<LootMessage>,

    client: &'a Client,
    info: &'a HudInfo,
    show: &'a Show,
    imgs: &'a Imgs,
    item_imgs: &'a ItemImgs,
    rot_imgs: &'a ImgsRot,
    fonts: &'a Fonts,
    localized_strings: &'a Localization,
    item_i18n: &'a ItemI18n,
    msm: &'a MaterialStatManifest,
    item_tooltip_manager: &'a mut ItemTooltipManager,
    pulse: f32,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}
impl<'a> LootScroller<'a> {
    pub fn new(
        new_messages: &'a mut VecDeque<LootMessage>,
        client: &'a Client,
        info: &'a HudInfo,
        show: &'a Show,
        imgs: &'a Imgs,
        item_imgs: &'a ItemImgs,
        rot_imgs: &'a ImgsRot,
        fonts: &'a Fonts,
        localized_strings: &'a Localization,
        item_i18n: &'a ItemI18n,
        msm: &'a MaterialStatManifest,
        item_tooltip_manager: &'a mut ItemTooltipManager,
        pulse: f32,
    ) -> Self {
        Self {
            new_messages,
            client,
            info,
            show,
            imgs,
            item_imgs,
            rot_imgs,
            fonts,
            localized_strings,
            item_i18n,
            msm,
            item_tooltip_manager,
            pulse,
            common: widget::CommonBuilder::default(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct LootMessage {
    pub item: Item,
    pub amount: u32,
    pub taken_by: Uid,
}

pub struct State {
    ids: Ids,
    messages: VecDeque<(LootMessage, f32)>, // (message, timestamp)

    last_hover_pulse: Option<f32>,
    last_auto_show_pulse: Option<f32>, // auto show if (for example) bag is open
}

impl<'a> Widget for LootScroller<'a> {
    type Event = ();
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
            messages: VecDeque::new(),
            last_hover_pulse: None,
            last_auto_show_pulse: None,
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;

        // Tooltips
        let item_tooltip = ItemTooltip::new(
            {
                // Edge images [t, b, r, l]
                // Corner images [tr, tl, br, bl]
                let edge = &self.rot_imgs.tt_side;
                let corner = &self.rot_imgs.tt_corner;
                ImageFrame::new(
                    [edge.cw180, edge.none, edge.cw270, edge.cw90],
                    [corner.none, corner.cw270, corner.cw90, corner.cw180],
                    Color::Rgba(0.08, 0.07, 0.04, 1.0),
                    5.0,
                )
            },
            self.client,
            self.info,
            self.imgs,
            self.item_imgs,
            self.pulse,
            self.msm,
            self.localized_strings,
            self.item_i18n,
        )
        .title_font_size(self.fonts.cyri.scale(20))
        .parent(ui.window)
        .desc_font_size(self.fonts.cyri.scale(12))
        .font_id(self.fonts.cyri.conrod_id)
        .desc_text_color(TEXT_COLOR);

        if !self.new_messages.is_empty() {
            let pulse = self.pulse;
            let oldest_merge_pulse = pulse - MAX_MERGE_TIME;

            state.update(|s| {
                s.messages.retain(|(message, t)| {
                    if *t >= oldest_merge_pulse {
                        if let Some(i) = self.new_messages.iter().position(|m| {
                            m.item.item_definition_id() == message.item.item_definition_id()
                                && m.taken_by == message.taken_by
                        }) {
                            self.new_messages[i].amount += message.amount;
                            false
                        } else {
                            true
                        }
                    } else {
                        true
                    }
                });
                s.messages
                    .extend(self.new_messages.drain(..).map(|message| (message, pulse)));
                while s.messages.len() > MAX_MESSAGES {
                    s.messages.pop_front();
                }
            });
            ui.scroll_widget(state.ids.message_box, [0.0, f64::MAX]);
        }

        // check if it collides with other windows
        if self.show.diary
            || self.show.map
            || self.show.open_windows != Windows::None
            || self.show.social
            || self.show.trade
        {
            if state.last_hover_pulse.is_some() || state.last_auto_show_pulse.is_some() {
                state.update(|s| {
                    s.last_hover_pulse = None;
                    s.last_auto_show_pulse = None;
                });
            }
        } else {
            //check if hovered
            if ui
                .rect_of(state.ids.message_box)
                .map(|r| r.pad_left(-6.0))
                .map_or(false, |r| r.is_over(ui.global_input().current.mouse.xy))
            {
                state.update(|s| s.last_hover_pulse = Some(self.pulse));
            }

            if state.ids.message_icons.len() < state.messages.len() {
                state.update(|s| {
                    s.ids
                        .message_icons
                        .resize(s.messages.len(), &mut ui.widget_id_generator())
                });
            }
            if state.ids.message_icon_bgs.len() < state.messages.len() {
                state.update(|s| {
                    s.ids
                        .message_icon_bgs
                        .resize(s.messages.len(), &mut ui.widget_id_generator())
                });
            }
            if state.ids.message_icon_frames.len() < state.messages.len() {
                state.update(|s| {
                    s.ids
                        .message_icon_frames
                        .resize(s.messages.len(), &mut ui.widget_id_generator())
                });
            }
            if state.ids.message_texts.len() < state.messages.len() {
                state.update(|s| {
                    s.ids
                        .message_texts
                        .resize(s.messages.len(), &mut ui.widget_id_generator())
                });
            }
            if state.ids.message_text_shadows.len() < state.messages.len() {
                state.update(|s| {
                    s.ids
                        .message_text_shadows
                        .resize(s.messages.len(), &mut ui.widget_id_generator())
                });
            }

            let hover_age = state
                .last_hover_pulse
                .map_or(1.0, |t| (self.pulse - t) / HOVER_FADE_OUT_TIME);
            let auto_show_age = state
                .last_auto_show_pulse
                .map_or(1.0, |t| (self.pulse - t) / AUTO_SHOW_FADE_OUT_TIME);

            let show_all_age = hover_age.min(auto_show_age);

            let messages_to_display = state
                .messages
                .iter()
                .rev()
                .map(|(message, t)| {
                    let age = (self.pulse - t) / MESSAGE_FADE_OUT_TIME;
                    (message, age)
                })
                .filter(|(_, age)| age.min(show_all_age) < 1.0)
                .collect::<Vec<_>>();

            let (mut list_messages, _) = List::flow_up(messages_to_display.len())
                .w_h(BOX_WIDTH, BOX_HEIGHT)
                .scroll_kids_vertically()
                .bottom_left_with_margins_on(ui.window, 308.0, 20.0)
                .set(state.ids.message_box, ui);

            //only show scrollbar if it is being hovered and needed
            if show_all_age < 1.0
                && ui
                    .widget_graph()
                    .widget(state.ids.message_box)
                    .and_then(|w| w.maybe_y_scroll_state)
                    .map_or(false, |s| s.scrollable_range_len > BOX_HEIGHT)
            {
                Scrollbar::y_axis(state.ids.message_box)
                    .thickness(5.0)
                    .rgba(0.33, 0.33, 0.33, 1.0 - show_all_age.powi(4))
                    .left_from(state.ids.message_box, 1.0)
                    .set(state.ids.scrollbar, ui);
            }

            while let Some(list_message) = list_messages.next(ui) {
                let i = list_message.i;

                let (message, age) = messages_to_display[i];
                let LootMessage {
                    item,
                    amount,
                    taken_by,
                } = message;

                let alpha = 1.0 - age.min(show_all_age).powi(4);

                let brightness = 1.0 / (age / 0.05 - 1.0).abs().clamp(0.01, 1.0);

                let shade_color = |color: Color| {
                    let color::Hsla(hue, sat, lum, alp) = color.to_hsl();
                    color::hsla(hue, sat / brightness, lum * brightness.sqrt(), alp * alpha)
                };

                let quality_col_image = match item.quality() {
                    Quality::Low => self.imgs.inv_slot_grey,
                    Quality::Common => self.imgs.inv_slot_common,
                    Quality::Moderate => self.imgs.inv_slot_green,
                    Quality::High => self.imgs.inv_slot_blue,
                    Quality::Epic => self.imgs.inv_slot_purple,
                    Quality::Legendary => self.imgs.inv_slot_gold,
                    Quality::Artifact => self.imgs.inv_slot_orange,
                    _ => self.imgs.inv_slot_red,
                };
                let quality_col = get_quality_col(&item);

                Image::new(self.imgs.pixel)
                    .color(Some(shade_color(quality_col.alpha(0.7))))
                    .w_h(ICON_BG_SIZE, ICON_BG_SIZE)
                    .top_left_with_margins_on(list_message.widget_id, MESSAGE_VERTICAL_PADDING, 0.0)
                    .set(state.ids.message_icon_bgs[i], ui);

                Image::new(quality_col_image)
                    .color(Some(shade_color(color::hsla(0.0, 0.0, 1.0, 1.0))))
                    .wh_of(state.ids.message_icon_bgs[i])
                    .middle_of(state.ids.message_icon_bgs[i])
                    .set(state.ids.message_icon_frames[i], ui);

                Image::new(animate_by_pulse(
                    &self.item_imgs.img_ids_or_not_found_img(item.into()),
                    self.pulse,
                ))
                .color(Some(shade_color(color::hsla(0.0, 0.0, 1.0, 1.0))))
                .w_h(ICON_SIZE, ICON_SIZE)
                .middle_of(state.ids.message_icon_bgs[i])
                .with_item_tooltip(
                    self.item_tooltip_manager,
                    core::iter::once(item as &dyn ItemDesc),
                    &None,
                    &item_tooltip,
                )
                .set(state.ids.message_icons[i], ui);

                let target_name = match self.client.player_list().get(taken_by) {
                    Some(info) => info.player_alias.clone(),
                    None => match self.client.state().ecs().entity_from_uid(*taken_by) {
                        Some(entity) => {
                            let stats = self.client.state().read_storage::<common::comp::Stats>();
                            stats
                                .get(entity)
                                .map_or(format!("<entity {}>", *taken_by), |e| e.name.to_owned())
                        },
                        None => format!("<uid {}>", *taken_by),
                    },
                };

                let (user_gender, is_you) = match self.client.player_list().get(taken_by) {
                    Some(player_info) => match player_info.character.as_ref() {
                        Some(character_info) => (
                            match character_info.gender {
                                Some(common::comp::Gender::Feminine) => "she".to_string(),
                                Some(common::comp::Gender::Masculine) => "he".to_string(),
                                None => "??".to_string(),
                            },
                            self.client.uid().expect("Client doesn't have a Uid!!!") == *taken_by,
                        ),
                        None => ("??".to_string(), false),
                    },
                    None => ("??".to_string(), false),
                };

                let label = self.localized_strings.get_msg_ctx(
                    "hud-loot-pickup-msg",
                    &i18n::fluent_args! {
                          "is_you" => is_you.to_string(),
                          "gender" => user_gender,
                          "actor" => target_name,
                          "amount" => amount,
                          "item" => {
                              let (name, _) =
                                  util::item_text(&item, self.localized_strings, self.item_i18n);
                              name
                          },
                    },
                );
                let label_font_size = 20;

                Text::new(&label)
                    .top_left_with_margins_on(
                        list_message.widget_id,
                        MESSAGE_VERTICAL_PADDING + 1.0,
                        ICON_BG_SIZE + ICON_LABEL_SPACER,
                    )
                    .font_id(self.fonts.cyri.conrod_id)
                    .font_size(self.fonts.cyri.scale(label_font_size))
                    .color(shade_color(quality_col))
                    .graphics_for(state.ids.message_icons[i])
                    .and(|text| {
                        let text_width = match text.get_x_dimension(ui) {
                            Dimension::Absolute(x) => x,
                            _ => f64::MAX,
                        }
                        .min(BOX_WIDTH - (ICON_BG_SIZE + ICON_LABEL_SPACER));
                        text.w(text_width)
                    })
                    .set(state.ids.message_texts[i], ui);
                Text::new(&label)
                    .depth(1.0)
                    .parent(list_message.widget_id)
                    .x_y_relative_to(state.ids.message_texts[i], -1.0, -1.0)
                    .wh_of(state.ids.message_texts[i])
                    .font_id(self.fonts.cyri.conrod_id)
                    .font_size(self.fonts.cyri.scale(label_font_size))
                    .color(shade_color(color::rgba(0.0, 0.0, 0.0, 1.0)))
                    .set(state.ids.message_text_shadows[i], ui);

                let height = 2.0 * MESSAGE_VERTICAL_PADDING
                    + ICON_BG_SIZE.max(
                        1.0 + ui
                            .rect_of(state.ids.message_texts[i])
                            .map_or(0.0, |r| r.h() + label_font_size as f64 / 3.0),
                        /* add to height since rect height does not account for lower parts of
                         * letters */
                    );

                let rect = Rectangle::fill_with([BOX_WIDTH, height], color::TRANSPARENT);

                list_message.set(rect, ui);
            }
        }
    }
}
