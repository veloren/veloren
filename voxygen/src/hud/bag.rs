use conrod_core::{
    builder_methods, color,
    text::font,
    widget::{self, Button, Image, Rectangle, Text, Scrollbar},
    widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};
use super::{
    img_ids::Imgs,
    font_ids::Fonts,
    TEXT_COLOR,
};

widget_ids! {
    struct Ids {
        bag_close,
        bag_contents,
        inv_alignment,
        inv_grid,
        inv_scrollbar,
        inv_slot_0,
        map_title,
    }
}

#[derive(WidgetCommon)]
pub struct Bag<'a> {
    inventory_space: u32,

    imgs: &'a Imgs,
    fonts: &'a Fonts,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    style: (),
}

impl<'a> Bag<'a> {
    pub fn new(inventory_space: u32, imgs: &'a Imgs, fonts: &'a Fonts) -> Self {
        Self {
            inventory_space,
            imgs,
            fonts,
            common: widget::CommonBuilder::default(),
            style: (),
        }
    }
}

pub struct State {
    ids: Ids,
}

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
        let widget::UpdateArgs {
            id,
            state,
            ui,
            style,
            ..
        } = args;

        // Contents
        Image::new(self.imgs.bag_contents)
            .w_h(307.0, 545.0)
            .bottom_right_with_margins_on(ui.window, 90.0, 5.0)
            .set(state.ids.bag_contents, ui);

        // Alignment for Grid
        Rectangle::fill_with([246.0, 465.0], color::TRANSPARENT)
            .top_left_with_margins_on(state.ids.bag_contents, 27.0, 23.0)
            .scroll_kids()
            .scroll_kids_vertically()
            .set(state.ids.inv_alignment, ui);
        // Grid
        Image::new(self.imgs.inv_grid)
            .w_h(232.0, 1104.0)
            .mid_top_with_margin_on(state.ids.inv_alignment, 0.0)
            .set(state.ids.inv_grid, ui);
        Scrollbar::y_axis(state.ids.inv_alignment)
            .thickness(5.0)
            .rgba(0.86, 0.86, 0.86, 0.1)
            .set(state.ids.inv_scrollbar, ui);

        // X-button
        if Button::image(self.imgs.close_button)
            .w_h(244.0 * 0.22 / 3.0, 244.0 * 0.22 / 3.0)
            .hover_image(self.imgs.close_button_hover)
            .press_image(self.imgs.close_button_press)
            .top_right_with_margins_on(state.ids.bag_contents, 5.0, 17.0)
            .set(state.ids.bag_close, ui)
            .was_clicked()
        {
            return Some(Event::Close);
        }

        if self.inventory_space > 0 {
            // First Slot
            Button::image(self.imgs.inv_slot)
                .top_left_with_margins_on(state.ids.inv_grid, 5.0, 5.0)
                .w_h(40.0, 40.0)
                .set(state.ids.inv_slot_0, ui);
        }

        None
    }
}
