use super::super::super::widget::image;

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

#[derive(Clone, Copy)]
pub struct Style {
    background: Option<Background>,
    checked: Option<image::Handle>,
}

impl Style {
    pub fn new(image: image::Handle, checked: image::Handle) -> Self {
        Self {
            background: Some(Background::new(image)),
            checked: Some(checked),
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

    pub fn pressed(&self) -> Option<image::Handle> { self.background.as_ref().map(|b| b.press) }

    pub fn checked(&self) -> Option<image::Handle> { self.checked }

    pub fn hovered(&self) -> Option<image::Handle> { self.background.as_ref().map(|b| b.hover) }

    pub fn background(&self) -> Option<image::Handle> {
        self.background.as_ref().map(|b| b.default)
    }
}

impl Default for Style {
    fn default() -> Self {
        Self {
            background: None,
            checked: None,
        }
    }
}
