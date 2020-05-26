use super::{super::Rotation, widget::image, Defaults, IcedRenderer, Primitive};
use iced::{button, mouse, Color, Element, Layout, Point, Rectangle};
use vek::Rgba;

#[derive(Clone, Copy)]
struct Background {
    default: image::Handle,
    hover: image::Handle,
    press: image::Handle,
}

impl Background {
    fn new(image: image::Handle) -> Self {
        Self {
            default: image,
            hover: image,
            press: image,
        }
    }
}
// TODO: consider a different place for this
// Note: for now all buttons have an image background
#[derive(Clone, Copy)]
pub struct Style {
    background: Option<Background>,
    enabled_text: Color,
    disabled_text: Color,
    /* greying out / changing text color
     *disabled: , */
}

impl Style {
    pub fn new(image: image::Handle) -> Self {
        Self {
            background: Some(Background::new(image)),
            ..Default::default()
        }
    }

    pub fn hover_image(mut self, image: image::Handle) -> Self {
        self.background = Some(match self.background {
            Some(mut background) => {
                background.hover = image;
                background
            },
            None => Background::new(image),
        });
        self
    }

    pub fn press_image(mut self, image: image::Handle) -> Self {
        self.background = Some(match self.background {
            Some(mut background) => {
                background.press = image;
                background
            },
            None => Background::new(image),
        });
        self
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.enabled_text = color;
        self
    }

    pub fn disabled_text_color(mut self, color: Color) -> Self {
        self.disabled_text = color;
        self
    }

    fn disabled(&self) -> (Option<image::Handle>, Color) {
        (
            self.background.as_ref().map(|b| b.default),
            self.disabled_text,
        )
    }

    fn pressed(&self) -> (Option<image::Handle>, Color) {
        (self.background.as_ref().map(|b| b.press), self.enabled_text)
    }

    fn hovered(&self) -> (Option<image::Handle>, Color) {
        (self.background.as_ref().map(|b| b.hover), self.enabled_text)
    }

    fn active(&self) -> (Option<image::Handle>, Color) {
        (
            self.background.as_ref().map(|b| b.default),
            self.enabled_text,
        )
    }
}

impl Default for Style {
    fn default() -> Self {
        Self {
            background: None,
            enabled_text: Color::WHITE,
            disabled_text: Color::from_rgb(0.5, 0.5, 0.5),
        }
    }
}

impl button::Renderer for IcedRenderer {
    // TODO: what if this gets large enough to not be copied around?
    type Style = Style;

    const DEFAULT_PADDING: u16 = 0;

    fn draw<M>(
        &mut self,
        defaults: &Self::Defaults,
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
