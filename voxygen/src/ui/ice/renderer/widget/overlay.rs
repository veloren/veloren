use super::super::{super::widget::overlay, IcedRenderer, Primitive};
use iced::{mouse::Interaction, Element, Layout, Point, Rectangle};

const BORDER_SIZE: u16 = 8;

impl overlay::Renderer for IcedRenderer {
    fn draw<M>(
        &mut self,
        defaults: &Self::Defaults,
        _bounds: Rectangle,
        cursor_position: Point,
        over: &Element<'_, M, Self>,
        over_layout: Layout<'_>,
        under: &Element<'_, M, Self>,
        under_layout: Layout<'_>,
    ) -> Self::Output {
        let (under, under_mouse_interaction) =
            under.draw(self, defaults, under_layout, cursor_position);

        let (over, over_mouse_interaction) =
            over.draw(self, defaults, over_layout, cursor_position);

        // TODO: this isn't perfect but should be obselete when iced gets layer support
        let mouse_interaction = if over_mouse_interaction == Interaction::Idle {
            under_mouse_interaction
        } else {
            over_mouse_interaction
        };

        let prim = Primitive::Group {
            primitives: vec![under, over],
        };

        (prim, mouse_interaction)
    }
}
