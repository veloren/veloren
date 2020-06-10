use super::super::super::widget::image;
use vek::Rgba;

#[derive(Clone, Copy)]
pub struct Style {
    pub cursor: Cursor,
    pub bar: Bar,
    pub labels: bool,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            cursor: Cursor::Color(Rgba::new(0.5, 0.5, 0.5, 1.0)),
            bar: Bar::Color(Rgba::new(0.5, 0.5, 0.5, 1.0)),
            labels: false,
        }
    }
}

#[derive(Clone, Copy)]
pub enum Cursor {
    Color(Rgba<f32>),
    Image(image::Handle, Rgba<u8>),
}

#[derive(Clone, Copy)]
pub enum Bar {
    Color(Rgba<f32>),
    Image(image::Handle, Rgba<u8>),
}

