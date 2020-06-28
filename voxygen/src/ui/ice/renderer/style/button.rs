use super::super::super::widget::image;
use iced::Color;
use vek::Rgba;

#[derive(Clone, Copy)]
struct Background {
    default: image::Handle,
    hover: image::Handle,
    press: image::Handle,
    color: Rgba<u8>,
}

impl Background {
    fn new(image: image::Handle) -> Self {
        Self {
            default: image,
            hover: image,
            press: image,
            color: Rgba::white(),
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

    // TODO: this needs to be refactored since the color isn't used if there is no
    // background
    pub fn image_color(mut self, color: Rgba<u8>) -> Self {
        if let Some(background) = &mut self.background {
            background.color = color;
        }
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

    pub fn disabled(&self) -> (Option<(image::Handle, Rgba<u8>)>, Color) {
        (
            self.background.as_ref().map(|b| (b.default, b.color)),
            self.disabled_text,
        )
    }

    pub fn pressed(&self) -> (Option<(image::Handle, Rgba<u8>)>, Color) {
        (
            self.background.as_ref().map(|b| (b.press, b.color)),
            self.enabled_text,
        )
    }

    pub fn hovered(&self) -> (Option<(image::Handle, Rgba<u8>)>, Color) {
        (
            self.background.as_ref().map(|b| (b.hover, b.color)),
            self.enabled_text,
        )
    }

    pub fn active(&self) -> (Option<(image::Handle, Rgba<u8>)>, Color) {
        (
            self.background.as_ref().map(|b| (b.default, b.color)),
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
