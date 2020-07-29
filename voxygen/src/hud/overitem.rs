use crate::ui::{fonts::ConrodVoxygenFonts, Ingameable};
use conrod_core::{
    widget::{self, Text},
    widget_ids, Color, Colorable, Positionable, Widget, WidgetCommon,
};

widget_ids! {
    struct Ids {
        // Name
        name_bg,
        name,
    }
}

/// ui widget containing everything that goes over a item
/// (Item, DistanceFromPlayer, Rarity, etc.)
#[derive(WidgetCommon)]
pub struct Overitem<'a> {
    name: &'a str,
    distance: &'a f32,
    fonts: &'a ConrodVoxygenFonts,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> Overitem<'a> {
    pub fn new(name: &'a str, distance: &'a f32, fonts: &'a ConrodVoxygenFonts) -> Self {
        Self {
            name,
            distance,
            fonts,
            common: widget::CommonBuilder::default(),
        }
    }
}

pub struct State {
    ids: Ids,
}

impl<'a> Ingameable for Overitem<'a> {
    fn prim_count(&self) -> usize {
        // Number of conrod primitives contained in the overitem isplay. TODO maybe
        // this could be done automatically?
        // - 2 Text::new for name
        2
    }
}

impl<'a> Widget for Overitem<'a> {
    type Event = ();
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;

        let font_size =
            ((1.0 - (self.distance / common::comp::MAX_PICKUP_RANGE_SQR)) * 30.0) as u32;

        // ItemName
        Text::new(&self.name)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(font_size)
            .color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
            .x_y(-1.0, 48.0)
            .set(state.ids.name_bg, ui);
        Text::new(&self.name)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(font_size)
            .color(Color::Rgba(0.61, 0.61, 0.89, 1.0))
            .x_y(0.0, 50.0)
            .set(state.ids.name, ui);
    }
}
