use super::super::{
    super::{widget::compound_graphic, Rotation},
    IcedRenderer, Primitive,
};
use common::util::srgba_to_linear;
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
                        GraphicKind::Color(color) => Primitive::Rectangle {
                            bounds,
                            linear_color: srgba_to_linear(color.map(|e| e as f32 * 255.0)),
                        },
                    })
                    .collect(),
            },
            mouse::Interaction::default(),
        )
    }
}
