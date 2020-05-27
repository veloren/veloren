use iced::{layout, Element, Hasher, Layout, Length, Point, Size, Widget};
use std::hash::Hash;

const DEFAULT_FILL_FRACTION: f32 = 1.0;
const DEFAULT_VERTICAL_ADJUSTMENT: f32 = 0.05;

/// Wraps the existing Text widget giving it more advanced layouting
/// capabilities
/// Centers child text widget and adjust the font size depending on the height
/// of the limits Assumes single line text is being used
pub struct FillText<R>
where
    R: iced::text::Renderer,
{
    //max_font_size: u16, uncomment if there is a use case for this
    /// Portion of the height of the limits which the font size should be
    fill_fraction: f32,
    /// Adjustment factor to center the text vertically
    /// Multiplied by font size and used to move the text up if positive
    // TODO: use the produced glyph geometry directly to do this and/or add support to
    // layouting library
    vertical_adjustment: f32,
    text: iced::Text<R>,
}

impl<R> FillText<R>
where
    R: iced::text::Renderer,
{
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            //max_font_size: u16::MAX,
            fill_fraction: DEFAULT_FILL_FRACTION,
            vertical_adjustment: DEFAULT_VERTICAL_ADJUSTMENT,
            text: iced::Text::new(label),
        }
    }

    pub fn fill_fraction(mut self, fraction: f32) -> Self {
        self.fill_fraction = fraction;
        self
    }

    pub fn vertical_adjustment(mut self, adjustment: f32) -> Self {
        self.vertical_adjustment = adjustment;
        self
    }

    pub fn color(mut self, color: impl Into<iced::Color>) -> Self {
        self.text = self.text.color(color);
        self
    }

    pub fn font(mut self, font: impl Into<R::Font>) -> Self {
        self.text = self.text.font(font);
        self
    }
}

impl<M, R> Widget<M, R> for FillText<R>
where
    R: iced::text::Renderer,
{
    fn width(&self) -> Length { Length::Fill }

    fn height(&self) -> Length { Length::Fill }

    fn layout(&self, renderer: &R, limits: &layout::Limits) -> layout::Node {
        let limits = limits.width(Length::Fill).height(Length::Fill);

        let size = limits.max();

        let font_size = (size.height * self.fill_fraction) as u16;

        let mut text =
            Widget::<M, _>::layout(&self.text.clone().size(font_size), renderer, &limits);

        // Size adjusted for centering
        text.align(
            iced::Align::Center,
            iced::Align::Center,
            Size::new(
                size.width,
                size.height - 2.0 * font_size as f32 * self.vertical_adjustment,
            ),
        );

        layout::Node::with_children(size, vec![text])
    }

    fn draw(
        &self,
        renderer: &mut R,
        defaults: &R::Defaults,
        layout: Layout<'_>,
        cursor_position: Point,
    ) -> R::Output {
        // Note: this breaks if the parent widget adjusts the bounds height
        let font_size = (layout.bounds().height * self.fill_fraction) as u16;
        Widget::<M, _>::draw(
            &self.text.clone().size(font_size),
            renderer,
            defaults,
            layout.children().next().unwrap(),
            cursor_position,
        )
    }

    fn hash_layout(&self, state: &mut Hasher) {
        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);

        self.fill_fraction.to_bits().hash(state);
        self.vertical_adjustment.to_bits().hash(state);
        Widget::<M, R>::hash_layout(&self.text, state);
    }
}

impl<'a, M, R> From<FillText<R>> for Element<'a, M, R>
where
    R: 'a + iced::text::Renderer,
{
    fn from(fill_text: FillText<R>) -> Element<'a, M, R> { Element::new(fill_text) }
}
