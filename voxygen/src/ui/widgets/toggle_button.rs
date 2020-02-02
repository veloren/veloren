use conrod_core::{
    image,
    widget::{self, button},
    widget_ids, Color, Positionable, Rect, Sizeable, Widget, WidgetCommon,
};

#[derive(Clone, WidgetCommon)]
pub struct ToggleButton {
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    value: bool,
    f_image: button::Image,
    t_image: button::Image,
}

widget_ids! {
    struct Ids {
        button,
    }
}

pub struct State {
    ids: Ids,
}

impl ToggleButton {
    pub fn new(value: bool, f_image_id: image::Id, t_image_id: image::Id) -> Self {
        ToggleButton {
            common: widget::CommonBuilder::default(),
            value,
            f_image: button::Image {
                image_id: f_image_id,
                hover_image_id: None,
                press_image_id: None,
                src_rect: None,
                color: button::ImageColor::None,
            },
            t_image: button::Image {
                image_id: t_image_id,
                hover_image_id: None,
                press_image_id: None,
                src_rect: None,
                color: button::ImageColor::None,
            },
        }
    }

    pub fn source_rectangle(mut self, rect: Rect) -> Self {
        self.f_image.src_rect = Some(rect);
        self.t_image.src_rect = Some(rect);
        self
    }

    pub fn image_colors(mut self, f_color: Color, t_color: Color) -> Self {
        self.f_image.color = button::ImageColor::Normal(f_color);
        self.t_image.color = button::ImageColor::Normal(t_color);
        self
    }

    pub fn image_color_with_feedback(mut self, f_color: Color, t_color: Color) -> Self {
        self.f_image.color = button::ImageColor::WithFeedback(f_color);
        self.t_image.color = button::ImageColor::WithFeedback(t_color);
        self
    }

    pub fn hover_images(mut self, f_id: image::Id, t_id: image::Id) -> Self {
        self.f_image.hover_image_id = Some(f_id);
        self.t_image.hover_image_id = Some(t_id);
        self
    }

    pub fn press_images(mut self, f_id: image::Id, t_id: image::Id) -> Self {
        self.f_image.press_image_id = Some(f_id);
        self.t_image.press_image_id = Some(t_id);
        self
    }
}

impl Widget for ToggleButton {
    type Event = bool;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style { () }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs {
            id,
            state,
            ui,
            rect,
            ..
        } = args;
        let ToggleButton {
            mut value,
            f_image,
            t_image,
            ..
        } = self;
        // Check if the button was clicked.
        // (Can't use `.set().was_clicked()` because we are changing the image after
        // setting the widget, which causes flickering since it takes a frame to
        // change after the mouse button is lifted).
        if ui.widget_input(state.ids.button).clicks().left().count() % 2 == 1 {
            value = !value;
        }
        let image = if value { t_image } else { f_image };
        let (x, y, w, h) = rect.x_y_w_h();
        // Button
        let mut button = button::Button::image(image.image_id)
            .x_y(x, y)
            .w_h(w, h)
            .parent(id);
        button.show = image;
        button.set(state.ids.button, ui);

        value
    }
}
