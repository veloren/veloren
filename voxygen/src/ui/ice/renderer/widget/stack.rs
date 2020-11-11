use super::super::{super::widget::stack, IcedRenderer, Primitive};
use iced::{mouse, Element, Layout, Point, Rectangle};

impl stack::Renderer for IcedRenderer {
    fn draw<M>(
        &mut self,
        defaults: &Self::Defaults,
        content: &[Element<'_, M, Self>],
        layout: Layout<'_>,
        cursor_position: Point,
        viewport: &Rectangle,
    ) -> Self::Output {
        let mut mouse_interaction = mouse::Interaction::default();

        (
            Primitive::Group {
                primitives: content
                    .iter()
                    .zip(layout.children())
                    .map(|(child, layout)| {
                        let (primitive, new_mouse_interaction) =
                            child.draw(self, defaults, layout, cursor_position, viewport);

                        if new_mouse_interaction > mouse_interaction {
                            mouse_interaction = new_mouse_interaction;
                        }

                        primitive
                    })
                    .collect(),
            },
            mouse_interaction,
        )
    }
}
