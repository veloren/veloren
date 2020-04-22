use super::{super::cache::FrameRenderer, Primitive};
use iced::{mouse, space, Rectangle};

impl space::Renderer for FrameRenderer<'_> {
    fn draw(&mut self, _bounds: Rectangle) -> Self::Output {
        (Primitive::Nothing, mouse::Interaction::default())
    }
}
