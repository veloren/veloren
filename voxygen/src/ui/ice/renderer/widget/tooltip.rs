use super::super::{super::widget::tooltip, IcedRenderer, Primitive};
use iced::{Element, Layout, Point, Rectangle};

impl tooltip::Renderer for IcedRenderer {
    fn draw<M>(
        &mut self,
        alpha: f32,
        defaults: &Self::Defaults,
        cursor_position: Point,
        viewport: &Rectangle,
        content: &Element<'_, M, Self>,
        content_layout: Layout<'_>,
    ) -> Self::Output {
        let (primitive, cursor_interaction) =
            content.draw(self, defaults, content_layout, cursor_position, viewport);
        (
            Primitive::Opacity {
                alpha,
                content: Box::new(primitive),
            },
            cursor_interaction,
        )
    }
}
