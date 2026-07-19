use super::{
    CRITICAL_HP_COLOR, HudInfo, LOW_HP_COLOR, Show, SlotGrid, TEXT_COLOR, UI_HIGHLIGHT_0, UI_MAIN,
    cr_color,
    img_ids::{Imgs, ImgsRot},
    item_imgs::ItemImgs,
    slot_grid::SlotEvents,
    slots::{ArmorSlot, EquipSlot, SlotManager},
};
use crate::{
    GlobalState,
    game_input::GameInput,
    hud::slot_grid::TabFilters,
    settings::{
        HudPositionSettings,
        hud_position::{
            DEFAULT_OTHER_BAG_HEIGHT, DEFAULT_OTHER_BAG_WIDTH, DEFAULT_OWN_BAG_HEIGHT,
            DEFAULT_OWN_BAG_WIDTH,
        },
    },
    ui::{
        ImageFrame, ItemTooltip, ItemTooltipManager, ItemTooltipable, Tooltip, TooltipManager,
        Tooltipable,
        fonts::Fonts,
        slot::{ContentSize, SlotMaker},
    },
    window::MenuInput,
};
use client::Client;
use common::{
    combat::{Damage, combat_rating, perception_dist_multiplier_from_stealth},
    comp::{
        Body, Energy, Health, Inventory, Poise, SkillSet, Stats,
        inventory::InventorySortOrder,
        item::{ItemDesc, ItemI18n, MaterialStatManifest},
    },
    recipe::RecipeBookManifest,
};
use conrod_core::{
    Color, Colorable, Positionable, Sizeable, UiCell, Widget, WidgetCommon, builder_method, color,
    widget::{self, Button, Image, Rectangle, Scrollbar, State as ConrodState, Text},
    widget_ids,
};
use i18n::Localization;
use std::borrow::Cow;

use specs::Entity as EcsEntity;
use vek::{Vec2, approx::AbsDiffEq};

const STATS: [&str; 6] = [
    "Health",
    "Energy",
    "Protection",
    "Combat Rating",
    "Stun Resilience",
    "Stealth",
];

widget_ids! {
    pub struct InventoryScrollerIds {
        draggable_area,
        inv_alignment,
        slot_grid,
        //coin_ico,
        space_txt,
        //coin_txt,
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

pub enum InventoryScrollerEvent {
    Drag(Vec2<f64>),
    Close,
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
    menu_events: &'a Vec<MenuInput>,
    active_content: usize,
    localized_strings: &'a Localization,
    item_i18n: &'a ItemI18n,
    show_stats: bool,
    show_bag_inv: bool,
    on_right: bool,
    global_state: &'a GlobalState,
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
    #[expect(clippy::too_many_arguments)]
    pub fn new(
        client: &'a Client,
        global_state: &'a GlobalState,
        imgs: &'a Imgs,
        item_imgs: &'a ItemImgs,
        fonts: &'a Fonts,
        item_tooltip_manager: &'a mut ItemTooltipManager,
        slot_manager: &'a mut SlotManager,
        pulse: f32,
        menu_events: &'a Vec<MenuInput>,
        active_content: usize,
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
            menu_events,
            active_content,
            localized_strings,
            item_i18n,
            show_stats,
            show_bag_inv,
            on_right,
            global_state,
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
        let bag_settings = &self.global_state.settings.hud_position;
        let bag_pos = if !self.on_right {
            bag_settings.bag.other
        } else {
            bag_settings.bag.own
        };

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
            bg = bg.bottom_right_with_margins_on(ui.window, bag_pos.y, bag_pos.x);
        } else {
            bg = bg.bottom_left_with_margins_on(ui.window, bag_pos.y, bag_pos.x);
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
        events: &mut Vec<InventoryScrollerEvent>,
        ui: &mut UiCell<'_>,
    ) {
        // MENU INPUTS: change the inventory button/filter focus
        // LocalFocus: change local window focus
        if self.active_content == 1 {
            for event in self.menu_events {
                match *event {
                    MenuInput::Apply => {
                        // TODO
                    },
                    MenuInput::Back => {
                        events.push(InventoryScrollerEvent::Close);
                    },
                    _ => {},
                }
            }
        }

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
        for event in SlotGrid::new(
            self.client,
            self.imgs,
            self.item_imgs,
            self.fonts,
            self.item_tooltip_manager,
            self.slot_manager,
            self.inventory,
            self.item_tooltip,
            self.localized_strings,
            self.item_i18n,
            self.entity,
            &self.global_state.window.last_input(),
            self.pulse,
            self.menu_events,
            self.is_us,
            self.details_mode,
            self.show_salvage,
        )
        .columns(9) // 6 columns and default spacing is better imo
        .spacing(0.0)
        .slot_size(if self.details_mode { 20.0 } else { 40.0 }) // 55.0 for 6 columns
        .wh_of(state.ids.inv_alignment)
        .top_left_of(state.ids.inv_alignment)
        .set(state.ids.slot_grid, ui)
        {
            match event {
                super::slot_grid::SlotEvents::Close => {
                    events.push(InventoryScrollerEvent::Close);
                },
                _ => {},
            }
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

    fn draggable_area(
        &self,
        state: &ConrodState<'_, InventoryScrollerState>,
        events: &mut Vec<InventoryScrollerEvent>,
        ui: &mut UiCell<'_>,
    ) {
        let bag_settings = &self.global_state.settings.hud_position;
        let bag_pos = if !self.on_right {
            bag_settings.bag.other
        } else {
            bag_settings.bag.own
        };

        let bag_size: Vec2<f64> = if !self.on_right {
            [DEFAULT_OTHER_BAG_WIDTH, DEFAULT_OTHER_BAG_HEIGHT].into()
        } else {
            [DEFAULT_OWN_BAG_WIDTH, DEFAULT_OWN_BAG_HEIGHT].into()
        };

        let pos_delta: Vec2<f64> = ui
            .widget_input(state.ids.draggable_area)
            .drags()
            .left()
            .map(|drag| Vec2::<f64>::from(drag.delta_xy))
            .sum();

        let pos_delta: Vec2<f64> = if !self.on_right {
            // Others (left side) bags use bottom_left_with_margins_on
            pos_delta
        } else {
            // Own (right side) bags use bottom_right_with_margins_on
            // which means we have to use positive margins to move left
            // so we have to invert the x value from the delta.
            pos_delta.with_x(-pos_delta.x)
        };

        let window_clamp = Vec2::new(ui.win_w, ui.win_h) - bag_size;

        let new_pos = (bag_pos + pos_delta)
            .map(|e| e.max(0.))
            .map2(window_clamp, |e, bounds| e.min(bounds));

        if new_pos.abs_diff_ne(&bag_pos, f64::EPSILON) {
            events.push(InventoryScrollerEvent::Drag(new_pos));
        }

        if ui
            .widget_input(state.ids.draggable_area)
            .clicks()
            .right()
            .count()
            == 1
        {
            events.push(InventoryScrollerEvent::Drag(if self.on_right {
                HudPositionSettings::default().bag.own
            } else {
                HudPositionSettings::default().bag.other
            }));
        }

        Rectangle::fill_with([424.0, 48.0], color::TRANSPARENT)
            .top_left_with_margin_on(self.bg_ids.bg_frame, 0.0)
            .set(state.ids.draggable_area, ui);
    }
}

impl Widget for InventoryScroller<'_> {
    type Event = Vec<InventoryScrollerEvent>;
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
        let mut events = Vec::new();
        self.background(ui);
        self.title(state, ui);
        self.scrollbar_and_slots(state, &mut events, ui);
        self.footer_metrics(state, ui);
        if self
            .global_state
            .settings
            .interface
            .toggle_draggable_windows
        {
            self.draggable_area(state, &mut events, ui);
        }
        events
    }
}

widget_ids! {
    pub struct BackgroundIds {
        bg,
        bg_frame,
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum BagTab {
    Gear,
    Inventory,
    Minerals,
    Food,
    Quest,
}

/// A collection of dependencies needed for tabs, intended to reduce parameter
/// bloat
pub struct TabPackage<'a> {
    global_state: &'a GlobalState,
    client: &'a Client,
    info: &'a HudInfo<'a>,
    imgs: &'a Imgs,
    item_imgs: &'a ItemImgs,
    fonts: &'a Fonts,
    localized_strings: &'a Localization,
    item_i18n: &'a ItemI18n,
}

widget_ids! {
    pub struct Bag2Ids {
        bag3,
        bag_title_dropshadow,
        bag_title,
        close_button,
        gear_tab,
        inventory_tab,
        minerals_tab,
        food_tab,
        quest_tab,
        gear_tab_content,
        inventory_tab_content,
        minerals_tab_content,
        food_tab_content,
        quest_tab_content,
        bag_temp1,
        bag_temp2,
        draggable_area,
    }
}

pub struct Bag2State {
    ids: Bag2Ids,
    bg_ids: BackgroundIds,
    active_tab: BagTab,
}

pub enum Bag2Event {
    Close,
    MoveBag(Vec2<f64>),
    BagExpand,
    SetDetailsMode(bool),
    ChangeInventorySortOrder(InventorySortOrder),
    SortInventory(InventorySortOrder),
    SwapEquippedWeapons,
}

#[derive(WidgetCommon)]
pub struct Bag2<'a> {
    global_state: &'a GlobalState,
    client: &'a Client,
    info: &'a HudInfo<'a>,
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
    rbm: &'a RecipeBookManifest,
    poise: &'a Poise,
    menu_events: &'a Vec<MenuInput>,
    is_player: bool,
}

impl<'a> Bag2<'a> {
    pub fn new(
        global_state: &'a GlobalState,
        client: &'a Client,
        info: &'a HudInfo<'a>,
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
        rbm: &'a RecipeBookManifest,
        poise: &'a Poise,
        menu_events: &'a Vec<MenuInput>,
        is_player: bool,
    ) -> Self {
        Self {
            global_state,
            client,
            info,
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
            health,
            energy,
            show,
            body,
            msm,
            rbm,
            poise,
            menu_events,
            is_player,
        }
    }

    fn draggable_area(
        &self,
        state: &ConrodState<'_, Bag2State>,
        events: &mut Vec<Bag2Event>,
        ui: &mut UiCell<'_>,
    ) {
        let bag_settings = &self.global_state.settings.hud_position;
        let bag_pos = if self.is_player {
            bag_settings.bag.own
        } else {
            bag_settings.bag.other
        };

        let bag_size: Vec2<f64> = if self.is_player {
            [DEFAULT_OWN_BAG_WIDTH, DEFAULT_OWN_BAG_HEIGHT].into()
        } else {
            [DEFAULT_OWN_BAG_WIDTH, DEFAULT_OWN_BAG_HEIGHT].into()
            //[DEFAULT_OTHER_BAG_WIDTH, DEFAULT_OTHER_BAG_HEIGHT].into()
        };

        let pos_delta: Vec2<f64> = ui
            .widget_input(state.ids.draggable_area)
            .drags()
            .left()
            .map(|drag| Vec2::<f64>::from(drag.delta_xy))
            .sum();

        let pos_delta: Vec2<f64> = if self.is_player {
            // Own (right side) bags use bottom_right_with_margins_on
            // which means we have to use positive margins to move left
            // so we have to invert the x value from the delta
            pos_delta.with_x(-pos_delta.x)
        } else {
            // Others (left side) bags use bottom_left_with_margins_on
            pos_delta
        };

        let window_clamp = Vec2::new(ui.win_w, ui.win_h) - bag_size;

        let new_pos = (bag_pos + pos_delta)
            .map(|e| e.max(0.))
            .map2(window_clamp, |e, bounds| e.min(bounds));

        if new_pos.abs_diff_ne(&bag_pos, f64::EPSILON) {
            events.push(Bag2Event::MoveBag(new_pos));
        }

        if ui
            .widget_input(state.ids.draggable_area)
            .clicks()
            .right()
            .count()
            == 1
        {
            events.push(Bag2Event::MoveBag(if self.is_player {
                HudPositionSettings::default().bag.own
            } else {
                HudPositionSettings::default().bag.other
            }));
        }

        Rectangle::fill_with([424.0, 48.0], color::TRANSPARENT)
            .top_left_with_margin_on(state.bg_ids.bg_frame, 0.0)
            .set(state.ids.draggable_area, ui);
    }
}

impl Widget for Bag2<'_> {
    type Event = Vec<Bag2Event>;
    type State = Bag2State;
    type Style = ();

    fn init_state(&self, mut id_gen: widget::id::Generator) -> Self::State {
        Bag2State {
            bg_ids: BackgroundIds {
                bg: id_gen.next(),
                bg_frame: id_gen.next(),
            },
            ids: Bag2Ids::new(id_gen),
            active_tab: BagTab::Inventory,
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        common_base::prof_span!("Bag2::update");
        let widget::UpdateArgs { state, ui, .. } = args;
        let _i18n = &self.localized_strings;
        let mut events = Vec::new();

        // MENU INPUTS: change bag tabs
        // PageDown: try to go left a tab (no wrap)
        // PageUp: try to go right a tab (no wrap)
        for event in self.menu_events {
            match *event {
                MenuInput::PageDown => {
                    state.update(|s| {
                        if s.active_tab == BagTab::Quest {
                            s.active_tab = BagTab::Food;
                        } else if s.active_tab == BagTab::Food {
                            s.active_tab = BagTab::Minerals;
                        } else if s.active_tab == BagTab::Minerals {
                            s.active_tab = BagTab::Inventory;
                        } else if s.active_tab == BagTab::Inventory {
                            s.active_tab = BagTab::Gear;
                        } else {
                            s.active_tab = BagTab::Gear;
                        }
                    });
                },
                MenuInput::PageUp => state.update(|s| {
                    if s.active_tab == BagTab::Gear {
                        s.active_tab = BagTab::Inventory;
                    } else if s.active_tab == BagTab::Inventory {
                        s.active_tab = BagTab::Minerals;
                    } else if s.active_tab == BagTab::Minerals {
                        s.active_tab = BagTab::Food;
                    } else if s.active_tab == BagTab::Food {
                        s.active_tab = BagTab::Quest;
                    } else {
                        s.active_tab = BagTab::Quest;
                    }
                }),
                // All other events are handled by child widgets
                _ => {},
            }
        }

        // Expanded gear screen shown next to the bag (placed first so that it is
        // rendered before the bag, in case an animation is added a some point it can
        // animate from behind the bag)
        if self.show.bag_menu_split {
            Bag3::new(
                self.global_state,
                self.client,
                self.info,
                self.imgs,
                self.item_imgs,
                self.fonts,
                self.rot_imgs,
                self.tooltip_manager,
                self.item_tooltip_manager,
                self.slot_manager,
                self.pulse,
                self.localized_strings,
                self.item_i18n,
                self.stats,
                self.skill_set,
                self.health,
                self.energy,
                self.show,
                self.body,
                self.msm,
                self.rbm,
                self.poise,
                self.menu_events,
                &state.bg_ids,
            )
            .set(state.ids.bag3, ui);
        }

        // Background image/frame
        let bag_pos = if self.is_player {
            &self.global_state.settings.hud_position.bag.own
        } else {
            &self.global_state.settings.hud_position.bag.other
        };
        let bg_img = if self.is_player {
            self.imgs.player_inv_bg_bag2
        } else {
            self.imgs.player_inv_bg_bag2
        };
        let _bg_frame_img = self.imgs.player_inv_frame_bag;

        let mut bg = Image::new(bg_img).w_h(424.0, 700.0).color(Some(UI_MAIN));

        if self.is_player {
            bg = bg.bottom_right_with_margins_on(ui.window, bag_pos.y, bag_pos.x);
        } else {
            bg = bg.bottom_left_with_margins_on(ui.window, bag_pos.y, bag_pos.x);
        }

        bg.set(state.bg_ids.bg, ui);

        Image::new(bg_img) // bg_frame_img
            .w_h(424.0, 700.0)
            .middle_of(state.bg_ids.bg)
            .color(Some(UI_HIGHLIGHT_0))
            .set(state.bg_ids.bg_frame, ui);

        // Window title
        let title_txt = match state.active_tab {
            BagTab::Gear => "Gear",
            BagTab::Inventory => "Inventory",
            BagTab::Minerals => "Minerals",
            BagTab::Food => "Food",
            BagTab::Quest => "Quest Items",
        };
        Text::new(title_txt)
            .x_y_relative_to(state.ids.bag_title, -2.0, -2.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(22))
            .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
            .set(state.ids.bag_title_dropshadow, ui);

        Text::new(title_txt)
            .mid_top_with_margin_on(state.bg_ids.bg_frame, 8.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(22))
            .color(TEXT_COLOR)
            .set(state.ids.bag_title, ui);

        // Draggable window space
        if self
            .global_state
            .settings
            .interface
            .toggle_draggable_windows
        {
            self.draggable_area(state, &mut events, ui);
        }

        // Close button
        if Button::image(self.imgs.close_btn)
            .w_h(24.0, 25.0)
            .hover_image(self.imgs.close_btn_hover)
            .press_image(self.imgs.close_btn_press)
            .top_right_of(state.bg_ids.bg_frame)
            .set(state.ids.close_button, ui)
            .was_clicked()
        {
            events.push(Bag2Event::Close);
        }

        // Minerals tab
        if Button::image(self.imgs.bag_ico)
            .w_h(30.0, 30.0)
            .mid_top_with_margin_on(state.ids.bag_title, 35.0)
            .color(if state.active_tab == BagTab::Minerals {
                Color::Rgba(1.0, 0.0, 1.0, 1.0)
            } else {
                Color::Rgba(1.0, 0.0, 1.0, 0.0)
            })
            .set(state.ids.minerals_tab, ui)
            .was_clicked()
        {
            state.update(|s| {
                s.active_tab = BagTab::Minerals;
            })
        }

        // Inventory tab
        if Button::image(self.imgs.bag_ico)
            .w_h(30.0, 30.0)
            .left_from(state.ids.minerals_tab, 10.0)
            .color(if state.active_tab == BagTab::Inventory {
                Color::Rgba(1.0, 0.0, 1.0, 1.0)
            } else {
                Color::Rgba(1.0, 0.0, 1.0, 0.0)
            })
            .set(state.ids.inventory_tab, ui)
            .was_clicked()
        {
            state.update(|s| {
                s.active_tab = BagTab::Inventory;
            })
        }

        // Gear tab
        if Button::image(self.imgs.char_art)
            .w_h(32.5, 30.0)
            .left_from(state.ids.inventory_tab, 10.0)
            .color(if state.active_tab == BagTab::Gear {
                Color::Rgba(1.0, 0.0, 1.0, 1.0)
            } else {
                Color::Rgba(1.0, 0.0, 1.0, 0.0)
            })
            .set(state.ids.gear_tab, ui)
            .was_clicked()
        {
            state.update(|s| {
                s.active_tab = BagTab::Gear;
            })
        }

        // Food tab
        if Button::image(self.imgs.bag_ico)
            .w_h(30.0, 30.0)
            .right_from(state.ids.minerals_tab, 10.0)
            .color(if state.active_tab == BagTab::Food {
                Color::Rgba(1.0, 0.0, 1.0, 1.0)
            } else {
                Color::Rgba(1.0, 0.0, 1.0, 0.0)
            })
            .set(state.ids.food_tab, ui)
            .was_clicked()
        {
            state.update(|s| {
                s.active_tab = BagTab::Food;
            })
        }

        // Quest items tab
        if Button::image(self.imgs.bag_ico)
            .w_h(30.0, 30.0)
            .right_from(state.ids.food_tab, 10.0)
            .color(if state.active_tab == BagTab::Food {
                Color::Rgba(1.0, 0.0, 1.0, 1.0)
            } else {
                Color::Rgba(1.0, 0.0, 1.0, 0.0)
            })
            .set(state.ids.quest_tab, ui)
            .was_clicked()
        {
            state.update(|s| {
                s.active_tab = BagTab::Quest;
            })
        }

        Text::new("<- Q")
            .left_from(state.ids.gear_tab, 5.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(22))
            .color(TEXT_COLOR)
            .set(state.ids.bag_temp1, ui);

        Text::new("E ->")
            .right_from(state.ids.quest_tab, 5.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(22))
            .color(TEXT_COLOR)
            .set(state.ids.bag_temp2, ui);

        // Display tab contents in remaining space
        if let Some(inventory) = self.client.inventories().get(self.info.viewpoint_entity) {
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
                self.rbm,
                Some(inventory),
                self.localized_strings,
                self.item_i18n,
            )
            .title_font_size(self.fonts.cyri.scale(20))
            .parent(ui.window)
            .desc_font_size(self.fonts.cyri.scale(12))
            .font_id(self.fonts.cyri.conrod_id)
            .desc_text_color(TEXT_COLOR);

            let tab_package = TabPackage {
                global_state: self.global_state,
                client: self.client,
                info: self.info,
                imgs: self.imgs,
                item_imgs: self.item_imgs,
                fonts: self.fonts,
                localized_strings: self.localized_strings,
                item_i18n: self.item_i18n,
            };

            match state.active_tab {
                BagTab::Gear => {
                    // Gear tab
                    for event in GearMenu::new(
                        &tab_package,
                        inventory,
                        &tooltip,
                        &bag_tooltip,
                        &item_tooltip,
                        self.tooltip_manager,
                        self.item_tooltip_manager,
                        self.slot_manager,
                        self.pulse,
                        &self.stats,
                        &self.skill_set,
                        &self.health,
                        &self.energy,
                        &self.show,
                        &self.body,
                        &self.msm,
                        &self.poise,
                        &self.menu_events,
                        &state.bg_ids,
                    )
                    .set(state.ids.gear_tab_content, ui)
                    {
                        match event {
                            GearMenuEvent::Close => events.push(Bag2Event::Close),
                            GearMenuEvent::SwapEquippedWeapons => {
                                events.push(Bag2Event::SwapEquippedWeapons)
                            },
                        }
                    }
                },
                BagTab::Inventory => {
                    // Inventory tab
                    for event in InventoryMenu::new(
                        &tab_package,
                        inventory,
                        &tooltip,
                        &bag_tooltip,
                        &item_tooltip,
                        self.tooltip_manager,
                        self.item_tooltip_manager,
                        self.slot_manager,
                        self.pulse,
                        &self.menu_events,
                        &state.bg_ids,
                        self.show.bag_details,
                    )
                    .set(state.ids.inventory_tab_content, ui)
                    {
                        match event {
                            InventoryMenuEvent::Close => events.push(Bag2Event::Close),
                            InventoryMenuEvent::BagExpand => events.push(Bag2Event::BagExpand),
                            InventoryMenuEvent::SetDetailsMode => {
                                events.push(Bag2Event::SetDetailsMode(!self.show.bag_details))
                            },
                            InventoryMenuEvent::ChangeInventorySortOrder => {
                                events.push(Bag2Event::ChangeInventorySortOrder(
                                    self.global_state.settings.inventory.sort_order.next(),
                                ))
                            },
                            InventoryMenuEvent::SortInventory => {
                                events.push(Bag2Event::SortInventory(
                                    self.global_state.settings.inventory.sort_order,
                                ));
                            },
                        }
                    }
                },
                BagTab::Minerals => {
                    // Minerals tab
                    for event in InventoryMenu::new(
                        &tab_package,
                        inventory,
                        &tooltip,
                        &bag_tooltip,
                        &item_tooltip,
                        self.tooltip_manager,
                        self.item_tooltip_manager,
                        self.slot_manager,
                        self.pulse,
                        &self.menu_events,
                        &state.bg_ids,
                        self.show.bag_details,
                    )
                    .filter(TabFilters::Minerals)
                    .set(state.ids.minerals_tab_content, ui)
                    {
                        match event {
                            InventoryMenuEvent::Close => events.push(Bag2Event::Close),
                            InventoryMenuEvent::BagExpand => events.push(Bag2Event::BagExpand),
                            InventoryMenuEvent::SetDetailsMode => {
                                events.push(Bag2Event::SetDetailsMode(!self.show.bag_details))
                            },
                            InventoryMenuEvent::ChangeInventorySortOrder => {
                                events.push(Bag2Event::ChangeInventorySortOrder(
                                    self.global_state.settings.inventory.sort_order.next(),
                                ))
                            },
                            InventoryMenuEvent::SortInventory => {
                                events.push(Bag2Event::SortInventory(
                                    self.global_state.settings.inventory.sort_order,
                                ));
                            },
                        }
                    }
                },
                BagTab::Food => {
                    // Food tab
                    for event in InventoryMenu::new(
                        &tab_package,
                        inventory,
                        &tooltip,
                        &bag_tooltip,
                        &item_tooltip,
                        self.tooltip_manager,
                        self.item_tooltip_manager,
                        self.slot_manager,
                        self.pulse,
                        &self.menu_events,
                        &state.bg_ids,
                        self.show.bag_details,
                    )
                    .filter(TabFilters::Food)
                    .set(state.ids.food_tab_content, ui)
                    {
                        match event {
                            InventoryMenuEvent::Close => events.push(Bag2Event::Close),
                            InventoryMenuEvent::BagExpand => events.push(Bag2Event::BagExpand),
                            InventoryMenuEvent::SetDetailsMode => {
                                events.push(Bag2Event::SetDetailsMode(!self.show.bag_details))
                            },
                            InventoryMenuEvent::ChangeInventorySortOrder => {
                                events.push(Bag2Event::ChangeInventorySortOrder(
                                    self.global_state.settings.inventory.sort_order.next(),
                                ))
                            },
                            InventoryMenuEvent::SortInventory => {
                                events.push(Bag2Event::SortInventory(
                                    self.global_state.settings.inventory.sort_order,
                                ));
                            },
                        }
                    }
                },
                BagTab::Quest => {
                    // Quest item tab
                    for event in InventoryMenu::new(
                        &tab_package,
                        inventory,
                        &tooltip,
                        &bag_tooltip,
                        &item_tooltip,
                        self.tooltip_manager,
                        self.item_tooltip_manager,
                        self.slot_manager,
                        self.pulse,
                        &self.menu_events,
                        &state.bg_ids,
                        self.show.bag_details,
                    )
                    .filter(TabFilters::QuestItems)
                    .set(state.ids.quest_tab_content, ui)
                    {
                        match event {
                            InventoryMenuEvent::Close => events.push(Bag2Event::Close),
                            InventoryMenuEvent::BagExpand => events.push(Bag2Event::BagExpand),
                            InventoryMenuEvent::SetDetailsMode => {
                                events.push(Bag2Event::SetDetailsMode(!self.show.bag_details))
                            },
                            InventoryMenuEvent::ChangeInventorySortOrder => {
                                events.push(Bag2Event::ChangeInventorySortOrder(
                                    self.global_state.settings.inventory.sort_order.next(),
                                ))
                            },
                            InventoryMenuEvent::SortInventory => {
                                events.push(Bag2Event::SortInventory(
                                    self.global_state.settings.inventory.sort_order,
                                ));
                            },
                        }
                    }
                },
            }
        }

        events
    }
}

widget_ids! {
    pub struct Bag3Ids {
        expanded_bag_title,
        expanded_bag_title_dropshadow,
        gear_tab_content,
    }
}

pub struct Bag3State {
    ids: Bag3Ids,
    bg_ids: BackgroundIds,
}

#[derive(WidgetCommon)]
pub struct Bag3<'a> {
    global_state: &'a GlobalState,
    client: &'a Client,
    info: &'a HudInfo<'a>,
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
    rbm: &'a RecipeBookManifest,
    poise: &'a Poise,
    menu_events: &'a Vec<MenuInput>,
    other_bg_ids: &'a BackgroundIds,
}

impl<'a> Bag3<'a> {
    pub fn new(
        global_state: &'a GlobalState,
        client: &'a Client,
        info: &'a HudInfo<'a>,
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
        rbm: &'a RecipeBookManifest,
        poise: &'a Poise,
        menu_events: &'a Vec<MenuInput>,
        other_bg_ids: &'a BackgroundIds,
    ) -> Self {
        Self {
            global_state,
            client,
            info,
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
            health,
            energy,
            show,
            body,
            msm,
            rbm,
            poise,
            menu_events,
            other_bg_ids,
        }
    }
}

impl Widget for Bag3<'_> {
    type Event = ();
    type State = Bag3State;
    type Style = ();

    fn init_state(&self, mut id_gen: widget::id::Generator) -> Self::State {
        Bag3State {
            bg_ids: BackgroundIds {
                bg: id_gen.next(),
                bg_frame: id_gen.next(),
            },
            ids: Bag3Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        common_base::prof_span!("Bag3::update");
        let widget::UpdateArgs { state, ui, .. } = args;
        //let i18n = &self.localized_strings;
        //let mut events = Vec::new();

        // Bacgkround image
        let bg_img = self.imgs.player_inv_bg_bag2;
        let _bg_frame_img = self.imgs.player_inv_frame_bag;
        Image::new(bg_img)
            .w_h(424.0, 700.0)
            .color(Some(UI_MAIN))
            .left_from(self.other_bg_ids.bg, 0.0)
            .set(state.bg_ids.bg, ui);

        Image::new(bg_img) // bg_frame_img
            .w_h(424.0, 700.0)
            .middle_of(state.bg_ids.bg)
            .color(Some(UI_HIGHLIGHT_0))
            .set(state.bg_ids.bg_frame, ui);

        // Window title
        let title_txt = "Gear";
        Text::new(title_txt)
            .x_y_relative_to(state.ids.expanded_bag_title, -2.0, -2.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(22))
            .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
            .set(state.ids.expanded_bag_title_dropshadow, ui);

        Text::new(title_txt)
            .mid_top_with_margin_on(state.bg_ids.bg_frame, 8.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(22))
            .color(TEXT_COLOR)
            .set(state.ids.expanded_bag_title, ui);

        // any tabs and buttons I may want from Bag2

        // Display gear content in remaining space
        if let Some(inventory) = self.client.inventories().get(self.info.viewpoint_entity) {
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
                self.rbm,
                Some(inventory),
                self.localized_strings,
                self.item_i18n,
            )
            .title_font_size(self.fonts.cyri.scale(20))
            .parent(ui.window)
            .desc_font_size(self.fonts.cyri.scale(12))
            .font_id(self.fonts.cyri.conrod_id)
            .desc_text_color(TEXT_COLOR);

            let tab_package = TabPackage {
                global_state: self.global_state,
                client: self.client,
                info: self.info,
                imgs: self.imgs,
                item_imgs: self.item_imgs,
                fonts: self.fonts,
                localized_strings: self.localized_strings,
                item_i18n: self.item_i18n,
            };

            // Gear tab
            for event in GearMenu::new(
                &tab_package,
                inventory,
                &tooltip,
                &bag_tooltip,
                &item_tooltip,
                self.tooltip_manager,
                self.item_tooltip_manager,
                self.slot_manager,
                self.pulse,
                &self.stats,
                &self.skill_set,
                &self.health,
                &self.energy,
                &self.show,
                &self.body,
                &self.msm,
                &self.poise,
                &self.menu_events,
                &state.bg_ids,
            )
            .set(state.ids.gear_tab_content, ui)
            {
                match event {
                    _ => {},
                }
            }
        }
    }
}

widget_ids! {
    pub struct InventoryMenuIds {
        scrollbar_bg,
        scrollbar_slots,
        inv_alignment,
        slot_grid,
        bag_details_btn,
        inventory_sort,
        inventory_sort_selected,
        bag_expand_btn,
        space_txt
    }
}

pub struct InventoryMenuState {
    ids: InventoryMenuIds,
}

pub enum InventoryMenuEvent {
    Close,
    BagExpand,
    SetDetailsMode,
    ChangeInventorySortOrder,
    SortInventory,
}

#[derive(WidgetCommon)]
pub struct InventoryMenu<'a> {
    tab_package: &'a TabPackage<'a>,
    inventory: &'a Inventory,
    tooltip: &'a Tooltip<'a>,
    bag_tooltip: &'a Tooltip<'a>,
    item_tooltip: &'a ItemTooltip<'a>,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    tooltip_manager: &'a mut TooltipManager,
    item_tooltip_manager: &'a mut ItemTooltipManager,
    slot_manager: &'a mut SlotManager,
    pulse: f32,
    menu_events: &'a Vec<MenuInput>,
    bag_ids: &'a BackgroundIds,
    bag_details: bool,
    filter: TabFilters,
}

impl<'a> InventoryMenu<'a> {
    builder_method! {
        pub filter { filter = TabFilters }
    }

    pub fn new(
        tab_package: &'a TabPackage<'a>,
        inventory: &'a Inventory,
        tooltip: &'a Tooltip<'a>,
        bag_tooltip: &'a Tooltip<'a>,
        item_tooltip: &'a ItemTooltip<'a>,
        tooltip_manager: &'a mut TooltipManager,
        item_tooltip_manager: &'a mut ItemTooltipManager,
        slot_manager: &'a mut SlotManager,
        pulse: f32,
        menu_events: &'a Vec<MenuInput>,
        bag_ids: &'a BackgroundIds,
        bag_details: bool,
    ) -> Self {
        Self {
            tab_package,
            inventory,
            tooltip,
            bag_tooltip,
            item_tooltip,
            common: widget::CommonBuilder::default(),
            tooltip_manager,
            item_tooltip_manager,
            slot_manager,
            pulse,
            menu_events,
            bag_ids,
            bag_details,
            filter: TabFilters::None,
        }
    }
}

impl Widget for InventoryMenu<'_> {
    type Event = Vec<InventoryMenuEvent>;
    type State = InventoryMenuState;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        InventoryMenuState {
            ids: InventoryMenuIds::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        common_base::prof_span!("InventoryMenu::update");
        let widget::UpdateArgs { state, ui, .. } = args;
        let i18n = &self.tab_package.localized_strings;
        let mut events = Vec::new();

        // MENU INPUTS: finish up later
        // Back: closes the window
        for event in self.menu_events {
            match *event {
                MenuInput::Back => {
                    events.push(InventoryMenuEvent::Close);
                },
                _ => {},
            }
        }

        // Top buttons
        // Sort inventory button with selected mode
        if Button::image(self.tab_package.imgs.inv_sort_selected_btn)
            .w_h(30.0, 17.0)
            .hover_image(self.tab_package.imgs.inv_sort_selected_btn_hover)
            .press_image(self.tab_package.imgs.inv_sort_selected_btn_press)
            .mid_top_with_margin_on(self.bag_ids.bg_frame, 90.0)
            //.left_from(state.ids.bag_details_btn, 10.0)
            .with_tooltip(
                self.tooltip_manager,
                &(match self.tab_package.global_state.settings.inventory.sort_order {
                    InventorySortOrder::Name => i18n.get_msg("hud-bag-sort_by_name"),
                    InventorySortOrder::Quality => i18n.get_msg("hud-bag-sort_by_quality"),
                    InventorySortOrder::Category => i18n.get_msg("hud-bag-sort_by_category"),
                    InventorySortOrder::Tag => i18n.get_msg("hud-bag-sort_by_tag"),
                    InventorySortOrder::Amount => i18n.get_msg("hud-bag-sort_by_quantity"),
                }),
                "",
                self.tooltip,
                color::WHITE,
            )
            .set(state.ids.inventory_sort_selected, ui)
            .was_clicked()
        {
            events.push(InventoryMenuEvent::SortInventory);
        }

        // Sort mode inventory button
        if Button::image(self.tab_package.imgs.inv_sort_btn)
            .w_h(30.0, 17.0)
            .hover_image(self.tab_package.imgs.inv_sort_btn_hover)
            .press_image(self.tab_package.imgs.inv_sort_btn_press)
            .left_from(state.ids.inventory_sort_selected, 10.0)
            .with_tooltip(
                self.tooltip_manager,
                &(match self
                    .tab_package
                    .global_state
                    .settings
                    .inventory
                    .sort_order
                    .next()
                {
                    InventorySortOrder::Name => i18n.get_msg("hud-bag-change_to_sort_by_name"),
                    InventorySortOrder::Quality => {
                        i18n.get_msg("hud-bag-change_to_sort_by_quality")
                    },
                    InventorySortOrder::Category => {
                        i18n.get_msg("hud-bag-change_to_sort_by_category")
                    },
                    InventorySortOrder::Tag => i18n.get_msg("hud-bag-change_to_sort_by_tag"),
                    InventorySortOrder::Amount => {
                        i18n.get_msg("hud-bag-change_to_sort_by_quantity")
                    },
                }),
                "",
                self.tooltip,
                color::WHITE,
            )
            .set(state.ids.inventory_sort, ui)
            .was_clicked()
        {
            events.push(InventoryMenuEvent::ChangeInventorySortOrder);
        }

        // Button to toggle grid/list mode
        let (txt, btn, hover, press) = if true {
            (
                "Grid mode",
                self.tab_package.imgs.grid_btn,
                self.tab_package.imgs.grid_btn_hover,
                self.tab_package.imgs.grid_btn_press,
            )
        } else {
            (
                "List mode",
                self.tab_package.imgs.list_btn,
                self.tab_package.imgs.list_btn_hover,
                self.tab_package.imgs.list_btn_press,
            )
        };
        if Button::image(btn)
            .w_h(32.0, 17.0)
            .hover_image(hover)
            .press_image(press)
            .right_from(state.ids.inventory_sort_selected, 10.0)
            .with_tooltip(self.tooltip_manager, txt, "", self.bag_tooltip, TEXT_COLOR)
            .set(state.ids.bag_details_btn, ui)
            .was_clicked()
        {
            events.push(InventoryMenuEvent::SetDetailsMode);
        }

        // Inventory space text
        let space_used = self.inventory.populated_slots();
        let space_max = self.inventory.slots().count();
        let bag_space = format!("{}/{}", space_used, space_max);
        let bag_space_percentage = space_used as f32 / space_max as f32;
        Text::new(&bag_space)
            .right_from(state.ids.bag_details_btn, 10.0)
            .font_id(self.tab_package.fonts.cyri.conrod_id)
            .font_size(self.tab_package.fonts.cyri.scale(14))
            .color(if bag_space_percentage < 0.8 {
                TEXT_COLOR
            } else if bag_space_percentage < 1.0 {
                LOW_HP_COLOR
            } else {
                CRITICAL_HP_COLOR
            })
            .set(state.ids.space_txt, ui);

        // Toggle the exanded gear|inventory window
        if Button::image(self.tab_package.imgs.collapse_btn)
            .w_h(30.0, 17.0)
            .hover_image(self.tab_package.imgs.collapse_btn_hover)
            .press_image(self.tab_package.imgs.collapse_btn_press)
            .left_from(state.ids.inventory_sort, 10.0)
            .with_tooltip(
                self.tooltip_manager,
                "Toggle expanded window",
                "",
                self.tooltip,
                color::WHITE,
            )
            .set(state.ids.bag_expand_btn, ui)
            .was_clicked()
        {
            events.push(InventoryMenuEvent::BagExpand);
        }

        // The grid width and item slot size are all pixel perferct in their alignment
        // However, the slot spacing has a few pixel mismatches, but shouldn't be
        // noticable
        let grid_width = 376.0; // 381.0; 376.0
        let grid_height = 565.0;

        // Alignment for Grid
        Rectangle::fill_with([grid_width, grid_height], color::TRANSPARENT)
            .mid_bottom_with_margin_on(self.bag_ids.bg_frame, 3.5)
            .scroll_kids_vertically()
            .set(state.ids.inv_alignment, ui);

        let space_max = self.inventory.slots().count();

        // Bag Slots
        for event in SlotGrid::new(
            self.tab_package.client,
            self.tab_package.imgs,
            self.tab_package.item_imgs,
            self.tab_package.fonts,
            self.item_tooltip_manager,
            self.slot_manager,
            self.inventory,
            self.item_tooltip,
            self.tab_package.localized_strings,
            self.tab_package.item_i18n,
            self.tab_package.info.viewpoint_entity,
            &self.tab_package.global_state.window.last_input(),
            self.pulse,
            self.menu_events,
            true, // is_us
            self.bag_details, // details_mode
            false, // show_salvage
        )
        .columns(6)
        .spacing(if self.bag_details { 0.0 } else { 5.8 }) // 5.8
        .slot_size(if self.bag_details { 20.0 } else { 57.8 }) // 58.5; 57.8
        .filter(self.filter)
        .wh_of(state.ids.inv_alignment)
        .top_left_of(state.ids.inv_alignment)
        .set(state.ids.slot_grid, ui)
        {
            match event {
                SlotEvents::Close => events.push(InventoryMenuEvent::Close),
                _ => {},
            }
        }

        // Slots scrollbar
        if space_max > 60 {
            // Scrollbar-BG
            Image::new(self.tab_package.imgs.scrollbar_bg_big)
                .w_h(9.0, 592.0)
                .bottom_right_with_margins_on(self.bag_ids.bg_frame, 30.0, 3.0)
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

        events
    }
}

widget_ids! {
    pub struct GearMenuIds {
        inv_alignment,
        slot_grid,
        scrollbar_bg,
        scrollbar_slots,
        // Armor slots
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

pub struct GearMenuState {
    ids: GearMenuIds,
    is_focused: bool,
    active_gear_slot: usize,
}

pub enum GearMenuEvent {
    Close,
    SwapEquippedWeapons,
}

#[derive(WidgetCommon)]
pub struct GearMenu<'a> {
    tab_package: &'a TabPackage<'a>,
    inventory: &'a Inventory,
    tooltip: &'a Tooltip<'a>,
    bag_tooltip: &'a Tooltip<'a>,
    item_tooltip: &'a ItemTooltip<'a>,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    tooltip_manager: &'a mut TooltipManager,
    item_tooltip_manager: &'a mut ItemTooltipManager,
    slot_manager: &'a mut SlotManager,
    pulse: f32,
    stats: &'a Stats,
    skill_set: &'a SkillSet,
    health: &'a Health,
    energy: &'a Energy,
    show: &'a Show,
    body: &'a Body,
    msm: &'a MaterialStatManifest,
    poise: &'a Poise,
    menu_events: &'a Vec<MenuInput>,
    bag_ids: &'a BackgroundIds,
}

impl<'a> GearMenu<'a> {
    pub fn new(
        tab_package: &'a TabPackage,
        inventory: &'a Inventory,
        tooltip: &'a Tooltip<'a>,
        bag_tooltip: &'a Tooltip<'a>,
        item_tooltip: &'a ItemTooltip<'a>,
        tooltip_manager: &'a mut TooltipManager,
        item_tooltip_manager: &'a mut ItemTooltipManager,
        slot_manager: &'a mut SlotManager,
        pulse: f32,
        stats: &'a Stats,
        skill_set: &'a SkillSet,
        health: &'a Health,
        energy: &'a Energy,
        show: &'a Show,
        body: &'a Body,
        msm: &'a MaterialStatManifest,
        poise: &'a Poise,
        menu_events: &'a Vec<MenuInput>,
        bag_ids: &'a BackgroundIds,
    ) -> Self {
        Self {
            tab_package,
            inventory,
            tooltip,
            bag_tooltip,
            item_tooltip,
            common: widget::CommonBuilder::default(),
            tooltip_manager,
            item_tooltip_manager,
            slot_manager,
            pulse,
            stats,
            skill_set,
            health,
            energy,
            show,
            body,
            msm,
            poise,
            menu_events,
            bag_ids,
        }
    }
}

impl Widget for GearMenu<'_> {
    type Event = Vec<GearMenuEvent>;
    type State = GearMenuState;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        GearMenuState {
            ids: GearMenuIds::new(id_gen),
            is_focused: false,
            active_gear_slot: 2,
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        common_base::prof_span!("GearMenu::update");
        let widget::UpdateArgs { state, ui, .. } = args;
        let i18n = &self.tab_package.localized_strings;
        let mut events = Vec::new();

        // MENU INPUTS: manage equipment
        // Up: try to go up one
        // Down: try to go down one
        // Left: try to go left one
        // Right: try to move right one
        // Apply: select highlighed gear slot
        // Back: closes the bag
        if state.is_focused == true {
            for event in self.menu_events {
                match *event {
                    MenuInput::Up => state.update(|s| {
                        // So many values to manual set...
                        match s.active_gear_slot {
                            // weapon switch button
                            0 => {},
                            // primary weapon left
                            1 => s.active_gear_slot = 5,
                            // secondary weapon left
                            2 => s.active_gear_slot = 6,
                            // secondary weapon right
                            3 => s.active_gear_slot = 6,
                            // primary weapon right
                            4 => s.active_gear_slot = 7,
                            // back
                            5 => s.active_gear_slot = 8,
                            // pants
                            6 => s.active_gear_slot = 9,
                            // shoes
                            7 => s.active_gear_slot = 10,
                            // jewelry left
                            8 => s.active_gear_slot = 11,
                            // belt
                            9 => s.active_gear_slot = 12,
                            // jewelry right
                            10 => s.active_gear_slot = 13,
                            // shoulder
                            11 => s.active_gear_slot = 14,
                            // chest
                            12 => s.active_gear_slot = 14,
                            // hands/gloves
                            13 => s.active_gear_slot = 14,
                            // jewelry center
                            14 => s.active_gear_slot = 15,
                            // hat
                            15 => {},
                            // tabard
                            16 => s.active_gear_slot = 17,
                            // glider
                            17 => s.active_gear_slot = 18,
                            // lantern
                            18 => {},
                            // Bag 1
                            19 => s.active_gear_slot = 16,
                            // Bag 2
                            20 => s.active_gear_slot = 19,
                            // Bag 3
                            21 => s.active_gear_slot = 20,
                            // Bag 4
                            22 => s.active_gear_slot = 21,
                            // reset to 1 if unexpected
                            _ => s.active_gear_slot = 1,
                        }
                    }),
                    MenuInput::Down => state.update(|s| {
                        match s.active_gear_slot {
                            // weapon switch button
                            0 => s.is_focused = false,
                            // primary weapon 1
                            1 => s.is_focused = false,
                            // secondary weapon 1
                            2 => s.is_focused = false,
                            // secondary weapon 2
                            3 => s.is_focused = false,
                            // primary weapon 2
                            4 => s.is_focused = false,
                            // back
                            5 => s.active_gear_slot = 1,
                            // pants
                            6 => s.active_gear_slot = 2,
                            // shoes
                            7 => s.active_gear_slot = 4,
                            // jewelry left
                            8 => s.active_gear_slot = 5,
                            // belt
                            9 => s.active_gear_slot = 6,
                            // jewelry right
                            10 => s.active_gear_slot = 7,
                            // shoulder
                            11 => s.active_gear_slot = 8,
                            // chest
                            12 => s.active_gear_slot = 9,
                            // hands/gloves
                            13 => s.active_gear_slot = 10,
                            // jewelry center
                            14 => s.active_gear_slot = 12,
                            // hat
                            15 => s.active_gear_slot = 14,
                            // tabard
                            16 => s.active_gear_slot = 19,
                            // glider
                            17 => s.active_gear_slot = 16,
                            // lantern
                            18 => s.active_gear_slot = 17,
                            // Bag 1
                            19 => s.active_gear_slot = 20,
                            // Bag 2
                            20 => s.active_gear_slot = 21,
                            // Bag 3
                            21 => s.active_gear_slot = 22,
                            // Bag 4
                            22 => s.is_focused = false,
                            // reset to 1 if unexpected
                            _ => s.active_gear_slot = 1,
                        }
                    }),
                    MenuInput::Left => state.update(|s| {
                        match s.active_gear_slot {
                            // weapon switch button
                            0 => {},
                            // primary weapon 1
                            1 => s.active_gear_slot = 0 + 1,
                            // secondary weapon 1
                            2 => s.active_gear_slot = 1,
                            // secondary weapon 2
                            3 => s.active_gear_slot = 2,
                            // primary weapon 2
                            4 => s.active_gear_slot = 3,
                            // back
                            5 => {},
                            // pants
                            6 => s.active_gear_slot = 5,
                            // shoes
                            7 => s.active_gear_slot = 6,
                            // jewelry left
                            8 => {},
                            // belt
                            9 => s.active_gear_slot = 8,
                            // jewelry right
                            10 => s.active_gear_slot = 9,
                            // shoulder
                            11 => {},
                            // chest
                            12 => s.active_gear_slot = 11,
                            // hands/gloves
                            13 => s.active_gear_slot = 12,
                            // jewelry center
                            14 => {},
                            // hat
                            15 => {},
                            // tabard
                            16 => s.active_gear_slot = 13,
                            // glider
                            17 => s.active_gear_slot = 14,
                            // lantern
                            18 => s.active_gear_slot = 15,
                            // Bag 1
                            19 => s.active_gear_slot = 13,
                            // Bag 2
                            20 => s.active_gear_slot = 10,
                            // Bag 3
                            21 => s.active_gear_slot = 7,
                            // Bag 4
                            22 => s.active_gear_slot = 4,
                            // reset to 1 if unexpected
                            _ => s.active_gear_slot = 1,
                        }
                    }),
                    MenuInput::Right => state.update(|s| {
                        match s.active_gear_slot {
                            // weapon switch button
                            0 => s.active_gear_slot = 1,
                            // primary weapon 1
                            1 => s.active_gear_slot = 2,
                            // secondary weapon 1
                            2 => s.active_gear_slot = 3,
                            // secondary weapon 2
                            3 => s.active_gear_slot = 4,
                            // primary weapon 1
                            4 => s.active_gear_slot = 22,
                            // back
                            5 => s.active_gear_slot = 6,
                            // pants
                            6 => s.active_gear_slot = 7,
                            // shoes
                            7 => s.active_gear_slot = 21,
                            // jewelry left
                            8 => s.active_gear_slot = 9,
                            // belt
                            9 => s.active_gear_slot = 10,
                            // jewelry right
                            10 => s.active_gear_slot = 20,
                            // shoulder
                            11 => s.active_gear_slot = 12,
                            // chest
                            12 => s.active_gear_slot = 13,
                            // hands/gloves
                            13 => s.active_gear_slot = 19,
                            // jewelry center
                            14 => s.active_gear_slot = 17,
                            // hat
                            15 => s.active_gear_slot = 18,
                            // tabard
                            16 => {},
                            // glider
                            17 => {},
                            // lantern
                            18 => {},
                            // Bag 1
                            19 => {},
                            // Bag 2
                            20 => {},
                            // Bag 3
                            21 => {},
                            // Bag 4
                            22 => {},
                            // reset to 1 if unexpected
                            _ => s.active_gear_slot = 1,
                        }
                    }),
                    MenuInput::Apply => {
                        // TODO
                    },
                    MenuInput::Back => events.push(GearMenuEvent::Close),
                    _ => {},
                }
            }
        }

        // Inventory slots (filtered to equipment only)
        let grid_width = 376.0;
        let grid_height = 200.0;

        Rectangle::fill_with([grid_width, grid_height], color::TRANSPARENT)
            .mid_bottom_with_margin_on(self.bag_ids.bg_frame, 3.5)
            .scroll_kids_vertically()
            .set(state.ids.inv_alignment, ui);

        let space_max = self.inventory.slots().count();

        // Bag slots
        for event in SlotGrid::new(
            self.tab_package.client,
            self.tab_package.imgs,
            self.tab_package.item_imgs,
            self.tab_package.fonts,
            self.item_tooltip_manager,
            self.slot_manager,
            self.inventory,
            self.item_tooltip,
            self.tab_package.localized_strings,
            self.tab_package.item_i18n,
            self.tab_package.info.viewpoint_entity,
            &self.tab_package.global_state.window.last_input(),
            self.pulse,
            self.menu_events,
            true,                  // is_us
            self.show.bag_details, // details_mode
            false,                 // show_salvage
        )
        .columns(6)
        .spacing(if self.show.bag_details { 0.0 } else { 5.8 })
        // If the items are in focused, then gear is not (and vice-versa)
        .is_focused(!state.is_focused)
        .slot_size(if self.show.bag_details { 20.0 } else { 57.8 })
        .filter(TabFilters::Gear)
        .wh_of(state.ids.inv_alignment)
        .top_left_of(state.ids.inv_alignment)
        .set(state.ids.slot_grid, ui)
        {
            match event {
                SlotEvents::Close => events.push(GearMenuEvent::Close),
                SlotEvents::ExitUp => {
                    // User went up, out of the item list
                    // Switch focus to this widget---gear slots
                    state.update(|s| {
                        s.is_focused = true;
                    })
                },
                _ => {},
            }
        }

        // Scrollbar
        if space_max > 24 {
            // Scrollbar-BG
            Image::new(self.tab_package.imgs.scrollbar_bg)
                .w_h(9.0, 173.0)
                .bottom_right_with_margins_on(self.bag_ids.bg_frame, 42.0, 3.0)
                .color(Some(UI_HIGHLIGHT_0))
                .set(state.ids.scrollbar_bg, ui);
            // Scrollbar
            Scrollbar::y_axis(state.ids.inv_alignment)
                .thickness(5.0)
                .h(123.0)
                .color(UI_MAIN)
                .middle_of(state.ids.scrollbar_bg)
                .set(state.ids.scrollbar_slots, ui);
        }

        // Armor slots
        let mut slot_maker = SlotMaker {
            empty_slot: self.tab_package.imgs.armor_slot_empty,
            hovered_slot: self.tab_package.imgs.skillbar_index,
            filled_slot: self.tab_package.imgs.armor_slot,
            selected_slot: self.tab_package.imgs.armor_slot_sel,
            background_color: Some(UI_HIGHLIGHT_0),
            content_size: ContentSize {
                width_height_ratio: 1.0,
                max_fraction: 0.75, /* Changes the item image size by setting a maximum
                                     * fraction
                                     * of either the width or height */
            },
            selected_content_scale: 1.067,
            amount_font: self.tab_package.fonts.cyri.conrod_id,
            amount_margins: Vec2::new(-4.0, 0.0),
            amount_font_size: self.tab_package.fonts.cyri.scale(12),
            amount_text_color: TEXT_COLOR,
            content_source: self.inventory,
            image_source: self.tab_package.item_imgs,
            slot_manager: Some(self.slot_manager),
            last_input: &self.tab_package.global_state.window.last_input(),
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
                if let Some(item) = self.inventory.equipped($slot) {
                    let manager = &mut *self.item_tooltip_manager;
                    $slot_maker
                        .with_item_tooltip(
                            manager,
                            core::iter::once(item as &dyn ItemDesc),
                            &None,
                            self.item_tooltip,
                        )
                        .set($slot_id, ui)
                } else {
                    let manager = &mut *self.tooltip_manager;
                    $slot_maker
                        .with_tooltip(
                            manager,
                            &i18n.get_msg($desc),
                            "",
                            self.tooltip,
                            color::WHITE,
                        )
                        .set($slot_id, ui)
                }
            };
        }

        let filled_slot = self.tab_package.imgs.armor_slot;
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
                self.inventory,
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
                    "Health" => self.tab_package.imgs.health_ico,
                    "Energy" => self.tab_package.imgs.energy_ico,
                    "Combat Rating" => self.tab_package.imgs.combat_rating_ico,
                    "Protection" => self.tab_package.imgs.protection_ico,
                    "Stun Resilience" => self.tab_package.imgs.stun_res_ico,
                    "Stealth" => self.tab_package.imgs.stealth_rating_ico,
                    _ => self.tab_package.imgs.nothing,
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
                            Some(self.inventory),
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
                            Some(self.inventory),
                            self.msm,
                            None,
                            Some(self.stats),
                        )) as i32
                );
                let stealth_txt = format!(
                    "{:.1}%",
                    ((1.0
                        - perception_dist_multiplier_from_stealth(
                            Some(self.inventory),
                            None,
                            self.msm
                        ))
                        * 100.0)
                );
                let btn = if i.0 == 0 {
                    btn.top_left_with_margins_on(self.bag_ids.bg_frame, 95.0, 10.0)
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
                    self.bag_tooltip,
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
                .font_id(self.tab_package.fonts.cyri.conrod_id)
                .font_size(self.tab_package.fonts.cyri.scale(14))
                .color(TEXT_COLOR)
                .graphics_for(state.ids.stat_icons[i.0])
                .set(state.ids.stat_txts[i.0], ui);
            }
            // Loadout Slots
            // Head
            let item_slot = EquipSlot::Armor(ArmorSlot::Head);
            let slot = slot_maker
                .fabricate(
                    item_slot,
                    [40.0; 2],
                    state.active_gear_slot == 15 && state.is_focused == true,
                    false,
                )
                .mid_top_with_margin_on(self.bag_ids.bg_frame, 100.0)
                .with_icon(
                    self.tab_package.imgs.head_bg,
                    Vec2::new(32.0, 40.0),
                    Some(UI_MAIN),
                )
                .filled_slot(filled_slot);

            let slot_id = state.ids.head_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-head");

            // Necklace
            let item_slot = EquipSlot::Armor(ArmorSlot::Neck);
            let slot = slot_maker
                .fabricate(
                    item_slot,
                    [40.0; 2],
                    state.active_gear_slot == 14 && state.is_focused == true,
                    false,
                )
                .mid_bottom_with_margin_on(state.ids.head_slot, -50.0)
                .with_icon(
                    self.tab_package.imgs.necklace_bg,
                    Vec2::new(40.0, 31.0),
                    Some(UI_MAIN),
                )
                .filled_slot(filled_slot);

            let slot_id = state.ids.neck_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-neck");

            // Chest
            //Image::new(self.imgs.armor_slot) // different graphics for empty/non empty
            let item_slot = EquipSlot::Armor(ArmorSlot::Chest);
            let slot = slot_maker
                .fabricate(
                    item_slot,
                    [80.0; 2],
                    state.active_gear_slot == 12 && state.is_focused == true,
                    false,
                )
                .mid_bottom_with_margin_on(state.ids.neck_slot, -90.0)
                .with_icon(
                    self.tab_package.imgs.chest_bg,
                    Vec2::new(64.0, 42.0),
                    Some(UI_MAIN),
                )
                .filled_slot(filled_slot);

            let slot_id = state.ids.chest_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-chest");

            // Shoulders
            let item_slot = EquipSlot::Armor(ArmorSlot::Shoulders);
            let slot = slot_maker
                .fabricate(
                    item_slot,
                    [65.0; 2],
                    state.active_gear_slot == 11 && state.is_focused == true,
                    false,
                )
                .bottom_left_with_margins_on(state.ids.chest_slot, 0.0, -80.0)
                .with_icon(
                    self.tab_package.imgs.shoulders_bg,
                    Vec2::new(60.0, 36.0),
                    Some(UI_MAIN),
                )
                .filled_slot(filled_slot);

            let slot_id = state.ids.shoulders_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-shoulders");

            // Hands
            let item_slot = EquipSlot::Armor(ArmorSlot::Hands);
            let slot = slot_maker
                .fabricate(
                    item_slot,
                    [65.0; 2],
                    state.active_gear_slot == 13 && state.is_focused == true,
                    false,
                )
                .bottom_right_with_margins_on(state.ids.chest_slot, 0.0, -80.0)
                .with_icon(
                    self.tab_package.imgs.hands_bg,
                    Vec2::new(55.0, 60.0),
                    Some(UI_MAIN),
                )
                .filled_slot(filled_slot);

            let slot_id = state.ids.hands_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-hands");

            // Belt
            let item_slot = EquipSlot::Armor(ArmorSlot::Belt);
            let slot = slot_maker
                .fabricate(
                    item_slot,
                    [40.0; 2],
                    state.active_gear_slot == 9 && state.is_focused == true,
                    false,
                )
                .mid_bottom_with_margin_on(state.ids.chest_slot, -50.0)
                .with_icon(
                    self.tab_package.imgs.belt_bg,
                    Vec2::new(40.0, 23.0),
                    Some(UI_MAIN),
                )
                .filled_slot(filled_slot);

            let slot_id = state.ids.belt_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-belt");

            // Legs
            let item_slot = EquipSlot::Armor(ArmorSlot::Legs);
            let slot = slot_maker
                .fabricate(
                    item_slot,
                    [80.0; 2],
                    state.active_gear_slot == 6 && state.is_focused == true,
                    false,
                )
                .mid_bottom_with_margin_on(state.ids.belt_slot, -90.0)
                .with_icon(
                    self.tab_package.imgs.legs_bg,
                    Vec2::new(48.0, 70.0),
                    Some(UI_MAIN),
                )
                .filled_slot(filled_slot);

            let slot_id = state.ids.legs_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-legs");

            // Ring right
            let item_slot = EquipSlot::Armor(ArmorSlot::Ring1);
            let slot = slot_maker
                .fabricate(
                    item_slot,
                    [40.0; 2],
                    state.active_gear_slot == 10 && state.is_focused == true,
                    false,
                )
                .bottom_left_with_margins_on(state.ids.hands_slot, -50.0, 0.0)
                .with_icon(
                    self.tab_package.imgs.ring_bg,
                    Vec2::new(36.0, 40.0),
                    Some(UI_MAIN),
                )
                .filled_slot(filled_slot);

            let slot_id = state.ids.ring1_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-ring");

            // Ring left
            let item_slot = EquipSlot::Armor(ArmorSlot::Ring2);
            let slot = slot_maker
                .fabricate(
                    item_slot,
                    [40.0; 2],
                    state.active_gear_slot == 8 && state.is_focused == true,
                    false,
                )
                .bottom_right_with_margins_on(state.ids.shoulders_slot, -50.0, 0.0)
                .with_icon(
                    self.tab_package.imgs.ring_bg,
                    Vec2::new(36.0, 40.0),
                    Some(UI_MAIN),
                )
                .filled_slot(filled_slot);

            let slot_id = state.ids.ring2_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-ring");

            // Back
            let item_slot = EquipSlot::Armor(ArmorSlot::Back);
            let slot = slot_maker
                .fabricate(
                    item_slot,
                    [40.0; 2],
                    state.active_gear_slot == 5 && state.is_focused == true,
                    false,
                )
                .down_from(state.ids.ring2_slot, 10.0)
                .with_icon(
                    self.tab_package.imgs.back_bg,
                    Vec2::new(33.0, 40.0),
                    Some(UI_MAIN),
                )
                .filled_slot(filled_slot);

            let slot_id = state.ids.back_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-back");

            // Foot
            let item_slot = EquipSlot::Armor(ArmorSlot::Feet);
            let slot = slot_maker
                .fabricate(
                    item_slot,
                    [40.0; 2],
                    state.active_gear_slot == 7 && state.is_focused == true,
                    false,
                )
                .down_from(state.ids.ring1_slot, 10.0)
                .with_icon(
                    self.tab_package.imgs.feet_bg,
                    Vec2::new(32.0, 40.0),
                    Some(UI_MAIN),
                )
                .filled_slot(filled_slot);

            let slot_id = state.ids.feet_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-feet");

            // Lantern
            let item_slot = EquipSlot::Lantern;
            let slot = slot_maker
                .fabricate(
                    item_slot,
                    [40.0; 2],
                    state.active_gear_slot == 18 && state.is_focused == true,
                    false,
                )
                .top_right_with_margins_on(self.bag_ids.bg_frame, 100.0, 5.0)
                .with_icon(
                    self.tab_package.imgs.lantern_bg,
                    Vec2::new(24.0, 38.0),
                    Some(UI_MAIN),
                )
                .filled_slot(filled_slot);

            let slot_id = state.ids.lantern_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-lantern");

            // Glider
            let item_slot = EquipSlot::Glider;
            let slot = slot_maker
                .fabricate(
                    item_slot,
                    [40.0; 2],
                    state.active_gear_slot == 17 && state.is_focused == true,
                    false,
                )
                .down_from(state.ids.lantern_slot, 5.0)
                .with_icon(
                    self.tab_package.imgs.glider_bg,
                    Vec2::new(38.0, 38.0),
                    Some(UI_MAIN),
                )
                .filled_slot(filled_slot);

            let slot_id = state.ids.glider_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-glider");

            // Tabard
            let item_slot = EquipSlot::Armor(ArmorSlot::Tabard);
            let slot = slot_maker
                .fabricate(
                    item_slot,
                    [40.0; 2],
                    state.active_gear_slot == 16 && state.is_focused == true,
                    false,
                )
                .down_from(state.ids.glider_slot, 5.0)
                .with_icon(
                    self.tab_package.imgs.tabard_bg,
                    Vec2::new(38.0, 38.0),
                    Some(UI_MAIN),
                )
                .filled_slot(filled_slot);

            let slot_id = state.ids.tabard_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-tabard");

            // Active Mainhand/Left-Slot
            let item_slot = EquipSlot::ActiveMainhand;
            let slot = slot_maker
                .fabricate(
                    item_slot,
                    [80.0; 2],
                    state.active_gear_slot == 1 && state.is_focused == true,
                    false,
                )
                .bottom_right_with_margins_on(state.ids.back_slot, -90.0, 0.0)
                .with_icon(
                    self.tab_package.imgs.mainhand_bg,
                    Vec2::new(75.0, 75.0),
                    Some(UI_MAIN),
                )
                .filled_slot(filled_slot);

            let slot_id = state.ids.active_mainhand_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-mainhand");

            // Active Offhand/Right-Slot
            let item_slot = EquipSlot::ActiveOffhand;
            let slot = slot_maker
                .fabricate(
                    item_slot,
                    [80.0; 2],
                    state.active_gear_slot == 4 && state.is_focused == true,
                    false,
                )
                .bottom_left_with_margins_on(state.ids.feet_slot, -90.0, 0.0)
                .with_icon(
                    self.tab_package.imgs.offhand_bg,
                    Vec2::new(75.0, 75.0),
                    Some(UI_MAIN),
                )
                .filled_slot(filled_slot);

            let slot_id = state.ids.active_offhand_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-offhand");

            // Inactive Mainhand/Left-Slot
            let item_slot = EquipSlot::InactiveMainhand;
            let slot = slot_maker
                .fabricate(
                    item_slot,
                    [35.0; 2],
                    state.active_gear_slot == 2 && state.is_focused == true,
                    false,
                )
                .bottom_right_with_margins_on(state.ids.active_mainhand_slot, 0.0, -47.0)
                .with_icon(
                    self.tab_package.imgs.mainhand_bg,
                    Vec2::new(35.0, 35.0),
                    Some(UI_MAIN),
                )
                .filled_slot(filled_slot);

            let slot_id = state.ids.inactive_mainhand_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-inactive_mainhand");

            // Inactive Offhand/Right-Slot
            let item_slot = EquipSlot::InactiveOffhand;
            let slot = slot_maker
                .fabricate(
                    item_slot,
                    [35.0; 2],
                    state.active_gear_slot == 3 && state.is_focused == true,
                    false,
                )
                .bottom_left_with_margins_on(state.ids.active_offhand_slot, 0.0, -47.0)
                .with_icon(
                    self.tab_package.imgs.offhand_bg,
                    Vec2::new(35.0, 35.0),
                    Some(UI_MAIN),
                )
                .filled_slot(filled_slot);

            let slot_id = state.ids.inactive_offhand_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-inactive_offhand");

            if Button::image(self.tab_package.imgs.swap_equipped_weapons_btn)
                .hover_image(self.tab_package.imgs.swap_equipped_weapons_btn_hover)
                .press_image(self.tab_package.imgs.swap_equipped_weapons_btn_press)
                .w_h(32.0, 40.0)
                .bottom_left_with_margins_on(self.bag_ids.bg_frame, 0.0, 23.3)
                .align_middle_y_of(state.ids.active_mainhand_slot)
                .with_tooltip(
                    self.tooltip_manager,
                    &i18n.get_msg("hud-bag-swap_equipped_weapons_title"),
                    &(if let Some(key) = self
                        .tab_package
                        .global_state
                        .settings
                        .controls
                        .get_binding(GameInput::SwapLoadout)
                    {
                        i18n.get_msg_ctx(
                            "hud-bag-swap_equipped_weapons_desc",
                            &i18n::fluent_args! {
                                "key" => key.display_string()
                            },
                        )
                    } else {
                        Cow::Borrowed("")
                    }),
                    self.tooltip,
                    color::WHITE,
                )
                .set(state.ids.swap_equipped_weapons_btn, ui)
                .was_clicked()
            {
                events.push(GearMenuEvent::SwapEquippedWeapons);
            }

            // Bag 1
            let item_slot = EquipSlot::Armor(ArmorSlot::Bag1);
            let slot = slot_maker
                .fabricate(
                    item_slot,
                    [40.0; 2],
                    state.active_gear_slot == 19 && state.is_focused == true,
                    false,
                )
                .down_from(state.ids.tabard_slot, 25.0)
                .with_icon(
                    self.tab_package.imgs.bag_bg,
                    Vec2::new(28.0, 24.0),
                    Some(UI_MAIN),
                )
                .filled_slot(filled_slot);

            let slot_id = state.ids.bag1_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-bag");

            // Bag 2
            let item_slot = EquipSlot::Armor(ArmorSlot::Bag2);
            let slot = slot_maker
                .fabricate(
                    item_slot,
                    [40.0; 2],
                    state.active_gear_slot == 20 && state.is_focused == true,
                    false,
                )
                .down_from(state.ids.bag1_slot, 5.0)
                .with_icon(
                    self.tab_package.imgs.bag_bg,
                    Vec2::new(28.0, 24.0),
                    Some(UI_MAIN),
                )
                .filled_slot(filled_slot);

            let slot_id = state.ids.bag2_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-bag");

            // Bag 3
            let item_slot = EquipSlot::Armor(ArmorSlot::Bag3);
            let slot = slot_maker
                .fabricate(
                    item_slot,
                    [40.0; 2],
                    state.active_gear_slot == 21 && state.is_focused == true,
                    false,
                )
                .down_from(state.ids.bag2_slot, 5.0)
                .with_icon(
                    self.tab_package.imgs.bag_bg,
                    Vec2::new(28.0, 24.0),
                    Some(UI_MAIN),
                )
                .filled_slot(filled_slot);

            let slot_id = state.ids.bag3_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-bag");

            // Bag 4
            let item_slot = EquipSlot::Armor(ArmorSlot::Bag4);
            let slot = slot_maker
                .fabricate(
                    item_slot,
                    [40.0; 2],
                    state.active_gear_slot == 22 && state.is_focused == true,
                    false,
                )
                .down_from(state.ids.bag3_slot, 2.0)
                .with_icon(
                    self.tab_package.imgs.bag_bg,
                    Vec2::new(28.0, 24.0),
                    Some(UI_MAIN),
                )
                .filled_slot(filled_slot);

            let slot_id = state.ids.bag4_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-bag");
        }

        events
    }
}

/*
widget_ids! {
    pub struct BagIds {
        inventory_scroller,
        bag_close,
        char_ico,
        inventory_sort,
        inventory_sort_selected,
        bag_expand_btn,
        bag_details_btn,
        // Armor Slots
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
    info: &'a HudInfo<'a>,
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
    rbm: &'a RecipeBookManifest,
    poise: &'a Poise,
    menu_events: &'a Vec<MenuInput>,
}

impl<'a> Bag<'a> {
    #[expect(clippy::too_many_arguments)]
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
        rbm: &'a RecipeBookManifest,
        poise: &'a Poise,
        menu_events: &'a Vec<MenuInput>,
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
            rbm,
            poise,
            menu_events,
        }
    }
}

pub struct BagState {
    ids: BagIds,
    bg_ids: BackgroundIds,

    active_content: usize,
    active_gear_slot: usize,
}

pub enum Event {
    BagExpand,
    Close,
    ChangeInventorySortOrder(InventorySortOrder),
    SortInventory(InventorySortOrder),
    SwapEquippedWeapons,
    SetDetailsMode(bool),
    MoveBag(Vec2<f64>),
}

impl Widget for Bag<'_> {
    type Event = Vec<Event>;
    type State = BagState;
    type Style = ();

    fn init_state(&self, mut id_gen: widget::id::Generator) -> Self::State {
        BagState {
            bg_ids: BackgroundIds {
                bg: id_gen.next(),
                bg_frame: id_gen.next(),
            },
            ids: BagIds::new(id_gen),
            active_content: 0,
            active_gear_slot: 1,
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        common_base::prof_span!("Bag::update");
        let widget::UpdateArgs { state, ui, .. } = args;
        let i18n = &self.localized_strings;

        let mut events = Vec::new();

        // If I change local focus to 0 immidiately, it will also be read by the
        // inventory which will also register the same input and change it 1. A fix for
        // the current set-up is to just change focus after the inventory has
        // been calculated
        let mut change_local_focus = false;

        // MENU INPUTS: manage gear elements
        // LocalFocus: change which parts of the screen you interact with (0 =
        // inventory, 1 = inventory filters/buttons, 2 = gear)
        // Up: try to go up in the gear list
        // Down: try to go down the gear list
        // Left: try to move left in the gear list
        // Right: try to move right in the gear list
        // Apply: TODO
        // Back: close the bag when gear menu is in focus
        if state.active_content == 2 {
            for event in self.menu_events {
                match *event {
                    MenuInput::LocalFocus => {
                        // Reset back to 0 (inventory)
                        // This whole interaction logic should probably be improved sometime
                        change_local_focus = true;
                    },
                    MenuInput::Up => state.update(|s| {
                        // So many values to manual set...
                        match s.active_gear_slot {
                            // weapon switch button
                            0 => {},
                            // primary weapon left
                            1 => s.active_gear_slot = 5,
                            // secondary weapon left
                            2 => s.active_gear_slot = 6,
                            // secondary weapon right
                            3 => s.active_gear_slot = 6,
                            // primary weapon right
                            4 => s.active_gear_slot = 7,
                            // back
                            5 => s.active_gear_slot = 8,
                            // pants
                            6 => s.active_gear_slot = 9,
                            // shoes
                            7 => s.active_gear_slot = 10,
                            // jewelry left
                            8 => s.active_gear_slot = 11,
                            // belt
                            9 => s.active_gear_slot = 12,
                            // jewelry right
                            10 => s.active_gear_slot = 13,
                            // shoulder
                            11 => s.active_gear_slot = 14,
                            // chest
                            12 => s.active_gear_slot = 14,
                            // hands/gloves
                            13 => s.active_gear_slot = 14,
                            // jewelry center
                            14 => s.active_gear_slot = 15,
                            // hat
                            15 => {},
                            // tabard
                            16 => s.active_gear_slot = 17,
                            // glider
                            17 => s.active_gear_slot = 18,
                            // lantern
                            18 => {},
                            // reset to 0 if unexpected
                            _ => s.active_gear_slot = 0,
                        }
                    }),
                    MenuInput::Down => state.update(|s| {
                        match s.active_gear_slot {
                            // weapon switch button
                            0 => {},
                            // primary weapon 1
                            1 => {},
                            // secondary weapon 1
                            2 => {},
                            // secondary weapon 2
                            3 => {},
                            // primary weapon 1
                            4 => {},
                            // back
                            5 => s.active_gear_slot = 1,
                            // pants
                            6 => s.active_gear_slot = 2,
                            // shoes
                            7 => s.active_gear_slot = 4,
                            // jewelry left
                            8 => s.active_gear_slot = 5,
                            // belt
                            9 => s.active_gear_slot = 6,
                            // jewelry right
                            10 => s.active_gear_slot = 7,
                            // shoulder
                            11 => s.active_gear_slot = 8,
                            // chest
                            12 => s.active_gear_slot = 9,
                            // hands/gloves
                            13 => s.active_gear_slot = 10,
                            // jewelry center
                            14 => s.active_gear_slot = 12,
                            // hat
                            15 => s.active_gear_slot = 14,
                            // tabard
                            16 => {},
                            // glider
                            17 => s.active_gear_slot = 16,
                            // lantern
                            18 => s.active_gear_slot = 17,
                            // reset to 0 if unexpected
                            _ => s.active_gear_slot = 0,
                        }
                    }),
                    MenuInput::Left => state.update(|s| {
                        match s.active_gear_slot {
                            // weapon switch button
                            0 => {},
                            // primary weapon 1
                            1 => s.active_gear_slot = 0 + 1,
                            // secondary weapon 1
                            2 => s.active_gear_slot = 1,
                            // secondary weapon 2
                            3 => s.active_gear_slot = 2,
                            // primary weapon 1
                            4 => s.active_gear_slot = 3,
                            // back
                            5 => {},
                            // pants
                            6 => s.active_gear_slot = 5,
                            // shoes
                            7 => s.active_gear_slot = 6,
                            // jewelry left
                            8 => {},
                            // belt
                            9 => s.active_gear_slot = 8,
                            // jewelry right
                            10 => s.active_gear_slot = 9,
                            // shoulder
                            11 => {},
                            // chest
                            12 => s.active_gear_slot = 11,
                            // hands/gloves
                            13 => s.active_gear_slot = 12,
                            // jewelry center
                            14 => {},
                            // hat
                            15 => {},
                            // tabard
                            16 => s.active_gear_slot = 13,
                            // glider
                            17 => s.active_gear_slot = 14,
                            // lantern
                            18 => s.active_gear_slot = 15,
                            // reset to 0 if unexpected
                            _ => s.active_gear_slot = 0,
                        }
                    }),
                    MenuInput::Right => state.update(|s| {
                        match s.active_gear_slot {
                            // weapon switch button
                            0 => s.active_gear_slot = 1,
                            // primary weapon 1
                            1 => s.active_gear_slot = 2,
                            // secondary weapon 1
                            2 => s.active_gear_slot = 3,
                            // secondary weapon 2
                            3 => s.active_gear_slot = 4,
                            // primary weapon 1
                            4 => s.active_gear_slot = 16,
                            // back
                            5 => s.active_gear_slot = 6,
                            // pants
                            6 => s.active_gear_slot = 7,
                            // shoes
                            7 => s.active_gear_slot = 16,
                            // jewelry left
                            8 => s.active_gear_slot = 9,
                            // belt
                            9 => s.active_gear_slot = 10,
                            // jewelry right
                            10 => s.active_gear_slot = 16,
                            // shoulder
                            11 => s.active_gear_slot = 12,
                            // chest
                            12 => s.active_gear_slot = 13,
                            // hands/gloves
                            13 => s.active_gear_slot = 16,
                            // jewelry center
                            14 => s.active_gear_slot = 17,
                            // hat
                            15 => s.active_gear_slot = 18,
                            // tabard
                            16 => {},
                            // glider
                            17 => {},
                            // lantern
                            18 => {},
                            // reset to 0 if unexpected
                            _ => s.active_gear_slot = 0,
                        }
                    }),
                    MenuInput::Apply => {
                        // TODO
                    },
                    MenuInput::Back => {
                        // Typically, we want child widgets to handle their own back events
                        // This back event only applies to the gear, which is in this widget
                        events.push(Event::Close);
                    },
                    _ => {},
                }
            }
        }

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

        if let Some(inventory) = self.client.inventories().get(self.info.viewpoint_entity) {
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
                self.rbm,
                Some(inventory),
                self.localized_strings,
                self.item_i18n,
            )
            .title_font_size(self.fonts.cyri.scale(20))
            .parent(ui.window)
            .desc_font_size(self.fonts.cyri.scale(12))
            .font_id(self.fonts.cyri.conrod_id)
            .desc_text_color(TEXT_COLOR);

            for event in InventoryScroller::new(
                self.client,
                self.global_state,
                self.imgs,
                self.item_imgs,
                self.fonts,
                self.item_tooltip_manager,
                self.slot_manager,
                self.pulse,
                self.menu_events,
                state.active_content,
                self.localized_strings,
                self.item_i18n,
                self.show.stats,
                self.show.bag_inv,
                true,
                &item_tooltip,
                self.localized_strings.get_content(&self.stats.name),
                self.info.viewpoint_entity,
                true,
                inventory,
                &state.bg_ids,
                self.show.crafting_fields.salvage,
                self.show.bag_details,
            )
            .set(state.ids.inventory_scroller, ui)
            {
                match event {
                    InventoryScrollerEvent::Drag(pos) => {
                        events.push(Event::MoveBag(pos));
                    },
                    InventoryScrollerEvent::ChangeLocalFocus(change) => state.update(|s| {
                        s.active_content = change;
                    }),
                    InventoryScrollerEvent::Close => {
                        events.push(Event::Close);
                    },
                }
            }

            // change local focus from gear to inventory after inventory actions have been
            // registered
            if change_local_focus {
                state.update(|s| {
                    s.active_content = 0;
                })
            }

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
                events.push(Event::SetDetailsMode(!self.show.bag_details));
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
                events.push(Event::BagExpand);
            }

            // Sort mode inventory button
            if Button::image(self.imgs.inv_sort_btn)
            .w_h(30.0, 17.0)
            .hover_image(self.imgs.inv_sort_btn_hover)
            .press_image(self.imgs.inv_sort_btn_press)
            .top_left_with_margins_on(state.bg_ids.bg_frame, buttons_top, 87.0) // 30 + 10 + 47
            .with_tooltip(
                self.tooltip_manager,
                &(match self.global_state.settings.inventory.sort_order.next() {
                    InventorySortOrder::Name => i18n.get_msg("hud-bag-change_to_sort_by_name"),
                    InventorySortOrder::Quality => i18n.get_msg("hud-bag-change_to_sort_by_quality"),
                    InventorySortOrder::Category => i18n.get_msg("hud-bag-change_to_sort_by_category"),
                    InventorySortOrder::Tag => i18n.get_msg("hud-bag-change_to_sort_by_tag"),
                    InventorySortOrder::Amount => i18n.get_msg("hud-bag-change_to_sort_by_quantity"),
                }),
                "",
                &tooltip,
                color::WHITE,
            )
            .set(state.ids.inventory_sort, ui)
            .was_clicked()
            {
                // cycle sorting mode
                events.push(Event::ChangeInventorySortOrder(
                    self.global_state.settings.inventory.sort_order.next(),
                ));
            }
            // Sort inventory button with selected mode
            if Button::image(self.imgs.inv_sort_selected_btn)
                .w_h(30.0, 17.0)
                .hover_image(self.imgs.inv_sort_selected_btn_hover)
                .press_image(self.imgs.inv_sort_selected_btn_press)
                .top_left_with_margins_on(state.bg_ids.bg_frame, buttons_top, 47.0)
                .with_tooltip(
                    self.tooltip_manager,
                    &(match self.global_state.settings.inventory.sort_order {
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
                .set(state.ids.inventory_sort_selected, ui)
                .was_clicked()
            {
                events.push(Event::SortInventory(
                    self.global_state.settings.inventory.sort_order,
                ));
            }

            // Armor Slots
            let mut slot_maker = SlotMaker {
                empty_slot: self.imgs.armor_slot_empty,
                hovered_slot: self.imgs.skillbar_index,
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
                last_input: &self.global_state.window.last_input(),
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
                                Some(self.stats),
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
                // Head
                let item_slot = EquipSlot::Armor(ArmorSlot::Head);
                let slot = slot_maker
                    .fabricate(
                        item_slot,
                        [45.0; 2],
                        state.active_gear_slot == 15 && state.active_content == 2,
                        false,
                    )
                    .mid_top_with_margin_on(state.bg_ids.bg_frame, 60.0)
                    .with_icon(self.imgs.head_bg, Vec2::new(32.0, 40.0), Some(UI_MAIN))
                    .filled_slot(filled_slot);

                let slot_id = state.ids.head_slot;
                set_tooltip!(slot, slot_id, item_slot, "hud-bag-head");

                // Necklace
                let item_slot = EquipSlot::Armor(ArmorSlot::Neck);
                let slot = slot_maker
                    .fabricate(
                        item_slot,
                        [45.0; 2],
                        state.active_gear_slot == 14 && state.active_content == 2,
                        false,
                    )
                    .mid_bottom_with_margin_on(state.ids.head_slot, -55.0)
                    .with_icon(self.imgs.necklace_bg, Vec2::new(40.0, 31.0), Some(UI_MAIN))
                    .filled_slot(filled_slot);

                let slot_id = state.ids.neck_slot;
                set_tooltip!(slot, slot_id, item_slot, "hud-bag-neck");

                // Chest
                //Image::new(self.imgs.armor_slot) // different graphics for empty/non empty
                let item_slot = EquipSlot::Armor(ArmorSlot::Chest);
                let slot = slot_maker
                    .fabricate(
                        item_slot,
                        [85.0; 2],
                        state.active_gear_slot == 12 && state.active_content == 2,
                        false,
                    )
                    .mid_bottom_with_margin_on(state.ids.neck_slot, -95.0)
                    .with_icon(self.imgs.chest_bg, Vec2::new(64.0, 42.0), Some(UI_MAIN))
                    .filled_slot(filled_slot);

                let slot_id = state.ids.chest_slot;
                set_tooltip!(slot, slot_id, item_slot, "hud-bag-chest");

                // Shoulders
                let item_slot = EquipSlot::Armor(ArmorSlot::Shoulders);
                let slot = slot_maker
                    .fabricate(
                        item_slot,
                        [70.0; 2],
                        state.active_gear_slot == 11 && state.active_content == 2,
                        false,
                    )
                    .bottom_left_with_margins_on(state.ids.chest_slot, 0.0, -80.0)
                    .with_icon(self.imgs.shoulders_bg, Vec2::new(60.0, 36.0), Some(UI_MAIN))
                    .filled_slot(filled_slot);

                let slot_id = state.ids.shoulders_slot;
                set_tooltip!(slot, slot_id, item_slot, "hud-bag-shoulders");

                // Hands
                let item_slot = EquipSlot::Armor(ArmorSlot::Hands);
                let slot = slot_maker
                    .fabricate(
                        item_slot,
                        [70.0; 2],
                        state.active_gear_slot == 13 && state.active_content == 2,
                        false,
                    )
                    .bottom_right_with_margins_on(state.ids.chest_slot, 0.0, -80.0)
                    .with_icon(self.imgs.hands_bg, Vec2::new(55.0, 60.0), Some(UI_MAIN))
                    .filled_slot(filled_slot);

                let slot_id = state.ids.hands_slot;
                set_tooltip!(slot, slot_id, item_slot, "hud-bag-hands");

                // Belt
                let item_slot = EquipSlot::Armor(ArmorSlot::Belt);
                let slot = slot_maker
                    .fabricate(
                        item_slot,
                        [45.0; 2],
                        state.active_gear_slot == 9 && state.active_content == 2,
                        false,
                    )
                    .mid_bottom_with_margin_on(state.ids.chest_slot, -55.0)
                    .with_icon(self.imgs.belt_bg, Vec2::new(40.0, 23.0), Some(UI_MAIN))
                    .filled_slot(filled_slot);

                let slot_id = state.ids.belt_slot;
                set_tooltip!(slot, slot_id, item_slot, "hud-bag-belt");

                // Legs
                let item_slot = EquipSlot::Armor(ArmorSlot::Legs);
                let slot = slot_maker
                    .fabricate(
                        item_slot,
                        [85.0; 2],
                        state.active_gear_slot == 6 && state.active_content == 2,
                        false,
                    )
                    .mid_bottom_with_margin_on(state.ids.belt_slot, -95.0)
                    .with_icon(self.imgs.legs_bg, Vec2::new(48.0, 70.0), Some(UI_MAIN))
                    .filled_slot(filled_slot);

                let slot_id = state.ids.legs_slot;
                set_tooltip!(slot, slot_id, item_slot, "hud-bag-legs");

                // Ring right
                let item_slot = EquipSlot::Armor(ArmorSlot::Ring1);
                let slot = slot_maker
                    .fabricate(
                        item_slot,
                        [45.0; 2],
                        state.active_gear_slot == 10 && state.active_content == 2,
                        false,
                    )
                    .bottom_left_with_margins_on(state.ids.hands_slot, -55.0, 0.0)
                    .with_icon(self.imgs.ring_bg, Vec2::new(36.0, 40.0), Some(UI_MAIN))
                    .filled_slot(filled_slot);

                let slot_id = state.ids.ring1_slot;
                set_tooltip!(slot, slot_id, item_slot, "hud-bag-ring");

                // Ring left
                let item_slot = EquipSlot::Armor(ArmorSlot::Ring2);
                let slot = slot_maker
                    .fabricate(
                        item_slot,
                        [45.0; 2],
                        state.active_gear_slot == 8 && state.active_content == 2,
                        false,
                    )
                    .bottom_right_with_margins_on(state.ids.shoulders_slot, -55.0, 0.0)
                    .with_icon(self.imgs.ring_bg, Vec2::new(36.0, 40.0), Some(UI_MAIN))
                    .filled_slot(filled_slot);

                let slot_id = state.ids.ring2_slot;
                set_tooltip!(slot, slot_id, item_slot, "hud-bag-ring");

                // Back
                let item_slot = EquipSlot::Armor(ArmorSlot::Back);
                let slot = slot_maker
                    .fabricate(
                        item_slot,
                        [45.0; 2],
                        state.active_gear_slot == 5 && state.active_content == 2,
                        false,
                    )
                    .down_from(state.ids.ring2_slot, 10.0)
                    .with_icon(self.imgs.back_bg, Vec2::new(33.0, 40.0), Some(UI_MAIN))
                    .filled_slot(filled_slot);

                let slot_id = state.ids.back_slot;
                set_tooltip!(slot, slot_id, item_slot, "hud-bag-back");

                // Foot
                let item_slot = EquipSlot::Armor(ArmorSlot::Feet);
                let slot = slot_maker
                    .fabricate(
                        item_slot,
                        [45.0; 2],
                        state.active_gear_slot == 7 && state.active_content == 2,
                        false,
                    )
                    .down_from(state.ids.ring1_slot, 10.0)
                    .with_icon(self.imgs.feet_bg, Vec2::new(32.0, 40.0), Some(UI_MAIN))
                    .filled_slot(filled_slot);

                let slot_id = state.ids.feet_slot;
                set_tooltip!(slot, slot_id, item_slot, "hud-bag-feet");

                // Lantern
                let item_slot = EquipSlot::Lantern;
                let slot = slot_maker
                    .fabricate(
                        item_slot,
                        [45.0; 2],
                        state.active_gear_slot == 18 && state.active_content == 2,
                        false,
                    )
                    .top_right_with_margins_on(state.bg_ids.bg_frame, 60.0, 5.0)
                    .with_icon(self.imgs.lantern_bg, Vec2::new(24.0, 38.0), Some(UI_MAIN))
                    .filled_slot(filled_slot);

                let slot_id = state.ids.lantern_slot;
                set_tooltip!(slot, slot_id, item_slot, "hud-bag-lantern");

                // Glider
                let item_slot = EquipSlot::Glider;
                let slot = slot_maker
                    .fabricate(
                        item_slot,
                        [45.0; 2],
                        state.active_gear_slot == 17 && state.active_content == 2,
                        false,
                    )
                    .down_from(state.ids.lantern_slot, 5.0)
                    .with_icon(self.imgs.glider_bg, Vec2::new(38.0, 38.0), Some(UI_MAIN))
                    .filled_slot(filled_slot);

                let slot_id = state.ids.glider_slot;
                set_tooltip!(slot, slot_id, item_slot, "hud-bag-glider");

                // Tabard
                let item_slot = EquipSlot::Armor(ArmorSlot::Tabard);
                let slot = slot_maker
                    .fabricate(
                        item_slot,
                        [45.0; 2],
                        state.active_gear_slot == 16 && state.active_content == 2,
                        false,
                    )
                    .down_from(state.ids.glider_slot, 5.0)
                    .with_icon(self.imgs.tabard_bg, Vec2::new(38.0, 38.0), Some(UI_MAIN))
                    .filled_slot(filled_slot);

                let slot_id = state.ids.tabard_slot;
                set_tooltip!(slot, slot_id, item_slot, "hud-bag-tabard");

                // Active Mainhand/Left-Slot
                let item_slot = EquipSlot::ActiveMainhand;
                let slot = slot_maker
                    .fabricate(
                        item_slot,
                        [85.0; 2],
                        state.active_gear_slot == 1 && state.active_content == 2,
                        false,
                    )
                    .bottom_right_with_margins_on(state.ids.back_slot, -95.0, 0.0)
                    .with_icon(self.imgs.mainhand_bg, Vec2::new(75.0, 75.0), Some(UI_MAIN))
                    .filled_slot(filled_slot);

                let slot_id = state.ids.active_mainhand_slot;
                set_tooltip!(slot, slot_id, item_slot, "hud-bag-mainhand");

                // Active Offhand/Right-Slot
                let item_slot = EquipSlot::ActiveOffhand;
                let slot = slot_maker
                    .fabricate(
                        item_slot,
                        [85.0; 2],
                        state.active_gear_slot == 4 && state.active_content == 2,
                        false,
                    )
                    .bottom_left_with_margins_on(state.ids.feet_slot, -95.0, 0.0)
                    .with_icon(self.imgs.offhand_bg, Vec2::new(75.0, 75.0), Some(UI_MAIN))
                    .filled_slot(filled_slot);

                let slot_id = state.ids.active_offhand_slot;
                set_tooltip!(slot, slot_id, item_slot, "hud-bag-offhand");

                // Inactive Mainhand/Left-Slot
                let item_slot = EquipSlot::InactiveMainhand;
                let slot = slot_maker
                    .fabricate(
                        item_slot,
                        [40.0; 2],
                        state.active_gear_slot == 2 && state.active_content == 2,
                        false,
                    )
                    .bottom_right_with_margins_on(state.ids.active_mainhand_slot, 3.0, -47.0)
                    .with_icon(self.imgs.mainhand_bg, Vec2::new(35.0, 35.0), Some(UI_MAIN))
                    .filled_slot(filled_slot);

                let slot_id = state.ids.inactive_mainhand_slot;
                set_tooltip!(slot, slot_id, item_slot, "hud-bag-inactive_mainhand");

                // Inactive Offhand/Right-Slot
                let item_slot = EquipSlot::InactiveOffhand;
                let slot = slot_maker
                    .fabricate(
                        item_slot,
                        [40.0; 2],
                        state.active_gear_slot == 3 && state.active_content == 2,
                        false,
                    )
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
                                    "key" => key.display_string()
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
                    events.push(Event::SwapEquippedWeapons);
                }
            }

            // Bag 1
            let item_slot = EquipSlot::Armor(ArmorSlot::Bag1);
            let slot = slot_maker
                .fabricate(item_slot, [35.0; 2], false, false)
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
                .fabricate(item_slot, [35.0; 2], false, false)
                .down_from(state.ids.bag1_slot, 2.0)
                .with_icon(self.imgs.bag_bg, Vec2::new(28.0, 24.0), Some(UI_MAIN))
                .filled_slot(filled_slot);

            let slot_id = state.ids.bag2_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-bag");

            // Bag 3
            let item_slot = EquipSlot::Armor(ArmorSlot::Bag3);
            let slot = slot_maker
                .fabricate(item_slot, [35.0; 2], false, false)
                .down_from(state.ids.bag2_slot, 2.0)
                .with_icon(self.imgs.bag_bg, Vec2::new(28.0, 24.0), Some(UI_MAIN))
                .filled_slot(filled_slot);

            let slot_id = state.ids.bag3_slot;
            set_tooltip!(slot, slot_id, item_slot, "hud-bag-bag");

            // Bag 4
            let item_slot = EquipSlot::Armor(ArmorSlot::Bag4);
            let slot = slot_maker
                .fabricate(item_slot, [35.0; 2], false, false)
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
                events.push(Event::Close);
            }
        }

        events
    }
}
*/
