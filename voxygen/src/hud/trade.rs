use super::{
    cr_color,
    img_ids::{Imgs, ImgsRot},
    item_imgs::ItemImgs,
    slots::{InventorySlot, SlotManager},
    util::loadout_slot_text,
    Show, CRITICAL_HP_COLOR, LOW_HP_COLOR, QUALITY_COMMON, TEXT_COLOR, UI_HIGHLIGHT_0, UI_MAIN,
};
use crate::{
    hud::get_quality_col,
    i18n::Localization,
    ui::{
        fonts::Fonts,
        slot::{ContentSize, SlotMaker},
        ImageFrame, Tooltip, TooltipManager, Tooltipable,
    },
};
use client::Client;
use common::comp::item::Quality;
use conrod_core::{
    color,
    widget::{self, Button, Image, Rectangle, Scrollbar, Text},
    widget_ids, Color, Colorable, Positionable, Sizeable, Widget, WidgetCommon,
};

pub struct State {
    ids: Ids,
}

pub enum Event {
    Close,
}

widget_ids! {
    pub struct Ids {
        trade_close,
        bg,
        bg_frame,
        trade_title_bg,
        trade_title,
    }
}

#[derive(WidgetCommon)]
pub struct Trade<'a> {
    client: &'a Client,
    imgs: &'a Imgs,
    item_imgs: &'a ItemImgs,
    fonts: &'a Fonts,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    rot_imgs: &'a ImgsRot,
    tooltip_manager: &'a mut TooltipManager,
    slot_manager: &'a mut SlotManager,
    localized_strings: &'a Localization,
    show: &'a Show,
}

impl<'a> Trade<'a> {
    pub fn new(
        client: &'a Client,
        imgs: &'a Imgs,
        item_imgs: &'a ItemImgs,
        fonts: &'a Fonts,
        rot_imgs: &'a ImgsRot,
        tooltip_manager: &'a mut TooltipManager,
        slot_manager: &'a mut SlotManager,
        localized_strings: &'a Localization,
        show: &'a Show,
    ) -> Self {
        Self {
            client,
            imgs,
            item_imgs,
            fonts,
            common: widget::CommonBuilder::default(),
            rot_imgs,
            tooltip_manager,
            slot_manager,
            localized_strings,
            show,
        }
    }
}

impl<'a> Widget for Trade<'a> {
    type Event = Option<Event>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;

        let mut event = None;

        let inventories = self.client.inventories();
        let inventory = match inventories.get(self.client.entity()) {
            Some(l) => l,
            None => return None,
        };

        let trade_tooltip = Tooltip::new({
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
        })
        .title_font_size(self.fonts.cyri.scale(15))
        .parent(ui.window)
        .desc_font_size(self.fonts.cyri.scale(12))
        .font_id(self.fonts.cyri.conrod_id)
        .desc_text_color(TEXT_COLOR);

        // BG
        Image::new(self.imgs.inv_bg_bag)
            .w_h(424.0, 708.0)
            .middle()
            .color(Some(UI_MAIN))
            .set(state.ids.bg, ui);
        Image::new(self.imgs.inv_frame_bag)
            .w_h(424.0, 708.0)
            .middle_of(state.ids.bg)
            .color(Some(UI_HIGHLIGHT_0))
            .set(state.ids.bg_frame, ui);
        // Title
        Text::new(&self.localized_strings.get("hud.trade.trade_window"))
            .mid_top_with_margin_on(state.ids.bg_frame, 9.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(20))
            .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
            .set(state.ids.trade_title_bg, ui);
        Text::new(&self.localized_strings.get("hud.trade.trade_window"))
            .top_left_with_margins_on(state.ids.trade_title_bg, 2.0, 2.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(20))
            .color(TEXT_COLOR)
            .set(state.ids.trade_title, ui);

        event
    }
}
