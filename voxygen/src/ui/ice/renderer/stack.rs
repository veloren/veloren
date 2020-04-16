use super::{super::widget::stack, IcedRenderer, Primitive};
use iced::{Element, Layout, MouseCursor, Point};

impl stack::Renderer for IcedRenderer {
    fn draw<M>(
        &mut self,
        defaults: &Self::Defaults,
        children: &[Element<'_, M, Self>],
        layout: Layout<'_>,
        cursor_position: Point,
    ) -> Self::Output {
        let mut mouse_cursor = MouseCursor::OutOfBounds;

        (
            Primitive::Group {
                primitives: children
                    .iter()
                    .zip(layout.children())
                    .map(|(child, layout)| {
                        let (primitive, new_mouse_cursor) =
                            child.draw(self, defaults, layout, cursor_position);

                        if new_mouse_cursor > mouse_cursor {
                            mouse_cursor = new_mouse_cursor;
                        }

                        primitive
                    })
                    .collect(),
            },
            mouse_cursor,
        )
    }
}
