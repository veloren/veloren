use iced::{
    layout, mouse, Clipboard, Element, Event, Hasher, Layout, Length, Point, Rectangle, Size,
    Widget,
};
use std::hash::Hash;

#[derive(Debug, Default)]
pub struct State {
    mouse_over: bool,
}
impl State {
    pub fn mouse_over(&self) -> bool { self.mouse_over }
}

#[derive(Debug)]
pub struct MouseDetector<'a> {
    width: Length,
    height: Length,
    state: &'a mut State,
}

impl<'a> MouseDetector<'a> {
    pub fn new(state: &'a mut State, width: Length, height: Length) -> Self {
        Self {
            width,
            height,
            state,
        }
    }
}

impl<'a, M, R> Widget<M, R> for MouseDetector<'a>
where
    R: Renderer,
{
    fn width(&self) -> Length { self.width }

    fn height(&self) -> Length { self.height }

    fn layout(&self, _renderer: &R, limits: &layout::Limits) -> layout::Node {
        let limits = limits.width(self.width).height(self.height);

        layout::Node::new(limits.resolve(Size::ZERO))
    }

    fn draw(
        &self,
        renderer: &mut R,
        _defaults: &R::Defaults,
        layout: Layout<'_>,
        _cursor_position: Point,
        _viewport: &Rectangle,
    ) -> R::Output {
        renderer.draw(layout.bounds())
    }

    fn hash_layout(&self, state: &mut Hasher) {
        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);

        self.width.hash(state);
        self.height.hash(state);
    }

    fn on_event(
        &mut self,
        event: Event,
        layout: Layout<'_>,
        _cursor_position: Point,
        _renderer: &R,
        _clipboard: &mut dyn Clipboard,
        _messages: &mut Vec<M>,
    ) -> iced::event::Status {
        if let Event::Mouse(mouse::Event::CursorMoved {
            position: Point { x, y },
        }) = event
        {
            let bounds = layout.bounds();
            let mouse_over = x > bounds.x
                && x < bounds.x + bounds.width
                && y > bounds.y
                && y < bounds.y + bounds.height;
            if mouse_over != self.state.mouse_over {
                self.state.mouse_over = mouse_over;
            }
        }

        iced::event::Status::Ignored
    }
}

pub trait Renderer: iced::Renderer {
    fn draw(&mut self, bounds: Rectangle) -> Self::Output;
}

impl<'a, M, R> From<MouseDetector<'a>> for Element<'a, M, R>
where
    R: Renderer,
    M: 'a,
{
    fn from(mouse_detector: MouseDetector<'a>) -> Element<'a, M, R> { Element::new(mouse_detector) }
}
