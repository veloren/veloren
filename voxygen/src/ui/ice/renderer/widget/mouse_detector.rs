use super::super::{super::widget::mouse_detector, IcedRenderer, Primitive};
use iced::{mouse, Rectangle};

impl mouse_detector::Renderer for IcedRenderer {
    fn draw(&mut self, _bounds: Rectangle) -> Self::Output {
        // TODO: mouse interaction if in bounds??
        (Primitive::Nothing, mouse::Interaction::default())
    }
}
