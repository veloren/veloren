use conrod_core::{
    builder_methods, image,
    position::Dimension,
    widget::{self, button},
    widget_ids, Color, Position, Positionable, Rect, Sizeable, Ui, Widget, WidgetCommon,
};
use vek::*;

#[derive(Clone, WidgetCommon)]
pub struct Ingame<W>
where
    W: Widget,
{
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    widget: W,
    pos: Vec3<f32>,
    // Number of pixels per 1 unit in world coordinates (ie a voxel)
    // Used for widgets that are rasterized before being sent to the gpu (text & images)
    // Potentially make this autmatic based on distance to camera?
    res: f32,
}

// TODO: add convenience function to this trait
pub trait Primitive {}
impl Primitive for widget::Line {}
impl Primitive for widget::Image {}
impl<I> Primitive for widget::PointPath<I> {}
impl Primitive for widget::Circle {}
impl<S> Primitive for widget::Oval<S> {}
impl<I> Primitive for widget::Polygon<I> {}
impl Primitive for widget::Rectangle {}
impl<S, I> Primitive for widget::Triangles<S, I> {}
impl<'a> Primitive for widget::Text<'a> {}

widget_ids! {
    struct Ids {
        prim,
    }
}

pub struct State {
    ids: Ids,
    pos: Vec3<f32>,
    res: f32,
}
impl State {
    // retrieve the postion and resolution as a tuple
    pub fn pos_res(&self) -> (Vec3<f32>, f32) {
        (self.pos, self.res)
    }
}

pub type Style = ();

impl<W: Widget + Primitive> Ingame<W> {
    pub fn from_primitive(pos: Vec3<f32>, widget: W) -> Self {
        Self {
            common: widget::CommonBuilder::default(),
            pos,
            widget,
            res: 1.0,
        }
    }
    builder_methods! {
        pub resolution { res = f32 }
    }
}

impl<W: Widget> Widget for Ingame<W> {
    type State = State;
    type Style = Style;
    type Event = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
            pos: Vec3::default(),
            res: 1.0,
        }
    }

    fn style(&self) -> Self::Style {
        ()
    }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { id, state, ui, .. } = args;
        let Ingame {
            widget, pos, res, ..
        } = self;

        // Update pos if it has changed
        if state.pos != pos || state.res != res {
            state.update(|s| {
                s.pos = pos;
                s.res = res;
            });
        }

        widget
            .graphics_for(ui.window)
            .x_y(0.0, 0.0)
            .parent(id) // is this needed
            .set(state.ids.prim, ui);
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
