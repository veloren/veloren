use conrod_core::{
    builder_methods, position::Dimension, widget, Position, Ui, UiCell, Widget, WidgetCommon,
};
use vek::*;

#[derive(Clone, WidgetCommon)]
pub struct Ingame<W> {
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    widget: W,
    parameters: IngameParameters,
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

#[derive(Copy, Clone, PartialEq)]
pub struct IngameParameters {
    // Number of primitive widgets to position in the game at the specified position
    // Note this could be more than the number of widgets in the widgets field since widgets can contain widgets
    pub num: usize,
    pub pos: Vec3<f32>,
    // Number of pixels per 1 unit in world coordinates (ie a voxel)
    // Used for widgets that are rasterized before being sent to the gpu (text & images)
    // Potentially make this automatic based on distance to camera?
    pub res: f32,
    // Whether the widgets should be scaled based on distance to the camera or if they should be a
    // fixed size (res is ignored in that case)
    pub fixed_scale: bool,
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
            parameters: IngameParameters {
                num: widget.prim_count(),
                pos,
                res: 1.0,
                fixed_scale: false,
            },
            widget,
        }
    }
    pub fn fixed_scale(mut self) -> Self {
        self.parameters.fixed_scale = true;
        self
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
            id: Some(id_gen.next()),
            parameters: self.parameters,
        }
    }

    fn style(&self) -> Self::Style {
        ()
    }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;
        let Ingame {
            widget, parameters, ..
        } = self;

        // Update pos if it has changed
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
    fn default_x_dimension(&self, _: &Ui) -> Dimension {
        Dimension::Absolute(1.0)
    }
    fn default_y_dimension(&self, _: &Ui) -> Dimension {
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
                fixed_scale: false,
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
            id: None,
            parameters: self.parameters,
        }
    }

    fn style(&self) -> Self::Style {
        ()
    }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { id: _, state, .. } = args;
        let IngameAnchor { parameters, .. } = self;

        // Update pos if it has changed
        if state.parameters != parameters {
            state.update(|s| {
                s.parameters = parameters;
            });
        }
    }
}
