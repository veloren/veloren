use super::{
    img_ids::{Imgs, ImgsRot},
    item_imgs::{ItemImgs, ItemKey},
    Event as HudEvent, Fonts, TEXT_COLOR,
};
use crate::ui::{ImageFrame, Tooltip, TooltipManager, Tooltipable};
use client::Client;
use conrod_core::{
    color, image,
    widget::{self, Button, Image, Rectangle /*, Scrollbar*/},
    widget_ids, Color, Positionable, Sizeable, Widget, WidgetCommon,
};

widget_ids! {
    struct Ids {
        bag_close,
        bag_top,
        bag_mid,
        bag_bot,
        inv_alignment,
        inv_grid_1,
        inv_grid_2,
        inv_scrollbar,
        inv_slots_0,
        map_title,
        inv_slots[],
        items[],
        tooltip[],
    }
}

#[derive(WidgetCommon)]
#[allow(dead_code)]
pub struct Bag<'a> {
    client: &'a Client,
    imgs: &'a Imgs,
    item_imgs: &'a ItemImgs,
    fonts: &'a Fonts,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    rot_imgs: &'a ImgsRot,
    tooltip_manager: &'a mut TooltipManager,
}

impl<'a> Bag<'a> {
    pub fn new(
        client: &'a Client,
        imgs: &'a Imgs,
        item_imgs: &'a ItemImgs,
        fonts: &'a Fonts,
        rot_imgs: &'a ImgsRot,
        tooltip_manager: &'a mut TooltipManager,
    ) -> Self {
        Self {
            client,
            imgs,
            item_imgs,
            fonts,
            common: widget::CommonBuilder::default(),
            rot_imgs,
            tooltip_manager,
        }
    }
}

pub struct State {
    ids: Ids,
    img_id_cache: Vec<Option<(ItemKey, image::Id)>>,
    selected_slot: Option<usize>,
}

const BAG_SCALE: f64 = 4.0;

pub enum Event {
    HudEvent(HudEvent),
    Close,
}

impl<'a> Widget for Bag<'a> {
    type State = State;
    type Style = ();
    type Event = Option<Event>;

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
            img_id_cache: Vec::new(),
            selected_slot: None,
        }
    }

    fn style(&self) -> Self::Style {
        ()
    }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;

        let mut event = None;

        let invs = self.client.inventories();
        let inventory = match invs.get(self.client.entity()) {
            Some(inv) => inv,
            None => return None,
        };
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
        .title_font_size(15)
        .parent(ui.window)
        .desc_font_size(12)
        .title_text_color(TEXT_COLOR)
        .desc_text_color(TEXT_COLOR);

        // Bag parts
        Image::new(self.imgs.bag_bot)
            .w_h(58.0 * BAG_SCALE, 9.0 * BAG_SCALE)
            .bottom_right_with_margins_on(ui.window, 60.0, 5.0)
            .set(state.ids.bag_bot, ui);
        let mid_height = ((inventory.len() + 4) / 5) as f64 * 44.0;
        Image::new(self.imgs.bag_mid)
            .w_h(58.0 * BAG_SCALE, mid_height)
            .up_from(state.ids.bag_bot, 0.0)
            .set(state.ids.bag_mid, ui);
        Image::new(self.imgs.bag_top)
            .w_h(58.0 * BAG_SCALE, 9.0 * BAG_SCALE)
            .up_from(state.ids.bag_mid, 0.0)
            .set(state.ids.bag_top, ui);

        // Alignment for Grid
        Rectangle::fill_with([56.0 * BAG_SCALE, mid_height], color::TRANSPARENT)
            .top_left_with_margins_on(state.ids.bag_mid, 0.0, 3.0 * BAG_SCALE)
            .scroll_kids_vertically()
            .set(state.ids.inv_alignment, ui);

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
            let x = i % 5;
            let y = i / 5;

            let is_selected = Some(i) == state.selected_slot;

            // Slot
            let slot_widget = Button::image(if !is_selected {
                self.imgs.inv_slot
            } else {
                self.imgs.inv_slot_sel
            })
            .top_left_with_margins_on(
                state.ids.inv_alignment,
                0.0 + y as f64 * (40.0 + 2.0),
                0.0 + x as f64 * (40.0 + 2.0),
            ) // conrod uses a (y,x) format for placing...
            // (the margin placement functions do this because that is the same order as "top left")
            .w_h(40.0, 40.0)
            .image_color(if is_selected {
                color::WHITE
            } else {
                color::DARK_YELLOW
            });

            let slot_widget_clicked = if let Some(item) = item {
                slot_widget
                    .with_tooltip(
                        self.tooltip_manager,
                        &item.name(),
                        &format!("{}\n{}", item.name(), item.description()),
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
                    }
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
                    }
                })
                .w_h(30.0, 30.0)
                .middle_of(state.ids.inv_slots[i]) // TODO: Items need to be assigned to a certain slot and then placed like in this example
                //.label("5x") // TODO: Quantity goes here...
                //.label_font_id(self.fonts.opensans)
                //.label_font_size(12)
                //.label_x(Relative::Scalar(10.0))
                //.label_y(Relative::Scalar(-10.0))
                //.label_color(TEXT_COLOR)
                //.parent(state.ids.inv_slots[i])
                .graphics_for(state.ids.inv_slots[i])
                .set(state.ids.items[i], ui);
            }
        }

        // Drop selected item
        if let Some(to_drop) = state.selected_slot {
            if ui.widget_input(ui.window).clicks().left().next().is_some() {
                event = Some(Event::HudEvent(HudEvent::DropInventorySlot(to_drop)));
            }
        }

        // Close button
        if Button::image(self.imgs.close_button)
            .w_h(28.0, 28.0)
            .hover_image(self.imgs.close_button_hover)
            .press_image(self.imgs.close_button_press)
            .top_right_with_margins_on(state.ids.bag_top, 0.0, 0.0)
            .set(state.ids.bag_close, ui)
            .was_clicked()
        {
            event = Some(Event::Close);
        }

        event
    }
}
