use super::{
    cr_color,
    img_ids::{Imgs, ImgsRot},
    item_imgs::ItemImgs,
    slots::{ArmorSlot, EquipSlot, InventorySlot, SlotManager},
    util, HudInfo, Show, CRITICAL_HP_COLOR, LOW_HP_COLOR, TEXT_COLOR, UI_HIGHLIGHT_0, UI_MAIN,
};
use crate::{
    game_input::GameInput,
    ui::{
        fonts::Fonts,
        slot::{ContentSize, SlotMaker},
        ImageFrame, ItemTooltip, ItemTooltipManager, ItemTooltipable, Tooltip, TooltipManager,
        Tooltipable,
    },
    GlobalState,
};
use client::Client;
use common::{
    assets::AssetExt,
    combat::{combat_rating, perception_dist_multiplier_from_stealth, Damage},
    comp::{
        inventory::{slot::Slot, InventorySortOrder},
        item::{ItemDef, ItemDesc, ItemI18n, MaterialStatManifest, Quality},
        Body, Energy, Health, Inventory, Poise, SkillSet, Stats,
    },
};
use conrod_core::{
    color,
    widget::{self, Button, Image, Rectangle, Scrollbar, State as ConrodState, Text},
    widget_ids, Color, Colorable, Positionable, Sizeable, UiCell, Widget, WidgetCommon,
};
use i18n::Localization;
use std::borrow::Cow;

use crate::hud::slots::SlotKind;
use specs::Entity as EcsEntity;
use std::{borrow::Borrow, sync::Arc};
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
        inv_slot_names[],
        inv_slot_amounts[],
        bg,
        bg_frame,
        char_ico,
        coin_ico,
        space_txt,
        coin_txt,
        inventory_title,
        inventory_title_bg,
        scrollbar_bg,
        second_phase_scrollbar_bg,
        scrollbar_slots,
        left_scrollbar_slots,
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
    item_i18n: &'a ItemI18n,
    show_stats: bool,
    show_bag_inv: bool,
    on_right: bool,
    item_tooltip: &'a ItemTooltip<'a>,
    playername: String,
    entity: EcsEntity,
    is_us: bool,
    inventory: &'a Inventory,
    bg_ids: &'a BackgroundIds,
    show_salvage: bool,
    details_mode: bool,
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
        item_i18n: &'a ItemI18n,
        show_stats: bool,
        show_bag_inv: bool,
        on_right: bool,
        item_tooltip: &'a ItemTooltip<'a>,
        playername: String,
        entity: EcsEntity,
        is_us: bool,
        inventory: &'a Inventory,
        bg_ids: &'a BackgroundIds,
        show_salvage: bool,
        details_mode: bool,
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
            item_i18n,
            show_stats,
            show_bag_inv,
            on_right,
            item_tooltip,
            playername,
            entity,
            is_us,
            inventory,
            bg_ids,
            show_salvage,
            details_mode,
        }
    }

    fn background(&mut self, ui: &mut UiCell<'_>) {
        let bg_id = if !self.on_right {
            self.imgs.inv_bg_bag
        } else {
            self.imgs.player_inv_bg_bag
        };

        let img_id = if !self.on_right {
            self.imgs.inv_frame_bag
        } else {
            self.imgs.player_inv_frame_bag
        };

        let mut bg = Image::new(if self.show_stats {
            self.imgs.inv_bg_stats
        } else if self.show_bag_inv {
            bg_id
        } else {
            self.imgs.inv_bg_armor
        })
        .w_h(
            424.0,
            if self.show_bag_inv && !self.on_right {
                548.0
            } else {
                708.0
            },
        );

        if self.on_right {
            bg = bg.bottom_right_with_margins_on(ui.window, 70.0, 5.0);
        } else {
            bg = bg.bottom_left_with_margins_on(ui.window, 230.0, 5.0);
        }

        bg.color(Some(UI_MAIN)).set(self.bg_ids.bg, ui);

        Image::new(if self.show_bag_inv {
            img_id
        } else {
            self.imgs.inv_frame
        })
        .w_h(
            424.0,
            if self.show_bag_inv && !self.on_right {
                548.0
            } else {
                708.0
            },
        )
        .middle_of(self.bg_ids.bg)
        .color(Some(UI_HIGHLIGHT_0))
        .set(self.bg_ids.bg_frame, ui);
    }

    fn title(&mut self, state: &ConrodState<'_, InventoryScrollerState>, ui: &mut UiCell<'_>) {
        Text::new(
            &self
                .localized_strings
                .get_msg_ctx("hud-bag-inventory", &i18n::fluent_args! {
                    "playername" => &*self.playername,
                }),
        )
        .mid_top_with_margin_on(self.bg_ids.bg_frame, 9.0)
        .font_id(self.fonts.cyri.conrod_id)
        .font_size(self.fonts.cyri.scale(22))
        .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
        .set(state.ids.inventory_title_bg, ui);
        Text::new(
            &self
                .localized_strings
                .get_msg_ctx("hud-bag-inventory", &i18n::fluent_args! {
                    "playername" => &*self.playername,
                }),
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
        } else if space_max > 135 && self.on_right {
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

        // This is just for the offeror inventory scrollbar
        if space_max >= 108 && !self.on_right && self.show_bag_inv {
            // Left bag scrollbar background
            Image::new(self.imgs.second_phase_scrollbar_bg)
                .w_h(9.0, 434.0)
                .bottom_right_with_margins_on(self.bg_ids.bg_frame, 42.0, 3.0)
                .color(Some(UI_HIGHLIGHT_0))
                .set(state.ids.second_phase_scrollbar_bg, ui);
            // Left bag scrollbar
            Scrollbar::y_axis(state.ids.inv_alignment)
                .thickness(5.0)
                .h(384.0)
                .color(UI_MAIN)
                .middle_of(state.ids.second_phase_scrollbar_bg)
                .set(state.ids.left_scrollbar_slots, ui);
        }

        let grid_width = 362.0;
        let grid_height = if self.show_bag_inv && !self.on_right {
            440.0 // This for the left bag
        } else if self.show_bag_inv && self.on_right {
            600.0 // This for the expanded right bag
        } else {
            200.0
        };

        // Alignment for Grid
        Rectangle::fill_with([grid_width, grid_height], color::TRANSPARENT)
            .bottom_left_with_margins_on(
                self.bg_ids.bg_frame,
                29.0,
                if self.show_bag_inv && !self.on_right {
                    28.0
                } else {
                    46.5
                },
            )
            .scroll_kids_vertically()
            .set(state.ids.inv_alignment, ui);

        // Bag Slots
        // Create available inventory slot widgets
        if state.ids.inv_slots.len() < self.inventory.capacity() {
            state.update(|s| {
                s.ids.inv_slots.resize(
                    self.inventory.capacity() + self.inventory.overflow_items().count(),
                    &mut ui.widget_id_generator(),
                );
            });
        }
        if state.ids.inv_slot_names.len() < self.inventory.capacity() {
            state.update(|s| {
                s.ids
                    .inv_slot_names
                    .resize(self.inventory.capacity(), &mut ui.widget_id_generator());
            });
        }
        if state.ids.inv_slot_amounts.len() < self.inventory.capacity() {
            state.update(|s| {
                s.ids
                    .inv_slot_amounts
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

        let mut i = 0;
        let mut items = self
            .inventory
            .slots_with_id()
            .map(|(slot, item)| (Slot::Inventory(slot), item.as_ref()))
            .chain(
                self.inventory
                    .overflow_items()
                    .enumerate()
                    .map(|(i, item)| (Slot::Overflow(i), Some(item))),
            )
            .collect::<Vec<_>>();
        if self.details_mode && !self.is_us {
            items.sort_by_cached_key(|(_, item)| {
                (
                    item.is_none(),
                    item.as_ref().map(|i| {
                        (
                            std::cmp::Reverse(i.quality()),
                            {
                                // TODO: we do double the work here, optimize?
                                let (name, _) =
                                    util::item_text(i, self.localized_strings, self.item_i18n);
                                name
                            },
                            i.amount(),
                        )
                    }),
                )
            });
        }
        for (pos, item) in items.into_iter() {
            if self.details_mode && !self.is_us && item.is_none() {
                continue;
            }
            let (x, y) = if self.details_mode {
                (0, i)
            } else {
                (i % 9, i / 9)
            };
            let slot_size = if self.details_mode { 20.0 } else { 40.0 };

            // Slot
            let mut slot_widget = slot_maker
                .fabricate(
                    InventorySlot {
                        slot: pos,
                        ours: self.is_us,
                        entity: self.entity,
                    },
                    [slot_size as f32; 2],
                )
                .top_left_with_margins_on(
                    state.ids.inv_alignment,
                    0.0 + y as f64 * slot_size,
                    0.0 + x as f64 * slot_size,
                );

            // Highlight slots are provided by the loadout item that the mouse is over
            if mouseover_loadout_slots.contains(&i) {
                slot_widget = slot_widget.with_background_color(Color::Rgba(1.0, 1.0, 1.0, 1.0));
            }

            if self.show_salvage && item.as_ref().map_or(false, |item| item.is_salvageable()) {
                slot_widget = slot_widget.with_background_color(Color::Rgba(1.0, 1.0, 1.0, 1.0));
            }

            if let Some(item) = item {
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

                let prices_info = self
                    .client
                    .pending_trade()
                    .as_ref()
                    .and_then(|(_, _, prices)| prices.clone());

                if self.show_salvage && item.is_salvageable() {
                    let salvage_result: Vec<_> = item
                        .salvage_output()
                        .map(|(material_id, _)| Arc::<ItemDef>::load_expect_cloned(material_id))
                        .map(|item| item as Arc<dyn ItemDesc>)
                        .collect();

                    let items = salvage_result
                        .iter()
                        .map(|item| item.borrow())
                        .chain(core::iter::once(item as &dyn ItemDesc));

                    slot_widget
                        .filled_slot(quality_col_img)
                        .with_item_tooltip(
                            self.item_tooltip_manager,
                            items,
                            &prices_info,
                            self.item_tooltip,
                        )
                        .set(state.ids.inv_slots[i], ui);
                } else {
                    slot_widget
                        .filled_slot(quality_col_img)
                        .with_item_tooltip(
                            self.item_tooltip_manager,
                            core::iter::once(item as &dyn ItemDesc),
                            &prices_info,
                            self.item_tooltip,
                        )
                        .set(state.ids.inv_slots[i], ui);
                }
                if self.details_mode {
                    let (name, _) = util::item_text(item, self.localized_strings, self.item_i18n);
                    Text::new(&name)
                        .top_left_with_margins_on(
                            state.ids.inv_alignment,
                            0.0 + y as f64 * slot_size,
                            30.0 + x as f64 * slot_size,
                        )
                        .font_id(self.fonts.cyri.conrod_id)
                        .font_size(self.fonts.cyri.scale(14))
                        .color(color::WHITE)
                        .set(state.ids.inv_slot_names[i], ui);

                    Text::new(&format!("{}", item.amount()))
                        .top_left_with_margins_on(
                            state.ids.inv_alignment,
                            0.0 + y as f64 * slot_size,
                            grid_width - 40.0 + x as f64 * slot_size,
                        )
                        .font_id(self.fonts.cyri.conrod_id)
                        .font_size(self.fonts.cyri.scale(14))
                        .color(color::WHITE)
                        .set(state.ids.inv_slot_amounts[i], ui);
                }
            } else {
                slot_widget.set(state.ids.inv_slots[i], ui);
            }
            i += 1;
        }
    }

    fn footer_metrics(
        &mut self,
        state: &ConrodState<'_, InventoryScrollerState>,
        ui: &mut UiCell<'_>,
    ) {
        let space_used = self.inventory.populated_slots();
        let space_max = self.inventory.slots().count();
        let bag_space = format!("{}/{}", space_used, space_max);
        let bag_space_percentage = space_used as f32 / space_max as f32;
        //let coin_itemdef =
        // Arc::<ItemDef>::load_expect_cloned("common.items.utility.coins"); let
        // coin_count = self.inventory.item_count(&coin_itemdef); TODO: Reuse
        // this to generally count a stackable item the player selected
        // let cheese_itemdef =
        // Arc::<ItemDef>::load_expect_cloned("common.items.food.cheese");
        // let cheese_count = self.inventory.item_count(&cheese_itemdef);

        // Coin Icon and Coin Text
        /*Image::new(self.imgs.coin_ico)
            .w_h(16.0, 17.0)
            .bottom_left_with_margins_on(self.bg_ids.bg_frame, 2.0, 43.0)
            .set(state.ids.coin_ico, ui);
        Text::new(&format!("{}", coin_count))
            .bottom_left_with_margins_on(self.bg_ids.bg_frame, 6.0, 64.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(14))
            .color(Color::Rgba(0.871, 0.863, 0.05, 1.0))
            .set(state.ids.coin_txt, ui);*/
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
        inventory_sort,
        scrollbar_bg,
        scrollbar_slots,
        tab_1,
        tab_2,
        tab_3,
        tab_4,
        bag_expand_btn,
        bag_details_btn,
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
        active_mainhand_slot,
        active_offhand_slot,
        inactive_mainhand_slot,
        inactive_offhand_slot,
        swap_equipped_weapons_btn,
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
    info: &'a HudInfo,
    global_state: &'a GlobalState,
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
    item_i18n: &'a ItemI18n,
    stats: &'a Stats,
    skill_set: &'a SkillSet,
    health: &'a Health,
    energy: &'a Energy,
    show: &'a Show,
    body: &'a Body,
    msm: &'a MaterialStatManifest,
    poise: &'a Poise,
}

impl<'a> Bag<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        client: &'a Client,
        info: &'a HudInfo,
        global_state: &'a GlobalState,
        imgs: &'a Imgs,
        item_imgs: &'a ItemImgs,
        fonts: &'a Fonts,
        rot_imgs: &'a ImgsRot,
        tooltip_manager: &'a mut TooltipManager,
        item_tooltip_manager: &'a mut ItemTooltipManager,
        slot_manager: &'a mut SlotManager,
        pulse: f32,
        localized_strings: &'a Localization,
        item_i18n: &'a ItemI18n,
        stats: &'a Stats,
        skill_set: &'a SkillSet,
        health: &'a Health,
        energy: &'a Energy,
        show: &'a Show,
        body: &'a Body,
        msm: &'a MaterialStatManifest,
        poise: &'a Poise,
    ) -> Self {
        Self {
            client,
            info,
            global_state,
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
            item_i18n,
            stats,
            skill_set,
            energy,
            health,
            show,
            body,
            msm,
            poise,
        }
    }
}
const STATS: [&str; 6] = [
    "Health",
    "Energy",
    "Protection",
    "Combat Rating",
    "Stun Resilience",
    "Stealth",
];

pub struct BagState {
    ids: BagIds,
    bg_ids: BackgroundIds,
}

pub enum Event {
    BagExpand,
    Close,
    SortInventory,
    SwapEquippedWeapons,
    SetDetailsMode(bool),
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

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        common_base::prof_span!("Bag::update");
        let widget::UpdateArgs { state, ui, .. } = args;
        let i18n = &self.localized_strings;
        let key_layout = &self.global_state.window.key_layout;

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
        let inventory = match inventories.get(self.info.viewpoint_entity) {
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

        InventoryScroller::new(
            self.client,
            self.imgs,
            self.item_imgs,
            self.fonts,
            self.item_tooltip_manager,
            self.slot_manager,
            self.pulse,
            self.localized_strings,
            self.item_i18n,
            self.show.stats,
            self.show.bag_inv,
            true,
            &item_tooltip,
            self.stats.name.to_string(),
            self.info.viewpoint_entity,
            true,
            inventory,
            &state.bg_ids,
            self.show.crafting_fields.salvage,
            self.show.bag_details,
        )
        .set(state.ids.inventory_scroller, ui);

        // Char Pixel-Art
        Image::new(self.imgs.char_art)
            .w_h(40.0, 37.0)
            .top_left_with_margins_on(state.bg_ids.bg, 4.0, 2.0)
            .set(state.ids.char_ico, ui);

        let buttons_top = if self.show.bag_inv { 53.0 } else { 460.0 };
        let (txt, btn, hover, press) = if self.show.bag_details {
            (
                "Grid mode",
                self.imgs.grid_btn,
                self.imgs.grid_btn_hover,
                self.imgs.grid_btn_press,
            )
        } else {
            (
                "List mode",
                self.imgs.list_btn,
                self.imgs.list_btn_hover,
                self.imgs.list_btn_press,
            )
        };
        let details_btn = Button::image(btn)
            .w_h(32.0, 17.0)
            .hover_image(hover)
            .press_image(press);
        if details_btn
            .mid_top_with_margin_on(state.bg_ids.bg_frame, buttons_top)
            .with_tooltip(self.tooltip_manager, txt, "", &bag_tooltip, TEXT_COLOR)
            .set(state.ids.bag_details_btn, ui)
            .was_clicked()
        {
            event = Some(Event::SetDetailsMode(!self.show.bag_details));
        }
        // Button to expand bag
        let (txt, btn, hover, press) = if self.show.bag_inv {
            (
                "Show Loadout",
                self.imgs.collapse_btn,
                self.imgs.collapse_btn_hover,
                self.imgs.collapse_btn_press,
            )
        } else {
            (
                "Expand Bag",
                self.imgs.expand_btn,
                self.imgs.expand_btn_hover,
                self.imgs.expand_btn_press,
            )
        };
        let expand_btn = Button::image(btn)
            .w_h(30.0, 17.0)
            .hover_image(hover)
            .press_image(press);

        // Only show expand button when it's needed...
        if (inventory.slots().count() > 45 || self.show.bag_inv)
            && expand_btn
                .top_right_with_margins_on(state.bg_ids.bg_frame, buttons_top, 37.0)
                .with_tooltip(self.tooltip_manager, txt, "", &bag_tooltip, TEXT_COLOR)
                .set(state.ids.bag_expand_btn, ui)
                .was_clicked()
        {
            event = Some(Event::BagExpand);
        }

        // Sort inventory button
        if Button::image(self.imgs.inv_sort_btn)
            .w_h(30.0, 17.0)
            .hover_image(self.imgs.inv_sort_btn_hover)
            .press_image(self.imgs.inv_sort_btn_press)
            .top_left_with_margins_on(state.bg_ids.bg_frame, buttons_top, 47.0)
            .with_tooltip(
                self.tooltip_manager,
                &(match inventory.next_sort_order() {
                    InventorySortOrder::Name => i18n.get_msg("hud-bag-sort_by_name"),
                    InventorySortOrder::Quality => i18n.get_msg("hud-bag-sort_by_quality"),
                    InventorySortOrder::Category => i18n.get_msg("hud-bag-sort_by_category"),
                    InventorySortOrder::Tag => i18n.get_msg("hud-bag-sort_by_tag"),
                    InventorySortOrder::Amount => i18n.get_msg("hud-bag-sort_by_quantity"),
                }),
                "",
                &tooltip,
                color::WHITE,
            )
            .set(state.ids.inventory_sort, ui)
            .was_clicked()
        {
            event = Some(Event::SortInventory);
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

        // NOTE: Yes, macros considered harmful.
        // Though, this code mutably captures two different fields of `self`
        // This works because it's different branches of if-let
        // so in reality borrow checker allows you to do this as you
        // capture only one field.
        //
        // The less impossible, but still tricky part is denote type of
        // `$slot_maker` which has 1 lifetype parameter and 3 type parameters
        // in such way that it implements all traits conrod needs.
        //
        // And final part is that this uses that much of arguments
        // that just by passing all of them, you will get about the same
        // amount of lines this macro has or even more.
        //
        // So considering how many times we copy-paste this code
        // and how easy this macro looks it sounds like lawful evil.
        //
        // What this actually does is checks if we have equipped item on this slot
        // and if we do, display item tooltip for it.
        // If not, just show text of slot name.
        macro_rules! set_tooltip {
            ($slot_maker:expr, $slot_id:expr, $slot:expr, $desc:expr) => {
                if let Some(item) = inventory.equipped($slot) {
                    let manager = &mut *self.item_tooltip_manager;
                    $slot_maker
                        .with_item_tooltip(
                            manager,
                            core::iter::once(item as &dyn ItemDesc),
                            &None,
                            &item_tooltip,
                        )
                        .set($slot_id, ui)
                } else {
                    let manager = &mut *self.tooltip_manager;
                    $slot_maker
                        .with_tooltip(manager, &i18n.get_msg($desc), "", &tooltip, color::WHITE)
                        .set($slot_id, ui)
                }
            };
        }

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
            let combat_rating = combat_rating(
                inventory,
                self.health,
                self.energy,
                self.poise,
                self.skill_set,
                *self.body,
                self.msm,
            )
            .min(999.9);
            let indicator_col = cr_color(combat_rating);
            for i in STATS.iter().copied().enumerate() {
                let btn = Button::image(match i.1 {
                    "Health" => self.imgs.health_ico,
                    "Energy" => self.imgs.energy_ico,
                    "Combat Rating" => self.imgs.combat_rating_ico,
                    "Protection" => self.imgs.protection_ico,
                    "Stun Resilience" => self.imgs.stun_res_ico,
                    "Stealth" => self.imgs.stealth_rating_ico,
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
                    (100.0
                        * Damage::compute_damage_reduction(
                            None,
                            Some(inventory),
                            Some(self.stats),
                            self.msm
                        )) as i32
                );
                let health_txt = format!("{}", self.health.maximum().round() as usize);
                let energy_txt = format!("{}", self.energy.maximum().round() as usize);
                let combat_rating_txt = format!("{}", (combat_rating * 10.0) as usize);
                let stun_res_txt = format!(
                    "{}",
                    (100.0
                        * Poise::compute_poise_damage_reduction(
                            Some(inventory),
                            self.msm,
                            None,
                            None
                        )) as i32
                );
                let stealth_txt = format!(
                    "{:.1}%",
                    ((1.0
                        - perception_dist_multiplier_from_stealth(
                            Some(inventory),
                            None,
                            self.msm
                        ))
                        * 100.0)
                );
                let btn = if i.0 == 0 {
                    btn.top_left_with_margins_on(state.bg_ids.bg_frame, 55.0, 10.0)
                } else {
                    btn.down_from(state.ids.stat_icons[i.0 - 1], 7.0)
                };
                let tooltip_head = match i.1 {
                    "Health" => i18n.get_msg("hud-bag-health"),
                    "Energy" => i18n.get_msg("hud-bag-energy"),
                    "Combat Rating" => i18n.get_msg("hud-bag-combat_rating"),
                    "Protection" => i18n.get_msg("hud-bag-protection"),
                    "Stun Resilience" => i18n.get_msg("hud-bag-stun_res"),
                    "Stealth" => i18n.get_msg("hud-bag-stealth"),
                    _ => Cow::Borrowed(""),
                };
                let tooltip_txt = match i.1 {
                    "Combat Rating" => i18n.get_msg("hud-bag-combat_rating_desc"),
                    "Protection" => i18n.get_msg("hud-bag-protection_desc"),
                    "Stun Resilience" => i18n.get_msg("hud-bag-stun_res_desc"),
                    _ => Cow::Borrowed(""),
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
                    "Energy" => &energy_txt,
                    "Combat Rating" => &combat_rating_txt,
                    "Protection" => &protection_txt,
                    "Stun Resilience" => &stun_res_txt,
                    "Stealth" => &stealth_txt,
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
            let item_slot = EquipSlot::Armor(ArmorSlot::Head);
            let slot = slot_maker
                .fabricate(item_slot, [45.0; 2])
                .mid_top_with_margin_on(state.bg_ids.bg_frame, 60.0)
                .with_icon(self.imgs.head_bg, Vec2::new(32.0, 40.0), Some(UI_MAIN))
                .filled_slot(filled_slot);

            let slot_id = state.ids.head_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-head");

            //  Necklace
            let item_slot = EquipSlot::Armor(ArmorSlot::Neck);
            let slot = slot_maker
                .fabricate(item_slot, [45.0; 2])
                .mid_bottom_with_margin_on(state.ids.head_slot, -55.0)
                .with_icon(self.imgs.necklace_bg, Vec2::new(40.0, 31.0), Some(UI_MAIN))
                .filled_slot(filled_slot);

            let slot_id = state.ids.neck_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-neck");

            // Chest
            //Image::new(self.imgs.armor_slot) // different graphics for empty/non empty
            let item_slot = EquipSlot::Armor(ArmorSlot::Chest);
            let slot = slot_maker
                .fabricate(item_slot, [85.0; 2])
                .mid_bottom_with_margin_on(state.ids.neck_slot, -95.0)
                .with_icon(self.imgs.chest_bg, Vec2::new(64.0, 42.0), Some(UI_MAIN))
                .filled_slot(filled_slot);

            let slot_id = state.ids.chest_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-chest");

            //  Shoulders
            let item_slot = EquipSlot::Armor(ArmorSlot::Shoulders);
            let slot = slot_maker
                .fabricate(item_slot, [70.0; 2])
                .bottom_left_with_margins_on(state.ids.chest_slot, 0.0, -80.0)
                .with_icon(self.imgs.shoulders_bg, Vec2::new(60.0, 36.0), Some(UI_MAIN))
                .filled_slot(filled_slot);

            let slot_id = state.ids.shoulders_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-shoulders");

            // Hands
            let item_slot = EquipSlot::Armor(ArmorSlot::Hands);
            let slot = slot_maker
                .fabricate(item_slot, [70.0; 2])
                .bottom_right_with_margins_on(state.ids.chest_slot, 0.0, -80.0)
                .with_icon(self.imgs.hands_bg, Vec2::new(55.0, 60.0), Some(UI_MAIN))
                .filled_slot(filled_slot);

            let slot_id = state.ids.hands_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-hands");

            // Belt
            let item_slot = EquipSlot::Armor(ArmorSlot::Belt);
            let slot = slot_maker
                .fabricate(item_slot, [45.0; 2])
                .mid_bottom_with_margin_on(state.ids.chest_slot, -55.0)
                .with_icon(self.imgs.belt_bg, Vec2::new(40.0, 23.0), Some(UI_MAIN))
                .filled_slot(filled_slot);

            let slot_id = state.ids.belt_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-belt");

            // Legs
            let item_slot = EquipSlot::Armor(ArmorSlot::Legs);
            let slot = slot_maker
                .fabricate(item_slot, [85.0; 2])
                .mid_bottom_with_margin_on(state.ids.belt_slot, -95.0)
                .with_icon(self.imgs.legs_bg, Vec2::new(48.0, 70.0), Some(UI_MAIN))
                .filled_slot(filled_slot);

            let slot_id = state.ids.legs_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-legs");

            // Ring
            let item_slot = EquipSlot::Armor(ArmorSlot::Ring1);
            let slot = slot_maker
                .fabricate(item_slot, [45.0; 2])
                .bottom_left_with_margins_on(state.ids.hands_slot, -55.0, 0.0)
                .with_icon(self.imgs.ring_bg, Vec2::new(36.0, 40.0), Some(UI_MAIN))
                .filled_slot(filled_slot);

            let slot_id = state.ids.ring1_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-ring");

            // Ring 2
            let item_slot = EquipSlot::Armor(ArmorSlot::Ring2);
            let slot = slot_maker
                .fabricate(item_slot, [45.0; 2])
                .bottom_right_with_margins_on(state.ids.shoulders_slot, -55.0, 0.0)
                .with_icon(self.imgs.ring_bg, Vec2::new(36.0, 40.0), Some(UI_MAIN))
                .filled_slot(filled_slot);

            let slot_id = state.ids.ring2_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-ring");

            // Back
            let item_slot = EquipSlot::Armor(ArmorSlot::Back);
            let slot = slot_maker
                .fabricate(item_slot, [45.0; 2])
                .down_from(state.ids.ring2_slot, 10.0)
                .with_icon(self.imgs.back_bg, Vec2::new(33.0, 40.0), Some(UI_MAIN))
                .filled_slot(filled_slot);

            let slot_id = state.ids.back_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-back");

            // Foot
            let item_slot = EquipSlot::Armor(ArmorSlot::Feet);
            let slot = slot_maker
                .fabricate(item_slot, [45.0; 2])
                .down_from(state.ids.ring1_slot, 10.0)
                .with_icon(self.imgs.feet_bg, Vec2::new(32.0, 40.0), Some(UI_MAIN))
                .filled_slot(filled_slot);

            let slot_id = state.ids.feet_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-feet");

            // Lantern
            let item_slot = EquipSlot::Lantern;
            let slot = slot_maker
                .fabricate(item_slot, [45.0; 2])
                .top_right_with_margins_on(state.bg_ids.bg_frame, 60.0, 5.0)
                .with_icon(self.imgs.lantern_bg, Vec2::new(24.0, 38.0), Some(UI_MAIN))
                .filled_slot(filled_slot);

            let slot_id = state.ids.lantern_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-lantern");

            // Glider
            let item_slot = EquipSlot::Glider;
            let slot = slot_maker
                .fabricate(item_slot, [45.0; 2])
                .down_from(state.ids.lantern_slot, 5.0)
                .with_icon(self.imgs.glider_bg, Vec2::new(38.0, 38.0), Some(UI_MAIN))
                .filled_slot(filled_slot);

            let slot_id = state.ids.glider_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-glider");

            // Tabard
            let item_slot = EquipSlot::Armor(ArmorSlot::Tabard);
            let slot = slot_maker
                .fabricate(item_slot, [45.0; 2])
                .down_from(state.ids.glider_slot, 5.0)
                .with_icon(self.imgs.tabard_bg, Vec2::new(38.0, 38.0), Some(UI_MAIN))
                .filled_slot(filled_slot);

            let slot_id = state.ids.tabard_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-tabard");

            // Active Mainhand/Left-Slot
            let item_slot = EquipSlot::ActiveMainhand;
            let slot = slot_maker
                .fabricate(item_slot, [85.0; 2])
                .bottom_right_with_margins_on(state.ids.back_slot, -95.0, 0.0)
                .with_icon(self.imgs.mainhand_bg, Vec2::new(75.0, 75.0), Some(UI_MAIN))
                .filled_slot(filled_slot);

            let slot_id = state.ids.active_mainhand_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-mainhand");

            // Active Offhand/Right-Slot
            let item_slot = EquipSlot::ActiveOffhand;
            let slot = slot_maker
                .fabricate(item_slot, [85.0; 2])
                .bottom_left_with_margins_on(state.ids.feet_slot, -95.0, 0.0)
                .with_icon(self.imgs.offhand_bg, Vec2::new(75.0, 75.0), Some(UI_MAIN))
                .filled_slot(filled_slot);

            let slot_id = state.ids.active_offhand_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-offhand");

            // Inactive Mainhand/Left-Slot
            let item_slot = EquipSlot::InactiveMainhand;
            let slot = slot_maker
                .fabricate(item_slot, [40.0; 2])
                .bottom_right_with_margins_on(state.ids.active_mainhand_slot, 3.0, -47.0)
                .with_icon(self.imgs.mainhand_bg, Vec2::new(35.0, 35.0), Some(UI_MAIN))
                .filled_slot(filled_slot);

            let slot_id = state.ids.inactive_mainhand_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-inactive_mainhand");

            // Inactive Offhand/Right-Slot
            let item_slot = EquipSlot::InactiveOffhand;
            let slot = slot_maker
                .fabricate(item_slot, [40.0; 2])
                .bottom_left_with_margins_on(state.ids.active_offhand_slot, 3.0, -47.0)
                .with_icon(self.imgs.offhand_bg, Vec2::new(35.0, 35.0), Some(UI_MAIN))
                .filled_slot(filled_slot);

            let slot_id = state.ids.inactive_offhand_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-inactive_offhand");

            if Button::image(self.imgs.swap_equipped_weapons_btn)
                .hover_image(self.imgs.swap_equipped_weapons_btn_hover)
                .press_image(self.imgs.swap_equipped_weapons_btn_press)
                .w_h(32.0, 40.0)
                .bottom_left_with_margins_on(state.bg_ids.bg_frame, 0.0, 23.3)
                .align_middle_y_of(state.ids.active_mainhand_slot)
                .with_tooltip(
                    self.tooltip_manager,
                    &i18n.get_msg("hud-bag-swap_equipped_weapons_title"),
                    &(if let Some(key) = self
                        .global_state
                        .settings
                        .controls
                        .get_binding(GameInput::SwapLoadout)
                    {
                        i18n.get_msg_ctx(
                            "hud-bag-swap_equipped_weapons_desc",
                            &i18n::fluent_args! {
                                "key" => key.display_string(key_layout)
                            },
                        )
                    } else {
                        Cow::Borrowed("")
                    }),
                    &tooltip,
                    color::WHITE,
                )
                .set(state.ids.swap_equipped_weapons_btn, ui)
                .was_clicked()
            {
                event = Some(Event::SwapEquippedWeapons);
            }
        }

        // Bag 1
        let item_slot = EquipSlot::Armor(ArmorSlot::Bag1);
        let slot = slot_maker
            .fabricate(item_slot, [35.0; 2])
            .bottom_left_with_margins_on(
                state.bg_ids.bg_frame,
                if self.show.bag_inv { 600.0 } else { 167.0 },
                3.0,
            )
            .with_icon(self.imgs.bag_bg, Vec2::new(28.0, 24.0), Some(UI_MAIN))
            .filled_slot(filled_slot);

        let slot_id = state.ids.bag1_slot;
        set_tooltip!(slot, slot_id, item_slot, "hud-bag-bag");

        // Bag 2
        let item_slot = EquipSlot::Armor(ArmorSlot::Bag2);
        let slot = slot_maker
            .fabricate(item_slot, [35.0; 2])
            .down_from(state.ids.bag1_slot, 2.0)
            .with_icon(self.imgs.bag_bg, Vec2::new(28.0, 24.0), Some(UI_MAIN))
            .filled_slot(filled_slot);

        let slot_id = state.ids.bag2_slot;
        set_tooltip!(slot, slot_id, item_slot, "hud-bag-bag");

        // Bag 3
        let item_slot = EquipSlot::Armor(ArmorSlot::Bag3);
        let slot = slot_maker
            .fabricate(item_slot, [35.0; 2])
            .down_from(state.ids.bag2_slot, 2.0)
            .with_icon(self.imgs.bag_bg, Vec2::new(28.0, 24.0), Some(UI_MAIN))
            .filled_slot(filled_slot);

        let slot_id = state.ids.bag3_slot;
        set_tooltip!(slot, slot_id, item_slot, "hud-bag-bag");

        // Bag 4
        let item_slot = EquipSlot::Armor(ArmorSlot::Bag4);
        let slot = slot_maker
            .fabricate(item_slot, [35.0; 2])
            .down_from(state.ids.bag3_slot, 2.0)
            .with_icon(self.imgs.bag_bg, Vec2::new(28.0, 24.0), Some(UI_MAIN))
            .filled_slot(filled_slot);

        let slot_id = state.ids.bag4_slot;
        set_tooltip!(slot, slot_id, item_slot, "hud-bag-bag");

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
