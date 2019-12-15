use super::super::super::graphic::{self, Rotation};
use iced::{layout, Element, Hasher, Layout, Length, Point, Size, Widget};
use std::hash::Hash;

// TODO: consider iced's approach to images and caching image data
// Also `Graphic` might be a better name for this is it wasn't already in use
// elsewhere

pub type Handle = (graphic::Id, Rotation);

pub struct Image {
    handle: Handle,
    size: Size,
}

impl Image {
    pub fn new(handle: Handle, w: f32, h: f32) -> Self {
        let size = Size::new(w, h);
        Self { handle, size }
    }
}

impl<M, R> Widget<M, R> for Image
where
    R: self::Renderer,
{
    fn width(&self) -> Length { Length::Fill }

    fn height(&self) -> Length { Length::Fill }

    fn layout(&self, _renderer: &R, _limits: &layout::Limits) -> layout::Node {
        // We don't care about aspect ratios here :p
        layout::Node::new(self.size)
        // Infinite sizes confusing
        //layout::Node::new(limits.resolve(self.size))
    }

    fn draw(
        &self,
        renderer: &mut R,
        _defaults: &R::Defaults,
        layout: Layout<'_>,
        _cursor_position: Point,
    ) -> R::Output {
        renderer.draw(self.handle, layout)
    }

    fn hash_layout(&self, state: &mut Hasher) {
        self.size.width.to_bits().hash(state);
        self.size.height.to_bits().hash(state);
    }
}

pub trait Renderer: iced::Renderer {
    fn draw(&mut self, handle: Handle, layout: Layout<'_>) -> Self::Output;
}

impl<'a, M, R> From<Image> for Element<'a, M, R>
where
    R: self::Renderer,
{
    fn from(image: Image) -> Element<'a, M, R> { Element::new(image) }
}
