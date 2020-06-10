use super::super::super::widget::image;
use vek::Rgba;

/// Container Border
pub enum Border {
    DoubleCornerless { inner: Rgba<u8>, outer: Rgba<u8> },
    None,
}

/// Background of the container
pub enum Style {
    Image(image::Handle, Rgba<u8>),
    Color(Rgba<u8>, Border),
    None,
}

impl Style {
    /// Shorthand for common case where the color of the image is not modified
    pub fn image(image: image::Handle) -> Self { Self::Image(image, Rgba::broadcast(255)) }

    /// Shorthand for a color background with no border
    pub fn color(color: Rgba<u8>) -> Self { Self::Color(color, Border::None) }

    /// Shorthand for a color background with a cornerless border
    pub fn color_double_cornerless_border(
        color: Rgba<u8>,
        inner: Rgba<u8>,
        outer: Rgba<u8>,
    ) -> Self {
        Self::Color(color, Border::DoubleCornerless { inner, outer })
    }
}

impl Default for Style {
    fn default() -> Self { Self::None }
}
