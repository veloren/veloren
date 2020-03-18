use super::{
    super::{widget::image, Rotation},
    IcedRenderer, Primitive,
};
use iced::MouseCursor;
use vek::Rgba;

impl image::Renderer for IcedRenderer {
    fn dimensions(&self, handle: image::Handle) -> (u32, u32) {
        self.cache
            .graphic_cache()
            .get_graphic_dims((handle, Rotation::None))
            // TODO: don't unwrap
            .unwrap()
    }

    fn draw(
        &mut self,
        handle: image::Handle,
        color: Rgba<u8>,
        layout: iced::Layout<'_>,
    ) -> Self::Output {
        (
            Primitive::Image {
                handle: (handle, Rotation::None),
                bounds: layout.bounds(),
                color,
            },
            MouseCursor::OutOfBounds,
        )
    }
}
