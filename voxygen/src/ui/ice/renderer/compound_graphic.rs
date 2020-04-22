use super::{
    super::{cache::FrameRenderer, widget::compound_graphic, Rotation},
    Primitive,
};
use compound_graphic::GraphicKind;
use iced::{mouse, Rectangle};

impl compound_graphic::Renderer for FrameRenderer<'_> {
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
