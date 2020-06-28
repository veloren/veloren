use iced::{
    layout, mouse, Align, Clipboard, Element, Event, Hasher, Layout, Length, Point, Rectangle,
    Size, Widget,
};
use std::hash::Hash;

/// A widget used to overlay one widget on top of another
/// Layout behaves similar to the iced::Container widget
/// Manages filtering out mouse input for the back widget if the mouse is over
/// the front widget
/// Alignment and padding is used for the front widget
pub struct Overlay<'a, M, R: self::Renderer> {
    padding: u16,
    width: Length,
    height: Length,
    max_width: u32,
    max_height: u32,
    horizontal_alignment: Align,
    vertical_alignment: Align,
    over: Element<'a, M, R>,
    under: Element<'a, M, R>,
    // add style etc as needed
}

impl<'a, M, R> Overlay<'a, M, R>
where
    R: self::Renderer,
{
    pub fn new<O, U>(over: O, under: U) -> Self
    where
        O: Into<Element<'a, M, R>>,
        U: Into<Element<'a, M, R>>,
    {
        Self {
            padding: 0,
            width: Length::Shrink,
            height: Length::Shrink,
            max_width: u32::MAX,
            max_height: u32::MAX,
            horizontal_alignment: Align::Start,
            vertical_alignment: Align::Start,
            over: over.into(),
            under: under.into(),
        }
    }

    pub fn padding(mut self, pad: u16) -> Self {
        self.padding = pad;
        self
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

    pub fn align_x(mut self, align_x: Align) -> Self {
        self.horizontal_alignment = align_x;
        self
    }

    pub fn align_y(mut self, align_y: Align) -> Self {
        self.vertical_alignment = align_y;
        self
    }

    pub fn center_x(mut self) -> Self {
        self.horizontal_alignment = Align::Center;
        self
    }

    pub fn center_y(mut self) -> Self {
        self.vertical_alignment = Align::Center;
        self
    }
}

impl<'a, M, R> Widget<M, R> for Overlay<'a, M, R>
where
    R: self::Renderer,
{
    fn width(&self) -> Length { self.width }

    fn height(&self) -> Length { self.height }

    fn layout(&self, renderer: &R, limits: &layout::Limits) -> layout::Node {
        let padding = self.padding as f32;

        let limits = limits
            .loose()
            .max_width(self.max_width)
            .max_height(self.max_height)
            .width(self.width)
            .height(self.height);

        let under = self.under.layout(renderer, &limits.loose());
        let under_size = under.size();

        let limits = limits.pad(padding);
        let mut over = self.over.layout(renderer, &limits.loose());
        let over_size = over.size();

        let size = limits.resolve(Size {
            width: under_size.width.max(over_size.width),
            height: under_size.height.max(over_size.height),
        });

        over.move_to(Point::new(padding, padding));
        over.align(self.horizontal_alignment, self.vertical_alignment, size);

        layout::Node::with_children(size.pad(padding), vec![over, under])
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
        self.over.on_event(
            event.clone(),
            layout,
            cursor_position,
            messages,
            renderer,
            clipboard,
        );

        // If mouse press check if over the overlay widget before sending to under
        // widget
        if !matches!(&event, Event::Mouse(mouse::Event::ButtonPressed(_)))
            || !layout
                .children()
                .next()
                .unwrap()
                .bounds()
                .contains(cursor_position)
        {
            self.under.on_event(
                event,
                layout,
                cursor_position,
                messages,
                renderer,
                clipboard,
            );
        }
    }

    fn draw(
        &self,
        renderer: &mut R,
        defaults: &R::Defaults,
        layout: Layout<'_>,
        cursor_position: Point,
    ) -> R::Output {
        let mut children = layout.children();
        renderer.draw(
            defaults,
            layout.bounds(),
            cursor_position,
            &self.over,
            children.next().unwrap(),
            &self.under,
            children.next().unwrap(),
        )
    }

    fn hash_layout(&self, state: &mut Hasher) {
        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);

        self.padding.hash(state);
        self.width.hash(state);
        self.height.hash(state);
        self.max_width.hash(state);
        self.max_height.hash(state);

        self.over.hash_layout(state);
        self.under.hash_layout(state);
    }
}

pub trait Renderer: iced::Renderer {
    fn draw<M>(
        &mut self,
        defaults: &Self::Defaults,
        bounds: Rectangle,
        cursor_position: Point,
        //style: &self::Style,
        over: &Element<'_, M, Self>,
        over_layout: Layout<'_>,
        under: &Element<'_, M, Self>,
        under_layout: Layout<'_>,
    ) -> Self::Output;
}

impl<'a, M, R> From<Overlay<'a, M, R>> for Element<'a, M, R>
where
    R: 'a + self::Renderer,
    M: 'a,
{
    fn from(overlay: Overlay<'a, M, R>) -> Element<'a, M, R> { Element::new(overlay) }
}
