use super::{
    img_ids::{Imgs, ImgsRot},
    item_imgs::{ItemImgs, ItemKey},
    Event as HudEvent, Show, TEXT_COLOR, UI_HIGHLIGHT_0, UI_MAIN, XP_COLOR,
};
use crate::{
    i18n::VoxygenLocalization,
    ui::{fonts::ConrodVoxygenFonts, ImageFrame, Tooltip, TooltipManager, Tooltipable},
};
use client::Client;
use common::comp::Stats;
use conrod_core::{
    color, image,
    widget::{self, Button, Image, Rectangle, Text},
    widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};

widget_ids! {
    struct Ids {
        bag_close,
        inv_alignment,
        inv_grid_1,
        inv_grid_2,
        inv_scrollbar,
        inv_slots_0,
        map_title,
        inv_slots[],
        items[],
        tooltip[],
        bg,
        bg_frame,
        char_art,
        inventory_title,
        inventory_title_bg,
        scrollbar_bg,
        stats_button,
        tab_1,
        tab_2,
        tab_3,
        tab_4,
        //Armor Slots
        slots_bg,
        //Stats
        stats_alignment,
        level,
        exp_rectangle,
        exp_progress_rectangle,
        expbar,
        exp,
        divider,
        statnames,
        stats,

    }
}

#[derive(WidgetCommon)]
#[allow(dead_code)]
pub struct Bag<'a> {
    client: &'a Client,
    imgs: &'a Imgs,
    item_imgs: &'a ItemImgs,
    fonts: &'a ConrodVoxygenFonts,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    rot_imgs: &'a ImgsRot,
    tooltip_manager: &'a mut TooltipManager,
    pulse: f32,
    localized_strings: &'a std::sync::Arc<VoxygenLocalization>,
    stats: &'a Stats,
    show: &'a Show,
}

impl<'a> Bag<'a> {
    pub fn new(
        client: &'a Client,
        imgs: &'a Imgs,
        item_imgs: &'a ItemImgs,
        fonts: &'a ConrodVoxygenFonts,
        rot_imgs: &'a ImgsRot,
        tooltip_manager: &'a mut TooltipManager,
        pulse: f32,
        localized_strings: &'a std::sync::Arc<VoxygenLocalization>,
        stats: &'a Stats,
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
            pulse,
            localized_strings,
            stats,
            show,
        }
    }
}

pub struct State {
    ids: Ids,
    img_id_cache: Vec<Option<(ItemKey, image::Id)>>,
    selected_slot: Option<usize>,
}

pub enum Event {
    HudEvent(HudEvent),
    Stats,
    Close,
}

impl<'a> Widget for Bag<'a> {
    type Event = Option<Event>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
            img_id_cache: Vec::new(),
            selected_slot: None,
        }
    }

    fn style(&self) -> Self::Style { () }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;

        let mut event = None;

        let invs = self.client.inventories();
        let inventory = match invs.get(self.client.entity()) {
            Some(inv) => inv,
            None => return None,
        };
        let exp_percentage = (self.stats.exp.current() as f64) / (self.stats.exp.maximum() as f64);
        let exp_treshold = format!(
            "{}/{} {}",
            self.stats.exp.current(),
            self.stats.exp.maximum(),
            &self.localized_strings.get("hud.bag.exp")
        );
        let level = (self.stats.level.level()).to_string();

        // Tooltips
        let item_tooltip = Tooltip::new({
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
        .title_text_color(TEXT_COLOR)
        .font_id(self.fonts.cyri.conrod_id)
        .desc_text_color(TEXT_COLOR);

        // BG
        Image::new(if self.show.stats {
            self.imgs.inv_bg_stats
        } else {
            self.imgs.inv_bg_armor
        })
        .w_h(424.0, 708.0)
        .bottom_right_with_margins_on(ui.window, 60.0, 5.0)
        .color(Some(UI_MAIN))
        .set(state.ids.bg, ui);
        Image::new(self.imgs.inv_frame)
            .w_h(424.0, 708.0)
            .middle_of(state.ids.bg)
            .color(Some(UI_HIGHLIGHT_0))
            .set(state.ids.bg_frame, ui);
        // Title
        Text::new(&format!(
            "{}{}",
            &self.stats.name,
            &self.localized_strings.get("hud.bag.inventory")
        ))
        .mid_top_with_margin_on(state.ids.bg_frame, 9.0)
        .font_id(self.fonts.cyri.conrod_id)
        .font_size(self.fonts.cyri.scale(22))
        .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
        .set(state.ids.inventory_title_bg, ui);
        Text::new(&format!(
            "{}{}",
            &self.stats.name,
            &self.localized_strings.get("hud.bag.inventory")
        ))
        .top_left_with_margins_on(state.ids.inventory_title_bg, 2.0, 2.0)
        .font_id(self.fonts.cyri.conrod_id)
        .font_size(self.fonts.cyri.scale(22))
        .color(TEXT_COLOR)
        .set(state.ids.inventory_title, ui);

        // Scrollbar-BG
        Image::new(self.imgs.scrollbar_bg)
            .w_h(9.0, 173.0)
            .bottom_right_with_margins_on(state.ids.bg_frame, 42.0, 3.0)
            .color(Some(UI_HIGHLIGHT_0))
            .set(state.ids.scrollbar_bg, ui);

        // Char Pixel-Art
        Image::new(self.imgs.char_art)
            .w_h(40.0, 37.0)
            .top_left_with_margins_on(state.ids.bg, 4.0, 2.0)
            .set(state.ids.char_art, ui);

        // Alignment for Grid
        Rectangle::fill_with([362.0, 200.0], color::TRANSPARENT)
            .bottom_left_with_margins_on(state.ids.bg_frame, 29.0, 44.0)
            .scroll_kids_vertically()
            .set(state.ids.inv_alignment, ui);

        if !self.show.stats {
            // Title
            Text::new(&format!(
                "{}{}",
                &self.stats.name,
                &self.localized_strings.get("hud.bag.inventory")
            ))
            .mid_top_with_margin_on(state.ids.bg_frame, 9.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(22))
            .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
            .set(state.ids.inventory_title_bg, ui);
            Text::new(&format!(
                "{}{}",
                &self.stats.name,
                &self.localized_strings.get("hud.bag.inventory")
            ))
            .top_left_with_margins_on(state.ids.inventory_title_bg, 2.0, 2.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(22))
            .color(TEXT_COLOR)
            .set(state.ids.inventory_title, ui);
            //Armor Slots
            //Slots BG
            Image::new(self.imgs.inv_runes)
                .w_h(424.0, 454.0)
                .mid_top_with_margin_on(state.ids.bg, 0.0)
                .color(Some(UI_HIGHLIGHT_0))
                .floating(true)
                .set(state.ids.slots_bg, ui);
            Image::new(self.imgs.inv_slots)
                .w_h(424.0, 401.0)
                .mid_top_with_margin_on(state.ids.bg, 57.0)
                .color(Some(UI_HIGHLIGHT_0))
                .set(state.ids.slots_bg, ui);
        } else {
            // Stats
            // Title
            Text::new(&format!(
                "{}{}",
                &self.stats.name,
                &self.localized_strings.get("hud.bag.stats")
            ))
            .mid_top_with_margin_on(state.ids.bg_frame, 9.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(22))
            .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
            .set(state.ids.inventory_title_bg, ui);
            Text::new(&format!(
                "{}{}",
                &self.stats.name,
                &self.localized_strings.get("hud.bag.stats")
            ))
            .top_left_with_margins_on(state.ids.inventory_title_bg, 2.0, 2.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(22))
            .color(TEXT_COLOR)
            .set(state.ids.inventory_title, ui);
            // Alignment for Stats
            Rectangle::fill_with([418.0, 384.0], color::TRANSPARENT)
                .mid_top_with_margin_on(state.ids.bg_frame, 48.0)
                .scroll_kids_vertically()
                .set(state.ids.stats_alignment, ui);
            // Level
            Text::new(&level)
                .mid_top_with_margin_on(state.ids.stats_alignment, 10.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(30))
                .color(TEXT_COLOR)
                .set(state.ids.level, ui);

            // Exp-Bar Background
            Rectangle::fill_with([170.0, 10.0], color::BLACK)
                .mid_top_with_margin_on(state.ids.stats_alignment, 50.0)
                .set(state.ids.exp_rectangle, ui);

            // Exp-Bar Progress
            Rectangle::fill_with([170.0 * (exp_percentage), 6.0], XP_COLOR) // 0.8 = Experience percentage
                .mid_left_with_margin_on(state.ids.expbar, 1.0)
                .set(state.ids.exp_progress_rectangle, ui);

            // Exp-Bar Foreground Frame
            Image::new(self.imgs.progress_frame)
                .w_h(170.0, 10.0)
                .middle_of(state.ids.exp_rectangle)
                .color(Some(UI_HIGHLIGHT_0))
                .set(state.ids.expbar, ui);

            // Exp-Text
            Text::new(&exp_treshold)
                .mid_top_with_margin_on(state.ids.expbar, 10.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(15))
                .color(TEXT_COLOR)
                .set(state.ids.exp, ui);

            // Divider
            /*Image::new(self.imgs.divider)
            .w_h(50.0, 5.0)
            .mid_top_with_margin_on(state.ids.exp, 30.0)
            .color(Some(UI_HIGHLIGHT_0))
            .set(state.ids.divider, ui);*/

            // Stats
            Text::new(
                &self
                    .localized_strings
                    .get("character_window.character_stats"),
            )
            .top_left_with_margins_on(state.ids.stats_alignment, 120.0, 150.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(16))
            .color(TEXT_COLOR)
            .set(state.ids.statnames, ui);

            Text::new(&format!(
                "{}\n\n{}\n\n{}",
                self.stats.endurance, self.stats.fitness, self.stats.willpower
            ))
            .top_right_with_margins_on(state.ids.stats_alignment, 120.0, 150.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(16))
            .color(TEXT_COLOR)
            .set(state.ids.stats, ui);
        }
        // Bag Slots
        // Create available inventory slot widgets
        if state.ids.inv_slots.len() < inventory.len() {
            state.update(|s| {
                s.ids
                    .inv_slots
                    .resize(inventory.len(), &mut ui.widget_id_generator());
            });
        }
        if state.ids.items.len() < inventory.len() {
            state.update(|s| {
                s.ids
                    .items
                    .resize(inventory.len(), &mut ui.widget_id_generator());
            });
        }
        // Expand img id cache to the number of slots
        if state.img_id_cache.len() < inventory.len() {
            state.update(|s| {
                s.img_id_cache.resize(inventory.len(), None);
            });
        }

        // Display inventory contents
        for (i, item) in inventory.slots().iter().enumerate() {
            let x = i % 9;
            let y = i / 9;

            let is_selected = Some(i) == state.selected_slot;

            // Slot

            let slot_widget = Button::image(if !is_selected {
                self.imgs.inv_slot
            } else {
                self.imgs.inv_slot_sel
            })
            .top_left_with_margins_on(
                state.ids.inv_alignment,
                0.0 + y as f64 * (40.0),
                0.0 + x as f64 * (40.0),
            )
            .wh([40.0; 2])
            .image_color(UI_MAIN);

            let slot_widget_clicked = if let Some(item) = item {
                slot_widget
                    .with_tooltip(
                        self.tooltip_manager,
                        &item.name(),
                        &format!(
                            "{}",
                            /* item.kind, item.effect(), */ item.description()
                        ),
                        &item_tooltip,
                    )
                    .set(state.ids.inv_slots[i], ui)
            } else {
                slot_widget.set(state.ids.inv_slots[i], ui)
            }
            .was_clicked();

            // Item
            if slot_widget_clicked {
                let selected_slot = match state.selected_slot {
                    Some(a) => {
                        if a == i {
                            event = Some(Event::HudEvent(HudEvent::UseInventorySlot(i)));
                        } else {
                            event = Some(Event::HudEvent(HudEvent::SwapInventorySlots(a, i)));
                        }
                        None
                    },
                    None if item.is_some() => Some(i),
                    None => None,
                };
                state.update(|s| s.selected_slot = selected_slot);
            }
            // Item
            if let Some(kind) = item.as_ref().map(|i| ItemKey::from(i)) {
                Button::image(match &state.img_id_cache[i] {
                    Some((cached_kind, id)) if cached_kind == &kind => *id,
                    _ => {
                        let id = self
                            .item_imgs
                            .img_id(kind.clone())
                            .unwrap_or(self.imgs.not_found);
                        state.update(|s| s.img_id_cache[i] = Some((kind, id)));
                        id
                    },
                })
                .wh(if is_selected { [32.0; 2] } else { [30.0; 2] })
                .middle_of(state.ids.inv_slots[i])
                .graphics_for(state.ids.inv_slots[i])
                .set(state.ids.items[i], ui);
            }
        }

        // Drop selected item
        if let Some(to_drop) = state.selected_slot {
            if ui.widget_input(ui.window).clicks().left().next().is_some() {
                event = Some(Event::HudEvent(HudEvent::DropInventorySlot(to_drop)));
                state.update(|s| s.selected_slot = None);
            }
        }
        // Stats Button
        if Button::image(self.imgs.button)
            .w_h(92.0, 22.0)
            .mid_top_with_margin_on(state.ids.bg, 435.0)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .label(if self.show.stats { "Armor" } else { "Stats" })
            .label_y(conrod_core::position::Relative::Scalar(1.0))
            .label_color(TEXT_COLOR)
            .label_font_size(self.fonts.cyri.scale(12))
            .label_font_id(self.fonts.cyri.conrod_id)
            .set(state.ids.stats_button, ui)
            .was_clicked()
        {
            return Some(Event::Stats);
        };
        // Tabs
        if Button::image(self.imgs.inv_tab_active)
            .w_h(28.0, 44.0)
            .bottom_left_with_margins_on(state.ids.bg, 172.0, 13.0)
            .image_color(UI_HIGHLIGHT_0)
            .set(state.ids.tab_1, ui)
            .was_clicked()
        {}
        if Button::image(self.imgs.inv_tab_inactive)
            .w_h(28.0, 44.0)
            .hover_image(self.imgs.inv_tab_inactive_hover)
            .press_image(self.imgs.inv_tab_inactive_press)
            .image_color(UI_HIGHLIGHT_0)
            .down_from(state.ids.tab_1, 0.0)
            .with_tooltip(self.tooltip_manager, "Not yet Available", "", &item_tooltip)
            .set(state.ids.tab_2, ui)
            .was_clicked()
        {}
        if Button::image(self.imgs.inv_tab_inactive)
            .w_h(28.0, 44.0)
            .hover_image(self.imgs.inv_tab_inactive_hover)
            .press_image(self.imgs.inv_tab_inactive_press)
            .down_from(state.ids.tab_2, 0.0)
            .image_color(UI_HIGHLIGHT_0)
            .with_tooltip(self.tooltip_manager, "Not yet Available", "", &item_tooltip)
            .set(state.ids.tab_3, ui)
            .was_clicked()
        {}
        if Button::image(self.imgs.inv_tab_inactive)
            .w_h(28.0, 44.0)
            .hover_image(self.imgs.inv_tab_inactive_hover)
            .press_image(self.imgs.inv_tab_inactive_press)
            .down_from(state.ids.tab_3, 0.0)
            .image_color(UI_HIGHLIGHT_0)
            .with_tooltip(self.tooltip_manager, "Not yet Available", "", &item_tooltip)
            .set(state.ids.tab_4, ui)
            .was_clicked()
        {}
        // Close button
        if Button::image(self.imgs.close_btn)
            .w_h(24.0, 25.0)
            .hover_image(self.imgs.close_btn_hover)
            .press_image(self.imgs.close_btn_press)
            .top_right_with_margins_on(state.ids.bg, 0.0, 0.0)
            .set(state.ids.bag_close, ui)
            .was_clicked()
        {
            event = Some(Event::Close);
        }
        event
    }
}
