use conrod_core::{
    builder_methods, image, text,
    widget::{self, button},
    widget_ids, Color, FontSize, Positionable, Rect, Sizeable, Widget, WidgetCommon,
};

#[derive(Clone, WidgetCommon)]
pub struct RadioList<'a, T> {
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    f_image: button::Image,
    t_image: button::Image,
    selected: usize,
    options_labels: &'a [(&'a T, &'a str)],
    label_style: widget::text::Style,
    label_spacing: f64,
    button_spacing: [f64; 2],
    button_dims: [f64; 2],
}

widget_ids! {
    struct Ids {
        buttons[],
        labels[],
    }
}

pub struct State {
    ids: Ids,
}

impl<'a, T> RadioList<'a, T> {
    builder_methods! {
        pub text_color { label_style.color = Some(Color) }
        pub font_size { label_style.font_size = Some(FontSize) }
        pub justify { label_style.justify = Some(text::Justify) }
        pub line_spacing { label_style.line_spacing = Some(f64) }
        pub label_spacing { label_spacing = f64 }
        pub button_spacing { button_spacing = [f64; 2] }
        pub button_dims { button_dims = [f64; 2] }
    }

    pub fn new(
        selected: usize,
        f_image_id: image::Id,
        t_image_id: image::Id,
        options_labels: &'a [(&'a T, &'a str)],
    ) -> Self {
        Self {
            common: widget::CommonBuilder::default(),
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
            selected,
            label_style: widget::text::Style::default(),
            options_labels,
            label_spacing: 10.0,
            button_spacing: [5.0, 5.0],
            button_dims: [15.0, 15.0],
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

impl<'a, T> Widget for RadioList<'a, T> {
    type Event = Option<(usize, &'a T)>;
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
        let Self {
            f_image,
            t_image,
            selected,
            options_labels,
            label_style,
            label_spacing,
            button_spacing,
            button_dims,
            ..
        } = self;

        // Ensure we have enough widget ids
        let num_items = options_labels.len();
        if state.ids.buttons.len() < num_items || state.ids.labels.len() < num_items {
            state.update(|s| {
                s.ids
                    .buttons
                    .resize(num_items, &mut ui.widget_id_generator());
                s.ids
                    .labels
                    .resize(num_items, &mut ui.widget_id_generator());
            });
        }

        // Check if the button was clicked.
        // (Can't use `.set().was_clicked()` because we are changing the image after
        // setting the widget, which causes flickering since it takes a frame to
        // change after the mouse button is lifted).
        let current_selection = (0..num_items)
            .find(|i| {
                ui.widget_input(state.ids.buttons[*i])
                    .clicks()
                    .left()
                    .count()
                    % 2
                    == 1
            })
            .unwrap_or(selected);

        let (x, y, w, h) = rect.x_y_w_h();
        for i in 0..num_items {
            let image = if i == current_selection {
                t_image
            } else {
                f_image
            };
            // Button
            let mut button = button::Button::image(image.image_id)
                .wh(button_dims)
                //TODO: implement default width / height functions
                .x_y(
                    x - w / 2.0 + button_spacing[0],
                    y - h / 2.0
                        - i as f64 * (button_dims[1] + button_spacing[1])
                        - button_spacing[1],
                )
                .parent(id);
            button.show = image;
            button.set(state.ids.buttons[i], ui);
            // Label
            widget::Text::new(options_labels[i].1)
                .graphics_for(state.ids.buttons[i])
                .parent(id)
                .with_style(label_style)
                .right_from(state.ids.buttons[i], label_spacing)
                .set(state.ids.labels[i], ui);
        }

        if current_selection != selected {
            Some((current_selection, options_labels[current_selection].0))
        } else {
            None
        }
    }
}
