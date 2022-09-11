use iced::{
    layout, Clipboard, Element, Event, Hasher, Layout, Length, Point, Rectangle, Size, Widget,
};
use std::{
    hash::Hash,
    sync::Mutex,
    time::{Duration, Instant},
};
use vek::*;

#[derive(Copy, Clone, Debug)]
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

#[derive(Copy, Clone, Debug)]
struct Show {
    hover_pos: Vec2<i32>,
    aabr: Aabr<i32>,
}

#[derive(Copy, Clone, Debug)]
enum State {
    Idle,
    Start(Hover),
    Showing(Show),
    Fading(Instant, Show, Option<Hover>),
}

// Reports which widget the mouse is over
#[derive(Copy, Clone, Debug)]
struct Update((Aabr<i32>, Vec2<i32>));

#[derive(Debug)]
// TODO: consider moving all this state into the Renderer
pub struct TooltipManager {
    state: State,
    update: Mutex<Option<Update>>,
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
            update: Mutex::new(None),
            hover_pos: Default::default(),
            hover_dur,
            fade_dur,
        }
    }

    /// Call this at the top of your view function or for minimum latency at the
    /// end of message handling
    /// that there is no tooltipped widget currently being hovered
    pub fn maintain(&mut self) {
        let update = self.update.get_mut().unwrap().take();
        // Handle changes based on pointer moving
        self.state = if let Some(Update((aabr, hover_pos))) = update {
            self.hover_pos = hover_pos;
            match self.state {
                State::Idle => State::Start(Hover::start(aabr)),
                State::Start(hover) if hover.aabr != aabr => State::Start(Hover::start(aabr)),
                State::Start(hover) => State::Start(hover),
                State::Showing(show) if show.aabr != aabr => {
                    State::Fading(Instant::now(), show, Some(Hover::start(aabr)))
                },
                State::Showing(show) => State::Showing(show),
                State::Fading(start, show, Some(hover)) if hover.aabr == aabr => {
                    State::Fading(start, show, Some(hover))
                },
                State::Fading(start, show, _) => {
                    State::Fading(start, show, Some(Hover::start(aabr)))
                },
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
            State::Start(Hover { start, aabr })
            | State::Fading(_, _, Some(Hover { start, aabr }))
                if start.elapsed() >= self.hover_dur =>
            {
                State::Showing(Show {
                    aabr,
                    hover_pos: self.hover_pos,
                })
            },
            State::Fading(start, _, hover) if start.elapsed() >= self.fade_dur => match hover {
                Some(hover) => State::Start(hover),
                None => State::Idle,
            },
            state @ State::Idle
            | state @ State::Start(_)
            | state @ State::Showing(_)
            | state @ State::Fading(_, _, _) => state,
        };
    }

    fn update(&self, update: Update) { *self.update.lock().unwrap() = Some(update); }

    /// Returns an options with the position of the cursor when the tooltip
    /// started being show and the transparency if it is fading
    fn showing(&self, aabr: Aabr<i32>) -> Option<(Point, f32)> {
        match self.state {
            State::Idle | State::Start(_) => None,
            State::Showing(show) => (show.aabr == aabr).then_some({
                (
                    Point {
                        x: show.hover_pos.x as f32,
                        y: show.hover_pos.y as f32,
                    },
                    1.0,
                )
            }),
            State::Fading(start, show, _) => (show.aabr == aabr)
                .then(|| {
                    (
                        Point {
                            x: show.hover_pos.x as f32,
                            y: show.hover_pos.y as f32,
                        },
                        1.0 - start.elapsed().as_secs_f32() / self.fade_dur.as_secs_f32(),
                    )
                })
                .filter(|(_, fade)| *fade > 0.0),
        }
    }
}

/// A widget used to display tooltips when the content element is hovered
pub struct Tooltip<'a, M, R: Renderer> {
    content: Element<'a, M, R>,
    hover_content: Box<dyn 'a + FnMut() -> Element<'a, M, R>>,
    manager: &'a TooltipManager,
}

impl<'a, M, R> Tooltip<'a, M, R>
where
    R: Renderer,
{
    pub fn new<C, H>(content: C, hover_content: H, manager: &'a TooltipManager) -> Self
    where
        C: Into<Element<'a, M, R>>,
        H: 'a + FnMut() -> Element<'a, M, R>,
    {
        Self {
            content: content.into(),
            hover_content: Box::new(hover_content),
            manager,
        }
    }
}

impl<'a, M, R> Widget<M, R> for Tooltip<'a, M, R>
where
    R: Renderer,
{
    fn width(&self) -> Length { self.content.width() }

    fn height(&self) -> Length { self.content.height() }

    fn layout(&self, renderer: &R, limits: &layout::Limits) -> layout::Node {
        self.content.layout(renderer, limits)
    }

    fn draw(
        &self,
        renderer: &mut R,
        defaults: &R::Defaults,
        layout: Layout<'_>,
        cursor_position: Point,
        viewport: &Rectangle,
    ) -> R::Output {
        let bounds = layout.bounds();
        if bounds.contains(cursor_position) {
            // TODO: these bounds aren't actually global (for example see how the Scrollable
            // widget handles its content) so it's not actually a good key to
            // use here
            let aabr = aabr_from_bounds(bounds);
            let m_pos = Vec2::new(
                cursor_position.x.trunc() as i32,
                cursor_position.y.trunc() as i32,
            );
            self.manager.update(Update((aabr, m_pos)));
        }

        self.content
            .draw(renderer, defaults, layout, cursor_position, viewport)
    }

    fn hash_layout(&self, state: &mut Hasher) {
        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);
        self.content.hash_layout(state);
    }

    fn on_event(
        &mut self,
        event: Event,
        layout: Layout<'_>,
        cursor_position: Point,
        renderer: &R,
        clipboard: &mut dyn Clipboard,
        messages: &mut Vec<M>,
    ) -> iced::event::Status {
        self.content.on_event(
            event,
            layout,
            cursor_position,
            renderer,
            clipboard,
            messages,
        )
    }

    fn overlay(&mut self, layout: Layout<'_>) -> Option<iced::overlay::Element<'_, M, R>> {
        let bounds = layout.bounds();
        let aabr = aabr_from_bounds(bounds);

        self.manager.showing(aabr).map(|(cursor_pos, alpha)| {
            iced::overlay::Element::new(
                Point::ORIGIN,
                Box::new(Overlay::new(
                    (self.hover_content)(),
                    cursor_pos,
                    bounds,
                    alpha,
                )),
            )
        })
    }
}

impl<'a, M, R> From<Tooltip<'a, M, R>> for Element<'a, M, R>
where
    R: 'a + Renderer,
    M: 'a,
{
    fn from(tooltip: Tooltip<'a, M, R>) -> Element<'a, M, R> { Element::new(tooltip) }
}

fn aabr_from_bounds(bounds: Rectangle) -> Aabr<i32> {
    let min = Vec2::new(bounds.x.trunc() as i32, bounds.y.trunc() as i32);
    let max = min + Vec2::new(bounds.width.trunc() as i32, bounds.height.trunc() as i32);
    Aabr { min, max }
}

struct Overlay<'a, M, R: Renderer> {
    content: Element<'a, M, R>,
    /// Cursor position
    cursor_position: Point,
    /// Area to avoid overlapping with
    avoid: Rectangle,
    /// Alpha for fading out
    alpha: f32,
}

impl<'a, M, R: Renderer> Overlay<'a, M, R> {
    pub fn new(
        content: Element<'a, M, R>,
        cursor_position: Point,
        avoid: Rectangle,
        alpha: f32,
    ) -> Self {
        Self {
            content,
            cursor_position,
            avoid,
            alpha,
        }
    }
}

impl<'a, M, R> iced::Overlay<M, R> for Overlay<'a, M, R>
where
    R: Renderer,
{
    fn layout(&self, renderer: &R, bounds: Size, position: Point) -> layout::Node {
        let avoid = Rectangle {
            x: self.avoid.x + position.x,
            y: self.avoid.y + position.y,
            ..self.avoid
        };
        let cursor_position = Point {
            x: self.cursor_position.x + position.x,
            y: self.cursor_position.y + position.y,
        };

        const PAD: f32 = 8.0; // TODO: allow configuration
        let space_above = (avoid.y - PAD).max(0.0);
        let space_below = (bounds.height - avoid.y - avoid.height - PAD).max(0.0);

        let limits = layout::Limits::new(
            Size::ZERO,
            Size::new(bounds.width, space_above.max(space_below)),
        );

        let mut node = self.content.layout(renderer, &limits);

        let size = node.size();

        node.move_to(Point {
            x: (bounds.width - size.width).min(cursor_position.x),
            y: if space_above >= space_below {
                avoid.y - size.height - PAD
            } else {
                avoid.y + avoid.height + PAD
            },
        });

        node
    }

    fn draw(
        &self,
        renderer: &mut R,
        defaults: &R::Defaults,
        layout: Layout<'_>,
        cursor_position: Point,
    ) -> R::Output {
        renderer.draw(
            self.alpha,
            defaults,
            cursor_position,
            &layout.bounds(),
            &self.content,
            layout,
        )
    }

    fn hash_layout(&self, state: &mut Hasher, position: Point) {
        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);

        (position.x as u32).hash(state);
        (position.y as u32).hash(state);
        (self.cursor_position.x as u32).hash(state);
        (self.avoid.x as u32).hash(state);
        (self.avoid.y as u32).hash(state);
        (self.avoid.height as u32).hash(state);
        (self.avoid.width as u32).hash(state);
        self.content.hash_layout(state);
    }
}

pub trait Renderer: iced::Renderer {
    fn draw<M>(
        &mut self,
        alpha: f32,
        defaults: &Self::Defaults,
        cursor_position: Point,
        viewport: &Rectangle,
        content: &Element<'_, M, Self>,
        content_layout: Layout<'_>,
    ) -> Self::Output;
}
