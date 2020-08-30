use super::super::{super::FontId, IcedRenderer, Primitive};
use glyph_brush::GlyphCruncher;
use iced::{
    mouse,
    text_input::{self, cursor},
    Color, Point, Rectangle,
};

const CURSOR_WIDTH: f32 = 2.0;
// Extra scroll offset past the cursor
const EXTRA_OFFSET: f32 = 10.0;

impl text_input::Renderer for IcedRenderer {
    type Font = FontId;
    type Style = ();

    fn default_size(&self) -> u16 {
        // TODO: make configurable
        20
    }

    fn measure_value(&self, value: &str, size: u16, font: Self::Font) -> f32 {
        // Using the physical scale might make this cached info usable below?
        // Although we also have a position of the screen there so this could be useless
        let p_scale = self.p_scale;

        let section = glyph_brush::Section {
            screen_position: (0.0, 0.0),
            bounds: (f32::INFINITY, f32::INFINITY),
            layout: Default::default(),
            text: vec![glyph_brush::Text {
                text: value,
                scale: (size as f32 * p_scale).into(),
                font_id: font.0,
                extra: (),
            }],
        };

        let mut glyph_calculator = self.cache.glyph_calculator();
        let width = glyph_calculator
            .glyph_bounds(section)
            .map_or(0.0, |rect| rect.width() / p_scale);

        // glyph_brush ignores the exterior spaces
        // or does it!!!
        // TODO: need better layout lib
        /*let exterior_spaces = value.len() - value.trim().len();

        if exterior_spaces > 0 {
            use glyph_brush::ab_glyph::{Font, ScaleFont};
            // Could cache this if it is slow
            let sur = format!("x{}x", value);
            let section = glyph_brush::Section {
                screen_position: (0.0, 0.0),
                bounds: (f32::INFINITY, f32::INFINITY),
                layout: Default::default(),
                text: vec![glyph_brush::Text {
                    text: &sur,
                    scale: (size as f32 * p_scale).into(),
                    font_id: font.0,
                    extra: (),
                }],
            i;
            let font = glyph_calculator.fonts()[font.0].as_scaled(size as f32);
            let space_id = font.glyph_id(' ');
            let x_id = font.glyph_id('x');
            let space_width = font.h_advance(space_id);
            let x_width = font.h_advance(x_id);
            let kern1 = font.kern(x_id, space_id);
            let kern2 = font.kern(space_id, x_id);
            dbg!(font.kern(x_id, x_id));
            let sur_width = glyph_calculator
                .glyph_bounds(section)
                .map_or(0.0, |rect| rect.width() / p_scale);
            dbg!(space_width);
            dbg!(width);
            dbg!(sur_width);
            let extra = x_width * 2.0 + dbg!(kern1) + dbg!(kern2);
            dbg!(extra);
            dbg!(sur_width - extra);
            width += exterior_spaces as f32 * space_width;
        }*/

        width
    }

    fn offset(
        &self,
        text_bounds: Rectangle,
        font: Self::Font,
        size: u16,
        value: &text_input::Value,
        state: &text_input::State,
    ) -> f32 {
        // Only need to offset if focused with cursor somewhere in the text
        if state.is_focused() {
            let cursor = state.cursor();

            let focus_position = match cursor.state(value) {
                cursor::State::Index(i) => i,
                cursor::State::Selection { end, .. } => end,
            };

            let (_, offset) = measure_cursor_and_scroll_offset(
                self,
                text_bounds,
                value,
                size,
                focus_position,
                font,
            );

            offset
        } else {
            0.0
        }
    }

    fn draw(
        &mut self,
        bounds: Rectangle,
        text_bounds: Rectangle,
        //defaults: &Self::Defaults, No defaults!!
        cursor_position: Point,
        font: Self::Font,
        size: u16,
        placeholder: &str,
        value: &text_input::Value,
        state: &text_input::State,
        _style_sheet: &Self::Style,
    ) -> Self::Output {
        let is_mouse_over = bounds.contains(cursor_position);

        /*
        let style = if state.is_focused() {
            style.focused()
        } else if is_mouse_over {
            style.hovered()
        } else {
            style.active()
        }; */

        let p_scale = self.p_scale;

        // Allocation :(
        let text = value.to_string();
        let text = if !text.is_empty() { Some(&*text) } else { None };

        // TODO: background from style, image?

        // TODO: color from style
        let color = if text.is_some() {
            Color::WHITE
        } else {
            Color::from_rgba(1.0, 1.0, 1.0, 0.3)
        };
        let linear_color = color.into_linear().into();

        let (cursor_primitive, scroll_offset) = if state.is_focused() {
            let cursor = state.cursor();

            let cursor_and_scroll_offset = |position| {
                measure_cursor_and_scroll_offset(self, text_bounds, value, size, position, font)
            };

            let (cursor_primitive, offset) = match cursor.state(value) {
                cursor::State::Index(position) => {
                    let (position, offset) = cursor_and_scroll_offset(position);
                    (
                        Primitive::Rectangle {
                            bounds: Rectangle {
                                x: text_bounds.x + position - CURSOR_WIDTH / p_scale / 2.0,
                                y: text_bounds.y,
                                width: CURSOR_WIDTH / p_scale,
                                height: text_bounds.height,
                            },
                            linear_color,
                        },
                        offset,
                    )
                },
                cursor::State::Selection { start, end } => {
                    let left = start.min(end);
                    let right = end.max(start);

                    let (left_position, left_offset) = cursor_and_scroll_offset(left);
                    let (right_position, right_offset) = cursor_and_scroll_offset(right);

                    let offset = if end == right {
                        right_offset
                    } else {
                        left_offset
                    };
                    let width = right_position - left_position;

                    (
                        Primitive::Rectangle {
                            bounds: Rectangle {
                                x: text_bounds.x + left_position,
                                y: text_bounds.y,
                                width,
                                height: text_bounds.height,
                            },
                            // TODO: selection color from stlye
                            linear_color: Color::from_rgba(1.0, 0.0, 1.0, 0.2).into_linear().into(),
                        },
                        offset,
                    )
                },
            };

            (Some(cursor_primitive), offset)
        } else {
            (None, 0.0)
        };

        let display_text = text.unwrap_or(if state.is_focused() { "" } else { placeholder });
        // Note: clip offset is an integer so we don't have to worry about not
        // accounting for that here where the alignment of the glyphs with
        // pixels affects rasterization
        let glyphs = self.position_glyphs(
            Rectangle {
                width: 1000.0, // hacky
                ..bounds
            },
            iced::HorizontalAlignment::Left,
            iced::VerticalAlignment::Center,
            display_text,
            size,
            font,
        );
        // TODO: delete if new arrangment behaves nicely
        /*let section = glyph_brush::Section {
            screen_position: (text_bounds.x * p_scale, text_bounds.center_y() * p_scale),
            bounds: (
                10000.0, /* text_bounds.width * p_scale */
                text_bounds.height * p_scale,
            ),
            layout: glyph_brush::Layout::SingleLine {
                line_breaker: Default::default(),
                h_align: glyph_brush::HorizontalAlign::Left,
                v_align: glyph_brush::VerticalAlign::Center,
            },
            text: vec![glyph_brush::Text {
                text: display_text,
                scale: (size as f32 * p_scale).into(),
                font_id: font.0,
                extra: (),
            }],
        };*/

        let text_primitive = Primitive::Text {
            glyphs,
            //size: size as f32,
            bounds,
            linear_color,
            /*font,
             *horizontal_alignment,
             *vertical_alignment, */
        };

        let primitive = match cursor_primitive {
            Some(cursor_primitive) => Primitive::Group {
                primitives: vec![cursor_primitive, text_primitive],
            },
            None => text_primitive,
        };

        // Probably already computed this somewhere
        let text_width = self.measure_value(display_text, size, font);

        let primitive = if text_width > text_bounds.width {
            Primitive::Clip {
                bounds: text_bounds,
                offset: (scroll_offset as u32, 0).into(),
                content: Box::new(primitive),
            }
        } else {
            primitive
        };

        (
            primitive,
            if is_mouse_over {
                mouse::Interaction::Text
            } else {
                mouse::Interaction::default()
            },
        )
    }
}

fn measure_cursor_and_scroll_offset(
    renderer: &IcedRenderer,
    text_bounds: Rectangle,
    value: &text_input::Value,
    size: u16,
    cursor_index: usize,
    font: FontId,
) -> (f32, f32) {
    use text_input::Renderer;

    // TODO: so much allocation (fyi .until() allocates)
    let text_before_cursor = value.until(cursor_index).to_string();

    let text_value_width = renderer.measure_value(&text_before_cursor, size, font);
    let offset = ((text_value_width + EXTRA_OFFSET) - text_bounds.width).max(0.0);

    (text_value_width, offset)
}
