use conrod_core::{widget, Position, Sizeable, Ui, UiCell, Widget, WidgetCommon};
use vek::*;

#[derive(Clone, WidgetCommon)]
pub struct Ingame<W> {
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    widget: W,
    prim_num: usize,
    pos: Vec3<f32>,
}

pub trait Ingameable: Widget + Sized {
    fn prim_count(&self) -> usize;
    // Note this is not responsible for the 3d positioning
    // Only call this directly if using IngameAnchor
    fn set_ingame(self, id: widget::Id, ui: &mut UiCell) -> Self::Event {
        self
            // should pass focus to the window if these are clicked
            // (they are not displayed where conrod thinks they are)
            .graphics_for(ui.window)
            .set(id, ui)
    }
    fn position_ingame(self, pos: Vec3<f32>) -> Ingame<Self> {
        Ingame::new(pos, self)
    }
}

pub trait PrimitiveMarker {}
impl PrimitiveMarker for widget::Line {}
impl PrimitiveMarker for widget::Image {}
impl<I> PrimitiveMarker for widget::PointPath<I> {}
impl PrimitiveMarker for widget::Circle {}
impl<S> PrimitiveMarker for widget::Oval<S> {}
impl<I> PrimitiveMarker for widget::Polygon<I> {}
impl PrimitiveMarker for widget::Rectangle {}
impl<S, I> PrimitiveMarker for widget::Triangles<S, I> {}
impl<'a> PrimitiveMarker for widget::Text<'a> {}

impl<P> Ingameable for P
where
    P: Widget + PrimitiveMarker,
{
    fn prim_count(&self) -> usize {
        1
    }
}

// All ingame widgets are now fixed scale
#[derive(Copy, Clone, PartialEq)]
pub struct IngameParameters {
    // Number of primitive widgets to position in the game at the specified position
    // Note this could be more than the number of widgets in the widgets field since widgets can contain widgets
    pub num: usize,
    pub pos: Vec3<f32>,
    pub dims: Vec2<f32>,
}

pub struct State {
    id: Option<widget::Id>,
    pub parameters: IngameParameters,
}

pub type Style = ();

impl<W: Ingameable> Ingame<W> {
    pub fn new(pos: Vec3<f32>, widget: W) -> Self {
        Self {
            common: widget::CommonBuilder::default(),
            prim_num: widget.prim_count(),
            pos,
            widget,
        }
    }
}

impl<W: Ingameable> Widget for Ingame<W> {
    type State = State;
    type Style = Style;
    type Event = W::Event;

    fn init_state(&self, mut id_gen: widget::id::Generator) -> Self::State {
        State {
            id: Some(id_gen.next()),
            parameters: IngameParameters {
                num: self.prim_num,
                pos: self.pos,
                dims: Vec2::zero(),
            },
        }
    }

    fn style(&self) -> Self::Style {
        ()
    }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;
        let Ingame {
            widget,
            prim_num,
            pos,
            ..
        } = self;

        let parameters = IngameParameters {
            num: prim_num,
            pos,
            dims: Vec2::<f64>::from(widget.get_wh(ui).unwrap_or([1.0, 1.0])).map(|e| e as f32),
        };

        // Update parameters if it has changed
        if state.parameters != parameters {
            state.update(|s| {
                s.parameters = parameters;
            });
        }

        widget.set_ingame(state.id.unwrap(), ui)
    }

    fn default_x_position(&self, _: &Ui) -> Position {
        Position::Absolute(0.0)
    }
    fn default_y_position(&self, _: &Ui) -> Position {
        Position::Absolute(0.0)
    }
}
