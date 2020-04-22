use iced::{
    text, Color, Font, HorizontalAlignment, mouse, Rectangle, Size, VerticalAlignment,
};
use super::{super::cache::FrameRenderer, Primitive}:

struct FontId(glyph_brush::FontId);

impl text::Renderer for FrameRenderer<'_> {
    type Font = FontId;
    const DEFAULT_SIZE: u16 = 20;

    fn measure(
        &self,
        content: &str,
        size: u16,
        font: Self::Font,
        bounds: Size,
    ) -> (f32, f32) {
        // Using the physical scale might make these cached info usable below?
        // Although we also have a position of the screen so this could be useless
        let p_scale = self.p_scale;
        // TODO: would be nice if the method was mut
        let section = glyph_brush::Section {
            text: content,
            scale: glyph_brush::rusttype::Scale::uniform(size as f32 * p_scale),
            font_id: font.0,
            bounds: (size.width, size.height),
            ..Default::default()
        };

        let maybe_rect = self.glyph_calc.borrow_mut().glyph_bounds(section);
        maybe_rect.map_or((0.0, 0.0), |rect| (rect.width() / p_scale, rect.height() / p_scale))
    }

    fn draw(
        &mut self,
        defaults: &Self::Defaults,
        bounds: Rectangle,
        content: &str,
        size: u16,
        font: Self::Font,
        color: Option<Color>,
        horizontal_alignment: HorizontalAlignment,
        vertical_alignment: VerticalAlignment,
    ) -> Self::Output {
        use glyph_brush::{HorizontalAlign, VerticalAlign};
        let h_align = match horizontal_alignment {
            HorizontalAlignment::Left => HorizontalAlign::Left,
            HorizontalAlignment::Center => HorizontalAlign::Center,
            HorizontalAlignment::Right => HorizontalAlign::Right,
        };

        let v_align = match vertical_alignment {
            VerticalAlignment::Top => VerticalAlign::Top,
            VerticalAlignment::Center => VerticalAlign::Center,
            VerticalAlignment::Bottom => VerticalAlign::Bottom,
        };

        let p_scale = self.p_scale;

        let section = glyph_brush::Section {
            text: content,
            // TODO: do snap to pixel thing here IF it is being done down the line
            screen_position: (bounds.x * p_scale, bounds.y * p_scale),
            bounds: (bounds.width * p_scale, bounds.height * p_scale),
            scale: glyph_brush::rusttype::Scale::uniform(size as f32 * p_scale),
            layout: glyph_brush::Layout::Wrap {
                line_breaker: Default::default(),
                h_align, 
                v_align,
            },
            font_id: font.0,
            ..Default::default()
        };

        let glyphs = self.glyph_calc.borrow_mut().glyphs(section).map(|positioned_glyph|
            (
                positioned_glyph,
                [0.0, 0.0, 0.0, 1.0], // Color
                font.0,
            )
            ).collect();

        (
            Primitive::Text {
                glyphs,
                //size: size as f32,
                bounds,
                color: color.unwrap_or(Color::BLACK).into_linear().into(),
                //font,
                //horizontal_alignment,
                //vertical_alignment,
            },
            mouse::Interaction::default,
        )
    }
}
