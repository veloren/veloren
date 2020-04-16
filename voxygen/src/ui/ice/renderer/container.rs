use super::IcedRenderer;
use iced::{container, Element, Layout, Point, Rectangle};

impl container::Renderer for IcedRenderer {
    type Style = ();

    fn draw<M>(
        &mut self,
        defaults: &Self::Defaults,
        _bounds: Rectangle,
        cursor_position: Point,
        _style_sheet: &Self::Style,
        content: &Element<'_, M, Self>,
        content_layout: Layout<'_>,
    ) -> Self::Output {
        let (content, mouse_cursor) = content.draw(self, defaults, content_layout, cursor_position);

        // We may have more stuff here if styles are used

        (content, mouse_cursor)
    }
}
