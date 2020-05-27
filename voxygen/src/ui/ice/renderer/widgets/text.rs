use super::super::{super::FontId, IcedRenderer, Primitive};
use glyph_brush::GlyphCruncher;
use iced::{mouse, text, Color, HorizontalAlignment, Rectangle, Size, VerticalAlignment};

impl text::Renderer for IcedRenderer {
    type Font = FontId;

    const DEFAULT_SIZE: u16 = 20;

    fn measure(&self, content: &str, size: u16, font: Self::Font, bounds: Size) -> (f32, f32) {
        // Using the physical scale might make these cached info usable below?
        // Although we also have a position of the screen so this could be useless
        let p_scale = self.p_scale;
        // TODO: would be nice if the method was mut
        let section = glyph_brush::Section {
            text: content,
            scale: glyph_brush::rusttype::Scale::uniform(size as f32 * p_scale),
            font_id: font.0,
            bounds: (bounds.width * p_scale, bounds.height * p_scale),
            ..Default::default()
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
        use glyph_brush::{HorizontalAlign, VerticalAlign};
        // glyph_brush thought it would be a great idea to change what the bounds and
        // position mean based on the alignment
        // TODO: add option to align based on the geometry of the rendered glyphs
        // instead of all possible glyphs
        let (x, h_align) = match horizontal_alignment {
            HorizontalAlignment::Left => (bounds.x, HorizontalAlign::Left),
            HorizontalAlignment::Center => (bounds.center_x(), HorizontalAlign::Center),
            HorizontalAlignment::Right => (bounds.x + bounds.width, HorizontalAlign::Right),
        };

        let (y, v_align) = match vertical_alignment {
            VerticalAlignment::Top => (bounds.y, VerticalAlign::Top),
            VerticalAlignment::Center => (bounds.center_y(), VerticalAlign::Center),
            VerticalAlignment::Bottom => (bounds.y + bounds.height, VerticalAlign::Bottom),
        };

        let p_scale = self.p_scale;

        let section = glyph_brush::Section {
            text: content,
            screen_position: (x * p_scale, y * p_scale),
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

        let glyphs = self
            .cache
            .glyph_cache_mut()
            .glyphs(section)
            .map(|positioned_glyph| {
                (
                    positioned_glyph.clone(), // :/
                    [0.0, 0.0, 0.0, 1.0],     // Color
                    font.0,
                )
            })
            .collect::<Vec<_>>();

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
