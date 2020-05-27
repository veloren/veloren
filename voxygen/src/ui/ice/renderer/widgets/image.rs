use super::super::{
    super::{widget::image, Rotation},
    IcedRenderer, Primitive,
};
use iced::mouse;
use vek::Rgba;

impl image::Renderer for IcedRenderer {
    fn dimensions(&self, handle: image::Handle) -> (u32, u32) { self.image_dims(handle) }

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
            mouse::Interaction::default(),
        )
    }
}
