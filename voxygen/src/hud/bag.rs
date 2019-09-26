use super::{
    img_ids::{Imgs, ImgsRot},
    Event as HudEvent, Fonts, TEXT_COLOR, TEXT_COLOR_2,
};
use crate::ui::{ImageFrame, Tooltip, TooltipManager, Tooltipable};
use client::Client;
use conrod_core::{
    color,
    position::Relative,
    widget::{self, Button, Image, Rectangle /*, Scrollbar*/},
    widget_ids, Color, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
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
pub struct Bag<'a> {
    client: &'a Client,
    imgs: &'a Imgs,
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
        fonts: &'a Fonts,
        rot_imgs: &'a ImgsRot,
        tooltip_manager: &'a mut TooltipManager,
    ) -> Self {
        Self {
            client,
            imgs,
            fonts,
            common: widget::CommonBuilder::default(),
            rot_imgs,
            tooltip_manager,
        }
    }
}

pub struct State {
    ids: Ids,
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
        .desc_font_size(10)
        .title_text_color(TEXT_COLOR)
        .desc_text_color(TEXT_COLOR);

        // Bag parts
        Image::new(self.imgs.bag_bot)
            .w_h(61.0 * BAG_SCALE, 9.0 * BAG_SCALE)
            .bottom_right_with_margins_on(ui.window, 60.0, 5.0)
            .set(state.ids.bag_bot, ui);
        Image::new(self.imgs.bag_top)
            .w_h(61.0 * BAG_SCALE, 9.0 * BAG_SCALE)
            .up_from(state.ids.bag_mid, 0.0)
            .set(state.ids.bag_top, ui);
        Image::new(self.imgs.bag_mid)
            .w_h(61.0 * BAG_SCALE, ((inventory.len() + 4) / 5) as f64 * 44.0)
            .up_from(state.ids.bag_bot, 0.0)
            .set(state.ids.bag_mid, ui);

        // Alignment for Grid
        Rectangle::fill_with(
            [54.0 * BAG_SCALE, ((inventory.len() + 4) / 5) as f64 * 44.0],
            color::TRANSPARENT,
        )
        .top_left_with_margins_on(state.ids.bag_top, 9.0 * BAG_SCALE, 3.0 * BAG_SCALE)
        .scroll_kids()
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

        // Display inventory contents

        for (i, item) in inventory.slots().iter().enumerate() {
            let x = i % 5;
            let y = i / 5;

            let is_selected = Some(i) == state.selected_slot;

            // Slot
            let slot_widget = Button::image(self.imgs.inv_slot)
                .top_left_with_margins_on(
                    state.ids.inv_alignment,
                    4.0 + y as f64 * (40.0 + 4.0),
                    4.0 + x as f64 * (40.0 + 4.0),
                ) // conrod uses a (y,x) format for placing...
                .parent(state.ids.inv_alignment) // Avoids the background overlapping available slots
                .w_h(40.0, 40.0)
                .image_color(if is_selected {
                    color::WHITE
                } else {
                    color::DARK_YELLOW
                })
                .floating(true);

            let slot_widget = if let Some(item) = item {
                slot_widget
                    .with_tooltip(
                        self.tooltip_manager,
                        &item.description(),
                        &item.category(),
                        &item_tooltip,
                    )                    
                    .set(state.ids.inv_slots[i], ui)
            } else {
                slot_widget.set(state.ids.inv_slots[i], ui)
            };

            // Item
            if slot_widget.was_clicked() {
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
            if item.is_some() {
                Button::image(self.imgs.potion_red) // TODO: Insert variable image depending on the item displayed in that slot
                    .w_h(4.0 * 4.4, 7.0 * 4.4) // TODO: Fix height and scale width correctly to that to avoid a stretched item image
                    .middle_of(state.ids.inv_slots[i]) // TODO: Items need to be assigned to a certain slot and then placed like in this example
                    .label("5x") // TODO: Quantity goes here...
                    .label_font_id(self.fonts.opensans)
                    .label_font_size(12)
                    .label_x(Relative::Scalar(10.0))
                    .label_y(Relative::Scalar(-10.0))
                    .label_color(TEXT_COLOR)
                    .parent(state.ids.inv_slots[i])
                    .graphics_for(state.ids.inv_slots[i])
                    .set(state.ids.items[i], ui);
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
