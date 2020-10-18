use iced::{
    layout, mouse, Align, Clipboard, Element, Event, Hasher, Layout, Length, Point, Rectangle,
    Size, Widget,
};
use std::time::Instant;
use std::time::Duration;
use std::hash::Hash;
use vek::*;

#[derive(Copy, Clone)]
struct Hover {
    start: Instant,
    aabr: Aabr<i32>,
}

impl Hover {
    fn start(aabr: Aabr<i32>) -> Self {
        Self {
            start: Instant::now(),
            aabr,
        }
    }
}

#[derive(Copy, Clone)]
struct Show {
    hover_pos: Vec2<i32>,
    aabr: Aabr<i32>,
}

#[derive(Copy, Clone)]
enum State {
    Idle,
    Start(Hover),
    Showing(Show),
    Fading(Instant, Show, Option<Hover>)
}

// Reports which widget the mouse is over
pub struct Update((Aabr<i32>, Vec2<i32>));

pub struct TooltipManager {
    state: State,
    hover_widget: Option<Aabr<i32>>,
    hover_pos: Vec2<i32>,
    // How long before a tooltip is displayed when hovering
    hover_dur: Duration,
    // How long it takes a tooltip to disappear
    fade_dur: Duration,
}

impl TooltipManager {
    pub fn new(hover_dur: Duration, fade_dur: Duration) -> Self {
        Self {
            state: State::Idle,
            hover_widget: None,
            hover_pos: Default::default(),
            hover_dur,
            fade_dur,
        }
    }
    /// Call this at the top of your view function or for minimum latency at the end of message
    /// handling
    /// Only call this once per frame as it assumes no updates being received between calls means
    /// that there is no tooltipped widget currently being hovered
    pub fn maintain(&mut self) {
        // Handle changes based on pointer moving
        self.state = if let Some(aabr) = self.hover_widget.take() {
            match self.state {
                State::Idle => State::Start(Hover::start(aabr)),
                State::Start(hover) if hover.aabr != aabr => State::Start(Hover::start(aabr)),
                State::Start(hover) => State::Start(hover),
                State::Showing(show) if show.aabr != aabr => State::Fading(Instant::now(), show, Some(Hover::start(aabr))),
                State::Showing(show) => State::Showing(show),
                State::Fading(start, show, Some(hover)) if hover.aabr == aabr => State::Fading(start, show, Some(hover)),
                State::Fading(start, show, _) => State::Fading(start, show, Some(Hover::start(aabr))),
            }
        } else {
            match self.state {
                State::Idle | State::Start(_) => State::Idle,
                State::Showing(show) => State::Fading(Instant::now(), show, None),
                State::Fading(start, show, _) => State::Fading(start, show, None),
            }
        };

        // Handle temporal changes
        self.state = match self.state {
            State::Start(Hover { start, aabr}) | State::Fading(_, _, Some(Hover { start, aabr })) if start.elapsed() >= self.hover_dur =>
                State::Showing(Show { aabr, hover_pos: self.hover_pos }),
            State::Fading(start, _, hover) if start.elapsed() >= self.fade_dur => match hover {
                Some(hover) => State::Start(hover),
                None => State::Idle,
            },
            state @ State::Idle | state @ State::Start(_) | state @ State::Showing(_) | state @ State::Fading(_, _, _) => state,
        }
    }

    pub fn update(&mut self, update: Update) {
        self.hover_widget = Some(update.0.0);
        self.hover_pos = update.0.1;
    }

    pub fn showing(&self, aabr: Aabr<i32>) -> bool {
        match self.state {
            State::Idle | State::Start(_) => false,
            State::Showing(show) | State::Fading(_, show, _) => show.aabr == aabr,
        }
    }
}


/// A widget used to display tooltips when the content element is hovered
pub struct Tooltip<'a, M, R: iced::Renderer> {
    content: Element<'a, M, R>,
    hover_content: Option<Box<dyn 'a + FnOnce() -> Element<'a, M, R>>>,
    on_update: Box<dyn Fn(Update) -> M>,
    manager: &'a TooltipManager,
}

impl<'a, M, R> Tooltip<'a, M, R>
where
    R: iced::Renderer,
{
    pub fn new<C, H, F>(content: C, hover_content: H, on_update: F, manager: &'a TooltipManager) -> Self
    where
        C: Into<Element<'a, M, R>>,
        H: 'a + FnOnce() -> Element<'a, M, R>,
        F: 'static + Fn(Update) -> M,
    {
        Self {
            content: content.into(),
            hover_content: Some(Box::new(hover_content)),
            on_update: Box::new(on_update),
            manager,
        }
    }
}

impl<'a, M, R> Widget<M, R> for Tooltip<'a, M, R>
where
    R: iced::Renderer,
{
    fn width(&self) -> Length { self.content.width() }

    fn height(&self) -> Length { self.content.height() }

    fn layout(&self, renderer: &R, limits: &layout::Limits) -> layout::Node {
        self.content.layout(renderer, limits)
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
        let bounds = layout.bounds();
        if bounds.contains(cursor_position) {
            let aabr = aabr_from_bounds(bounds);
            let m_pos = Vec2::new(cursor_position.x.trunc() as i32, cursor_position.y.trunc() as i32);
            messages.push((self.on_update)(Update((aabr, m_pos))));
        }

        self.content.on_event(
            event,
            layout,
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
        self.content.draw(
            renderer,
            defaults,
            layout,
            cursor_position,
        )
    }

    fn overlay(
        &mut self,
        layout: Layout<'_>,
    ) -> Option<iced::overlay::Element<'_, M, R>> {
        let bounds = layout.bounds();
        let aabr = aabr_from_bounds(bounds);

        self.manager.showing(aabr)
            .then(|| self.hover_content.take())
            .flatten()
            .map(|content| iced::overlay::Element::new(
                Point { x: self.manager.hover_pos.x as f32, y: self.manager.hover_pos.y as f32 },
                Box::new(Overlay::new(
                    content(),
                    bounds,
                ))
            ))

    }

    fn hash_layout(&self, state: &mut Hasher) {
        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);
        self.content.hash_layout(state);
    }
}

impl<'a, M, R> From<Tooltip<'a, M, R>> for Element<'a, M, R>
where
    R: 'a + iced::Renderer,
    M: 'a,
{
    fn from(tooltip: Tooltip<'a, M, R>) -> Element<'a, M, R> { Element::new(tooltip) }
}

fn aabr_from_bounds(bounds: iced::Rectangle) -> Aabr<i32> {
    let min = Vec2::new(bounds.x.trunc() as i32, bounds.y.trunc() as i32);
    let max = min + Vec2::new(bounds.width.trunc() as i32, bounds.height.trunc() as i32);
    Aabr { min, max }
}

struct Overlay<'a, M, R: iced::Renderer> {
    content: Element<'a, M, R>,
    /// Area to avoid overlapping with
    avoid: Rectangle,
}

impl<'a, M, R: iced::Renderer> Overlay<'a, M, R>
{
    pub fn new(content: Element<'a, M, R>, avoid: Rectangle) -> Self {
        Self { content, avoid }
    }
}

impl<'a, M, R> iced::Overlay<M, R>
    for Overlay<'a, M, R>
where
    R: iced::Renderer,
{
    fn layout(
        &self,
        renderer: &R,
        bounds: Size,
        position: Point,
    ) -> layout::Node {
        // TODO: Avoid avoid area
        let space_below = bounds.height - position.y;
        let space_above = position.y;

        let limits = layout::Limits::new(
            Size::ZERO,
            Size::new(
                bounds.width - position.x,
                if space_below > space_above {
                    space_below
                } else {
                    space_above
                },
            ),
        )
        .width(self.content.width());

        let mut node = self.content.layout(renderer, &limits);

        node.move_to(position - iced::Vector::new(0.0, node.size().height));

        node
    }

    fn hash_layout(&self, state: &mut Hasher, position: Point) {
        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);

        (position.x as u32).hash(state);
        (position.y as u32).hash(state);
        self.content.hash_layout(state);
    }

    fn draw(
        &self,
        renderer: &mut R,
        defaults: &R::Defaults,
        layout: Layout<'_>,
        cursor_position: Point,
    ) -> R::Output {
        self.content.draw(renderer, defaults, layout, cursor_position)
    }
}
