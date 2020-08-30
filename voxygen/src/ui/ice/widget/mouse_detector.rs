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
    //on_enter: M,
    //on_exit: M,
    state: &'a mut State,
}

impl<'a> MouseDetector<'a> {
    pub fn new(
        state: &'a mut State,
        //on_enter: M,
        //on_exit: M,
        width: Length,
        height: Length,
    ) -> Self {
        Self {
            state,
            //on_enter,
            //on_exit,
            width,
            height,
        }
    }
}

impl<'a, M, R> Widget<M, R> for MouseDetector<'a>
where
    R: self::Renderer,
{
    fn width(&self) -> Length { self.width }

    fn height(&self) -> Length { self.height }

    fn layout(&self, _renderer: &R, limits: &layout::Limits) -> layout::Node {
        let limits = limits.width(self.width).height(self.height);

        layout::Node::new(limits.resolve(Size::ZERO))
    }

    fn on_event(
        &mut self,
        event: Event,
        layout: Layout<'_>,
        _cursor_position: Point,
        _messages: &mut Vec<M>,
        _renderer: &R,
        _clipboard: Option<&dyn Clipboard>,
    ) {
        if let Event::Mouse(mouse::Event::CursorMoved { x, y }) = event {
            let bounds = layout.bounds();
            let mouse_over = x > bounds.x
                && x < bounds.x + bounds.width
                && y > bounds.y
                && y < bounds.y + bounds.height;
            if mouse_over != self.state.mouse_over {
                self.state.mouse_over = mouse_over;

                /*messages.push(if mouse_over {
                    self.on_enter.clone()
                } else {
                    self.on_exit.clone()
                });*/
            }
        }
    }

    fn draw(
        &self,
        renderer: &mut R,
        _defaults: &R::Defaults,
        layout: Layout<'_>,
        _cursor_position: Point,
    ) -> R::Output {
        renderer.draw(layout.bounds())
    }

    fn hash_layout(&self, state: &mut Hasher) {
        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);

        self.width.hash(state);
        self.height.hash(state);
    }
}

pub trait Renderer: iced::Renderer {
    fn draw(&mut self, bounds: Rectangle) -> Self::Output;
}

impl<'a, M, R> From<MouseDetector<'a>> for Element<'a, M, R>
where
    R: self::Renderer,
    M: 'a,
{
    fn from(mouse_detector: MouseDetector<'a>) -> Element<'a, M, R> { Element::new(mouse_detector) }
}
