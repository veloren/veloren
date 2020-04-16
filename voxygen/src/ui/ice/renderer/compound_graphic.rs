use super::{
    super::{widget::compound_graphic, Rotation},
    IcedRenderer, Primitive,
};
use compound_graphic::GraphicKind;
use iced::{MouseCursor, Rectangle};
use vek::Rgba;

impl compound_graphic::Renderer for IcedRenderer {
    fn draw<I>(
        &mut self,
        graphics: I,
        //color: Rgba<u8>,
        _layout: iced::Layout<'_>,
    ) -> Self::Output
    where
        I: Iterator<Item = (Rectangle, GraphicKind)>,
    {
        (
            Primitive::Group {
                primitives: graphics
                    .map(|(bounds, kind)| match kind {
                        GraphicKind::Image(handle) => Primitive::Image {
                            handle: (handle, Rotation::None),
                            bounds,
                            color: Rgba::broadcast(255),
                        },
                        GraphicKind::Color(color) => Primitive::Rectangle { bounds, color },
                    })
                    .collect(),
            },
            MouseCursor::OutOfBounds,
        )
    }
}
