use super::super::{IcedRenderer, Primitive};
use iced::{Rectangle, mouse, space};

impl space::Renderer for IcedRenderer {
    fn draw(&mut self, _bounds: Rectangle) -> Self::Output {
        (Primitive::Nothing, mouse::Interaction::default())
    }
}
