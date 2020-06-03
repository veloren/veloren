use super::super::{super::Rotation, style, IcedRenderer, Primitive};
use common::util::srgba_to_linear;
use iced::{container, Element, Layout, Point, Rectangle};

impl container::Renderer for IcedRenderer {
    type Style = style::container::Style;

    fn draw<M>(
        &mut self,
        defaults: &Self::Defaults,
        bounds: Rectangle,
        cursor_position: Point,
        style_sheet: &Self::Style,
        content: &Element<'_, M, Self>,
        content_layout: Layout<'_>,
    ) -> Self::Output {
        let (content, mouse_interaction) =
            content.draw(self, defaults, content_layout, cursor_position);

        let prim = match style_sheet {
            Self::Style::Image(handle, color) => {
                let background = Primitive::Image {
                    handle: (*handle, Rotation::None),
                    bounds,
                    color: *color,
                };

                Primitive::Group {
                    primitives: vec![background, content],
                }
            },
            Self::Style::Color(color) => {
                let background = Primitive::Rectangle {
                    bounds,
                    linear_color: srgba_to_linear(color.map(|e| e as f32 / 255.0)),
                };

                Primitive::Group {
                    primitives: vec![background, content],
                }
            },
            Self::Style::None => content,
        };

        (prim, mouse_interaction)
    }
}
