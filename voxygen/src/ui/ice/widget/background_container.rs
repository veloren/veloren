use iced::{layout, Element, Hasher, Layout, Length, Point, Size, Widget};
use std::u32;
use vek::Rgba;

// Note: it might be more efficient to make this generic over the content type

// Note: maybe we could just use the container styling for this

/// This widget is displays a background image behind it's content
pub struct BackgroundContainer<'a, M, R: self::Renderer> {
    width: Length,
    height: Length,
    max_width: u32,
    max_height: u32,
    background: super::image::Handle,
    fix_aspect_ratio: bool,
    content: Element<'a, M, R>,
    color: Rgba<u8>,
}

impl<'a, M, R> BackgroundContainer<'a, M, R>
where
    R: self::Renderer,
{
    pub fn new(background: super::image::Handle, content: impl Into<Element<'a, M, R>>) -> Self {
        Self {
            width: Length::Shrink,
            height: Length::Shrink,
            max_width: u32::MAX,
            max_height: u32::MAX,
            background,
            fix_aspect_ratio: false,
            content: content.into(),
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

    pub fn max_width(mut self, max_width: u32) -> Self {
        self.max_width = max_width;
        self
    }

    pub fn max_height(mut self, max_height: u32) -> Self {
        self.max_height = max_height;
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

impl<'a, M, R> Widget<M, R> for BackgroundContainer<'a, M, R>
where
    R: self::Renderer,
{
    fn width(&self) -> Length { self.width }

    fn height(&self) -> Length { self.height }

    fn layout(&self, renderer: &R, limits: &layout::Limits) -> layout::Node {
        let limits = limits
            .loose() // why does iced's container do this?
            .max_width(self.max_width)
            .max_height(self.max_height)
            .width(self.width)
            .height(self.height);

        let (size, content) = if self.fix_aspect_ratio {
            let (w, h) = renderer.dimensions(self.background);
            // To fix the aspect ratio we have to have a separate layout from the content
            // because we can't force the content to have a specific aspect ratio
            let aspect_ratio = w as f32 / h as f32;
            // To do this we need to figure out the max width/height of the limits
            // and then adjust one down to meet the aspect ratio
            let max_size = limits.max();
            let (max_width, max_height) = (max_size.width as f32, max_size.height as f32);
            let max_aspect_ratio = max_width / max_height;
            let limits = if max_aspect_ratio > aspect_ratio {
                limits.max_width((max_height * aspect_ratio) as u32)
            } else {
                limits.max_height((max_width / aspect_ratio) as u32)
            };
            // Get content size
            // again, why is loose() used here?
            let content = self.content.layout(renderer, &limits.loose());
            // This time we need to adjust up to meet the aspect ratio
            // so that the container is larger than the contents
            let content_size = content.size();
            let content_aspect_ratio = content_size.width as f32 / content_size.height as f32;
            let size = if content_aspect_ratio > aspect_ratio {
                Size::new(content_size.width, content_size.width / aspect_ratio)
            } else {
                Size::new(content_size.height * aspect_ratio, content_size.width)
            };

            (size, content)
        } else {
            // again, why is loose() used here?
            let content = self.content.layout(renderer, &limits.loose());
            let size = limits.resolve(content.size());
            //self.content.layout(renderer, limits)

            (size, content)
        };

        layout::Node::with_children(size, vec![content])
    }

    fn draw(
        &self,
        renderer: &mut R,
        defaults: &R::Defaults,
        layout: Layout<'_>,
        cursor_position: Point,
    ) -> R::Output {
        self::Renderer::draw(
            renderer,
            defaults,
            layout,
            cursor_position,
            self.background,
            self.color,
            &self.content,
            layout.children().next().unwrap(),
        )
    }

    fn hash_layout(&self, state: &mut Hasher) { self.content.hash_layout(state); }
}

pub trait Renderer: iced::Renderer + super::image::Renderer {
    fn draw<M>(
        &mut self,
        defaults: &Self::Defaults,
        layout: Layout<'_>,
        cursor_position: Point,
        background: super::image::Handle,
        color: Rgba<u8>,
        content: &Element<'_, M, Self>,
        content_layout: Layout<'_>,
    ) -> Self::Output;
}

// They got to live ¯\_(ツ)_/¯
impl<'a, M: 'a, R: 'a> From<BackgroundContainer<'a, M, R>> for Element<'a, M, R>
where
    R: self::Renderer,
{
    fn from(background_container: BackgroundContainer<'a, M, R>) -> Element<'a, M, R> {
        Element::new(background_container)
    }
}
