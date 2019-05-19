use conrod_core::{
    builder_methods, image,
    position::Dimension,
    widget::{self, button, Id},
    widget_ids, Color, Position, Positionable, Rect, Sizeable, Ui, UiCell, Widget, WidgetCommon,
};
use std::slice;
use vek::*;

#[derive(Clone, WidgetCommon)]
pub struct Ingame<W> {
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    widget: W,
    parameters: IngameParameters,
}

pub trait Ingameable: Sized {
    type Event;
    fn prim_count(&self) -> usize;
    fn set_ingame(self, ids: Ids, parent_id: Id, ui: &mut UiCell) -> Self::Event;
    fn init_ids(mut id_gen: widget::id::Generator) -> Ids;
    fn position_ingame(self, pos: Vec3<f32>) -> Ingame<Self> {
        Ingame::new(pos, self)
    }
}

// Note this is not responsible for the positioning
// Only call this directly if using IngameAnchor
pub fn set_ingame<W: Widget>(widget: W, parent_id: Id, id: Id, ui: &mut UiCell) -> W::Event {
    widget
        // should pass focus to the window if these are clicked
        // (they are not displayed where conrod thinks they are)
        .graphics_for(ui.window)
        //.parent(id) // is this needed
        .set(id, ui)
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
    type Event = P::Event;
    fn prim_count(&self) -> usize {
        1
    }
    fn set_ingame(self, ids: Ids, parent_id: Id, ui: &mut UiCell) -> Self::Event {
        let id = ids.one().unwrap();

        set_ingame(self, parent_id, id, ui)
    }
    fn init_ids(mut id_gen: widget::id::Generator) -> Ids {
        Ids::One(id_gen.next())
    }
}

trait IngameWidget: Ingameable + Widget {}
impl<T> IngameWidget for T where T: Ingameable + Widget {}

impl<W, E> Ingameable for (W, E)
where
    W: IngameWidget,
    E: IngameWidget,
{
    type Event = (<W as Widget>::Event, <E as Widget>::Event);
    fn prim_count(&self) -> usize {
        self.0.prim_count() + self.1.prim_count()
    }
    fn set_ingame(self, ids: Ids, parent_id: Id, ui: &mut UiCell) -> Self::Event {
        let (w1, w2) = self;
        let [id1, id2] = ids.two().unwrap();
        (
            set_ingame(w1, parent_id, id1, ui),
            set_ingame(w2, parent_id, id2, ui),
        )
    }
    fn init_ids(mut id_gen: widget::id::Generator) -> Ids {
        Ids::Two([id_gen.next(), id_gen.next()])
    }
}
impl<W, E, R> Ingameable for (W, E, R)
where
    W: IngameWidget,
    E: IngameWidget,
    R: IngameWidget,
{
    type Event = (
        <W as Widget>::Event,
        <E as Widget>::Event,
        <R as Widget>::Event,
    );
    fn prim_count(&self) -> usize {
        self.0.prim_count() + self.1.prim_count() + self.2.prim_count()
    }
    fn set_ingame(self, ids: Ids, parent_id: Id, ui: &mut UiCell) -> Self::Event {
        let (w1, w2, w3) = self;
        let ids = ids.three().unwrap();
        (
            set_ingame(w1, parent_id, ids[0], ui),
            set_ingame(w2, parent_id, ids[1], ui),
            set_ingame(w3, parent_id, ids[2], ui),
        )
    }
    fn init_ids(mut id_gen: widget::id::Generator) -> Ids {
        Ids::Three([id_gen.next(), id_gen.next(), id_gen.next()])
    }
}
impl<W, E, R, T> Ingameable for (W, E, R, T)
where
    W: IngameWidget,
    E: IngameWidget,
    R: IngameWidget,
    T: IngameWidget,
{
    type Event = (
        <W as Widget>::Event,
        <E as Widget>::Event,
        <R as Widget>::Event,
        <T as Widget>::Event,
    );
    fn prim_count(&self) -> usize {
        self.0.prim_count() + self.1.prim_count() + self.2.prim_count() + self.3.prim_count()
    }
    fn set_ingame(self, ids: Ids, parent_id: Id, ui: &mut UiCell) -> Self::Event {
        let (w1, w2, w3, w4) = self;
        let ids = ids.four().unwrap();
        (
            set_ingame(w1, parent_id, ids[0], ui),
            set_ingame(w2, parent_id, ids[1], ui),
            set_ingame(w3, parent_id, ids[2], ui),
            set_ingame(w4, parent_id, ids[3], ui),
        )
    }
    fn init_ids(mut id_gen: widget::id::Generator) -> Ids {
        Ids::Four([id_gen.next(), id_gen.next(), id_gen.next(), id_gen.next()])
    }
}

#[derive(Clone, Copy)]
enum Ids {
    None,
    One(Id),
    Two([Id; 2]),
    Three([Id; 3]),
    Four([Id; 4]),
}
impl Ids {
    fn one(self) -> Option<Id> {
        match self {
            Ids::One(id) => Some(id),
            _ => None,
        }
    }
    fn two(self) -> Option<[Id; 2]> {
        match self {
            Ids::Two(ids) => Some(ids),
            _ => None,
        }
    }
    fn three(self) -> Option<[Id; 3]> {
        match self {
            Ids::Three(ids) => Some(ids),
            _ => None,
        }
    }
    fn four(self) -> Option<[Id; 4]> {
        match self {
            Ids::Four(ids) => Some(ids),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
pub struct IngameParameters {
    // Number of primitive widgets to position in the game at the specified position
    // Note this could be more than the number of widgets in the widgets field since widgets can contain widgets
    pub num: usize,
    pub pos: Vec3<f32>,
    // Number of pixels per 1 unit in world coordinates (ie a voxel)
    // Used for widgets that are rasterized before being sent to the gpu (text & images)
    // Potentially make this autmatic based on distance to camera?
    pub res: f32,
}

pub struct State {
    ids: Ids,
    pub parameters: IngameParameters,
}

pub type Style = ();

impl<W: Ingameable> Ingame<W> {
    pub fn new(pos: Vec3<f32>, widget: W) -> Self {
        Self {
            common: widget::CommonBuilder::default(),
            parameters: IngameParameters {
                num: widget.prim_count(),
                pos,
                res: 1.0,
            },
            widget,
        }
    }
    builder_methods! {
        pub resolution { parameters.res = f32 }
    }
}

impl<W: Ingameable> Widget for Ingame<W> {
    type State = State;
    type Style = Style;
    type Event = W::Event;

    fn init_state(&self, mut id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: W::init_ids(id_gen),
            parameters: self.parameters,
        }
    }

    fn style(&self) -> Self::Style {
        ()
    }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { id, state, ui, .. } = args;
        let Ingame {
            widget, parameters, ..
        } = self;

        // Update pos if it has changed
        if state.parameters != parameters {
            state.update(|s| {
                s.parameters = parameters;
            });
        }

        widget.set_ingame(state.ids, id, ui)
    }

    fn default_x_position(&self, ui: &Ui) -> Position {
        Position::Absolute(0.0)
    }
    fn default_y_position(&self, ui: &Ui) -> Position {
        Position::Absolute(0.0)
    }
    fn default_x_dimension(&self, ui: &Ui) -> Dimension {
        Dimension::Absolute(1.0)
    }
    fn default_y_dimension(&self, ui: &Ui) -> Dimension {
        Dimension::Absolute(1.0)
    }
}

// Use this if you have multiple widgets that you want to place at the same spot in-game
// but don't want to create a new custom widget to contain them both
// Note: widgets must be set immediately after settings this
// Note: remove this if it ends up unused
#[derive(Clone, WidgetCommon)]
pub struct IngameAnchor {
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    parameters: IngameParameters,
}
impl IngameAnchor {
    pub fn new(pos: Vec3<f32>) -> Self {
        IngameAnchor {
            common: widget::CommonBuilder::default(),
            parameters: IngameParameters {
                num: 0,
                pos,
                res: 1.0,
            },
        }
    }
    pub fn for_widget(mut self, widget: impl Ingameable) -> Self {
        self.parameters.num += widget.prim_count();
        self
    }
    pub fn for_widgets(mut self, widget: impl Ingameable, n: usize) -> Self {
        self.parameters.num += n * widget.prim_count();
        self
    }
    pub fn for_prims(mut self, num: usize) -> Self {
        self.parameters.num += num;
        self
    }
}

impl Widget for IngameAnchor {
    type State = State;
    type Style = Style;
    type Event = ();

    fn init_state(&self, _: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::None,
            parameters: self.parameters,
        }
    }

    fn style(&self) -> Self::Style {
        ()
    }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { id, state, ui, .. } = args;
        let IngameAnchor { parameters, .. } = self;

        // Update pos if it has changed
        if state.parameters != parameters {
            state.update(|s| {
                s.parameters = parameters;
            });
        }
    }
}
