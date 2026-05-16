use conrod_core::{Position, Sizeable, Ui, Widget, WidgetCommon, position::Dimension, widget};
use vek::*;

/// Extension trait for positioning a widget at a 3D point in the game world.
pub trait Ingameable: Widget + Sized {
    /// Positions a widget as if it was at `pos` in the 3D game world.
    ///
    /// Wraps the widget in a parent `Ingame` widget which collaborates with
    /// custom code in our UI renderering logic to pass the positioning
    /// information through.
    ///
    /// Note, the widget size will not be scaled based on distance for
    /// performance and stylistic purposes.
    ///
    /// Note, widgets set in the `Widget::update` impl of `self` should not use
    /// `position_ingame` themselves. I.E. nested usage is not supported.
    fn position_ingame(self, pos: Vec3<f32>) -> Ingame<Self> { Ingame::new(pos, self) }
}

impl<W: Widget> Ingameable for W {}

// All ingame widgets are now fixed scale
#[derive(Copy, Clone, PartialEq)]
pub struct IngameParameters {
    pub pos: Vec3<f32>,
    pub dims: Vec2<f32>,
}

/// Positions wrapped `widget` in the 3D game world at `pos`.
///
/// The position on screen will depend on the 3D camera.
///
/// This uses some custom logic in the UI renderer to detect when this widget is
/// encountered and extract `IngameParameters` which are applied until the
/// `IngameEndMarker` widget is encountered. This relies on the details of
/// `conrod` to ensure that `IngameEndMarker` will be encountered after
/// all primitives produced by the wrapped widget.
#[derive(Clone, WidgetCommon)]
pub struct Ingame<W> {
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    widget: W,
    pos: Vec3<f32>,
}

pub struct State {
    inner_id: widget::Id,
    end_id: widget::Id,
    pub(in crate::ui) parameters: IngameParameters,
}

impl<W: Ingameable> Ingame<W> {
    fn new(pos: Vec3<f32>, widget: W) -> Self {
        Self {
            common: widget::CommonBuilder::default(),
            pos,
            widget,
        }
    }
}

impl<W: Ingameable> Widget for Ingame<W> {
    type Event = W::Event;
    type State = State;
    type Style = ();

    fn init_state(&self, mut id_gen: widget::id::Generator) -> Self::State {
        State {
            inner_id: id_gen.next(),
            end_id: id_gen.next(),
            parameters: IngameParameters {
                pos: self.pos,
                dims: Vec2::zero(),
            },
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;
        let Ingame { widget, pos, .. } = self;

        let parameters = IngameParameters {
            pos,
            dims: Vec2::<f64>::from(widget.get_wh(ui).unwrap_or([1.0, 1.0])).map(|e| e as f32),
        };

        // Update parameters if it has changed
        if state.parameters != parameters {
            state.update(|s| {
                s.parameters = parameters;
            });
        }

        let event = widget
            // should pass focus to the window if these are clicked
            // (they are not displayed where conrod thinks they are)
            .graphics_for(ui.window)
            .set(state.inner_id, ui);
        IngameEndMarker::default().set(state.end_id, ui);
        event
    }

    fn default_x_position(&self, _: &Ui) -> Position { Position::Absolute(0.0) }

    fn default_y_position(&self, _: &Ui) -> Position { Position::Absolute(0.0) }

    fn default_x_dimension(&self, _: &Ui) -> Dimension { Dimension::Absolute(0.0) }

    fn default_y_dimension(&self, _: &Ui) -> Dimension { Dimension::Absolute(0.0) }
}

// This must be a unique type to detect it in the rendering primitives.
pub(in crate::ui) struct IngameEndMarkerState;

#[derive(WidgetCommon, Default)]
struct IngameEndMarker {
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl Widget for IngameEndMarker {
    type Event = ();
    type State = IngameEndMarkerState;
    type Style = ();

    fn init_state(&self, _id_gen: widget::id::Generator) -> Self::State { IngameEndMarkerState }

    fn style(&self) -> Self::Style {}

    fn update(self, _args: widget::UpdateArgs<Self>) -> Self::Event {}

    fn default_x_position(&self, _: &Ui) -> Position { Position::Absolute(0.0) }

    fn default_y_position(&self, _: &Ui) -> Position { Position::Absolute(0.0) }

    fn default_x_dimension(&self, _: &Ui) -> Dimension { Dimension::Absolute(0.0) }

    fn default_y_dimension(&self, _: &Ui) -> Dimension { Dimension::Absolute(0.0) }
}
