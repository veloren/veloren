use iced::{
    layout, Clipboard, Element, Event, Hasher, Layout, Length, Point, Rectangle, Size, Widget,
};
use std::{hash::Hash, u32};

// Note: it might be more efficient to make this generic over the content type?

enum AspectRatio<I> {
    /// Image Id
    Image(I),
    /// width / height
    Ratio(f32),
}

impl<I: Hash> Hash for AspectRatio<I> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Self::Image(i) => i.hash(state),
            Self::Ratio(r) => r.to_bits().hash(state),
        }
    }
}

/// Provides a container that takes on a fixed aspect ratio
/// Thus, can be used to fix the aspect ratio of content if it is set to
/// Length::Fill The aspect ratio may be based on that of an image, in which
/// case the ratio is obtained from the renderer
pub struct AspectRatioContainer<'a, M, R: self::Renderer> {
    max_width: u32,
    max_height: u32,
    aspect_ratio: AspectRatio<R::ImageHandle>,
    content: Element<'a, M, R>,
}

impl<'a, M, R> AspectRatioContainer<'a, M, R>
where
    R: self::Renderer,
{
    pub fn new(content: impl Into<Element<'a, M, R>>) -> Self {
        Self {
            max_width: u32::MAX,
            max_height: u32::MAX,
            aspect_ratio: AspectRatio::Ratio(1.0),
            content: content.into(),
        }
    }

    /// Set the ratio (width/height)
    pub fn ratio(mut self, ratio: f32) -> Self {
        self.aspect_ratio = AspectRatio::Ratio(ratio);
        self
    }

    /// Use the ratio of the provided image
    pub fn ratio_of_image(mut self, handle: R::ImageHandle) -> Self {
        self.aspect_ratio = AspectRatio::Image(handle);
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
}

impl<'a, M, R> Widget<M, R> for AspectRatioContainer<'a, M, R>
where
    R: self::Renderer,
{
    fn width(&self) -> Length { Length::Shrink }

    fn height(&self) -> Length { Length::Shrink }

    fn layout(&self, renderer: &R, limits: &layout::Limits) -> layout::Node {
        let limits = limits
            .loose()
            .max_width(self.max_width)
            .max_height(self.max_height);

        let aspect_ratio = match &self.aspect_ratio {
            AspectRatio::Image(handle) => {
                let (pixel_w, pixel_h) = renderer.dimensions(handle);

                // Just in case
                // could convert to gracefully handling
                debug_assert!(pixel_w != 0);
                debug_assert!(pixel_h != 0);

                pixel_w as f32 / pixel_h as f32
            },
            AspectRatio::Ratio(ratio) => *ratio,
        };

        // We need to figure out the max width/height of the limits
        // and then adjust one down to meet the aspect ratio
        let max_size = limits.max();
        let (max_width, max_height) = (max_size.width as f32, max_size.height as f32);
        let max_aspect_ratio = max_width / max_height;
        let limits = if max_aspect_ratio > aspect_ratio {
            limits.max_width((max_height * aspect_ratio) as u32)
        } else {
            limits.max_height((max_width / aspect_ratio) as u32)
        };

        // Remove fill limits in case one of the parents was Shrink
        let limits = layout::Limits::new(Size::ZERO, limits.max());
        let content = self.content.layout(renderer, &limits);

        layout::Node::with_children(limits.max(), vec![content])
    }

    fn on_event(
        &mut self,
        event: Event,
        layout: Layout<'_>,
        cursor_position: Point,
        messages: &mut Vec<M>,
        renderer: &R,
        clipboard: Option<&dyn Clipboard>,
    ) {
        self.content.on_event(
            event,
            layout.children().next().unwrap(),
            cursor_position,
            messages,
            renderer,
            clipboard,
        );
    }

    fn draw(
        &self,
        renderer: &mut R,
        defaults: &R::Defaults,
        layout: Layout<'_>,
        cursor_position: Point,
    ) -> R::Output {
        renderer.draw(
            defaults,
            layout.bounds(),
            cursor_position,
            &self.content,
            layout.children().next().unwrap(),
        )
    }

    fn hash_layout(&self, state: &mut Hasher) {
        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);

        self.max_width.hash(state);
        self.max_height.hash(state);
        self.aspect_ratio.hash(state);
        // TODO: add pixel dims (need renderer)

        self.content.hash_layout(state);
    }
}

pub trait Renderer: iced::Renderer {
    /// The style supported by this renderer.
    //type Style: Default;
    /// The handle used by this renderer for images.
    type ImageHandle: Hash;

    fn dimensions(&self, handle: &Self::ImageHandle) -> (u32, u32);

    fn draw<M>(
        &mut self,
        defaults: &Self::Defaults,
        bounds: Rectangle,
        cursor_position: Point,
        //style: &Self::Style,
        content: &Element<'_, M, Self>,
        content_layout: Layout<'_>,
    ) -> Self::Output;
}

// They got to live ¯\_(ツ)_/¯
impl<'a, M, R> From<AspectRatioContainer<'a, M, R>> for Element<'a, M, R>
where
    R: 'a + self::Renderer,
    M: 'a,
{
    fn from(widget: AspectRatioContainer<'a, M, R>) -> Element<'a, M, R> { Element::new(widget) }
}
