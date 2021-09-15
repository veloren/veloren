use super::super::{super::Rotation, style, IcedRenderer, Primitive};
use iced::{checkbox, mouse, Rectangle};

impl checkbox::Renderer for IcedRenderer {
    // TODO: what if this gets large enough to not be copied around?
    type Style = style::checkbox::Style;

    const DEFAULT_SIZE: u16 = 20;
    const DEFAULT_SPACING: u16 = 15;

    fn draw(
        &mut self,
        bounds: Rectangle,
        is_checked: bool,
        is_mouse_over: bool,
        (label, _): Self::Output,
        style: &Self::Style,
    ) -> Self::Output {
        let default_rect = || Primitive::Rectangle {
            bounds,
            linear_color: vek::Rgba::broadcast(1.0),
        };

        let background_image = match (is_checked, is_mouse_over) {
            (true, true) => style.bg_hover_checked(),
            (true, false) => style.bg_checked(),
            (false, true) => style.bg_hover(),
            (false, false) => style.bg_default(),
        };

        let background = background_image
            .map(|image| Primitive::Image {
                handle: (image, Rotation::None),
                bounds,
                color: vek::Rgba::broadcast(255),
                source_rect: None,
            })
            .unwrap_or_else(default_rect);

        (
            Primitive::Group {
                primitives: if is_checked {
                    let check = style
                        .check()
                        .map(|image| Primitive::Image {
                            handle: (image, Rotation::None),
                            bounds,
                            color: vek::Rgba::broadcast(255),
                            source_rect: None,
                        })
                        .unwrap_or_else(default_rect);

                    vec![background, check, label]
                } else {
                    vec![background, label]
                },
            },
            if is_mouse_over {
                mouse::Interaction::Pointer
            } else {
                mouse::Interaction::default()
            },
        )
    }
}
