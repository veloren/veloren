use super::super::super::widget::image;
use vek::Rgba;

/// Background of the container
pub enum Style {
    Image(image::Handle, Rgba<u8>),
    Color(Rgba<u8>),
    None,
}

impl Style {
    /// Shorthand for common case where the color of the image is not modified
    pub fn image(image: image::Handle) -> Self { Self::Image(image, Rgba::broadcast(255)) }
}

impl Default for Style {
    fn default() -> Self { Self::None }
}
