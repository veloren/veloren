use super::{
    TEXT_COLOR, UI_MAIN,
    img_ids::Imgs,
    item_imgs::ItemImgs,
    slots::{InventorySlot, SlotManager},
    util,
};
use crate::{
    hud::slots::SlotKind,
    ui::{
        ItemTooltip, ItemTooltipManager, ItemTooltipable,
        fonts::Fonts,
        slot::{ContentSize, SlotMaker},
    },
    window::{LastInput, MenuInput},
};
use client::Client;
use common::{
    assets::AssetExt,
    comp::{
        Inventory,
        inventory::slot::Slot,
        item::{ItemDef, ItemDesc, ItemI18n, ItemKind, Quality},
    },
};
use conrod_core::{
    Borderable, Color, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
    builder_methods, color,
    widget::{self, Button, Rectangle, Text},
    widget_ids,
};
use i18n::Localization;
use specs::Entity as EcsEntity;
use std::{borrow::Borrow, sync::Arc};
use vek::Vec2;

#[derive(PartialEq)]
pub enum TabFilters {
    Gear,
    //Ingredients,
    //Food,
    //QuestItems,
    None,
}

pub enum SlotEvents {
    Close,
    ExitUp,
    ExitLeft,
    ExitRight,
    ExitDown,
    FilteredSize(usize),
}

#[derive(WidgetCommon)]
pub struct SlotGrid<'a> {
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    client: &'a Client,
    imgs: &'a Imgs,
    item_imgs: &'a ItemImgs,
    fonts: &'a Fonts,
    item_tooltip_manager: &'a mut ItemTooltipManager,
    slot_manager: &'a mut SlotManager,
    inventory: &'a Inventory,
    item_tooltip: &'a ItemTooltip<'a>,
    localized_strings: &'a Localization,
    item_i18n: &'a ItemI18n,
    entity: EcsEntity,
    last_input: &'a LastInput,
    pulse: f32,
    menu_events: &'a Vec<MenuInput>,
    is_us: bool,
    details_mode: bool,
    show_salvage: bool,
    columns: usize,
    spacing: f64,
    slot_size: f64,
    is_focused: bool,
    filter: TabFilters,
}

widget_ids! {
    struct Ids {
        item_slots[],
        inv_slot_names[],
        inv_slot_amounts[],

        context_menu,
        spacing_below,
    }
}

pub struct State {
    ids: Ids,

    active_context_slot: Option<SlotKind>,
    // TODO: switch from 2D coordinates to 1D coordinates for optimization
    context_menu_pos: [f64; 2],
    active_slot: [usize; 2],
}

impl<'a> SlotGrid<'a> {
    builder_methods! {
        pub columns { columns = usize }
        pub spacing { spacing = f64 }
        pub slot_size { slot_size = f64 }
        pub is_focused { is_focused = bool }
        pub filter { filter = TabFilters }
    }

    #[expect(clippy::too_many_arguments)]
    pub fn new(
        client: &'a Client,
        imgs: &'a Imgs,
        item_imgs: &'a ItemImgs,
        fonts: &'a Fonts,
        item_tooltip_manager: &'a mut ItemTooltipManager,
        slot_manager: &'a mut SlotManager,
        inventory: &'a Inventory,
        item_tooltip: &'a ItemTooltip<'a>,
        localized_strings: &'a Localization,
        item_i18n: &'a ItemI18n,
        entity: EcsEntity,
        last_input: &'a LastInput,
        pulse: f32,
        menu_events: &'a Vec<MenuInput>,
        is_us: bool,
        details_mode: bool,
        show_salvage: bool,
    ) -> Self {
        SlotGrid {
            common: widget::CommonBuilder::default(),
            client,
            imgs,
            item_imgs,
            fonts,
            item_tooltip_manager,
            slot_manager,
            inventory,
            item_tooltip,
            localized_strings,
            item_i18n,
            entity,
            last_input,
            pulse,
            menu_events,
            is_us,
            details_mode,
            show_salvage,
            columns: 6,
            slot_size: 55.0,
            spacing: 6.0,
            is_focused: true,
            filter: TabFilters::None,
        }
    }
}

impl<'a> Widget for SlotGrid<'a> {
    type Event = Vec<SlotEvents>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
            active_context_slot: None,
            context_menu_pos: [0.0, 0.0],
            active_slot: [0, 0],
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { id, state, ui, .. } = args;

        // Capture selected slot
        let selected = self.slot_manager.selected();
        if selected.is_none() {
            state.update(|s| {
                // If nothing is selected, the context menu should never be open
                s.active_context_slot = None;
            })
        }

        // Filter inventory
        let inventory_iter = || {
            self.inventory
                .slots_with_id()
                .map(|(slot, item)| (Slot::Inventory(slot), item.as_ref()))
                .chain(
                    self.inventory
                        .overflow_items()
                        .enumerate()
                        .map(|(i, item)| (Slot::Overflow(i), Some(item))),
                )
        };
        let mut items = inventory_iter()
            .filter(|(_, items_list)| match self.filter {
                TabFilters::Gear => {
                    if let Some(item) = items_list {
                        matches!(
                            &*item.kind(),
                            ItemKind::Tool(_)
                                | ItemKind::ModularComponent(_)
                                | ItemKind::Lantern(_)
                                | ItemKind::Armor(_)
                                | ItemKind::Glider
                        )
                    } else {
                        false
                    }
                },
                // The below commented code is used to filter the entire inventory done into
                // specific categories. These additional inventory tabs were experimented
                // with, but not included in the initial bag redesign MR
                /*
                TabFilters::Ingredients => {
                    if let Some(item) = items_list {
                        match &*item.kind() {
                            // Allowing it because 'descriptor' isn't being used
                            #[allow(deprecated)]
                            ItemKind::Ingredient { descriptor: _ } => true,
                            _ => false,
                        }
                    } else {
                        false
                    }
                },
                TabFilters::Food => {
                    if let Some(item) = items_list {
                        item.tags().contains(&ItemTag::Food)
                            | item.tags().contains(&ItemTag::Potion)
                    } else {
                        false
                    }
                },
                TabFilters::QuestItems => {
                    if let Some(item) = items_list {
                        matches!(&*item.kind(), ItemKind::Quest)
                    } else {
                        false
                    }
                },
                */
                TabFilters::None => true,
            })
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

        // Add the first empty slot (if any) to the items list to help with removing
        // gear and visualizing when more space is available
        if self.filter != TabFilters::None
            && let Some(empty_slot) = inventory_iter().find(|(_, item)| item.is_none())
        {
            items.push(empty_slot);
        }

        // Calculate formatting info
        let total_slots = items.len();
        let cols = if self.details_mode { 1 } else { self.columns };

        let mut events = Vec::new();

        // MENU INPUTS: change the slot focus
        // Up: go up a row (no wrap)
        // Down: go down a row (no wrap)
        // Left: move left a column (no wrap)
        // Right: move right a column (no wrap)
        // LocalFocus: Change local focus
        // Apply: select the current slot
        // Back: close the bag menu
        let mut clicked = false;
        if selected.is_none() && self.is_focused {
            for event in self.menu_events {
                match *event {
                    MenuInput::Up => state.update(|s| {
                        let [x, y] = s.active_slot;
                        if y > 0 {
                            s.active_slot = [x, y - 1];
                        } else {
                            events.push(SlotEvents::ExitUp);
                        }
                    }),
                    MenuInput::Down => state.update(|s| {
                        let [x, y] = s.active_slot;
                        if y < (total_slots / cols) {
                            s.active_slot = [x, y + 1];
                        } else {
                            events.push(SlotEvents::ExitDown);
                        }
                    }),
                    MenuInput::Left => state.update(|s| {
                        let [x, y] = s.active_slot;
                        if x > 0 {
                            s.active_slot = [x - 1, y];
                        } else {
                            events.push(SlotEvents::ExitLeft);
                        }
                    }),
                    MenuInput::Right => state.update(|s| {
                        let [x, y] = s.active_slot;
                        // Only go right if there are slots to go to
                        if x < self.columns - 1 && (y * cols) + (x + 1) < total_slots {
                            s.active_slot = [x + 1, y];
                        } else {
                            events.push(SlotEvents::ExitRight);
                        }
                    }),
                    MenuInput::Apply => {
                        clicked = true;
                    },
                    MenuInput::Back => {
                        events.push(SlotEvents::Close);
                    },
                    _ => {},
                }
            }
        }

        // Create available inventory slot widgets
        if state.ids.item_slots.len() < total_slots {
            state.update(|s| {
                s.ids
                    .item_slots
                    .resize(total_slots, &mut ui.widget_id_generator());
                s.ids
                    .inv_slot_names
                    .resize(total_slots, &mut ui.widget_id_generator());
                s.ids
                    .inv_slot_amounts
                    .resize(total_slots, &mut ui.widget_id_generator());
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
            hovered_slot: self.imgs.skillbar_index,
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
            last_input: self.last_input,
            pulse: self.pulse,
        };

        for (i, (pos, item)) in items.into_iter().enumerate() {
            if self.details_mode && !self.is_us && item.is_none() {
                continue;
            }
            let (x, y) = if self.details_mode {
                (0, i)
            } else {
                (i % self.columns, i / self.columns)
            };

            // Inventory slot details
            let x_pos = (x as f64 * (self.slot_size + self.spacing)).floor();
            let y_pos = (y as f64 * (self.slot_size + self.spacing)).floor();
            let inv_slot = InventorySlot {
                slot: pos,
                ours: self.is_us,
                entity: self.entity,
            };

            // Check if active menu navigation hover
            let menu_hover = state.active_slot[0] == x
                && state.active_slot[1] == y // Is it the current slot
                && selected.is_none() // Is the context menu not open
                && self.is_focused; // Is focus on the inventory

            let mut slot_widget = slot_maker
                .fabricate(inv_slot, [self.slot_size as f32; 2], menu_hover, clicked)
                .top_left_with_margins_on(
                    id,
                    // Decimal values might cause pixel mismatches between slots, use floor to try
                    // to avoid this
                    (y as f64 * (self.slot_size + self.spacing)).floor(),
                    (x as f64 * (self.slot_size + self.spacing)).floor(),
                );

            // Highlight slots are provided by the loadout item (bag) that the mouse is over
            if self.filter == TabFilters::None && mouseover_loadout_slots.contains(&i) {
                slot_widget = slot_widget.with_background_color(Color::Rgba(1.0, 1.0, 1.0, 1.0));
            }

            if self.show_salvage && item.as_ref().is_some_and(|item| item.is_salvageable()) {
                slot_widget = slot_widget.with_background_color(Color::Rgba(1.0, 1.0, 1.0, 1.0));
            }

            // Highlight in red the slots that are overflow
            if matches!(pos, Slot::Overflow(_)) {
                slot_widget = slot_widget.with_background_color(Color::Rgba(1.0, 0.0, 0.0, 1.0));
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

                    let salvage_items = salvage_result
                        .iter()
                        .map(|item| item.borrow())
                        .chain(core::iter::once(item as &dyn ItemDesc));

                    slot_widget
                        .filled_slot(quality_col_img)
                        .with_item_tooltip(
                            self.item_tooltip_manager,
                            salvage_items,
                            &prices_info,
                            self.item_tooltip,
                        )
                        .set(state.ids.item_slots[i], ui);
                } else {
                    slot_widget
                        .filled_slot(quality_col_img)
                        .with_item_tooltip(
                            self.item_tooltip_manager,
                            core::iter::once(item as &dyn ItemDesc),
                            &prices_info,
                            self.item_tooltip,
                        )
                        .set(state.ids.item_slots[i], ui);
                }
                if self.details_mode {
                    let (name, _) = util::item_text(item, self.localized_strings, self.item_i18n);
                    // TODO: text is not aligned with list mode icons, need to fix
                    Text::new(&name)
                        .top_left_with_margins_on(
                            id,
                            0.0 + y as f64 * self.slot_size,
                            30.0 + x as f64 * self.slot_size,
                        )
                        .font_id(self.fonts.cyri.conrod_id)
                        .font_size(self.fonts.cyri.scale(14))
                        .color(color::WHITE)
                        .set(state.ids.inv_slot_names[i], ui);

                    let col = self.columns;
                    let size = self.columns;
                    let space = self.spacing as usize;
                    let current_width = ((col * size) + ((col - 1) * space)) as f64;
                    Text::new(&format!("{}", item.amount()))
                        .top_left_with_margins_on(
                            id,
                            0.0 + y as f64 * self.slot_size,
                            current_width - 40.0_f64 * self.slot_size,
                        )
                        .font_id(self.fonts.cyri.conrod_id)
                        .font_size(self.fonts.cyri.scale(14))
                        .color(color::WHITE)
                        .set(state.ids.inv_slot_amounts[i], ui);
                }
            } else {
                slot_widget.set(state.ids.item_slots[i], ui);
            }

            // Record the position and details of any selected slot
            if selected == Some(inv_slot.into()) {
                state.update(|s| {
                    s.active_context_slot = selected;
                    let menu_width = 130.0;
                    let offset = if x < self.columns / 2 {
                        self.slot_size // Place to the right
                    } else {
                        -menu_width // Place to the left
                    };
                    s.context_menu_pos = [x_pos + offset, y_pos];
                });
            }
        }

        // Add padding beneath the last row of items to make scrolling feel more natural
        if !state.ids.item_slots.is_empty() {
            Rectangle::fill_with([1.0, 15.0], color::TRANSPARENT)
                .down_from(state.ids.item_slots[state.ids.item_slots.len() - 1], 0.0)
                .set(state.ids.spacing_below, ui);
        }

        // Not exactly an event, but I have to return the filtered list size somehow
        events.push(SlotEvents::FilteredSize(total_slots));

        // Open context menu if any slot is selected
        if state.active_context_slot.is_some() {
            let context_use = self.localized_strings.get_msg("hud-context-menu-use");
            // add `Move` context action
            // add `Split` context action
            let context_drop = self.localized_strings.get_msg("hud-context-menu-drop");
            let context_cancel = self.localized_strings.get_msg("hud-context-menu-cancel");

            let actions = [context_use, context_drop, context_cancel];
            // TODO: instead of storing [x,y] coordinates, consider storing the widget id
            let [x, y] = state.context_menu_pos;
            let total_h = (actions.len() as f64 * 25.0) + ((actions.len() as f64 + 1.0) * 2.0);

            let event = ContextMenu::new(
                &actions,
                self.fonts,
                self.imgs,
                self.menu_events,
                self.last_input,
            )
            .top_left_with_margins_on(id, y, x)
            .w_h(130.0, total_h)
            .set(state.ids.context_menu, ui);

            if let Some(index) = event {
                match index {
                    0 => self.slot_manager.use_selected(),
                    1 => self.slot_manager.dropped_selected(),
                    2 => self.slot_manager.idle(),
                    _ => self.slot_manager.idle(),
                }

                state.update(|s| s.active_context_slot = None);
            }
        }

        events
    }
}

#[derive(WidgetCommon)]
struct ContextMenu<'a, T: 'a + AsRef<str>> {
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    actions: &'a [T],
    fonts: &'a Fonts,
    imgs: &'a Imgs,
    menu_events: &'a Vec<MenuInput>,
    last_input: &'a LastInput,
}

widget_ids! {
    struct ContextMenuIds {
        bg,
        buttons[],
    }
}

struct ContextState {
    ids: ContextMenuIds,

    active_slot: usize,
}

impl<'a, T: AsRef<str>> ContextMenu<'a, T> {
    fn new(
        actions: &'a [T],
        fonts: &'a Fonts,
        imgs: &'a Imgs,
        menu_events: &'a Vec<MenuInput>,
        last_input: &'a LastInput,
    ) -> Self {
        ContextMenu {
            common: widget::CommonBuilder::default(),
            actions,
            fonts,
            imgs,
            menu_events,
            last_input,
        }
    }
}

impl<'a, T: AsRef<str>> Widget for ContextMenu<'a, T> {
    type Event = Option<usize>;
    type State = ContextState;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        ContextState {
            ids: ContextMenuIds::new(id_gen),
            active_slot: 0,
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs {
            id,
            state,
            ui,
            rect,
            ..
        } = args;
        let mut clicked_index = None;

        let item_h = 25.0;
        let spacing = 2.0;
        let actions_len = self.actions.len();

        // MENU INPUTS: navigate up and down the list
        // Up: go up an item (wrap?)
        // Down: go down an item (wrap?)
        // Apply: select the current list item
        // Back: close the context menu
        let mut clicked = false;
        for event in self.menu_events {
            match *event {
                MenuInput::Up => state.update(|s| {
                    let y = s.active_slot;
                    if y > 0 {
                        s.active_slot = y - 1;
                    }
                }),
                MenuInput::Down => state.update(|s| {
                    let y = s.active_slot;
                    if y < actions_len - 1 {
                        s.active_slot = y + 1;
                    }
                }),
                MenuInput::Apply => {
                    clicked = true;
                },
                MenuInput::Back => {
                    // Assume the last selection is `Close` for now
                    clicked_index = Some(actions_len - 1);
                },
                _ => {},
            }
        }

        // Draw background
        Rectangle::fill_with(rect.dim(), Color::Rgba(0.2, 0.2, 0.2, 0.99))
            .middle_of(id)
            .set(state.ids.bg, ui);

        if state.ids.buttons.len() < actions_len {
            state.update(|s| {
                s.ids
                    .buttons
                    .resize(actions_len, &mut ui.widget_id_generator());
            });
        }

        // Position buttons
        for (i, label) in self.actions.iter().enumerate() {
            let btn_id = state.ids.buttons[i];
            let active_btn = state.active_slot == i;
            let btn = Button::image(self.imgs.nothing)
                .color(color::BLACK)
                .border(20.0)
                .border_color(
                    if active_btn && (*self.last_input == LastInput::Keyboard || *self.last_input == LastInput::Controller) {
                        color::YELLOW
                    } else {
                        color::WHITE
                    }
                )
                .label(label.as_ref())
                .label_font_size(self.fonts.cyri.scale(12))
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_color(
                    if active_btn && (*self.last_input == LastInput::Keyboard || *self.last_input == LastInput::Controller) {
                        color::YELLOW
                    } else {
                        color::WHITE
                    }
                )
                .hover_image(self.imgs.selection_hover) // Puts a border around the button
                .press_image(self.imgs.selection_press)
                .image_color(color::rgba(1.0, 0.82, 0.27, 1.0))
                .h(item_h)
                .w(rect.w() - (spacing * 2.0))
                .parent(state.ids.bg);

            let placed_btn = if i == 0 {
                btn.mid_top_with_margin_on(state.ids.bg, spacing)
            } else {
                btn.down_from(state.ids.buttons[i - 1], spacing)
            };

            if placed_btn.set(btn_id, ui).was_clicked() || (clicked && active_btn) {
                clicked_index = Some(i);
            }
        }

        clicked_index
    }
}
