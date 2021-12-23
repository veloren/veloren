use super::super::super::widget::image;

#[derive(Clone, Copy)]
struct Background {
    default: image::Handle,
    hover: image::Handle,
    checked: image::Handle,
    hover_checked: image::Handle,
}

impl Background {
    fn new(image: image::Handle) -> Self {
        Self {
            default: image,
            hover: image,
            checked: image,
            hover_checked: image,
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct Style {
    background: Option<Background>,
    check: Option<image::Handle>,
}

impl Style {
    pub fn new(image: image::Handle, check: image::Handle) -> Self {
        Self {
            background: Some(Background::new(image)),
            check: Some(check),
        }
    }

    #[must_use]
    pub fn bg_hover_image(mut self, image: image::Handle) -> Self {
        self.background = Some(match self.background {
            Some(mut background) => {
                background.hover = image;
                background
            },
            None => Background::new(image),
        });
        self
    }

    #[must_use]
    pub fn bg_checked_image(mut self, image: image::Handle) -> Self {
        self.background = Some(match self.background {
            Some(mut background) => {
                background.checked = image;
                background
            },
            None => Background::new(image),
        });
        self
    }

    #[must_use]
    pub fn bg_hover_checked_image(mut self, image: image::Handle) -> Self {
        self.background = Some(match self.background {
            Some(mut background) => {
                background.hover_checked = image;
                background
            },
            None => Background::new(image),
        });
        self
    }

    pub fn check(&self) -> Option<image::Handle> { self.check }

    pub fn bg_checked(&self) -> Option<image::Handle> {
        self.background.as_ref().map(|b| b.checked)
    }

    pub fn bg_hover(&self) -> Option<image::Handle> { self.background.as_ref().map(|b| b.hover) }

    pub fn bg_hover_checked(&self) -> Option<image::Handle> {
        self.background.as_ref().map(|b| b.hover_checked)
    }

    pub fn bg_default(&self) -> Option<image::Handle> {
        self.background.as_ref().map(|b| b.default)
    }
}
