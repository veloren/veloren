use super::{img_ids::Imgs, Fonts, TEXT_COLOR};
use client::Client;
use conrod_core::{
    color,
    position::Relative,
    widget::{self, Button, Image, Rectangle /*, Scrollbar*/},
    widget_ids, /*Color, Colorable,*/ Labelable, Positionable, Sizeable, Widget, WidgetCommon,
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
        inv_slot_0,
        map_title,
        inv_slot[],
        item1,
    }
}

#[derive(WidgetCommon)]
pub struct Bag<'a> {
    client: &'a Client,
    imgs: &'a Imgs,
    fonts: &'a Fonts,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> Bag<'a> {
    pub fn new(client: &'a Client, imgs: &'a Imgs, fonts: &'a Fonts) -> Self {
        Self {
            client,
            imgs,
            fonts,
            common: widget::CommonBuilder::default(),
        }
    }
}

pub struct State {
    ids: Ids,
}

const BAG_SCALE: f64 = 4.0;

pub enum Event {
    Close,
}

impl<'a> Widget for Bag<'a> {
    type State = State;
    type Style = ();
    type Event = Option<Event>;

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {
        ()
    }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;

        let inventory_slots = self
            .client
            .inventories()
            .get(self.client.entity())
            .map(|inv| inv.slots().len())
            .unwrap_or(0);

        // Bag parts
        Image::new(self.imgs.bag_bot)
            .w_h(61.0 * BAG_SCALE, 9.0 * BAG_SCALE)
            .bottom_right_with_margins_on(ui.window, 60.0, 5.0)
            .set(state.ids.bag_bot, ui);
        Image::new(self.imgs.bag_mid)
            .w_h(61.0 * BAG_SCALE, ((inventory_slots + 4) / 5) as f64 * 44.0)
            .up_from(state.ids.bag_bot, 0.0)
            .set(state.ids.bag_mid, ui);
        Image::new(self.imgs.bag_top)
            .w_h(61.0 * BAG_SCALE, 9.0 * BAG_SCALE)
            .up_from(state.ids.bag_mid, 0.0)
            .set(state.ids.bag_top, ui);

        // Alignment for Grid
        Rectangle::fill_with(
            [54.0 * BAG_SCALE, ((inventory_slots + 4) / 5) as f64 * 44.0],
            color::TRANSPARENT,
        )
        .top_left_with_margins_on(state.ids.bag_top, 9.0 * BAG_SCALE, 3.0 * BAG_SCALE)
        .scroll_kids()
        .scroll_kids_vertically()
        .set(state.ids.inv_alignment, ui);

        // Grid
        /*Image::new(self.imgs.inv_grid)
            .w_h(61.0 * BAG_SCALE, 111.0 * BAG_SCALE)
            .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.5)))
            .mid_top_with_margin_on(state.ids.inv_alignment, 0.0)
            .set(state.ids.inv_grid_1, ui);
        Image::new(self.imgs.inv_grid)
            .w_h(61.0 * BAG_SCALE, 111.0 * BAG_SCALE)
            .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.5)))
            .mid_top_with_margin_on(state.ids.inv_alignment, 110.0 * BAG_SCALE)
            .set(state.ids.inv_grid_2, ui);
        Scrollbar::y_axis(state.ids.inv_alignment)
            .thickness(5.0)
            .rgba(0.33, 0.33, 0.33, 1.0)
            .set(state.ids.inv_scrollbar, ui);*/

        // Create available inventory slot widgets
        if state.ids.inv_slot.len() < inventory_slots {
            state.update(|s| {
                s.ids
                    .inv_slot
                    .resize(inventory_slots, &mut ui.widget_id_generator());
            });
        }
        // "Allowed" max. inventory space should be handled serverside and thus isn't limited in the UI
        for i in 0..inventory_slots {
            let x = i % 5;
            let y = i / 5;
            Button::image(self.imgs.inv_slot)
                .top_left_with_margins_on(
                    state.ids.inv_alignment,
                    4.0 + y as f64 * (40.0 + 4.0),
                    4.0 + x as f64 * (40.0 + 4.0),
                ) // conrod uses a (y,x) format for placing...
                .parent(state.ids.bag_mid) // Avoids the background overlapping available slots
                .w_h(40.0, 40.0)
                .set(state.ids.inv_slot[i], ui);
        }
        // Test Item
        if inventory_slots > 0 {
            Button::image(self.imgs.potion_red) // TODO: Insert variable image depending on the item displayed in that slot
                .w_h(4.0 * 4.4, 7.0 * 4.4) // TODO: Fix height and scale width correctly to that to avoid a stretched item image
                .middle_of(state.ids.inv_slot[0]) // TODO: Items need to be assigned to a certain slot and then placed like in this example
                .label("5x") // TODO: Quantity goes here...
                .label_font_id(self.fonts.opensans)
                .label_font_size(12)
                .label_x(Relative::Scalar(10.0))
                .label_y(Relative::Scalar(-10.0))
                .label_color(TEXT_COLOR)
                .set(state.ids.item1, ui); // TODO: Add widget_id generator for displayed items
        }
        // X-button
        if Button::image(self.imgs.close_button)
            .w_h(28.0, 28.0)
            .hover_image(self.imgs.close_button_hover)
            .press_image(self.imgs.close_button_press)
            .top_right_with_margins_on(state.ids.bag_top, 0.0, 0.0)
            .set(state.ids.bag_close, ui)
            .was_clicked()
        {
            Some(Event::Close)
        } else {
            None
        }
    }
}
