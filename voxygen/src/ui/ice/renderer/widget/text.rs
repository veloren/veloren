use super::super::{super::FontId, IcedRenderer, Primitive};
use glyph_brush::GlyphCruncher;
use iced::{mouse, text, Color, HorizontalAlignment, Rectangle, Size, VerticalAlignment};

impl text::Renderer for IcedRenderer {
    type Font = FontId;

    // TODO: expose as setting
    fn default_size(&self) -> u16 { 20 }

    fn measure(&self, content: &str, size: u16, font: Self::Font, bounds: Size) -> (f32, f32) {
        // Using the physical scale might make these cached info usable below?
        // Although we also have a position of the screen so this could be useless
        let p_scale = self.p_scale;
        // TODO: would be nice if the method was mut
        let section = glyph_brush::Section {
            screen_position: (0.0, 0.0),
            bounds: (bounds.width * p_scale, bounds.height * p_scale),
            layout: Default::default(),
            text: vec![glyph_brush::Text {
                text: content,
                scale: (size as f32 * p_scale).into(),
                font_id: font.0,
                extra: (),
            }],
        };

        let maybe_rect = self.cache.glyph_calculator().glyph_bounds(section);
        maybe_rect.map_or((0.0, 0.0), |rect| {
            (rect.width() / p_scale, rect.height() / p_scale)
        })
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
        let glyphs = self.position_glyphs(
            bounds,
            horizontal_alignment,
            vertical_alignment,
            content,
            size,
            font,
        );

        (
            Primitive::Text {
                glyphs,
                //size: size as f32,
                bounds,
                linear_color: color.unwrap_or(defaults.text_color).into_linear().into(),
                /*font,
                 *horizontal_alignment,
                 *vertical_alignment, */
            },
            mouse::Interaction::default(),
        )
    }
}
