use super::super::super::graphic::{self, Rotation};
use iced::{layout, Element, Hasher, Layout, Length, Point, Size, Widget};
use std::hash::Hash;

// TODO: consider iced's approach to images and caching image data
// Also `Graphic` might be a better name for this is it wasn't already in use
// elsewhere

pub type Handle = (graphic::Id, Rotation);

pub struct Image {
    handle: Handle,
    width: Length,
    height: Length,
}

impl Image {
    pub fn new(handle: Handle) -> Self {
        let width = Length::Fill;
        let height = Length::Fill;
        Self {
            handle,
            width,
            height,
        }
    }

    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }
}

impl<M, R> Widget<M, R> for Image
where
    R: self::Renderer,
{
    fn width(&self) -> Length { self.width }

    fn height(&self) -> Length { self.height }

    fn layout(&self, _renderer: &R, limits: &layout::Limits) -> layout::Node {
        // We don't care about aspect ratios here :p
        let size = limits.width(self.width).height(self.height).max();

        layout::Node::new(size)
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
        self.width.hash(state);
        self.height.hash(state);
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
