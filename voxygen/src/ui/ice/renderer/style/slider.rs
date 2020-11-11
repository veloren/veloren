use super::super::super::widget::image;
use vek::Rgba;

#[derive(Clone, Copy)]
pub struct Style {
    pub cursor: Cursor,
    pub bar: Bar,
    pub labels: bool,
    pub cursor_size: (u16, u16),
    pub bar_height: u16,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            cursor: Cursor::Color(Rgba::new(0.5, 0.5, 0.5, 1.0)),
            bar: Bar::Color(Rgba::new(0.5, 0.5, 0.5, 1.0)),
            labels: false,
            cursor_size: (8, 16),
            bar_height: 6,
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
    Image(image::Handle, Rgba<u8>, u16),
}

impl Style {
    pub fn images(
        cursor: image::Handle,
        bar: image::Handle,
        bar_pad: u16,
        cursor_size: (u16, u16),
        bar_height: u16,
    ) -> Self {
        Self {
            cursor: Cursor::Image(cursor, Rgba::white()),
            bar: Bar::Image(bar, Rgba::white(), bar_pad),
            labels: false,
            cursor_size,
            bar_height,
        }
    }
}
