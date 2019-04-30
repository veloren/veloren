use conrod_core::{
    builder_methods, color,
    text::font,
    widget::{self, Button, Image, Rectangle, Text},
    widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};
use super::{
    img_ids::Imgs,
    font_ids::Fonts,
    TEXT_COLOR,
};

widget_ids! {
    struct Ids {
        map_bg,
        map_close,
        map_frame,
        map_frame_l,
        map_frame_r,
        map_icon,
        map_title,
    }
}

#[derive(WidgetCommon)]
pub struct Map<'a> {
    imgs: &'a Imgs,
    fonts: &'a Fonts,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    style: (),
}

impl<'a> Map<'a> {
    pub fn new(imgs: &'a Imgs, fonts: &'a Fonts) -> Self {
        Self {
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

impl<'a> Widget for Map<'a> {
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

        // BG
        Image::new(self.imgs.map_bg)
            .w_h(824.0, 488.0)
            .middle_of(ui.window)
            .set(state.ids.map_bg, ui);

        // Frame
        Image::new(self.imgs.map_frame_l)
            .top_left_with_margins_on(state.ids.map_bg, 0.0, 0.0)
            .w_h(412.0, 488.0)
            .set(state.ids.map_frame_l, ui);

        Image::new(self.imgs.map_frame_r)
            .top_right_with_margins_on(state.ids.map_bg, 0.0, 0.0)
            .w_h(1648.0 / 4.0, 1952.0 / 4.0)
            .set(state.ids.map_frame_r, ui);

        // Icon
        Image::new(self.imgs.map_icon)
            .w_h(224.0 / 3.0, 224.0 / 3.0)
            .top_left_with_margins_on(state.ids.map_frame, -10.0, -10.0)
            .set(state.ids.map_icon, ui);

        // X-Button
        if Button::image(self.imgs.close_button)
            .w_h(4.0*2.0, 4.0*2.0)
            .hover_image(self.imgs.close_button_hover)
            .press_image(self.imgs.close_button_press)
            .top_right_with_margins_on(state.ids.map_frame_r, 1.0, 1.0)
            .set(state.ids.map_close, ui)
            .was_clicked()
        {
            return Some(Event::Close);
        }

        // Title
        Text::new("Map")
            .mid_top_with_margin_on(state.ids.map_bg, -7.0)
            .font_size(50)
            .color(TEXT_COLOR)
            .set(state.ids.map_title, ui);

        None
    }
}
