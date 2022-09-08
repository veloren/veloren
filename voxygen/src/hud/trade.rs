use conrod_core::{
    color,
    position::Relative,
    widget::{self, Button, Image, Rectangle, State as ConrodState, Text, TextEdit},
    widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, UiCell, Widget, WidgetCommon,
};
use specs::Entity as EcsEntity;
use vek::*;

use client::Client;
use common::{
    comp::{
        inventory::item::{ItemDesc, MaterialStatManifest, Quality},
        Inventory, Stats,
    },
    trade::{PendingTrade, SitePrices, TradeAction, TradePhase},
};
use common_net::sync::WorldSyncExt;
use i18n::Localization;

use crate::{
    hud::bag::{BackgroundIds, InventoryScroller},
    ui::{
        fonts::Fonts,
        slot::{ContentSize, SlotMaker},
        ImageFrame, ItemTooltip, ItemTooltipManager, ItemTooltipable,
    },
};

use super::{
    img_ids::{Imgs, ImgsRot},
    item_imgs::ItemImgs,
    slots::{SlotKind, SlotManager, TradeSlot},
    Hud, HudInfo, Show, TradeAmountInput, TEXT_COLOR, TEXT_GRAY_COLOR, UI_HIGHLIGHT_0, UI_MAIN,
};
use std::borrow::Cow;

#[derive(Debug)]
pub enum HudUpdate {
    Focus(widget::Id),
    Submit,
}

pub struct State {
    ids: Ids,
    bg_ids: BackgroundIds,
}

widget_ids! {
    pub struct Ids {
        trade_close,
        bg,
        bg_frame,
        trade_title_bg,
        trade_title,
        inv_alignment[],
        inv_slots[],
        inv_textslots[],
        offer_headers[],
        accept_indicators[],
        phase_indicator,
        accept_button,
        decline_button,
        inventory_scroller,
        amount_bg,
        amount_notice,
        amount_open_label,
        amount_open_btn,
        amount_open_ovlay,
        amount_input,
        amount_btn,
    }
}

#[derive(WidgetCommon)]
pub struct Trade<'a> {
    client: &'a Client,
    info: &'a HudInfo,
    imgs: &'a Imgs,
    item_imgs: &'a ItemImgs,
    fonts: &'a Fonts,
    rot_imgs: &'a ImgsRot,
    item_tooltip_manager: &'a mut ItemTooltipManager,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    slot_manager: &'a mut SlotManager,
    localized_strings: &'a Localization,
    msm: &'a MaterialStatManifest,
    pulse: f32,
    show: &'a mut Show,
}

impl<'a> Trade<'a> {
    pub fn new(
        client: &'a Client,
        info: &'a HudInfo,
        imgs: &'a Imgs,
        item_imgs: &'a ItemImgs,
        fonts: &'a Fonts,
        rot_imgs: &'a ImgsRot,
        item_tooltip_manager: &'a mut ItemTooltipManager,
        slot_manager: &'a mut SlotManager,
        localized_strings: &'a Localization,
        msm: &'a MaterialStatManifest,
        pulse: f32,
        show: &'a mut Show,
    ) -> Self {
        Self {
            client,
            info,
            imgs,
            item_imgs,
            fonts,
            rot_imgs,
            item_tooltip_manager,
            common: widget::CommonBuilder::default(),
            slot_manager,
            localized_strings,
            msm,
            pulse,
            show,
        }
    }
}

const MAX_TRADE_SLOTS: usize = 16;

impl<'a> Trade<'a> {
    fn background(&mut self, state: &mut ConrodState<'_, State>, ui: &mut UiCell<'_>) {
        Image::new(self.imgs.inv_middle_bg_bag)
            .w_h(424.0, 482.0)
            .color(Some(UI_MAIN))
            .mid_bottom_with_margin_on(ui.window, 295.0)
            .set(state.ids.bg, ui);
        Image::new(self.imgs.inv_middle_frame)
            .w_h(424.0, 482.0)
            .middle_of(state.ids.bg)
            .color(Some(UI_HIGHLIGHT_0))
            .set(state.ids.bg_frame, ui);
    }

    fn title(&mut self, state: &mut ConrodState<'_, State>, ui: &mut UiCell<'_>) {
        Text::new(&self.localized_strings.get_msg("hud-trade-trade_window"))
            .mid_top_with_margin_on(state.ids.bg_frame, 9.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(20))
            .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
            .set(state.ids.trade_title_bg, ui);
        Text::new(&self.localized_strings.get_msg("hud-trade-trade_window"))
            .top_left_with_margins_on(state.ids.trade_title_bg, 2.0, 2.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(20))
            .color(TEXT_COLOR)
            .set(state.ids.trade_title, ui);
    }

    fn phase_indicator(
        &mut self,
        state: &mut ConrodState<'_, State>,
        ui: &mut UiCell<'_>,
        trade: &'a PendingTrade,
    ) {
        let phase_text = match trade.phase() {
            TradePhase::Mutate => self
                .localized_strings
                .get_msg("hud-trade-phase1_description"),
            TradePhase::Review => self
                .localized_strings
                .get_msg("hud-trade-phase2_description"),
            TradePhase::Complete => self
                .localized_strings
                .get_msg("hud-trade-phase3_description"),
        };

        Text::new(&phase_text)
            .mid_top_with_margin_on(state.ids.bg, 70.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(20))
            .color(Color::Rgba(1.0, 1.0, 1.0, 1.0))
            .set(state.ids.phase_indicator, ui);
    }

    fn item_pane(
        &mut self,
        state: &mut ConrodState<'_, State>,
        ui: &mut UiCell<'_>,
        trade: &'a PendingTrade,
        prices: &'a Option<SitePrices>,
        ours: bool,
    ) -> Option<TradeAction> {
        let inventories = self.client.inventories();
        let check_if_us = |who: usize| -> Option<_> {
            let uid = trade.parties[who];
            let entity = self.client.state().ecs().entity_from_uid(uid.0)?;
            let is_ours = entity == self.client.entity();
            Some(((who, uid, entity), is_ours))
        };
        let (who, uid, entity) = match check_if_us(0)? {
            (x, is_ours) if ours == is_ours => x,
            _ => check_if_us(1)?.0,
        };
        // TODO: update in accordance with https://gitlab.com/veloren/veloren/-/issues/960
        let inventory = inventories.get(entity)?;

        // Alignment for Grid
        let mut alignment = Rectangle::fill_with([200.0, 180.0], color::TRANSPARENT);
        if !ours {
            alignment = alignment.top_left_with_margins_on(state.ids.bg, 180.0, 32.5);
        } else {
            alignment = alignment.right_from(state.ids.inv_alignment[1 - who], 0.0);
        }
        alignment
            .scroll_kids_vertically()
            .set(state.ids.inv_alignment[who], ui);

        let name = self
            .client
            .player_list()
            .get(&uid)
            .map(|info| info.player_alias.clone())
            .or_else(|| {
                self.client
                    .state()
                    .read_storage::<Stats>()
                    .get(entity)
                    .map(|e| e.name.to_owned())
            })
            .unwrap_or_else(|| format!("Player {}", who));

        let offer_header = if ours {
            self.localized_strings.get_msg("hud-trade-your_offer")
        } else {
            self.localized_strings.get_msg("hud-trade-their_offer")
        };

        Text::new(&offer_header)
            .up_from(state.ids.inv_alignment[who], 20.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(20))
            .color(Color::Rgba(1.0, 1.0, 1.0, 1.0))
            .set(state.ids.offer_headers[who], ui);

        let has_accepted = trade.accept_flags[who];
        let accept_indicator =
            self.localized_strings
                .get_msg_ctx("hud-trade-has_accepted", &i18n::fluent_args! {
                    "playername" => &name,
                });
        Text::new(&accept_indicator)
            .down_from(state.ids.inv_alignment[who], 50.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(20))
            .color(Color::Rgba(
                1.0,
                1.0,
                1.0,
                if has_accepted { 1.0 } else { 0.0 },
            ))
            .set(state.ids.accept_indicators[who], ui);

        let mut invslots: Vec<_> = trade.offers[who].iter().map(|(k, v)| (*k, *v)).collect();
        invslots.sort();
        let tradeslots: Vec<_> = invslots
            .into_iter()
            .enumerate()
            .map(|(index, (k, quantity))| TradeSlot {
                index,
                quantity,
                invslot: Some(k),
                ours,
                entity,
            })
            .collect();

        if matches!(trade.phase(), TradePhase::Mutate) {
            self.phase1_itemwidget(
                state,
                ui,
                inventory,
                who,
                ours,
                entity,
                name,
                prices,
                &tradeslots,
            );
        } else {
            self.phase2_itemwidget(state, ui, inventory, who, ours, entity, &tradeslots);
        }

        None
    }

    fn phase1_itemwidget(
        &mut self,
        state: &mut ConrodState<'_, State>,
        ui: &mut UiCell<'_>,
        inventory: &Inventory,
        who: usize,
        ours: bool,
        entity: EcsEntity,
        name: String,
        prices: &'a Option<SitePrices>,
        tradeslots: &[TradeSlot],
    ) {
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
        )
        .title_font_size(self.fonts.cyri.scale(20))
        .parent(ui.window)
        .desc_font_size(self.fonts.cyri.scale(12))
        .font_id(self.fonts.cyri.conrod_id)
        .desc_text_color(TEXT_COLOR);

        if !ours {
            InventoryScroller::new(
                self.client,
                self.imgs,
                self.item_imgs,
                self.fonts,
                self.item_tooltip_manager,
                self.slot_manager,
                self.pulse,
                self.localized_strings,
                false,
                true,
                false,
                &item_tooltip,
                name,
                entity,
                false,
                inventory,
                &state.bg_ids,
                false,
            )
            .set(state.ids.inventory_scroller, ui);
        }

        let mut slot_maker = SlotMaker {
            empty_slot: self.imgs.inv_slot,
            filled_slot: self.imgs.inv_slot,
            selected_slot: self.imgs.inv_slot_sel,
            background_color: Some(UI_MAIN),
            content_size: ContentSize {
                width_height_ratio: 1.0,
                max_fraction: 0.75,
            },
            selected_content_scale: 1.067,
            amount_font: self.fonts.cyri.conrod_id,
            amount_margins: Vec2::new(-4.0, 0.0),
            amount_font_size: self.fonts.cyri.scale(12),
            amount_text_color: TEXT_COLOR,
            content_source: inventory,
            image_source: self.item_imgs,
            slot_manager: Some(self.slot_manager),
            pulse: self.pulse,
        };

        if state.ids.inv_slots.len() < 2 * MAX_TRADE_SLOTS {
            state.update(|s| {
                s.ids
                    .inv_slots
                    .resize(2 * MAX_TRADE_SLOTS, &mut ui.widget_id_generator());
            });
        }

        for i in 0..MAX_TRADE_SLOTS {
            let x = i % 4;
            let y = i / 4;

            let slot = tradeslots.get(i).cloned().unwrap_or(TradeSlot {
                index: i,
                quantity: 0,
                invslot: None,
                ours,
                entity,
            });
            // Slot
            let slot_widget = slot_maker
                .fabricate(slot, [40.0; 2])
                .top_left_with_margins_on(
                    state.ids.inv_alignment[who],
                    0.0 + y as f64 * (40.0),
                    0.0 + x as f64 * (40.0),
                );
            let slot_id = state.ids.inv_slots[i + who * MAX_TRADE_SLOTS];
            if let Some(Some(item)) = slot.invslot.and_then(|slotid| inventory.slot(slotid)) {
                let quality_col_img = match item.quality() {
                    Quality::Low => self.imgs.inv_slot_grey,
                    Quality::Common => self.imgs.inv_slot_common,
                    Quality::Moderate => self.imgs.inv_slot_green,
                    Quality::High => self.imgs.inv_slot_blue,
                    Quality::Epic => self.imgs.inv_slot_purple,
                    Quality::Legendary => self.imgs.inv_slot_gold,
                    Quality::Artifact => self.imgs.inv_slot_orange,
                    _ => self.imgs.inv_slot_red,
                };

                slot_widget
                    .filled_slot(quality_col_img)
                    .with_item_tooltip(
                        self.item_tooltip_manager,
                        core::iter::once(item as &dyn ItemDesc),
                        prices,
                        &item_tooltip,
                    )
                    .set(slot_id, ui);
            } else {
                slot_widget.set(slot_id, ui);
            }
        }
    }

    fn phase2_itemwidget(
        &mut self,
        state: &mut ConrodState<'_, State>,
        ui: &mut UiCell<'_>,
        inventory: &Inventory,
        who: usize,
        ours: bool,
        entity: EcsEntity,
        tradeslots: &[TradeSlot],
    ) {
        if state.ids.inv_textslots.len() < 2 * MAX_TRADE_SLOTS {
            state.update(|s| {
                s.ids
                    .inv_textslots
                    .resize(2 * MAX_TRADE_SLOTS, &mut ui.widget_id_generator());
            });
        }
        let mut total_quantity = 0;
        for i in 0..MAX_TRADE_SLOTS {
            let slot = tradeslots.get(i).cloned().unwrap_or(TradeSlot {
                index: i,
                quantity: 0,
                invslot: None,
                ours,
                entity,
            });
            total_quantity += slot.quantity;
            let itemname = slot
                .invslot
                .and_then(|i| inventory.get(i))
                .map(|i| i.name())
                .unwrap_or(Cow::Borrowed(""));
            let is_present = slot.quantity > 0 && slot.invslot.is_some();
            Text::new(&format!("{} x {}", slot.quantity, itemname))
                .top_left_with_margins_on(state.ids.inv_alignment[who], 10.0 + i as f64 * 30.0, 0.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(20))
                .color(Color::Rgba(
                    1.0,
                    1.0,
                    1.0,
                    if is_present { 1.0 } else { 0.0 },
                ))
                .set(state.ids.inv_textslots[i + who * MAX_TRADE_SLOTS], ui);
        }
        if total_quantity == 0 {
            Text::new("Nothing!")
                .top_left_with_margins_on(state.ids.inv_alignment[who], 10.0, 0.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(20))
                .color(Color::Rgba(
                    1.0,
                    0.25 + 0.25 * (4.0 * self.pulse).sin(),
                    0.0,
                    1.0,
                ))
                .set(state.ids.inv_textslots[who * MAX_TRADE_SLOTS], ui);
        }
    }

    fn accept_decline_buttons(
        &mut self,
        state: &mut ConrodState<'_, State>,
        ui: &mut UiCell<'_>,
        trade: &'a PendingTrade,
    ) -> Option<TradeAction> {
        let mut event = None;
        let (hover_img, press_img, accept_button_luminance) = if trade.is_empty_trade() {
            //Darken the accept button if the trade is empty.
            (
                self.imgs.button,
                self.imgs.button,
                Color::Rgba(0.6, 0.6, 0.6, 1.0),
            )
        } else {
            (
                self.imgs.button_hover,
                self.imgs.button_press,
                Color::Rgba(1.0, 1.0, 1.0, 1.0),
            )
        };
        if Button::image(self.imgs.button)
            .w_h(31.0 * 5.0, 12.0 * 2.0)
            .hover_image(hover_img)
            .press_image(press_img)
            .image_color(accept_button_luminance)
            .bottom_left_with_margins_on(state.ids.bg, 90.0, 47.0)
            .label(&self.localized_strings.get_msg("hud-trade-accept"))
            .label_font_size(self.fonts.cyri.scale(14))
            .label_color(TEXT_COLOR)
            .label_font_id(self.fonts.cyri.conrod_id)
            .label_y(Relative::Scalar(2.0))
            .set(state.ids.accept_button, ui)
            .was_clicked()
        {
            event = Some(TradeAction::Accept(trade.phase()));
        }

        if Button::image(self.imgs.button)
            .w_h(31.0 * 5.0, 12.0 * 2.0)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .right_from(state.ids.accept_button, 20.0)
            .label(&self.localized_strings.get_msg("hud-trade-decline"))
            .label_font_size(self.fonts.cyri.scale(14))
            .label_color(TEXT_COLOR)
            .label_font_id(self.fonts.cyri.conrod_id)
            .label_y(Relative::Scalar(2.0))
            .set(state.ids.decline_button, ui)
            .was_clicked()
        {
            event = Some(TradeAction::Decline);
        }
        event
    }

    fn input_item_amount(
        &mut self,
        state: &mut ConrodState<'_, State>,
        ui: &mut UiCell<'_>,
        trade: &'a PendingTrade,
    ) -> Option<HudUpdate> {
        let mut event = None;
        let selected = self.slot_manager.selected().and_then(|s| match s {
            SlotKind::Trade(t_s) => t_s.invslot.and_then(|slot| {
                let who: usize = trade.offers[0].get(&slot).and(Some(0)).unwrap_or(1);
                self.client
                    .inventories()
                    .get(t_s.entity)?
                    .get(slot)
                    .map(|item| (t_s.ours, slot, item.amount(), who))
            }),
            _ => None,
        });
        Rectangle::fill([132.0, 20.0])
            .bottom_right_with_margins_on(state.ids.bg_frame, 16.0, 32.0)
            .hsla(
                0.0,
                0.0,
                0.0,
                if self.show.trade_amount_input_key.is_some() {
                    0.75
                } else {
                    0.35
                },
            )
            .set(state.ids.amount_bg, ui);
        if let Some((ours, slot, inv, who)) = selected {
            self.show.trade_amount_input_key = None;
            // Text for the amount of items offered.
            let input = trade.offers[who]
                .get(&slot)
                .map(|u| format!("{}", u))
                .unwrap_or_else(String::new);
            Text::new(&input)
                .top_left_with_margins_on(state.ids.amount_bg, 0.0, 22.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .color(TEXT_COLOR.alpha(0.7))
                .set(state.ids.amount_open_label, ui);
            if Button::image(self.imgs.edit_btn)
                .hover_image(self.imgs.edit_btn_hover)
                .press_image(self.imgs.edit_btn_press)
                .mid_left_with_margin_on(state.ids.amount_bg, 2.0)
                .w_h(16.0, 16.0)
                .set(state.ids.amount_open_btn, ui)
                .was_clicked()
            {
                event = Some(HudUpdate::Focus(state.ids.amount_input));
                self.slot_manager.idle();
                self.show.trade_amount_input_key =
                    Some(TradeAmountInput::new(slot, input, inv, ours, who));
            }
            Rectangle::fill_with([132.0, 20.0], color::TRANSPARENT)
                .top_left_of(state.ids.amount_bg)
                .graphics_for(state.ids.amount_open_btn)
                .set(state.ids.amount_open_ovlay, ui);
        } else if let Some(key) = &mut self.show.trade_amount_input_key {
            if !Hud::is_captured::<TextEdit>(ui) && key.input_painted {
                // If the text edit is not captured submit the amount.
                event = Some(HudUpdate::Submit);
            }

            if Button::image(self.imgs.close_btn)
                .hover_image(self.imgs.close_btn_hover)
                .press_image(self.imgs.close_btn_press)
                .mid_left_with_margin_on(state.ids.amount_bg, 2.0)
                .w_h(16.0, 16.0)
                .set(state.ids.amount_btn, ui)
                .was_clicked()
            {
                event = Some(HudUpdate::Submit);
            }
            // Input for making TradeAction requests
            key.input_painted = true;
            let text_color = key.err.as_ref().and(Some(color::RED)).unwrap_or(TEXT_COLOR);
            if let Some(new_input) = TextEdit::new(&key.input)
                .mid_left_with_margin_on(state.ids.amount_bg, 22.0)
                .w_h(138.0, 20.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .color(text_color)
                .set(state.ids.amount_input, ui)
            {
                if new_input != key.input {
                    key.input = new_input.trim().to_owned();
                    if !key.input.is_empty() {
                        // trade amount can change with (shift||ctrl)-click
                        let amount = *trade.offers[key.who].get(&key.slot).unwrap_or(&0);
                        match key.input.parse::<i32>() {
                            Ok(new_amount) => {
                                key.input = format!("{}", new_amount);
                                if new_amount > -1 && new_amount <= key.inv as i32 {
                                    key.err = None;
                                    let delta = new_amount - amount as i32;
                                    key.submit_action =
                                        TradeAction::item(key.slot, delta, key.ours);
                                } else {
                                    key.err = Some("out of range".to_owned());
                                    key.submit_action = None;
                                }
                            },
                            Err(_) => {
                                key.err = Some("bad quantity".to_owned());
                                key.submit_action = None;
                            },
                        }
                    } else {
                        key.submit_action = None;
                    }
                }
            }
        } else {
            // placeholder text when no trade slot is selected
            Text::new(&self.localized_strings.get_msg("hud-trade-amount_input"))
                .middle_of(state.ids.amount_bg)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .color(TEXT_GRAY_COLOR.alpha(0.25))
                .set(state.ids.amount_notice, ui);
        }
        event
    }

    fn close_button(
        &mut self,
        state: &mut ConrodState<'_, State>,
        ui: &mut UiCell<'_>,
    ) -> Option<TradeAction> {
        if Button::image(self.imgs.close_btn)
            .w_h(24.0, 25.0)
            .hover_image(self.imgs.close_btn_hover)
            .press_image(self.imgs.close_btn_press)
            .top_right_with_margins_on(state.ids.bg, 0.0, 0.0)
            .set(state.ids.trade_close, ui)
            .was_clicked()
        {
            Some(TradeAction::Decline)
        } else {
            None
        }
    }
}

impl<'a> Widget for Trade<'a> {
    type Event = Option<Result<TradeAction, HudUpdate>>;
    type State = State;
    type Style = ();

    fn init_state(&self, mut id_gen: widget::id::Generator) -> Self::State {
        State {
            bg_ids: BackgroundIds {
                bg: id_gen.next(),
                bg_frame: id_gen.next(),
            },
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(mut self, args: widget::UpdateArgs<Self>) -> Self::Event {
        common_base::prof_span!("Trade::update");
        let widget::UpdateArgs { state, ui, .. } = args;

        let mut event = None;
        let (trade, prices) = match self.client.pending_trade() {
            Some((_, trade, prices)) => (trade, prices),
            None => return Some(Ok(TradeAction::Decline)),
        };

        if state.ids.inv_alignment.len() < 2 {
            state.update(|s| {
                s.ids.inv_alignment.resize(2, &mut ui.widget_id_generator());
            });
        }
        if state.ids.offer_headers.len() < 2 {
            state.update(|s| {
                s.ids.offer_headers.resize(2, &mut ui.widget_id_generator());
            });
        }
        if state.ids.accept_indicators.len() < 2 {
            state.update(|s| {
                s.ids
                    .accept_indicators
                    .resize(2, &mut ui.widget_id_generator());
            });
        }

        self.background(state, ui);
        self.title(state, ui);
        self.phase_indicator(state, ui, trade);

        event = self.item_pane(state, ui, trade, prices, false).or(event);
        event = self.item_pane(state, ui, trade, prices, true).or(event);
        event = self.accept_decline_buttons(state, ui, trade).or(event);
        event = self.close_button(state, ui).or(event);
        self.input_item_amount(state, ui, trade)
            .map(Err)
            .or_else(|| event.map(Ok))
    }
}
