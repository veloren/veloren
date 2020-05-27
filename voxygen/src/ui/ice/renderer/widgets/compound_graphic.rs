use super::super::{
    super::{widget::compound_graphic, Rotation},
    IcedRenderer, Primitive,
};
use compound_graphic::GraphicKind;
use iced::{mouse, Rectangle};

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
                        GraphicKind::Image(handle, color) => Primitive::Image {
                            handle: (handle, Rotation::None),
                            bounds,
                            color,
                        },
                        GraphicKind::Color(color) => Primitive::Rectangle { bounds, color },
                    })
                    .collect(),
            },
            mouse::Interaction::default(),
        )
    }
}
