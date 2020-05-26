use iced::{layout, Clipboard, Element, Event, Hasher, Layout, Length, Point, Size, Widget};
use std::{hash::Hash, u32};

// Note: it might be more efficient to make this generic over the content type

// Note: maybe we could just use the container styling for this (not really with
// the aspect ratio stuff)

#[derive(Copy, Clone, Hash)]
pub struct Padding {
    pub top: u16,
    pub bottom: u16,
    pub right: u16,
    pub left: u16,
}

impl Padding {
    pub fn new() -> Self {
        Padding {
            top: 0,
            bottom: 0,
            right: 0,
            left: 0,
        }
    }

    pub fn top(mut self, pad: u16) -> Self {
        self.top = pad;
        self
    }

    pub fn bottom(mut self, pad: u16) -> Self {
        self.bottom = pad;
        self
    }

    pub fn right(mut self, pad: u16) -> Self {
        self.right = pad;
        self
    }

    pub fn left(mut self, pad: u16) -> Self {
        self.left = pad;
        self
    }

    pub fn vertical(mut self, pad: u16) -> Self {
        self.top = pad;
        self.bottom = pad;
        self
    }

    pub fn horizontal(mut self, pad: u16) -> Self {
        self.left = pad;
        self.right = pad;
        self
    }
}

pub trait Background<R: iced::Renderer>: Sized {
    // The intended implementors already store the state accessed in the three
    // functions below
    fn width(&self) -> Length;
    fn height(&self) -> Length;
    fn aspect_ratio_fixed(&self) -> bool;
    fn pixel_dims(&self, renderer: &R) -> (u16, u16);
    fn draw(
        &self,
        renderer: &mut R,
        defaults: &R::Defaults,
        layout: Layout<'_>,
        cursor_position: Point,
    ) -> R::Output;
}

/// This widget is displays a background image behind it's content
pub struct BackgroundContainer<'a, M, R: self::Renderer, B: Background<R>> {
    //width: Length,
    //height: Length,
    max_width: u32,
    max_height: u32,
    background: B,
    // Padding in same pixel units as background image
    // Scaled relative to the background's scaling
    padding: Padding,
    content: Element<'a, M, R>,
}

impl<'a, M, R, B> BackgroundContainer<'a, M, R, B>
where
    R: self::Renderer,
    B: Background<R>,
{
    pub fn new(background: B, content: impl Into<Element<'a, M, R>>) -> Self {
        Self {
            //width: Length::Shrink,
            //height: Length::Shrink,
            max_width: u32::MAX,
            max_height: u32::MAX,
            background,
            padding: Padding::new(),
            content: content.into(),
        }
    }

    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }

    /*pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }*/

    pub fn max_width(mut self, max_width: u32) -> Self {
        self.max_width = max_width;
        self
    }

    pub fn max_height(mut self, max_height: u32) -> Self {
        self.max_height = max_height;
        self
    }

    // Consider having these wire into underlying background
    /*pub fn fix_aspect_ratio(mut self) -> Self {
        self.fix_aspect_ratio = true;
        self
    }

    pub fn color(mut self, color: Rgba<u8>) -> Self {
        self.color = color;
        self
    }*/
}

impl<'a, M, R, B> Widget<M, R> for BackgroundContainer<'a, M, R, B>
where
    R: self::Renderer,
    B: Background<R>,
{
    // Uses the width and height from the background
    fn width(&self) -> Length { self.background.width() }

    fn height(&self) -> Length { self.background.height() }

    fn layout(&self, renderer: &R, limits: &layout::Limits) -> layout::Node {
        let limits = limits
            .loose() // why does iced's container do this?
            .max_width(self.max_width)
            .max_height(self.max_height)
            .width(self.width())
            .height(self.height());

        let (pixel_w, pixel_h) = self.background.pixel_dims(renderer);
        let (horizontal_pad_frac, vertical_pad_frac, top_pad_frac, left_pad_frac) = {
            let Padding {
                top,
                bottom,
                right,
                left,
            } = self.padding;
            // Just in case
            // could convert to gracefully handling
            debug_assert!(pixel_w != 0);
            debug_assert!(pixel_h != 0);
            debug_assert!(top + bottom < pixel_h);
            debug_assert!(right + left < pixel_w);
            (
                (right + left) as f32 / pixel_w as f32,
                (top + bottom) as f32 / pixel_h as f32,
                top as f32 / pixel_h as f32,
                left as f32 / pixel_w as f32,
            )
        };

        let (size, content) = if self.background.aspect_ratio_fixed() {
            // To fix the aspect ratio we have to have a separate layout from the content
            // because we can't force the content to have a specific aspect ratio
            let aspect_ratio = pixel_w as f32 / pixel_h as f32;

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
            // Account for padding at max size in the limits for the children
            let limits = limits.shrink({
                let max = limits.max();
                Size::new(
                    max.width * horizontal_pad_frac,
                    max.height * vertical_pad_frac,
                )
            });

            // Get content size
            // again, why is loose() used here?
            let mut content = self.content.layout(renderer, &limits.loose());

            // TODO: handle cases where self and/or children are not Length::Fill
            // If fill use max_size
            //if match self.width(), self.height()

            // This time we need to adjust up to meet the aspect ratio
            // so that the container is larger than the contents
            let mut content_size = content.size();
            // Add minimum padding to content size (this works to ensure we have enough
            // space for padding because the available space can only increase)
            content_size.width /= 1.0 - horizontal_pad_frac;
            content_size.height /= 1.0 - vertical_pad_frac;
            let content_aspect_ratio = content_size.width as f32 / content_size.height as f32;
            let size = if content_aspect_ratio > aspect_ratio {
                Size::new(content_size.width, content_size.width / aspect_ratio)
            } else {
                Size::new(content_size.height * aspect_ratio, content_size.height)
            };

            // Move content to account for padding
            content.move_to(Point::new(
                left_pad_frac * size.width,
                top_pad_frac * size.height,
            ));

            (size, content)
        } else {
            // Account for padding at max size in the limits for the children
            let limits = limits
                .shrink({
                    let max = limits.max();
                    Size::new(
                        max.width * horizontal_pad_frac,
                        max.height * vertical_pad_frac,
                    )
                })
                .loose(); // again, why is loose() used here?

            let mut content = self.content.layout(renderer, &limits);

            let mut size = limits.resolve(content.size());
            // Add padding back
            size.width /= 1.0 - horizontal_pad_frac;
            size.height /= 1.0 - vertical_pad_frac;

            // Move to account for padding
            content.move_to(Point::new(
                left_pad_frac * size.width,
                top_pad_frac * size.height,
            ));
            // No aligning since child is currently assumed to be fill

            (size, content)
        };

        layout::Node::with_children(size, vec![content])
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
            &self.background,
            layout,
            &self.content,
            layout.children().next().unwrap(),
            cursor_position,
        )
    }

    fn hash_layout(&self, state: &mut Hasher) {
        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);

        self.width().hash(state);
        self.height().hash(state);
        self.max_width.hash(state);
        self.max_height.hash(state);
        self.background.aspect_ratio_fixed().hash(state);
        self.padding.hash(state);
        // TODO: add pixel dims (need renderer)

        self.content.hash_layout(state);
    }
}

pub trait Renderer: iced::Renderer {
    fn draw<M, B>(
        &mut self,
        defaults: &Self::Defaults,
        background: &B,
        background_layout: Layout<'_>,
        content: &Element<'_, M, Self>,
        content_layout: Layout<'_>,
        cursor_position: Point,
    ) -> Self::Output
    where
        B: Background<Self>;
}

// They got to live ¯\_(ツ)_/¯
impl<'a, M: 'a, R: 'a, B> From<BackgroundContainer<'a, M, R, B>> for Element<'a, M, R>
where
    R: self::Renderer,
    B: 'a + Background<R>,
{
    fn from(background_container: BackgroundContainer<'a, M, R, B>) -> Element<'a, M, R> {
        Element::new(background_container)
    }
}
