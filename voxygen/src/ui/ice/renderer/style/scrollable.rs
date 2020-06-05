use super::super::super::widget::image;
use vek::Rgba;

#[derive(Clone, Copy)]
pub struct Style {
    pub track: Option<Track>,
    pub scroller: Scroller,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            track: None,
            scroller: Scroller::Color(Rgba::new(128, 128, 128, 255)),
        }
    }
}

#[derive(Clone, Copy)]
pub enum Track {
    Color(Rgba<u8>),
    Image(image::Handle, Rgba<u8>),
}

#[derive(Clone, Copy)]
pub enum Scroller {
    Color(Rgba<u8>),
    Image {
        ends: image::Handle,
        mid: image::Handle,
        color: Rgba<u8>,
    },
}
