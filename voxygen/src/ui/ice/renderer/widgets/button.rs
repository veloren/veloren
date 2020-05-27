use super::super::{super::Rotation, Defaults, IcedRenderer, Primitive};
use iced::{button, mouse, Element, Layout, Point, Rectangle};
use vek::Rgba;

impl button::Renderer for IcedRenderer {
    // TODO: what if this gets large enough to not be copied around?
    type Style = super::super::style::ButtonStyle;

    const DEFAULT_PADDING: u16 = 0;

    fn draw<M>(
        &mut self,
        _defaults: &Self::Defaults,
        bounds: Rectangle,
        cursor_position: Point,
        is_disabled: bool,
        is_pressed: bool,
        style: &Self::Style,
        content: &Element<'_, M, Self>,
        content_layout: Layout<'_>,
    ) -> Self::Output {
        let is_mouse_over = bounds.contains(cursor_position);

        let (maybe_image, text_color) = if is_disabled {
            style.disabled()
        } else if is_mouse_over {
            if is_pressed {
                style.pressed()
            } else {
                style.hovered()
            }
        } else {
            style.active()
        };

        let (content, _) = content.draw(
            self,
            &Defaults { text_color },
            content_layout,
            cursor_position,
        );

        let primitive = if let Some(handle) = maybe_image {
            let background = Primitive::Image {
                handle: (handle, Rotation::None),
                bounds,
                color: Rgba::broadcast(255),
            };

            Primitive::Group {
                primitives: vec![background, content],
            }
        } else {
            content
        };

        let mouse_interaction = if is_mouse_over {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        };

        (primitive, mouse_interaction)
    }
}
