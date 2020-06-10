use super::super::{super::Rotation, style, IcedRenderer, Primitive};
use common::util::srgba_to_linear;
use iced::{container, Element, Layout, Point, Rectangle};
use style::container::Border;

const BORDER_SIZE: u16 = 8;

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
            Self::Style::Color(color, border) => {
                let linear_color = srgba_to_linear(color.map(|e| e as f32 / 255.0));

                let primitives = match border {
                    Border::None => {
                        let background = Primitive::Rectangle {
                            bounds,
                            linear_color,
                        };

                        vec![background, content]
                    },
                    Border::DoubleCornerless { inner, outer } => {
                        let border_size = f32::from(BORDER_SIZE)
                            .min(bounds.width / 4.0)
                            .min(bounds.height / 4.0);

                        let center = Primitive::Rectangle {
                            bounds: Rectangle {
                                x: bounds.x + border_size * 2.0,
                                y: bounds.y + border_size * 2.0,
                                width: bounds.width - border_size * 4.0,
                                height: bounds.height - border_size * 4.0,
                            },
                            linear_color,
                        };

                        let linear_color = srgba_to_linear(outer.map(|e| e as f32 / 255.0));
                        let top = Primitive::Rectangle {
                            bounds: Rectangle {
                                x: bounds.x + border_size,
                                y: bounds.y,
                                width: bounds.width - border_size * 2.0,
                                height: border_size,
                            },
                            linear_color,
                        };
                        let bottom = Primitive::Rectangle {
                            bounds: Rectangle {
                                x: bounds.x + border_size,
                                y: bounds.y + bounds.height - border_size,
                                width: bounds.width - border_size * 2.0,
                                height: border_size,
                            },
                            linear_color,
                        };
                        let left = Primitive::Rectangle {
                            bounds: Rectangle {
                                x: bounds.x,
                                y: bounds.y + border_size,
                                width: border_size,
                                height: bounds.height - border_size * 2.0,
                            },
                            linear_color,
                        };
                        let right = Primitive::Rectangle {
                            bounds: Rectangle {
                                x: bounds.x + bounds.width - border_size,
                                y: bounds.y + border_size,
                                width: border_size,
                                height: bounds.height - border_size * 2.0,
                            },
                            linear_color,
                        };

                        let linear_color = srgba_to_linear(inner.map(|e| e as f32 / 255.0));
                        let top_inner = Primitive::Rectangle {
                            bounds: Rectangle {
                                x: bounds.x + border_size,
                                y: bounds.y + border_size,
                                width: bounds.width - border_size * 2.0,
                                height: border_size,
                            },
                            linear_color,
                        };
                        let bottom_inner = Primitive::Rectangle {
                            bounds: Rectangle {
                                x: bounds.x + border_size,
                                y: bounds.y + bounds.height - border_size * 2.0,
                                width: bounds.width - border_size * 2.0,
                                height: border_size,
                            },
                            linear_color,
                        };
                        let left_inner = Primitive::Rectangle {
                            bounds: Rectangle {
                                x: bounds.x + border_size,
                                y: bounds.y + border_size * 2.0,
                                width: border_size,
                                height: bounds.height - border_size * 4.0,
                            },
                            linear_color,
                        };
                        let right_inner = Primitive::Rectangle {
                            bounds: Rectangle {
                                x: bounds.x + bounds.width - border_size * 2.0,
                                y: bounds.y + border_size * 2.0,
                                width: border_size,
                                height: bounds.height - border_size * 4.0,
                            },
                            linear_color,
                        };

                        vec![
                            center,
                            top,
                            bottom,
                            left,
                            right,
                            top_inner,
                            bottom_inner,
                            left_inner,
                            right_inner,
                            content,
                        ]
                    },
                };

                Primitive::Group { primitives }
            },
            Self::Style::None => content,
        };

        (prim, mouse_interaction)
    }
}
