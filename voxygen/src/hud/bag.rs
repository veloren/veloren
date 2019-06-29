use super::{img_ids::Imgs, Fonts};
use conrod_core::{
    color,
    widget::{self, Button, Image, Rectangle, Scrollbar},
    widget_ids, Colorable, Positionable, Sizeable, Widget, WidgetCommon,
};

widget_ids! {
    struct Ids {
        bag_close,
        bag_contents,
        inv_alignment,
        inv_grid_1,
        inv_grid_2,
        inv_scrollbar,
        inv_slot_0,
        map_title,
        inv_slot[],
    }
}

#[derive(WidgetCommon)]
pub struct Bag<'a> {
    inventory_space: usize,

    imgs: &'a Imgs,
    _fonts: &'a Fonts,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> Bag<'a> {
    pub fn new(inventory_space: usize, imgs: &'a Imgs, fonts: &'a Fonts) -> Self {
        Self {
            inventory_space,
            imgs,
            _fonts,
            common: widget::CommonBuilder::default(),
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
        let widget::UpdateArgs { state, ui, .. } = args;

        // Contents
        Image::new(self.imgs.bag_contents)
            .w_h(68.0 * 4.0, 123.0 * 4.0)
            .bottom_right_with_margins_on(ui.window, 60.0, 5.0)
            .set(state.ids.bag_contents, ui);

        // Alignment for Grid
        Rectangle::fill_with([58.0 * 4.0 - 5.0, 100.0 * 4.0], color::TRANSPARENT)
            .top_left_with_margins_on(state.ids.bag_contents, 11.0 * 4.0, 5.0 * 4.0)
            .scroll_kids()
            .scroll_kids_vertically()
            .set(state.ids.inv_alignment, ui);
        // Grid
        Image::new(self.imgs.inv_grid)
            .w_h(58.0 * 4.0, 111.0 * 4.0)
            .mid_top_with_margin_on(state.ids.inv_alignment, 0.0)
            .set(state.ids.inv_grid_1, ui);
        Image::new(self.imgs.inv_grid)
            .w_h(58.0 * 4.0, 111.0 * 4.0)
            .mid_top_with_margin_on(state.ids.inv_alignment, 110.0 * 4.0)
            .set(state.ids.inv_grid_2, ui);
        Scrollbar::y_axis(state.ids.inv_alignment)
            .thickness(5.0)
            .rgba(0.33, 0.33, 0.33, 1.0)
            .set(state.ids.inv_scrollbar, ui);
        // Create available inventory slot widgets         
        dbg!(self.inventory_space);
       if state.ids.inv_slot.len() < self.inventory_space {
        state.update(|s| { s.ids.inv_slot.resize(self.inventory_space, &mut ui.widget_id_generator());});}
        for i in 0..self.inventory_space {
            let x = i % 5;
            let y = i / 5; 
            Button::image(self.imgs.inv_slot)
                .top_left_with_margins_on(state.ids.inv_grid_1, 4.0 + y as f64 * (40.0 + 4.0), 4.0 + x as f64 * (40.0 + 4.0))          
                .w_h(40.0, 40.0)          
                .set(state.ids.inv_slot[i], ui);
            }      
        // X-button
        if Button::image(self.imgs.close_button)
            .w_h(28.0, 28.0)
            .hover_image(self.imgs.close_button_hover)
            .press_image(self.imgs.close_button_press)
            .top_right_with_margins_on(state.ids.bag_contents, 0.0, 0.0)
            .set(state.ids.bag_close, ui)
            .was_clicked()
        {
            Some(Event::Close)
        } else {
            None
        }
    }
}
