use conrod_core::{
    color,
    widget::{self, Button, Image, Rectangle, Text},
    widget_ids, Colorable, Positionable, Sizeable, Widget, WidgetCommon,
};

use super::{img_ids::Imgs, Fonts, Show, TEXT_COLOR};
use client::{self, Client};

use std::{cell::RefCell, rc::Rc};

widget_ids! {
    struct Ids {
        mmap_frame,
        mmap_frame_bg,
        mmap_location,
        mmap_button,
    }
}

#[derive(WidgetCommon)]
pub struct MiniMap<'a> {
    show: &'a Show,

    client: &'a Client,

    imgs: &'a Imgs,
    fonts: &'a Fonts,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> MiniMap<'a> {
    pub fn new(show: &'a Show, client: &'a Client, imgs: &'a Imgs, fonts: &'a Fonts) -> Self {
        Self {
            show,
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

pub enum Event {
    Toggle,
}

impl<'a> Widget for MiniMap<'a> {
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

        if self.show.mini_map {
            Image::new(self.imgs.mmap_frame)
                .w_h(100.0 * 2.0, 100.0 * 2.0)
                .top_right_with_margins_on(ui.window, 5.0, 5.0)
                .set(state.ids.mmap_frame, ui);

            Rectangle::fill_with([92.0 * 2.0, 82.0 * 2.0], color::TRANSPARENT)
                .mid_top_with_margin_on(state.ids.mmap_frame, 13.0 * 2.0 + 2.0)
                .set(state.ids.mmap_frame_bg, ui);
        } else {
            Image::new(self.imgs.mmap_frame_closed)
                .w_h(100.0 * 2.0, 11.0 * 2.0)
                .top_right_with_margins_on(ui.window, 5.0, 5.0)
                .set(state.ids.mmap_frame, ui);
        }

        if Button::image(if self.show.mini_map {
            self.imgs.mmap_open
        } else {
            self.imgs.mmap_closed
        })
        .w_h(100.0 * 0.2, 100.0 * 0.2)
        .hover_image(if self.show.mini_map {
            self.imgs.mmap_open_hover
        } else {
            self.imgs.mmap_closed_hover
        })
        .press_image(if self.show.mini_map {
            self.imgs.mmap_open_press
        } else {
            self.imgs.mmap_closed_press
        })
        .top_right_with_margins_on(state.ids.mmap_frame, 0.0, 0.0)
        .set(state.ids.mmap_button, ui)
        .was_clicked()
        {
            return Some(Event::Toggle);
        }

        // Title
        match self.client.current_chunk() {
            Some(chunk) => Text::new(chunk.meta().name())
                .mid_top_with_margin_on(state.ids.mmap_frame, 3.0)
                .font_size(14)
                .color(TEXT_COLOR)
                .set(state.ids.mmap_location, ui),
            None => Text::new(" ")
                .mid_top_with_margin_on(state.ids.mmap_frame, 3.0)
                .font_size(14)
                .color(TEXT_COLOR)
                .set(state.ids.mmap_location, ui),
        }

        None
    }
}
