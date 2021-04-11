use super::{
    cr_color,
    img_ids::{Imgs, ImgsRot},
    item_imgs::ItemImgs,
    slots::{ArmorSlot, EquipSlot, InventorySlot, SlotManager},
    Show, CRITICAL_HP_COLOR, LOW_HP_COLOR, TEXT_COLOR, UI_HIGHLIGHT_0, UI_MAIN,
};
use crate::{
    i18n::Localization,
    ui::{
        fonts::Fonts,
        slot::{ContentSize, SlotMaker},
        ImageFrame, ItemTooltip, ItemTooltipManager, ItemTooltipable, Tooltip, TooltipManager,
        Tooltipable,
    },
};
use client::Client;
use common::{
    assets::AssetExt,
    combat::{combat_rating, Damage},
    comp::{
        item::{ItemDef, MaterialStatManifest, Quality},
        Body, Energy, Health, Inventory, Poise, Stats,
    },
};
use conrod_core::{
    color,
    widget::{self, Button, Image, Rectangle, Scrollbar, State as ConrodState, Text},
    widget_ids, Color, Colorable, Positionable, Sizeable, UiCell, Widget, WidgetCommon,
};

use crate::hud::slots::SlotKind;
use specs::Entity as EcsEntity;
use std::sync::Arc;
use vek::Vec2;

widget_ids! {
    pub struct InventoryScrollerIds {
        test,
        bag_close,
        inv_alignment,
        inv_grid_1,
        inv_grid_2,
        inv_scrollbar,
        inv_slots_0,
        inv_slots[],
        //tooltip[],
        bg,
        bg_frame,
        char_ico,
        coin_ico,
        //cheese_ico,
        space_txt,
        coin_txt,
        //cheese_txt,
        inventory_title,
        inventory_title_bg,
        scrollbar_bg,
        scrollbar_slots,
    }
}

pub struct InventoryScrollerState {
    ids: InventoryScrollerIds,
}

#[derive(WidgetCommon)]
pub struct InventoryScroller<'a> {
    client: &'a Client,
    imgs: &'a Imgs,
    item_imgs: &'a ItemImgs,
    fonts: &'a Fonts,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    item_tooltip_manager: &'a mut ItemTooltipManager,
    slot_manager: &'a mut SlotManager,
    pulse: f32,
    localized_strings: &'a Localization,
    show_stats: bool,
    show_bag_inv: bool,
    on_right: bool,
    item_tooltip: &'a ItemTooltip<'a>,
    playername: String,
    entity: EcsEntity,
    is_us: bool,
    inventory: &'a Inventory,
    bg_ids: &'a BackgroundIds,
}

impl<'a> InventoryScroller<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        client: &'a Client,
        imgs: &'a Imgs,
        item_imgs: &'a ItemImgs,
        fonts: &'a Fonts,
        item_tooltip_manager: &'a mut ItemTooltipManager,
        slot_manager: &'a mut SlotManager,
        pulse: f32,
        localized_strings: &'a Localization,
        show_stats: bool,
        show_bag_inv: bool,
        on_right: bool,
        item_tooltip: &'a ItemTooltip<'a>,
        playername: String,
        entity: EcsEntity,
        is_us: bool,
        inventory: &'a Inventory,
        bg_ids: &'a BackgroundIds,
    ) -> Self {
        InventoryScroller {
            client,
            imgs,
            item_imgs,
            fonts,
            common: widget::CommonBuilder::default(),
            item_tooltip_manager,
            slot_manager,
            pulse,
            localized_strings,
            show_stats,
            show_bag_inv,
            on_right,
            item_tooltip,
            playername,
            entity,
            is_us,
            inventory,
            bg_ids,
        }
    }

    fn background(&mut self, ui: &mut UiCell<'_>) {
        let mut bg = Image::new(if self.show_stats {
            self.imgs.inv_bg_stats
        } else if self.show_bag_inv {
            self.imgs.inv_bg_bag
        } else {
            self.imgs.inv_bg_armor
        })
        .w_h(424.0, 708.0);
        if self.on_right {
            bg = bg.bottom_right_with_margins_on(ui.window, 60.0, 5.0);
        } else {
            bg = bg.bottom_left_with_margins_on(ui.window, 60.0, 5.0);
        }
        bg.color(Some(UI_MAIN)).set(self.bg_ids.bg, ui);
        Image::new(if self.show_bag_inv {
            self.imgs.inv_frame_bag
        } else {
            self.imgs.inv_frame
        })
        .w_h(424.0, 708.0)
        .middle_of(self.bg_ids.bg)
        .color(Some(UI_HIGHLIGHT_0))
        .set(self.bg_ids.bg_frame, ui);
    }

    fn title(&mut self, state: &mut ConrodState<'_, InventoryScrollerState>, ui: &mut UiCell<'_>) {
        Text::new(
            &self
                .localized_strings
                .get("hud.bag.inventory")
                .replace("{playername}", &*self.playername),
        )
        .mid_top_with_margin_on(self.bg_ids.bg_frame, 9.0)
        .font_id(self.fonts.cyri.conrod_id)
        .font_size(self.fonts.cyri.scale(22))
        .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
        .set(state.ids.inventory_title_bg, ui);
        Text::new(
            &self
                .localized_strings
                .get("hud.bag.inventory")
                .replace("{playername}", &*self.playername),
        )
        .top_left_with_margins_on(state.ids.inventory_title_bg, 2.0, 2.0)
        .font_id(self.fonts.cyri.conrod_id)
        .font_size(self.fonts.cyri.scale(22))
        .color(TEXT_COLOR)
        .set(state.ids.inventory_title, ui);
    }

    fn scrollbar_and_slots(
        &mut self,
        state: &mut ConrodState<'_, InventoryScrollerState>,
        ui: &mut UiCell<'_>,
    ) {
        let space_max = self.inventory.slots().count();
        // Slots Scrollbar
        if space_max > 45 && !self.show_bag_inv {
            // Scrollbar-BG
            Image::new(self.imgs.scrollbar_bg)
                .w_h(9.0, 173.0)
                .bottom_right_with_margins_on(self.bg_ids.bg_frame, 42.0, 3.0)
                .color(Some(UI_HIGHLIGHT_0))
                .set(state.ids.scrollbar_bg, ui);
            // Scrollbar
            Scrollbar::y_axis(state.ids.inv_alignment)
                .thickness(5.0)
                .h(123.0)
                .color(UI_MAIN)
                .middle_of(state.ids.scrollbar_bg)
                .set(state.ids.scrollbar_slots, ui);
        } else if space_max > 135 {
            // Scrollbar-BG
            Image::new(self.imgs.scrollbar_bg_big)
                .w_h(9.0, 592.0)
                .bottom_right_with_margins_on(self.bg_ids.bg_frame, 42.0, 3.0)
                .color(Some(UI_HIGHLIGHT_0))
                .set(state.ids.scrollbar_bg, ui);
            // Scrollbar
            Scrollbar::y_axis(state.ids.inv_alignment)
                .thickness(5.0)
                .h(542.0)
                .color(UI_MAIN)
                .middle_of(state.ids.scrollbar_bg)
                .set(state.ids.scrollbar_slots, ui);
        };
        // Alignment for Grid
        Rectangle::fill_with(
            [362.0, if self.show_bag_inv { 600.0 } else { 200.0 }],
            color::TRANSPARENT,
        )
        .bottom_left_with_margins_on(self.bg_ids.bg_frame, 29.0, 46.5)
        .scroll_kids_vertically()
        .set(state.ids.inv_alignment, ui);

        // Bag Slots
        // Create available inventory slot widgets
        if state.ids.inv_slots.len() < self.inventory.capacity() {
            state.update(|s| {
                s.ids
                    .inv_slots
                    .resize(self.inventory.capacity(), &mut ui.widget_id_generator());
            });
        }
        // Determine the range of inventory slots that are provided by the loadout item
        // that the mouse is over
        let mouseover_loadout_slots = self
            .slot_manager
            .mouse_over_slot
            .and_then(|x| {
                if let SlotKind::Equip(e) = x {
                    self.inventory.get_slot_range_for_equip_slot(e)
                } else {
                    None
                }
            })
            .unwrap_or(0usize..0usize);

        // Display inventory contents
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
            content_source: self.inventory,
            image_source: self.item_imgs,
            slot_manager: Some(self.slot_manager),
            pulse: self.pulse,
        };

        for (i, (pos, item)) in self.inventory.slots_with_id().enumerate() {
            let x = i % 9;
            let y = i / 9;

            // Slot
            let mut slot_widget = slot_maker
                .fabricate(
                    InventorySlot {
                        slot: pos,
                        ours: self.is_us,
                        entity: self.entity,
                    },
                    [40.0; 2],
                )
                .top_left_with_margins_on(
                    state.ids.inv_alignment,
                    0.0 + y as f64 * (40.0),
                    0.0 + x as f64 * (40.0),
                );

            // Highlight slots are provided by the loadout item that the mouse is over
            if mouseover_loadout_slots.contains(&i) {
                slot_widget = slot_widget.with_background_color(Color::Rgba(1.0, 1.0, 1.0, 1.0));
            }

            if let Some(item) = item {
                let quality_col_img = match item.quality() {
                    Quality::Low => self.imgs.inv_slot_grey,
                    Quality::Common => self.imgs.inv_slot,
                    Quality::Moderate => self.imgs.inv_slot_green,
                    Quality::High => self.imgs.inv_slot_blue,
                    Quality::Epic => self.imgs.inv_slot_purple,
                    Quality::Legendary => self.imgs.inv_slot_gold,
                    Quality::Artifact => self.imgs.inv_slot_orange,
                    _ => self.imgs.inv_slot_red,
                };

                let prices_info = self
                    .client
                    .pending_trade()
                    .as_ref()
                    .and_then(|(_, _, prices)| prices.clone());

                slot_widget
                    .filled_slot(quality_col_img)
                    .with_item_tooltip(
                        self.item_tooltip_manager,
                        item,
                        &prices_info,
                        self.item_tooltip,
                    )
                    .set(state.ids.inv_slots[i], ui);
            } else {
                slot_widget.set(state.ids.inv_slots[i], ui);
            }
        }
    }

    fn footer_metrics(
        &mut self,
        state: &mut ConrodState<'_, InventoryScrollerState>,
        ui: &mut UiCell<'_>,
    ) {
        let space_used = self.inventory.populated_slots();
        let space_max = self.inventory.slots().count();
        let bag_space = format!("{}/{}", space_used, space_max);
        let bag_space_percentage = space_used as f32 / space_max as f32;
        let coin_itemdef = Arc::<ItemDef>::load_expect_cloned("common.items.utility.coins");
        let coin_count = self.inventory.item_count(&coin_itemdef);
        // TODO: Reuse this to generally count a stackable item the player selected
        //let cheese_itemdef =
        // Arc::<ItemDef>::load_expect_cloned("common.items.food.cheese");
        // let cheese_count = self.inventory.item_count(&cheese_itemdef);

        // Coin Icon and Coin Text
        Image::new(self.imgs.coin_ico)
            .w_h(16.0, 17.0)
            .bottom_left_with_margins_on(self.bg_ids.bg_frame, 2.0, 43.0)
            .set(state.ids.coin_ico, ui);
        Text::new(&format!("{}", coin_count))
            .bottom_left_with_margins_on(self.bg_ids.bg_frame, 6.0, 64.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(14))
            .color(Color::Rgba(0.871, 0.863, 0.05, 1.0))
            .set(state.ids.coin_txt, ui);
        // TODO: Add a customizable counter for stackable items here
        // TODO: Cheese is funny until it's real
        /*Image::new(self.imgs.cheese_ico)
            .w_h(16.0, 17.0)
            .bottom_left_with_margins_on(self.bg_ids.bg_frame, 2.0, 110.0)
            .set(state.ids.cheese_ico, ui);
        Text::new(&format!("{}", cheese_count))
            .bottom_left_with_margins_on(self.bg_ids.bg_frame, 6.0, 144.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(14))
            .color(Color::Rgba(0.871, 0.863, 0.05, 1.0))
            .set(state.ids.cheese_txt, ui);*/
        //Free Bag-Space
        Text::new(&bag_space)
            .bottom_right_with_margins_on(self.bg_ids.bg_frame, 6.0, 43.0)
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
    }
}

impl<'a> Widget for InventoryScroller<'a> {
    type Event = ();
    type State = InventoryScrollerState;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        InventoryScrollerState {
            ids: InventoryScrollerIds::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(mut self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;
        self.background(ui);
        self.title(state, ui);
        self.scrollbar_and_slots(state, ui);
        self.footer_metrics(state, ui);
    }
}

widget_ids! {
    pub struct BackgroundIds {
        bg,
        bg_frame,
    }
}

widget_ids! {
    pub struct BagIds {
        test,
        inventory_scroller,
        bag_close,
        //tooltip[],
        char_ico,
        coin_ico,
        space_txt,
        inventory_title,
        inventory_title_bg,
        scrollbar_bg,
        scrollbar_slots,
        tab_1,
        tab_2,
        tab_3,
        tab_4,
        bag_expand_btn,
        // Armor Slots
        slots_bg,
        head_slot,
        neck_slot,
        chest_slot,
        shoulders_slot,
        hands_slot,
        legs_slot,
        belt_slot,
        lantern_slot,
        ring1_slot,
        ring2_slot,
        feet_slot,
        back_slot,
        tabard_slot,
        glider_slot,
        mainhand_slot,
        offhand_slot,
        bag1_slot,
        bag2_slot,
        bag3_slot,
        bag4_slot,
        // Stats
        stat_icons[],
        stat_txts[],
    }
}

#[derive(WidgetCommon)]
pub struct Bag<'a> {
    client: &'a Client,
    imgs: &'a Imgs,
    item_imgs: &'a ItemImgs,
    fonts: &'a Fonts,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    rot_imgs: &'a ImgsRot,
    tooltip_manager: &'a mut TooltipManager,
    item_tooltip_manager: &'a mut ItemTooltipManager,
    slot_manager: &'a mut SlotManager,
    pulse: f32,
    localized_strings: &'a Localization,
    stats: &'a Stats,
    health: &'a Health,
    energy: &'a Energy,
    show: &'a Show,
    body: &'a Body,
    msm: &'a MaterialStatManifest,
}

impl<'a> Bag<'a> {
    #[allow(clippy::too_many_arguments)] // TODO: Pending review in #587
    pub fn new(
        client: &'a Client,
        imgs: &'a Imgs,
        item_imgs: &'a ItemImgs,
        fonts: &'a Fonts,
        rot_imgs: &'a ImgsRot,
        tooltip_manager: &'a mut TooltipManager,
        item_tooltip_manager: &'a mut ItemTooltipManager,
        slot_manager: &'a mut SlotManager,
        pulse: f32,
        localized_strings: &'a Localization,
        stats: &'a Stats,
        health: &'a Health,
        energy: &'a Energy,
        show: &'a Show,
        body: &'a Body,
        msm: &'a MaterialStatManifest,
    ) -> Self {
        Self {
            client,
            imgs,
            item_imgs,
            fonts,
            common: widget::CommonBuilder::default(),
            rot_imgs,
            tooltip_manager,
            item_tooltip_manager,
            slot_manager,
            pulse,
            localized_strings,
            stats,
            energy,
            health,
            show,
            body,
            msm,
        }
    }
}
const STATS: [&str; 5] = [
    "Health",
    "Stamina",
    "Protection",
    "Combat Rating",
    "Stun Resilience",
];

pub struct BagState {
    ids: BagIds,
    bg_ids: BackgroundIds,
}

pub enum Event {
    BagExpand,
    Close,
}

impl<'a> Widget for Bag<'a> {
    type Event = Option<Event>;
    type State = BagState;
    type Style = ();

    fn init_state(&self, mut id_gen: widget::id::Generator) -> Self::State {
        BagState {
            bg_ids: BackgroundIds {
                bg: id_gen.next(),
                bg_frame: id_gen.next(),
            },
            ids: BagIds::new(id_gen),
        }
    }

    #[allow(clippy::unused_unit)] // TODO: Pending review in #587
    fn style(&self) -> Self::Style { () }

    #[allow(clippy::useless_format)] // TODO: Pending review in #587
    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;

        let mut event = None;
        let bag_tooltip = Tooltip::new({
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
        let inventories = self.client.inventories();
        let inventory = match inventories.get(self.client.entity()) {
            Some(l) => l,
            None => return None,
        };

        // Tooltips
        let tooltip = Tooltip::new({
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

        InventoryScroller::new(
            self.client,
            self.imgs,
            self.item_imgs,
            self.fonts,
            self.item_tooltip_manager,
            self.slot_manager,
            self.pulse,
            self.localized_strings,
            self.show.stats,
            self.show.bag_inv,
            true,
            &item_tooltip,
            self.stats.name.to_string(),
            self.client.entity(),
            true,
            &inventory,
            &state.bg_ids,
        )
        .set(state.ids.inventory_scroller, ui);

        // Char Pixel-Art
        Image::new(self.imgs.char_art)
            .w_h(40.0, 37.0)
            .top_left_with_margins_on(state.bg_ids.bg, 4.0, 2.0)
            .set(state.ids.char_ico, ui);
        // Button to expand bag
        let txt = if self.show.bag_inv {
            "Show Loadout"
        } else {
            "Expand Bag"
        };
        let expand_btn = Button::image(if self.show.bag_inv {
            self.imgs.collapse_btn
        } else {
            self.imgs.expand_btn
        })
        .w_h(30.0, 17.0)
        .hover_image(if self.show.bag_inv {
            self.imgs.collapse_btn_hover
        } else {
            self.imgs.expand_btn_hover
        })
        .press_image(if self.show.bag_inv {
            self.imgs.collapse_btn_press
        } else {
            self.imgs.expand_btn_press
        });
        // Only show expand button when it's needed...
        let space_max = inventory.slots().count();
        if space_max > 45 && !self.show.bag_inv {
            if expand_btn
                .top_left_with_margins_on(state.bg_ids.bg_frame, 460.0, 211.5)
                .with_tooltip(self.tooltip_manager, &txt, "", &bag_tooltip, TEXT_COLOR)
                .set(state.ids.bag_expand_btn, ui)
                .was_clicked()
            {
                event = Some(Event::BagExpand);
            }
        } else if self.show.bag_inv {
            //... but always show it when the bag is expanded
            if expand_btn
                .top_left_with_margins_on(state.bg_ids.bg_frame, 53.0, 211.5)
                .with_tooltip(self.tooltip_manager, &txt, "", &bag_tooltip, TEXT_COLOR)
                .set(state.ids.bag_expand_btn, ui)
                .was_clicked()
            {
                event = Some(Event::BagExpand);
            }
        }

        // Armor Slots
        let mut slot_maker = SlotMaker {
            empty_slot: self.imgs.armor_slot_empty,
            filled_slot: self.imgs.armor_slot,
            selected_slot: self.imgs.armor_slot_sel,
            background_color: Some(UI_HIGHLIGHT_0),
            content_size: ContentSize {
                width_height_ratio: 1.0,
                max_fraction: 0.75, /* Changes the item image size by setting a maximum
                                     * fraction
                                     * of either the width or height */
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
        let i18n = &self.localized_strings;
        let filled_slot = self.imgs.armor_slot;
        if !self.show.bag_inv {
            // Stat icons and text
            state.update(|s| {
                s.ids
                    .stat_icons
                    .resize(STATS.len(), &mut ui.widget_id_generator())
            });
            state.update(|s| {
                s.ids
                    .stat_txts
                    .resize(STATS.len(), &mut ui.widget_id_generator())
            });
            // Stats
            let combat_rating =
                combat_rating(inventory, self.health, self.stats, *self.body, &self.msm).min(999.9);
            let indicator_col = cr_color(combat_rating);
            for i in STATS.iter().copied().enumerate() {
                let btn = Button::image(match i.1 {
                    "Health" => self.imgs.health_ico,
                    "Stamina" => self.imgs.stamina_ico,
                    "Combat Rating" => self.imgs.combat_rating_ico,
                    "Protection" => self.imgs.protection_ico,
                    "Stun Resilience" => self.imgs.stun_res_ico,
                    _ => self.imgs.nothing,
                })
                .w_h(20.0, 20.0)
                .image_color(if i.1 == "Combat Rating" {
                    indicator_col
                } else {
                    TEXT_COLOR
                });
                let protection_txt = format!(
                    "{}%",
                    (100.0 * Damage::compute_damage_reduction(Some(inventory), Some(self.stats)))
                        as i32
                );
                let health_txt = format!("{}", (self.health.maximum() as f32 / 10.0) as usize);
                let stamina_txt = format!("{}", (self.energy.maximum() as f32 / 10.0) as usize);
                let combat_rating_txt = format!("{}", (combat_rating * 10.0) as usize);
                let stun_res_txt = format!(
                    "{}",
                    (100.0 * Poise::compute_poise_damage_reduction(inventory)) as i32
                );
                let btn = if i.0 == 0 {
                    btn.top_left_with_margins_on(state.bg_ids.bg_frame, 55.0, 10.0)
                } else {
                    btn.down_from(state.ids.stat_icons[i.0 - 1], 7.0)
                };
                let tooltip_head = match i.1 {
                    "Health" => i18n.get("hud.bag.health"),
                    "Stamina" => i18n.get("hud.bag.stamina"),
                    "Combat Rating" => i18n.get("hud.bag.combat_rating"),
                    "Protection" => i18n.get("hud.bag.protection"),
                    "Stun Resilience" => i18n.get("hud.bag.stun_res"),
                    _ => "",
                };
                let tooltip_txt = match i.1 {
                    "Combat Rating" => i18n.get("hud.bag.combat_rating_desc"),
                    "Protection" => i18n.get("hud.bag.protection_desc"),
                    "Stun Resilience" => i18n.get("hud.bag.stun_res_desc"),
                    _ => "",
                };
                btn.with_tooltip(
                    self.tooltip_manager,
                    &tooltip_head,
                    &tooltip_txt,
                    &bag_tooltip,
                    TEXT_COLOR,
                )
                .set(state.ids.stat_icons[i.0], ui);
                Text::new(match i.1 {
                    "Health" => &health_txt,
                    "Stamina" => &stamina_txt,
                    "Combat Rating" => &combat_rating_txt,
                    "Protection" => &protection_txt,
                    "Stun Resilience" => &stun_res_txt,
                    _ => "",
                })
                .right_from(state.ids.stat_icons[i.0], 10.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .color(TEXT_COLOR)
                .graphics_for(state.ids.stat_icons[i.0])
                .set(state.ids.stat_txts[i.0], ui);
            }
            // Loadout Slots
            //  Head
            let head_item = inventory
                .equipped(EquipSlot::Armor(ArmorSlot::Head))
                .map(|item| item.to_owned());

            let slot = slot_maker
                .fabricate(EquipSlot::Armor(ArmorSlot::Head), [45.0; 2])
                .mid_top_with_margin_on(state.bg_ids.bg_frame, 60.0)
                .with_icon(self.imgs.head_bg, Vec2::new(32.0, 40.0), Some(UI_MAIN))
                .filled_slot(filled_slot);
            if let Some(item) = head_item {
                slot.with_item_tooltip(self.item_tooltip_manager, &item, &None, &item_tooltip)
                    .set(state.ids.head_slot, ui)
            } else {
                slot.with_tooltip(
                    self.tooltip_manager,
                    i18n.get("hud.bag.head"),
                    "",
                    &tooltip,
                    color::WHITE,
                )
                .set(state.ids.head_slot, ui)
            }

            //  Necklace
            let neck_item = inventory
                .equipped(EquipSlot::Armor(ArmorSlot::Neck))
                .map(|item| item.to_owned());
            let slot = slot_maker
                .fabricate(EquipSlot::Armor(ArmorSlot::Neck), [45.0; 2])
                .mid_bottom_with_margin_on(state.ids.head_slot, -55.0)
                .with_icon(self.imgs.necklace_bg, Vec2::new(40.0, 31.0), Some(UI_MAIN))
                .filled_slot(filled_slot);
            if let Some(item) = neck_item {
                slot.with_item_tooltip(self.item_tooltip_manager, &item, &None, &item_tooltip)
                    .set(state.ids.neck_slot, ui)
            } else {
                slot.with_tooltip(
                    self.tooltip_manager,
                    i18n.get("hud.bag.neck"),
                    "",
                    &tooltip,
                    color::WHITE,
                )
                .set(state.ids.neck_slot, ui)
            }
            // Chest
            //Image::new(self.imgs.armor_slot) // different graphics for empty/non empty
            let chest_item = inventory
                .equipped(EquipSlot::Armor(ArmorSlot::Chest))
                .map(|item| item.to_owned());

            let slot = slot_maker
                .fabricate(EquipSlot::Armor(ArmorSlot::Chest), [85.0; 2])
                .mid_bottom_with_margin_on(state.ids.neck_slot, -95.0)
                .with_icon(self.imgs.chest_bg, Vec2::new(64.0, 42.0), Some(UI_MAIN))
                .filled_slot(filled_slot);
            if let Some(item) = chest_item {
                slot.with_item_tooltip(self.item_tooltip_manager, &item, &None, &item_tooltip)
                    .set(state.ids.chest_slot, ui)
            } else {
                slot.with_tooltip(
                    self.tooltip_manager,
                    i18n.get("hud.bag.chest"),
                    "",
                    &tooltip,
                    color::WHITE,
                )
                .set(state.ids.chest_slot, ui)
            }
            //  Shoulders
            let shoulder_item = inventory
                .equipped(EquipSlot::Armor(ArmorSlot::Shoulders))
                .map(|item| item.to_owned());

            let slot = slot_maker
                .fabricate(EquipSlot::Armor(ArmorSlot::Shoulders), [70.0; 2])
                .bottom_left_with_margins_on(state.ids.chest_slot, 0.0, -80.0)
                .with_icon(self.imgs.shoulders_bg, Vec2::new(60.0, 36.0), Some(UI_MAIN))
                .filled_slot(filled_slot);
            if let Some(item) = shoulder_item {
                slot.with_item_tooltip(self.item_tooltip_manager, &item, &None, &item_tooltip)
                    .set(state.ids.shoulders_slot, ui)
            } else {
                slot.with_tooltip(
                    self.tooltip_manager,
                    i18n.get("hud.bag.shoulders"),
                    "",
                    &tooltip,
                    color::WHITE,
                )
                .set(state.ids.shoulders_slot, ui)
            }
            // Hands
            let hands_item = inventory
                .equipped(EquipSlot::Armor(ArmorSlot::Hands))
                .map(|item| item.to_owned());
            let slot = slot_maker
                .fabricate(EquipSlot::Armor(ArmorSlot::Hands), [70.0; 2])
                .bottom_right_with_margins_on(state.ids.chest_slot, 0.0, -80.0)
                .with_icon(self.imgs.hands_bg, Vec2::new(55.0, 60.0), Some(UI_MAIN))
                .filled_slot(filled_slot);
            if let Some(item) = hands_item {
                slot.with_item_tooltip(self.item_tooltip_manager, &item, &None, &item_tooltip)
                    .set(state.ids.hands_slot, ui)
            } else {
                slot.with_tooltip(
                    self.tooltip_manager,
                    i18n.get("hud.bag.hands"),
                    "",
                    &tooltip,
                    color::WHITE,
                )
                .set(state.ids.hands_slot, ui)
            }
            // Belt
            let belt_item = inventory
                .equipped(EquipSlot::Armor(ArmorSlot::Belt))
                .map(|item| item.to_owned());
            let slot = slot_maker
                .fabricate(EquipSlot::Armor(ArmorSlot::Belt), [45.0; 2])
                .mid_bottom_with_margin_on(state.ids.chest_slot, -55.0)
                .with_icon(self.imgs.belt_bg, Vec2::new(40.0, 23.0), Some(UI_MAIN))
                .filled_slot(filled_slot);
            if let Some(item) = belt_item {
                slot.with_item_tooltip(self.item_tooltip_manager, &item, &None, &item_tooltip)
                    .set(state.ids.belt_slot, ui)
            } else {
                slot.with_tooltip(
                    self.tooltip_manager,
                    i18n.get("hud.bag.belt"),
                    "",
                    &tooltip,
                    color::WHITE,
                )
                .set(state.ids.belt_slot, ui)
            }
            // Legs
            let legs_item = inventory
                .equipped(EquipSlot::Armor(ArmorSlot::Legs))
                .map(|item| item.to_owned());
            let slot = slot_maker
                .fabricate(EquipSlot::Armor(ArmorSlot::Legs), [85.0; 2])
                .mid_bottom_with_margin_on(state.ids.belt_slot, -95.0)
                .with_icon(self.imgs.legs_bg, Vec2::new(48.0, 70.0), Some(UI_MAIN))
                .filled_slot(filled_slot);
            if let Some(item) = legs_item {
                slot.with_item_tooltip(self.item_tooltip_manager, &item, &None, &item_tooltip)
                    .set(state.ids.legs_slot, ui)
            } else {
                slot.with_tooltip(
                    self.tooltip_manager,
                    i18n.get("hud.bag.legs"),
                    "",
                    &tooltip,
                    color::WHITE,
                )
                .set(state.ids.legs_slot, ui)
            }
            // Ring
            let ring1_item = inventory
                .equipped(EquipSlot::Armor(ArmorSlot::Ring1))
                .map(|item| item.to_owned());
            let slot = slot_maker
                .fabricate(EquipSlot::Armor(ArmorSlot::Ring1), [45.0; 2])
                .bottom_left_with_margins_on(state.ids.hands_slot, -55.0, 0.0)
                .with_icon(self.imgs.ring_bg, Vec2::new(36.0, 40.0), Some(UI_MAIN))
                .filled_slot(filled_slot);
            if let Some(item) = ring1_item {
                slot.with_item_tooltip(self.item_tooltip_manager, &item, &None, &item_tooltip)
                    .set(state.ids.ring1_slot, ui)
            } else {
                slot.with_tooltip(
                    self.tooltip_manager,
                    i18n.get("hud.bag.ring"),
                    "",
                    &tooltip,
                    color::WHITE,
                )
                .set(state.ids.ring1_slot, ui)
            }
            // Ring 2
            let ring2_item = inventory
                .equipped(EquipSlot::Armor(ArmorSlot::Ring2))
                .map(|item| item.to_owned());
            let slot = slot_maker
                .fabricate(EquipSlot::Armor(ArmorSlot::Ring2), [45.0; 2])
                .bottom_right_with_margins_on(state.ids.shoulders_slot, -55.0, 0.0)
                .with_icon(self.imgs.ring_bg, Vec2::new(36.0, 40.0), Some(UI_MAIN))
                .filled_slot(filled_slot);
            if let Some(item) = ring2_item {
                slot.with_item_tooltip(self.item_tooltip_manager, &item, &None, &item_tooltip)
                    .set(state.ids.ring2_slot, ui)
            } else {
                slot.with_tooltip(
                    self.tooltip_manager,
                    i18n.get("hud.bag.ring"),
                    "",
                    &tooltip,
                    color::WHITE,
                )
                .set(state.ids.ring2_slot, ui)
            }
            // Back
            let back_item = inventory
                .equipped(EquipSlot::Armor(ArmorSlot::Back))
                .map(|item| item.to_owned());
            let slot = slot_maker
                .fabricate(EquipSlot::Armor(ArmorSlot::Back), [45.0; 2])
                .down_from(state.ids.ring2_slot, 10.0)
                .with_icon(self.imgs.back_bg, Vec2::new(33.0, 40.0), Some(UI_MAIN))
                .filled_slot(filled_slot);
            if let Some(item) = back_item {
                slot.with_item_tooltip(self.item_tooltip_manager, &item, &None, &item_tooltip)
                    .set(state.ids.back_slot, ui)
            } else {
                slot.with_tooltip(
                    self.tooltip_manager,
                    i18n.get("hud.bag.back"),
                    "",
                    &tooltip,
                    color::WHITE,
                )
                .set(state.ids.back_slot, ui)
            }
            // Foot
            let feet_item = inventory
                .equipped(EquipSlot::Armor(ArmorSlot::Feet))
                .map(|item| item.to_owned());
            let slot = slot_maker
                .fabricate(EquipSlot::Armor(ArmorSlot::Feet), [45.0; 2])
                .down_from(state.ids.ring1_slot, 10.0)
                .with_icon(self.imgs.feet_bg, Vec2::new(32.0, 40.0), Some(UI_MAIN))
                .filled_slot(filled_slot);
            if let Some(item) = feet_item {
                slot.with_item_tooltip(self.item_tooltip_manager, &item, &None, &item_tooltip)
                    .set(state.ids.feet_slot, ui)
            } else {
                slot.with_tooltip(
                    self.tooltip_manager,
                    i18n.get("hud.bag.feet"),
                    "",
                    &tooltip,
                    color::WHITE,
                )
                .set(state.ids.feet_slot, ui)
            }
            // Lantern
            let lantern_item = inventory
                .equipped(EquipSlot::Lantern)
                .map(|item| item.to_owned());
            let slot = slot_maker
                .fabricate(EquipSlot::Lantern, [45.0; 2])
                .top_right_with_margins_on(state.bg_ids.bg_frame, 60.0, 5.0)
                .with_icon(self.imgs.lantern_bg, Vec2::new(24.0, 38.0), Some(UI_MAIN))
                .filled_slot(filled_slot);
            if let Some(item) = lantern_item {
                slot.with_item_tooltip(self.item_tooltip_manager, &item, &None, &item_tooltip)
                    .set(state.ids.lantern_slot, ui)
            } else {
                slot.with_tooltip(
                    self.tooltip_manager,
                    i18n.get("hud.bag.lantern"),
                    "",
                    &tooltip,
                    color::WHITE,
                )
                .set(state.ids.lantern_slot, ui)
            }
            // Glider
            let glider_item = inventory
                .equipped(EquipSlot::Glider)
                .map(|item| item.to_owned());
            let slot = slot_maker
                .fabricate(EquipSlot::Glider, [45.0; 2])
                .down_from(state.ids.lantern_slot, 5.0)
                .with_icon(self.imgs.glider_bg, Vec2::new(38.0, 38.0), Some(UI_MAIN))
                .filled_slot(filled_slot);
            if let Some(item) = glider_item {
                slot.with_item_tooltip(self.item_tooltip_manager, &item, &None, &item_tooltip)
                    .set(state.ids.glider_slot, ui)
            } else {
                slot.with_tooltip(
                    self.tooltip_manager,
                    i18n.get("hud.bag.glider"),
                    "",
                    &tooltip,
                    color::WHITE,
                )
                .set(state.ids.glider_slot, ui)
            }
            // Tabard
            let tabard_item = inventory
                .equipped(EquipSlot::Armor(ArmorSlot::Tabard))
                .map(|item| item.to_owned());
            let slot = slot_maker
                .fabricate(EquipSlot::Armor(ArmorSlot::Tabard), [45.0; 2])
                .down_from(state.ids.glider_slot, 5.0)
                .with_icon(self.imgs.tabard_bg, Vec2::new(38.0, 38.0), Some(UI_MAIN))
                .filled_slot(filled_slot);
            if let Some(item) = tabard_item {
                slot.with_item_tooltip(self.item_tooltip_manager, &item, &None, &item_tooltip)
                    .set(state.ids.tabard_slot, ui)
            } else {
                slot.with_tooltip(
                    self.tooltip_manager,
                    i18n.get("hud.bag.tabard"),
                    "",
                    &tooltip,
                    color::WHITE,
                )
                .set(state.ids.tabard_slot, ui)
            }
            // Mainhand/Left-Slot
            let mainhand_item = inventory
                .equipped(EquipSlot::Mainhand)
                .map(|item| item.to_owned());

            let slot = slot_maker
                .fabricate(EquipSlot::Mainhand, [85.0; 2])
                .bottom_right_with_margins_on(state.ids.back_slot, -95.0, 0.0)
                .with_icon(self.imgs.mainhand_bg, Vec2::new(75.0, 75.0), Some(UI_MAIN))
                .filled_slot(filled_slot);
            if let Some(item) = mainhand_item {
                slot.with_item_tooltip(self.item_tooltip_manager, &item, &None, &item_tooltip)
                    .set(state.ids.mainhand_slot, ui)
            } else {
                slot.with_tooltip(
                    self.tooltip_manager,
                    i18n.get("hud.bag.mainhand"),
                    "",
                    &tooltip,
                    color::WHITE,
                )
                .set(state.ids.mainhand_slot, ui)
            }
            // Offhand/Right-Slot
            let offhand_item = inventory
                .equipped(EquipSlot::Offhand)
                .map(|item| item.to_owned());
            let slot = slot_maker
                .fabricate(EquipSlot::Offhand, [85.0; 2])
                .bottom_left_with_margins_on(state.ids.feet_slot, -95.0, 0.0)
                .with_icon(self.imgs.offhand_bg, Vec2::new(75.0, 75.0), Some(UI_MAIN))
                .filled_slot(filled_slot);
            if let Some(item) = offhand_item {
                slot.with_item_tooltip(self.item_tooltip_manager, &item, &None, &item_tooltip)
                    .set(state.ids.offhand_slot, ui)
            } else {
                slot.with_tooltip(
                    self.tooltip_manager,
                    i18n.get("hud.bag.offhand"),
                    "",
                    &tooltip,
                    color::WHITE,
                )
                .set(state.ids.offhand_slot, ui)
            }
        }
        // Bag 1
        let bag1_item = inventory
            .equipped(EquipSlot::Armor(ArmorSlot::Bag1))
            .map(|item| item.to_owned());
        let slot = slot_maker
            .fabricate(EquipSlot::Armor(ArmorSlot::Bag1), [35.0; 2])
            .bottom_left_with_margins_on(
                state.bg_ids.bg_frame,
                if self.show.bag_inv { 600.0 } else { 167.0 },
                3.0,
            )
            .with_icon(self.imgs.bag_bg, Vec2::new(28.0, 24.0), Some(UI_MAIN))
            .filled_slot(filled_slot);
        if let Some(item) = bag1_item {
            slot.with_item_tooltip(self.item_tooltip_manager, &item, &None, &item_tooltip)
                .set(state.ids.bag1_slot, ui)
        } else {
            slot.with_tooltip(
                self.tooltip_manager,
                i18n.get("hud.bag.bag"),
                "",
                &tooltip,
                color::WHITE,
            )
            .set(state.ids.bag1_slot, ui)
        }
        // Bag 2
        let bag2_item = inventory
            .equipped(EquipSlot::Armor(ArmorSlot::Bag2))
            .map(|item| item.to_owned());
        let slot = slot_maker
            .fabricate(EquipSlot::Armor(ArmorSlot::Bag2), [35.0; 2])
            .down_from(state.ids.bag1_slot, 2.0)
            .with_icon(self.imgs.bag_bg, Vec2::new(28.0, 24.0), Some(UI_MAIN))
            .filled_slot(filled_slot);
        if let Some(item) = bag2_item {
            slot.with_item_tooltip(self.item_tooltip_manager, &item, &None, &item_tooltip)
                .set(state.ids.bag2_slot, ui)
        } else {
            slot.with_tooltip(
                self.tooltip_manager,
                i18n.get("hud.bag.bag"),
                "",
                &tooltip,
                color::WHITE,
            )
            .set(state.ids.bag2_slot, ui)
        }
        // Bag 3
        let bag3_item = inventory
            .equipped(EquipSlot::Armor(ArmorSlot::Bag3))
            .map(|item| item.to_owned());
        let slot = slot_maker
            .fabricate(EquipSlot::Armor(ArmorSlot::Bag3), [35.0; 2])
            .down_from(state.ids.bag2_slot, 2.0)
            .with_icon(self.imgs.bag_bg, Vec2::new(28.0, 24.0), Some(UI_MAIN))
            .filled_slot(filled_slot);
        if let Some(item) = bag3_item {
            slot.with_item_tooltip(self.item_tooltip_manager, &item, &None, &item_tooltip)
                .set(state.ids.bag3_slot, ui)
        } else {
            slot.with_tooltip(
                self.tooltip_manager,
                i18n.get("hud.bag.bag"),
                "",
                &tooltip,
                color::WHITE,
            )
            .set(state.ids.bag3_slot, ui)
        }
        // Bag 4
        let bag4_item = inventory
            .equipped(EquipSlot::Armor(ArmorSlot::Bag4))
            .map(|item| item.to_owned());
        let slot = slot_maker
            .fabricate(EquipSlot::Armor(ArmorSlot::Bag4), [35.0; 2])
            .down_from(state.ids.bag3_slot, 2.0)
            .with_icon(self.imgs.bag_bg, Vec2::new(28.0, 24.0), Some(UI_MAIN))
            .filled_slot(filled_slot);
        if let Some(item) = bag4_item {
            slot.with_item_tooltip(self.item_tooltip_manager, &item, &None, &item_tooltip)
                .set(state.ids.bag4_slot, ui)
        } else {
            slot.with_tooltip(
                self.tooltip_manager,
                i18n.get("hud.bag.bag"),
                "",
                &tooltip,
                color::WHITE,
            )
            .set(state.ids.bag4_slot, ui)
        }

        // Close button
        if Button::image(self.imgs.close_btn)
            .w_h(24.0, 25.0)
            .hover_image(self.imgs.close_btn_hover)
            .press_image(self.imgs.close_btn_press)
            .top_right_with_margins_on(state.bg_ids.bg, 0.0, 0.0)
            .set(state.ids.bag_close, ui)
            .was_clicked()
        {
            event = Some(Event::Close);
        }
        event
    }
}
