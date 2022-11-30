use super::super::{super::Rotation, style, IcedRenderer, Primitive};
use common::util::srgba_to_linear;
use core::ops::RangeInclusive;
use iced::{mouse, slider, Point, Rectangle};
use style::slider::{Bar, Cursor, Style};

const CURSOR_DRAG_SHIFT: f32 = 0.7;

impl slider::Renderer for IcedRenderer {
    type Style = Style;

    const DEFAULT_HEIGHT: u16 = 25;

    fn draw(
        &mut self,
        bounds: Rectangle,
        cursor_position: Point,
        range: RangeInclusive<f32>,
        value: f32,
        is_dragging: bool,
        style: &Self::Style,
    ) -> Self::Output {
        let bar_bounds = Rectangle {
            height: style.bar_height as f32,
            y: bounds.y + (bounds.height - style.bar_height as f32) / 2.0,
            ..bounds
        };
        let bar = match style.bar {
            Bar::Color(color) => Primitive::Rectangle {
                bounds: bar_bounds,
                linear_color: srgba_to_linear(color),
            },
            // Note: bar_pad adds to the size of the bar currently since the dragging logic wouldn't
            // account for shrinking the area that the cursor is shown in
            Bar::Image(handle, color, bar_pad) => Primitive::Image {
                handle: (handle, Rotation::None),
                bounds: Rectangle {
                    x: bar_bounds.x - bar_pad as f32,
                    width: bar_bounds.width + bar_pad as f32 * 2.0,
                    ..bar_bounds
                },
                color,
                source_rect: None,
            },
        };

        let (cursor_width, cursor_height) = style.cursor_size;
        let (cursor_width, cursor_height) = (f32::from(cursor_width), f32::from(cursor_height));
        let (min, max) = range.into_inner();
        let offset = bounds.width * (value - min) / (max - min);
        let cursor_bounds = Rectangle {
            x: bounds.x + offset - cursor_width / 2.0,
            y: bounds.y
                + if is_dragging { CURSOR_DRAG_SHIFT } else { 0.0 }
                + (bounds.height - cursor_height) / 2.0,
            width: cursor_width,
            height: cursor_height,
        };
        let cursor = match style.cursor {
            Cursor::Color(color) => Primitive::Rectangle {
                bounds: cursor_bounds,
                linear_color: srgba_to_linear(color),
            },
            Cursor::Image(handle, color) => Primitive::Image {
                handle: (handle, Rotation::None),
                bounds: cursor_bounds,
                color,
                source_rect: None,
            },
        };

        let interaction = if is_dragging {
            mouse::Interaction::Grabbing
        } else if cursor_bounds.contains(cursor_position) {
            mouse::Interaction::Grab
        } else if bar_bounds.contains(cursor_position) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::Idle
        };

        #[allow(clippy::if_same_then_else)] // TODO: remove
        let primitives = if style.labels {
            // TODO text label on left and right ends
            vec![bar, cursor]
        } else {
            // TODO Cursor text label
            vec![bar, cursor]
        };
        (Primitive::Group { primitives }, interaction)
    }
}
