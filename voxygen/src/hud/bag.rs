use super::{
    img_ids::{Imgs, ImgsRot},
    item_imgs::{ItemImgs, ItemKey},
    slot_kinds::{HudSlotManager, InventorySlot},
    Event as HudEvent, Show, CRITICAL_HP_COLOR, LOW_HP_COLOR, TEXT_COLOR, UI_HIGHLIGHT_0, UI_MAIN,
    XP_COLOR,
};
use crate::{
    i18n::VoxygenLocalization,
    ui::{
        fonts::ConrodVoxygenFonts, slot::SlotMaker, ImageFrame, Tooltip, TooltipManager,
        Tooltipable,
    },
};
use client::Client;
use common::comp::Stats;
use conrod_core::{
    color, image,
    widget::{self, Button, Image, Rectangle, Text},
    widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};
use vek::Vec2;

widget_ids! {
    pub struct Ids {
        test,
        bag_close,
        inv_alignment,
        inv_grid_1,
        inv_grid_2,
        inv_scrollbar,
        inv_slots_0,
        map_title,
        inv_slots[],
        items[],
        amounts[],
        amounts_bg[],
        tooltip[],
        bg,
        bg_frame,
        char_ico,
        coin_ico,
        space_txt,
        currency_txt,
        inventory_title,
        inventory_title_bg,
        scrollbar_bg,
        stats_button,
        tab_1,
        tab_2,
        tab_3,
        tab_4,
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
        //Armor Slots
        slots_bg,
        head_bg,
        neck_bg,
        chest_bg,
        shoulder_bg,
        hands_bg,
        legs_bg,
        belt_bg,
        ring_r_bg,
        ring_l_bg,
        foot_bg,
        back_bg,
        tabard_bg,
        mainhand_bg,
        offhand_bg,
        head_ico,
        neck_ico,
        chest_ico,
        shoulder_ico,
        hands_ico,
        legs_ico,
        belt_ico,
        ring_r_ico,
        ring_l_ico,
        foot_ico,
        back_ico,
        tabard_ico,
        mainhand_ico,
        offhand_ico,
        end_ico,
        fit_ico,
        wp_ico,
    }
}

#[derive(WidgetCommon)]
pub struct Bag<'a> {
    client: &'a Client,
    imgs: &'a Imgs,
    item_imgs: &'a ItemImgs,
    fonts: &'a ConrodVoxygenFonts,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    rot_imgs: &'a ImgsRot,
    tooltip_manager: &'a mut TooltipManager,
    _pulse: f32,
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
            _pulse: pulse,
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
        let space_used = inventory.amount;
        let space_max = inventory.slots.len();
        let bag_space = format!("{}/{}", space_used, space_max);
        let bag_space_percentage = space_used as f32 / space_max as f32;
        let level = (self.stats.level.level()).to_string();
        let currency = 0; // TODO: Add as a Stat maybe?

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
            .set(state.ids.char_ico, ui);
        // Coin Icon and Currency Text
        Image::new(self.imgs.coin_ico)
            .w_h(16.0, 17.0)
            .bottom_left_with_margins_on(state.ids.bg_frame, 2.0, 43.0)
            .set(state.ids.coin_ico, ui);
        Text::new(&format!("{}", currency))
            .bottom_left_with_margins_on(state.ids.bg_frame, 6.0, 64.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(14))
            .color(Color::Rgba(0.871, 0.863, 0.05, 1.0))
            .set(state.ids.currency_txt, ui);
        //Free Bag-Space
        Text::new(&bag_space)
            .bottom_right_with_margins_on(state.ids.bg_frame, 6.0, 43.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(14))
            .color(if bag_space_percentage < 0.8 {
                TEXT_COLOR
            } else if bag_space_percentage < 1.0 {
                LOW_HP_COLOR
            } else {
                CRITICAL_HP_COLOR
            })
            .set(state.ids.space_txt, ui);
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
            /*Image::new(self.imgs.inv_runes)
                .w_h(424.0, 454.0)
                .mid_top_with_margin_on(state.ids.bg, 0.0)
                .color(Some(UI_HIGHLIGHT_0))
                .floating(true)
                .set(state.ids.slots_bg, ui);
            Image::new(self.imgs.inv_slots)
            .w_h(424.0, 401.0)
            .mid_top_with_margin_on(state.ids.bg, 57.0)
            .color(Some(UI_HIGHLIGHT_0))
            .set(state.ids.slots_bg, ui);*/
            // Armor Slots
            //Head
            Image::new(self.imgs.armor_slot)
                .w_h(45.0, 45.0)
                .mid_top_with_margin_on(state.ids.bg_frame, 60.0)
                .color(Some(UI_HIGHLIGHT_0))
                .set(state.ids.head_bg, ui);
            Button::image(self.imgs.head_bg)
                .w_h(32.0, 40.0)
                .image_color(UI_MAIN)
                .middle_of(state.ids.head_bg)
                .with_tooltip(self.tooltip_manager, "Helmet", "", &item_tooltip)
                .set(state.ids.head_ico, ui);
            //Necklace
            Image::new(self.imgs.armor_slot)
                .w_h(45.0, 45.0)
                .mid_bottom_with_margin_on(state.ids.head_bg, -55.0)
                .color(Some(UI_HIGHLIGHT_0))
                .set(state.ids.neck_bg, ui);
            Button::image(self.imgs.necklace_bg)
                .w_h(40.0, 31.0)
                .image_color(UI_MAIN)
                .middle_of(state.ids.neck_bg)
                .with_tooltip(self.tooltip_manager, "Neck", "", &item_tooltip)
                .set(state.ids.neck_ico, ui);
            //Chest
            Image::new(self.imgs.armor_slot) // different graphics for empty/non empty
                .w_h(85.0, 85.0)
                .mid_bottom_with_margin_on(state.ids.neck_bg, -95.0)
                .color(Some(UI_HIGHLIGHT_0))
                .set(state.ids.chest_bg, ui);
            Button::image(self.imgs.chest_bg)
                .w_h(64.0, 42.0)
                .image_color(UI_MAIN)
                .middle_of(state.ids.chest_bg)
                .with_tooltip(self.tooltip_manager, "Chest", "", &item_tooltip)
                .set(state.ids.chest_ico, ui);
            //Shoulder
            Image::new(self.imgs.armor_slot)
                .w_h(70.0, 70.0)
                .bottom_left_with_margins_on(state.ids.chest_bg, 0.0, -80.0)
                .color(Some(UI_HIGHLIGHT_0))
                .set(state.ids.shoulder_bg, ui);
            Button::image(self.imgs.shoulders_bg)
                .w_h(60.0, 36.0)
                .image_color(UI_MAIN)
                .middle_of(state.ids.shoulder_bg)
                .with_tooltip(self.tooltip_manager, "Shoulders", "", &item_tooltip)
                .set(state.ids.shoulder_ico, ui);
            //Hands
            Image::new(self.imgs.armor_slot)
                .w_h(70.0, 70.0)
                .bottom_right_with_margins_on(state.ids.chest_bg, 0.0, -80.0)
                .color(Some(UI_HIGHLIGHT_0))
                .set(state.ids.hands_bg, ui);
            Button::image(self.imgs.hands_bg)
                .w_h(55.0, 60.0)
                .image_color(UI_MAIN)
                .middle_of(state.ids.hands_bg)
                .with_tooltip(self.tooltip_manager, "Hands", "", &item_tooltip)
                .set(state.ids.hands_ico, ui);
            //Belt
            Image::new(self.imgs.armor_slot)
                .w_h(45.0, 45.0)
                .mid_bottom_with_margin_on(state.ids.chest_bg, -55.0)
                .color(Some(UI_HIGHLIGHT_0))
                .set(state.ids.belt_bg, ui);
            Button::image(self.imgs.belt_bg)
                .w_h(40.0, 23.0)
                .image_color(UI_MAIN)
                .middle_of(state.ids.belt_bg)
                .with_tooltip(self.tooltip_manager, "Belt", "", &item_tooltip)
                .set(state.ids.belt_ico, ui);
            //Legs
            Image::new(self.imgs.armor_slot)
                .w_h(85.0, 85.0)
                .mid_bottom_with_margin_on(state.ids.belt_bg, -95.0)
                .color(Some(UI_HIGHLIGHT_0))
                .set(state.ids.legs_bg, ui);
            Button::image(self.imgs.legs_bg)
                .w_h(48.0, 70.0)
                .image_color(UI_MAIN)
                .middle_of(state.ids.legs_bg)
                .with_tooltip(self.tooltip_manager, "Legs", "", &item_tooltip)
                .set(state.ids.legs_ico, ui);
            //Ring-L
            Image::new(self.imgs.armor_slot)
                .w_h(45.0, 45.0)
                .bottom_right_with_margins_on(state.ids.shoulder_bg, -55.0, 0.0)
                .color(Some(UI_HIGHLIGHT_0))
                .set(state.ids.ring_l_bg, ui);
            Button::image(self.imgs.ring_l_bg)
                .w_h(36.0, 40.0)
                .image_color(UI_MAIN)
                .middle_of(state.ids.ring_l_bg)
                .with_tooltip(self.tooltip_manager, "Left Ring", "", &item_tooltip)
                .set(state.ids.ring_l_ico, ui);
            //Ring-R
            Image::new(self.imgs.armor_slot)
                .w_h(45.0, 45.0)
                .bottom_left_with_margins_on(state.ids.hands_bg, -55.0, 0.0)
                .color(Some(UI_HIGHLIGHT_0))
                .set(state.ids.ring_r_bg, ui);
            Button::image(self.imgs.ring_r_bg)
                .w_h(36.0, 40.0)
                .image_color(UI_MAIN)
                .middle_of(state.ids.ring_r_bg)
                .with_tooltip(self.tooltip_manager, "Right Ring", "", &item_tooltip)
                .set(state.ids.ring_r_ico, ui);
            //Back
            Image::new(self.imgs.armor_slot)
                .w_h(45.0, 45.0)
                .down_from(state.ids.ring_l_bg, 10.0)
                .color(Some(UI_HIGHLIGHT_0))
                .set(state.ids.back_bg, ui);
            Button::image(self.imgs.back_bg)
                .w_h(33.0, 40.0)
                .image_color(UI_MAIN)
                .middle_of(state.ids.back_bg)
                .with_tooltip(self.tooltip_manager, "Back", "", &item_tooltip)
                .set(state.ids.back_ico, ui);
            //Foot
            Image::new(self.imgs.armor_slot)
                .w_h(45.0, 45.0)
                .down_from(state.ids.ring_r_bg, 10.0)
                .color(Some(UI_HIGHLIGHT_0))
                .set(state.ids.foot_bg, ui);
            Button::image(self.imgs.feet_bg)
                .w_h(32.0, 40.0)
                .image_color(UI_MAIN)
                .middle_of(state.ids.foot_bg)
                .with_tooltip(self.tooltip_manager, "Feet", "", &item_tooltip)
                .set(state.ids.foot_ico, ui);
            //Tabard
            Image::new(self.imgs.armor_slot)
                .w_h(70.0, 70.0)
                .top_right_with_margins_on(state.ids.bg_frame, 80.5, 53.0)
                .color(Some(UI_HIGHLIGHT_0))
                .set(state.ids.tabard_bg, ui);
            Button::image(self.imgs.tabard_bg)
                .w_h(60.0, 60.0)
                .image_color(UI_MAIN)
                .middle_of(state.ids.tabard_bg)
                .with_tooltip(self.tooltip_manager, "Tabard", "", &item_tooltip)
                .set(state.ids.tabard_ico, ui);
            //Mainhand/Left-Slot
            Image::new(self.imgs.armor_slot)
                .w_h(85.0, 85.0)
                .bottom_right_with_margins_on(state.ids.back_bg, -95.0, 0.0)
                .color(Some(UI_HIGHLIGHT_0))
                .set(state.ids.mainhand_bg, ui);
            Button::image(self.imgs.mainhand_bg)
                .w_h(75.0, 75.0)
                .image_color(UI_MAIN)
                .middle_of(state.ids.mainhand_bg)
                .with_tooltip(self.tooltip_manager, "Mainhand", "", &item_tooltip)
                .set(state.ids.mainhand_ico, ui);
            //Offhand/Right-Slot
            Image::new(self.imgs.armor_slot)
                .w_h(85.0, 85.0)
                .bottom_left_with_margins_on(state.ids.foot_bg, -95.0, 0.0)
                .color(Some(UI_HIGHLIGHT_0))
                .set(state.ids.offhand_bg, ui);
            Button::image(self.imgs.offhand_bg)
                .w_h(75.0, 75.0)
                .image_color(UI_MAIN)
                .middle_of(state.ids.offhand_bg)
                .with_tooltip(self.tooltip_manager, "Offhand", "", &item_tooltip)
                .set(state.ids.offhand_ico, ui);
        } else {
            // Stats
            // Title
            Text::new(&format!(
                "{}{}",
                &self.stats.name,
                &self.localized_strings.get("hud.bag.stats_title")
            ))
            .mid_top_with_margin_on(state.ids.bg_frame, 9.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(22))
            .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
            .set(state.ids.inventory_title_bg, ui);
            Text::new(&format!(
                "{}{}",
                &self.stats.name,
                &self.localized_strings.get("hud.bag.stats_title")
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
            Image::new(self.imgs.endurance_ico)
                .w_h(30.0, 30.0)
                .top_left_with_margins_on(state.ids.statnames, -10.0, -40.0)
                .color(Some(UI_HIGHLIGHT_0))
                .set(state.ids.end_ico, ui);
            Image::new(self.imgs.fitness_ico)
                .w_h(30.0, 30.0)
                .down_from(state.ids.end_ico, 10.0)
                .color(Some(UI_HIGHLIGHT_0))
                .set(state.ids.fit_ico, ui);
            Image::new(self.imgs.willpower_ico)
                .w_h(30.0, 30.0)
                .down_from(state.ids.fit_ico, 10.0)
                .color(Some(UI_HIGHLIGHT_0))
                .set(state.ids.wp_ico, ui);

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
        // Display inventory contents
        // TODO: add slot manager
        let slot_manager: Option<&mut HudSlotManager> = None;
        let mut slot_maker = SlotMaker {
            background: self.imgs.inv_slot,
            selected_background: self.imgs.inv_slot_sel,
            background_color: Some(UI_MAIN),
            content_size: Vec2::broadcast(30.0),
            selected_content_size: Vec2::broadcast(32.0),
            amount_font: self.fonts.cyri.conrod_id,
            amount_margins: Vec2::new(-4.0, 0.0),
            amount_font_size: self.fonts.cyri.scale(12),
            amount_text_color: TEXT_COLOR,
            content_source: inventory,
            image_source: self.item_imgs,
            slot_manager,
        };
        for (i, item) in inventory.slots().iter().enumerate() {
            let x = i % 9;
            let y = i / 9;

            // Slot
            let slot_widget = slot_maker
                .fabricate(InventorySlot(i))
                .top_left_with_margins_on(
                    state.ids.inv_alignment,
                    0.0 + y as f64 * (40.0),
                    0.0 + x as f64 * (40.0),
                )
                .wh([40.0; 2]);
            if let Some(item) = item {
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
                    .set(state.ids.inv_slots[i], ui);
            } else {
                slot_widget.set(state.ids.inv_slots[i], ui);
            }
        }

        // Drop selected item
        //    if ui.widget_input(ui.window).clicks().left().next().is_some() {

        // Stats Button
        if Button::image(self.imgs.button)
            .w_h(92.0, 22.0)
            .mid_top_with_margin_on(state.ids.bg, 435.0)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .label(if self.show.stats {
                &self.localized_strings.get("hud.bag.armor")
            } else {
                &self.localized_strings.get("hud.bag.stats")
            })
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
            .image_color(UI_MAIN)
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
