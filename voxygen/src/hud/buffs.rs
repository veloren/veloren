use super::{
    img_ids::{Imgs, ImgsRot},
    TEXT_COLOR,
};
use crate::{
    i18n::VoxygenLocalization,
    ui::{fonts::ConrodVoxygenFonts, ImageFrame, Tooltip, TooltipManager, Tooltipable},
    GlobalState,
};
use client::Client;
use common::comp::Stats;
use conrod_core::{
    color,
    widget::{self, Button, Image, Rectangle, Text},
    widget_ids, Color, Colorable, Positionable, Sizeable, Widget, WidgetCommon,
};
use inline_tweak::*;
widget_ids! {
    struct Ids {
        align,
        buffs_align,
        debuffs_align,
        buff_test,
        debuff_test,
    }
}
#[derive(WidgetCommon)]
pub struct Buffs<'a> {
    client: &'a Client,
    imgs: &'a Imgs,
    fonts: &'a ConrodVoxygenFonts,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    global_state: &'a GlobalState,
    rot_imgs: &'a ImgsRot,
    tooltip_manager: &'a mut TooltipManager,
    localized_strings: &'a std::sync::Arc<VoxygenLocalization>,
    stats: &'a Stats,
}

impl<'a> Buffs<'a> {
    #[allow(clippy::too_many_arguments)] // TODO: Pending review in #587
    pub fn new(
        client: &'a Client,
        imgs: &'a Imgs,
        fonts: &'a ConrodVoxygenFonts,
        global_state: &'a GlobalState,
        rot_imgs: &'a ImgsRot,
        tooltip_manager: &'a mut TooltipManager,
        localized_strings: &'a std::sync::Arc<VoxygenLocalization>,
        stats: &'a Stats,
    ) -> Self {
        Self {
            client,
            imgs,
            fonts,
            common: widget::CommonBuilder::default(),
            global_state,
            rot_imgs,
            tooltip_manager,
            localized_strings,
            stats,
        }
    }
}

pub struct State {
    ids: Ids,
}

impl<'a> Widget for Buffs<'a> {
    type Event = ();
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
        let localized_strings = self.localized_strings;
        let buffs_tooltip = Tooltip::new({
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
        // Alignment
        Rectangle::fill_with([484.0, 100.0], color::TRANSPARENT)
            .mid_bottom_with_margin_on(ui.window, tweak!(92.0))
            .set(state.ids.align, ui);
        Rectangle::fill_with([484.0 / 2.0, 90.0], color::TRANSPARENT)
            .bottom_left_with_margins_on(state.ids.align, 0.0, 0.0)
            .set(state.ids.debuffs_align, ui);
        Rectangle::fill_with([484.0 / 2.0, 90.0], color::TRANSPARENT)
            .bottom_right_with_margins_on(state.ids.align, 0.0, 0.0)
            .set(state.ids.buffs_align, ui);
        // Test Widgets
        Image::new(self.imgs.debuff_skull_0)
            .w_h(20.0, 20.0)
            .bottom_right_with_margins_on(state.ids.debuffs_align, 0.0, 1.0)
            .set(state.ids.debuff_test, ui);
        Image::new(self.imgs.buff_plus_0)
            .w_h(20.0, 20.0)
            .bottom_left_with_margins_on(state.ids.buffs_align, 0.0, 1.0)
            .set(state.ids.buff_test, ui);
    }
}
