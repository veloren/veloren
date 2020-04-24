use conrod_core::{
    image,
    widget::{
        self,
        image::{State, Style},
    },
    Widget, WidgetCommon,
};

/// This widget is like conrod's `Image` widget except it always returns false
/// for is_over so widgets under it are still interactable
#[derive(WidgetCommon)]
pub struct GhostImage {
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    image_id: image::Id,
    style: Style,
}

impl GhostImage {
    pub fn new(image_id: image::Id) -> Self {
        Self {
            common: widget::CommonBuilder::default(),
            image_id,
            style: Style::default(),
        }
    }
}

impl Widget for GhostImage {
    type Event = ();
    type State = State;
    type Style = Style;

    fn init_state(&self, _: widget::id::Generator) -> Self::State {
        State {
            src_rect: None,
            image_id: self.image_id,
        }
    }

    fn style(&self) -> Self::Style { self.style.clone() }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, .. } = args;

        if state.image_id != self.image_id {
            state.update(|state| state.image_id = self.image_id)
        }
    }

    // This is what we are here for
    fn is_over(&self) -> widget::IsOverFn { |_, _, _| widget::IsOver::Bool(false) }
}
