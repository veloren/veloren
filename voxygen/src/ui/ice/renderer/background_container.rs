use super::{
    super::widget::{background_container, image},
    IcedRenderer, Primitive,
};
use iced::{Element, Layout, Point};

impl background_container::Renderer for IcedRenderer {
    fn draw<M>(
        &mut self,
        defaults: &Self::Defaults,
        layout: Layout<'_>,
        cursor_position: Point,
        background: image::Handle,
        color: vek::Rgba<u8>,
        content: &Element<'_, M, Self>,
        content_layout: Layout<'_>,
    ) -> Self::Output {
        let image_primitive = image::Renderer::draw(self, background, color, layout).0;
        let (content_primitive, mouse_cursor) =
            content.draw(self, defaults, content_layout, cursor_position);
        (
            Primitive::Group {
                primitives: vec![image_primitive, content_primitive],
            },
            mouse_cursor,
        )
    }
}
