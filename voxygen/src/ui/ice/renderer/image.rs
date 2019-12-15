use super::{super::widget::image, IcedRenderer, Primitive};
use iced::MouseCursor;

impl image::Renderer for IcedRenderer {
    fn draw(&mut self, handle: image::Handle, layout: iced::Layout<'_>) -> Self::Output {
        (
            Primitive::Image {
                handle,
                bounds: layout.bounds(),
            },
            MouseCursor::OutOfBounds,
        )
    }
}
