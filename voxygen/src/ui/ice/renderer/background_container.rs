use super::{super::widget::background_container, IcedRenderer, Primitive};
use iced::{Element, Layout, Point};

impl background_container::Renderer for IcedRenderer {
    fn draw<M, B>(
        &mut self,
        defaults: &Self::Defaults,
        background: &B,
        background_layout: Layout<'_>,
        content: &Element<'_, M, Self>,
        content_layout: Layout<'_>,
        cursor_position: Point,
    ) -> Self::Output
    where
        B: background_container::Background<Self>,
    {
        let back_primitive = background
            .draw(self, defaults, background_layout, cursor_position)
            .0;
        let (content_primitive, mouse_cursor) =
            content.draw(self, defaults, content_layout, cursor_position);
        (
            Primitive::Group {
                primitives: vec![back_primitive, content_primitive],
            },
            mouse_cursor,
        )
    }
}
