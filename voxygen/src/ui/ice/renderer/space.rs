use super::{IcedRenderer, Primitive};
use iced::{space, MouseCursor, Rectangle};

impl space::Renderer for IcedRenderer {
    fn draw(&mut self, _bounds: Rectangle) -> Self::Output {
        (Primitive::Nothing, MouseCursor::OutOfBounds)
    }
}
