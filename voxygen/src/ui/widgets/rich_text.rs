use crate::hud::{controller_icons as icon_utils, img_ids::Imgs};
use conrod_core::{
    Color, FontSize, Positionable, Sizeable, Ui, Widget, WidgetCommon, builder_methods, image,
    position::Dimension, text, widget, widget_ids,
};
use regex::Regex;
use std::sync::LazyLock;

// represents a piece of the rich text flow
enum TextSegment<'a> {
    Text(&'a str),
    Image(image::Id), // font size is [w, h]
    Newline,
}

pub struct State {
    ids: Ids,
}

widget_ids! {
    struct Ids {
        text_ids[],
        image_ids[],
    }
}

/// a widget for rendering text with inline images/icons
///
/// `RichText` automatically parses strings for tags (e.g., `:south:`) and
/// replaces them with the corresponding icons
#[derive(WidgetCommon)]
pub struct RichText<'a> {
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    segments: Vec<TextSegment<'a>>,
    text_segments: usize,
    image_segments: usize,
    style: widget::text::Style,
    font_id: Option<text::font::Id>,
    line_spacing: f64,
}

impl<'a> RichText<'a> {
    builder_methods! {
        pub color { style.color = Some(Color) }
        pub font_size { style.font_size = Some(FontSize) }
        pub font_id { font_id = Some(text::font::Id) }
        pub line_spacing { line_spacing = f64 }
        pub justify { style.justify = Some(text::Justify) }
    }

    /// creates a new `RichText` widget
    ///
    /// # arguments
    /// * `string` - the text to display. Use tags like `:name:` to insert
    ///   images
    /// * `imgs` - Imgs object to fetch conrod image ids
    pub fn new(string: &'a str, imgs: &'a Imgs) -> Self {
        let (segments, text_segments, image_segments) = Self::parse(string, imgs);

        RichText {
            common: widget::CommonBuilder::default(),
            segments,
            text_segments,
            image_segments,
            style: widget::text::Style::default(),
            font_id: None,
            line_spacing: 5.0,
        }
    }

    // do a forward pass through the input to pre-process it
    // TODO: auto add a "\n" character if input goes passed max defined width
    fn parse(input: &'a str, imgs: &Imgs) -> (Vec<TextSegment<'a>>, usize, usize) {
        // a magical incantation that splits strings by double colons (e.g., ":icon:")
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r":(?P<tag>[^:\s]+):").expect("Invalid Regex"));
        let mut segments = Vec::new();
        let mut text_segments = 0;
        let mut image_segments = 0;
        let mut last_end = 0;

        for check in RE.captures_iter(input) {
            let whole = check.get(0).unwrap();
            let tag = &check["tag"];

            // push the text before the icon
            if whole.start() > last_end {
                let text_strip = &input[last_end..whole.start()];
                // handle newlines
                for (i, line) in text_strip.split('\n').enumerate() {
                    if i > 0 {
                        segments.push(TextSegment::Newline);
                    }
                    if !line.is_empty() {
                        segments.push(TextSegment::Text(line));
                        text_segments += 1;
                    }
                }
            }

            // add icon to output
            let img_id = icon_utils::get_controller_icon_id_from_string(tag, imgs);

            segments.push(TextSegment::Image(img_id));
            image_segments += 1;

            last_end = whole.end();
        }

        // push trailing text
        if last_end < input.len() {
            let trailing = &input[last_end..];
            for (i, line) in trailing.split('\n').enumerate() {
                if i > 0 {
                    segments.push(TextSegment::Newline);
                }
                if !line.is_empty() {
                    segments.push(TextSegment::Text(line));
                    text_segments += 1;
                }
            }
        }

        (segments, text_segments, image_segments)
    }

    // helper to calculate the total bounding box of all segments.
    fn calculate_dimensions(&self, ui: &Ui) -> [f64; 2] {
        let font_id = self.font_id.or(ui.theme.font_id).expect("No font provided");
        let font = ui.fonts.get(font_id).expect("Font not found");
        let font_size = self.style.font_size(&ui.theme);
        let line_height = font_size as f64 + self.line_spacing;

        let mut max_w: f64 = 0.0;
        let mut current_w: f64 = 0.0;
        let mut total_h: f64 = line_height;

        for segment in &self.segments {
            match segment {
                TextSegment::Text(s) => {
                    current_w += text::line::width(s, font, font_size);
                },
                TextSegment::Image(_) => {
                    current_w += font_size as f64;
                },
                TextSegment::Newline => {
                    max_w = max_w.max(current_w);
                    current_w = 0.0;
                    total_h += line_height;
                },
            }
        }
        [max_w.max(current_w), total_h]
    }
}

impl<'a> Widget for RichText<'a> {
    type Event = ();
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {}

    fn default_x_dimension(&self, ui: &Ui) -> Dimension {
        let [w, _h] = self.calculate_dimensions(ui);
        Dimension::Absolute(w)
    }

    fn default_y_dimension(&self, ui: &Ui) -> Dimension {
        let [_w, h] = self.calculate_dimensions(ui);
        Dimension::Absolute(h)
    }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs {
            id,
            state,
            ui,
            rect,
            ..
        } = args;

        let font_size = self.style.font_size(ui.theme());
        let line_height = font_size as f64 + self.line_spacing;
        let font_id = self.font_id.or(ui.theme.font_id).expect("No font provided");

        let available_width = rect.w();
        let justify = self.style.justify(ui.theme());

        // calculate the line widths on a line-by-line basis for proper text alignment
        let line_widths: Vec<f64> = {
            let font = ui.fonts.get(font_id).expect("Font not found"); // borrows ui
            let mut current_width = 0.0;
            let mut widths = Vec::new();

            for segment in &self.segments {
                match segment {
                    TextSegment::Text(s) => {
                        current_width += text::line::width(s, font, font_size);
                    },
                    TextSegment::Image(_) => {
                        current_width += font_size as f64;
                    },
                    TextSegment::Newline => {
                        widths.push(current_width);
                        current_width = 0.0;
                    },
                }
            }

            widths.push(current_width);
            widths
        };

        // dynamically update number of widgets and use the appropriate conrod primitive
        state.update(|s| {
            s.ids
                .text_ids
                .resize(self.text_segments, &mut ui.widget_id_generator());
            s.ids
                .image_ids
                .resize(self.image_segments, &mut ui.widget_id_generator());
        });

        let mut y_cursor = 0.0;
        let mut text_idx = 0;
        let mut image_idx = 0;
        let mut line_idx = 0;

        let mut x_cursor = match justify {
            text::Justify::Left => 0.0,
            text::Justify::Right => available_width - line_widths[line_idx],
            text::Justify::Center => (available_width - line_widths[line_idx]) / 2.0,
        };

        for segment in &self.segments {
            match segment {
                TextSegment::Text(string) => {
                    if string.is_empty() {
                        continue;
                    }

                    let text_width = {
                        let font = ui.fonts.get(font_id).expect("Font not found");
                        text::line::width(string, font, font_size)
                    };

                    widget::Text::new(string)
                        .with_style(self.style)
                        .font_id(font_id)
                        .parent(id)
                        .graphics_for(id)
                        .top_left_with_margins_on(id, y_cursor, x_cursor)
                        .set(state.ids.text_ids[text_idx], ui);

                    x_cursor += text_width;
                    text_idx += 1;
                },
                TextSegment::Image(image_id) => {
                    let image_size = font_size as f64;
                    // should this not be hardcoded to 1.5? It seems to properly align images
                    // regardless of font_size
                    let v_offset = 1.5;

                    // not sure if I like coloring icons with text, but I'll leave it for now
                    // opacity value is important though
                    let color = self.style.color.unwrap_or(Color::Rgba(1.0, 1.0, 1.0, 1.0));
                    widget::Image::new(*image_id)
                        .wh([image_size, image_size])
                        .parent(id)
                        .graphics_for(id)
                        .color(Some(color))
                        .top_left_with_margins_on(id, y_cursor + v_offset, x_cursor)
                        .set(state.ids.image_ids[image_idx], ui);

                    x_cursor += image_size;
                    image_idx += 1;
                },
                TextSegment::Newline => {
                    line_idx += 1;
                    x_cursor = match justify {
                        text::Justify::Left => 0.0,
                        text::Justify::Right => available_width - line_widths[line_idx],
                        text::Justify::Center => (available_width - line_widths[line_idx]) / 2.0,
                    };
                    y_cursor += line_height;
                },
            }
        }
    }
}
