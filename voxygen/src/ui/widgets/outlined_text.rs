use conrod_core::{
    builder_methods, text,
    widget::{self, Text},
    widget_ids, Color, FontSize, Positionable, Sizeable, Widget, WidgetCommon,
};

#[derive(Clone, WidgetCommon)]
pub struct OutlinedText<'a> {
    #[conrod(common_builder)]
    common: widget::CommonBuilder,

    text: &'a str,
    text_style: widget::text::Style,
    outline_color: Option<Color>,
    outline_width: f64,
}

widget_ids! {
    struct Ids{
        base,
        outline1,
        outline2,
        outline3,
        outline4,
    }
}

pub struct State {
    ids: Ids,
}

impl<'a> OutlinedText<'a> {
    builder_methods! {
        pub color {text_style.color = Some(Color)}
        pub outline_color {outline_color = Some(Color)}

        pub font_size {text_style.font_size = Some(FontSize)}
        pub outline_width {outline_width = f64}
    }

    pub fn new(text: &'a str) -> Self {
        Self {
            common: widget::CommonBuilder::default(),
            text,

            text_style: widget::text::Style::default(),
            outline_color: None,
            outline_width: 0.0,
        }
    }

    #[must_use]
    pub fn font_id(mut self, font_id: text::font::Id) -> Self {
        self.text_style.font_id = Some(Some(font_id));
        self
    }
}

impl<'a> Widget for OutlinedText<'a> {
    type Event = ();
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
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

        let mut outline_style = self.text_style;
        outline_style.color = self.outline_color;

        let shift = self.outline_width;
        Text::new(self.text)
            .with_style(self.text_style)
            .xy(rect.xy())
            .wh(rect.dim())
            .parent(id)
            .depth(-1.0)
            .set(state.ids.base, ui);

        Text::new(self.text)
            .with_style(outline_style)
            .x_y_relative_to(state.ids.base, shift, shift)
            .wh_of(state.ids.base)
            .set(state.ids.outline1, ui);

        Text::new(self.text)
            .with_style(outline_style)
            .x_y_relative_to(state.ids.base, -shift, shift)
            .wh_of(state.ids.base)
            .set(state.ids.outline2, ui);

        Text::new(self.text)
            .with_style(outline_style)
            .x_y_relative_to(state.ids.base, shift, -shift)
            .wh_of(state.ids.base)
            .set(state.ids.outline3, ui);

        Text::new(self.text)
            .with_style(outline_style)
            .x_y_relative_to(state.ids.base, -shift, -shift)
            .wh_of(state.ids.base)
            .set(state.ids.outline4, ui);
    }
}
