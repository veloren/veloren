use super::super::graphic;
use iced::{layout, Element, Hasher, Layout, Length, Point, Widget};
use std::hash::Hash;
use vek::Rgba;

// TODO: consider iced's approach to images and caching image data
// Also `Graphic` might be a better name for this is it wasn't already in use
// elsewhere

pub type Handle = graphic::Id;

pub struct Image {
    handle: Handle,
    width: Length,
    height: Length,
    fix_aspect_ratio: bool,
    color: Rgba<u8>,
}

impl Image {
    pub fn new(handle: Handle) -> Self {
        let width = Length::Fill;
        let height = Length::Fill;
        Self {
            handle,
            width,
            height,
            fix_aspect_ratio: false,
            color: Rgba::broadcast(255),
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

    pub fn fix_aspect_ratio(mut self) -> Self {
        self.fix_aspect_ratio = true;
        self
    }

    pub fn color(mut self, color: Rgba<u8>) -> Self {
        self.color = color;
        self
    }
}

impl<M, R> Widget<M, R> for Image
where
    R: self::Renderer,
{
    fn width(&self) -> Length { self.width }

    fn height(&self) -> Length { self.height }

    fn layout(&self, renderer: &R, limits: &layout::Limits) -> layout::Node {
        let mut size = limits.width(self.width).height(self.height).max();

        if self.fix_aspect_ratio {
            let aspect_ratio = {
                let (w, h) = renderer.dimensions(self.handle);
                w as f32 / h as f32
            };

            let max_aspect_ratio = size.width / size.height;

            if max_aspect_ratio > aspect_ratio {
                size.width = size.height * aspect_ratio;
            } else {
                size.height = size.width / aspect_ratio;
            }
        }

        layout::Node::new(size)
    }

    fn draw(
        &self,
        renderer: &mut R,
        _defaults: &R::Defaults,
        layout: Layout<'_>,
        _cursor_position: Point,
    ) -> R::Output {
        renderer.draw(self.handle, self.color, layout)
    }

    fn hash_layout(&self, state: &mut Hasher) {
        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);

        self.width.hash(state);
        self.height.hash(state);
        self.fix_aspect_ratio.hash(state);
        // TODO: also depends on dims but we have no way to access
    }
}

pub trait Renderer: iced::Renderer {
    fn dimensions(&self, handle: Handle) -> (u32, u32);
    fn draw(&mut self, handle: Handle, color: Rgba<u8>, layout: Layout<'_>) -> Self::Output;
}

impl<'a, M, R> From<Image> for Element<'a, M, R>
where
    R: self::Renderer,
{
    fn from(image: Image) -> Element<'a, M, R> { Element::new(image) }
}

impl<R> super::background_container::Background<R> for Image
where
    R: self::Renderer,
{
    fn width(&self) -> Length { self.width }

    fn height(&self) -> Length { self.height }

    fn aspect_ratio_fixed(&self) -> bool { self.fix_aspect_ratio }

    fn pixel_dims(&self, renderer: &R) -> (u16, u16) {
        let (w, h) = renderer.dimensions(self.handle);
        (w as u16, h as u16)
    }

    fn draw(
        &self,
        renderer: &mut R,
        _defaults: &R::Defaults,
        layout: Layout<'_>,
        _cursor_position: Point,
    ) -> R::Output {
        renderer.draw(self.handle, self.color, layout)
    }
}
