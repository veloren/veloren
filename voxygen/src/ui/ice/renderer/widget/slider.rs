use super::super::{super::Rotation, style, IcedRenderer, Primitive};
use common::util::srgba_to_linear;
use iced::{slider, mouse, Rectangle, Point};
use core::ops::RangeInclusive;
use style::slider::{Bar, Cursor, Style};

const CURSOR_WIDTH: f32 = 10.0;
const CURSOR_HEIGHT: f32 = 16.0;
const BAR_HEIGHT: f32 = 18.0;

impl slider::Renderer for IcedRenderer {
    type Style = Style;
    fn height(&self) -> u32 { 20 }
    fn draw(
        &mut self,
        bounds: Rectangle,
        cursor_position: Point,
        range: RangeInclusive<f32>,
        value: f32,
        is_dragging: bool,
        style: &Self::Style
    ) -> Self::Output {

        let bar_bounds = Rectangle {
            height: BAR_HEIGHT,
            ..bounds
        };
        let bar = match style.bar {
            Bar::Color(color) => Primitive::Rectangle {
                bounds: bar_bounds,
                linear_color: srgba_to_linear(color),
            },
            Bar::Image(handle, color) => Primitive::Image {
                handle: (handle, Rotation::None),
                bounds: bar_bounds,
                color,
            },
        };

        let (max, min) = range.into_inner();
        let offset = bounds.width as f32 * (max - min ) / (value - min);
        let cursor_bounds = Rectangle {
            x: bounds.x + offset - CURSOR_WIDTH / 2.0,
            y: bounds.y + if is_dragging { 2.0 } else { 0.0 },
            width: CURSOR_WIDTH,
            height: CURSOR_HEIGHT,
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

        let primitives = if style.labels {
            // TODO text label on left and right ends
            vec![bar, cursor]
        } else {
            // TODO Cursor text label
            vec![bar, cursor]
        };
        (Primitive::Group{primitives}, interaction)
    }
}
