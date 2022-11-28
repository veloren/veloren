use super::graphic::{Graphic, GraphicCache, Id as GraphicId};
use crate::{
    error::Error,
    render::{Renderer, Texture, UiTextureBindGroup},
};
use common::assets::{self, AssetExt};
use glyph_brush::GlyphBrushBuilder;
use std::cell::{RefCell, RefMut};
use vek::*;

// Multiplied by current window size
const GLYPH_CACHE_SIZE: u32 = 1;
// Glyph cache tolerances
// TODO: consider scaling based on dpi as well as providing as an option to the
// user
const SCALE_TOLERANCE: f32 = 0.5;
const POSITION_TOLERANCE: f32 = 0.5;

type GlyphBrush = glyph_brush::GlyphBrush<(Aabr<f32>, Aabr<f32>), ()>;

// TODO: might not need pub
pub type Font = glyph_brush::ab_glyph::FontArc;

pub fn load_font(specifier: &str) -> Font { Font::load_expect(specifier).cloned() }

#[derive(Clone, Copy, Default)]
pub struct FontId(pub(super) glyph_brush::FontId);

pub struct Cache {
    glyph_brush: RefCell<GlyphBrush>,
    glyph_cache_tex: (Texture, UiTextureBindGroup),
    graphic_cache: GraphicCache,
}

// TODO: Should functions be returning UiError instead of Error?
impl Cache {
    pub fn new(renderer: &mut Renderer, default_font: Font) -> Result<Self, Error> {
        let (w, h) = renderer.resolution().into_tuple();

        let max_texture_size = renderer.max_texture_size();

        let glyph_cache_dims =
            Vec2::new(w, h).map(|e| (e * GLYPH_CACHE_SIZE).clamp(512, max_texture_size));

        let glyph_brush = GlyphBrushBuilder::using_font(default_font)
            .initial_cache_size((glyph_cache_dims.x, glyph_cache_dims.y))
            .draw_cache_scale_tolerance(SCALE_TOLERANCE)
            .draw_cache_position_tolerance(POSITION_TOLERANCE)
            .build();

        let glyph_cache_tex = {
            let tex = renderer.create_dynamic_texture(glyph_cache_dims);
            let bind = renderer.ui_bind_texture(&tex);
            (tex, bind)
        };

        Ok(Self {
            glyph_brush: RefCell::new(glyph_brush),
            glyph_cache_tex,
            graphic_cache: GraphicCache::new(renderer),
        })
    }

    pub fn glyph_cache_tex(&self) -> &(Texture, UiTextureBindGroup) { &self.glyph_cache_tex }

    pub fn glyph_cache_mut_and_tex(&mut self) -> (&mut GlyphBrush, &(Texture, UiTextureBindGroup)) {
        (self.glyph_brush.get_mut(), &self.glyph_cache_tex)
    }

    pub fn glyph_cache_mut(&mut self) -> &mut GlyphBrush { self.glyph_brush.get_mut() }

    pub fn glyph_calculator(&self) -> RefMut<GlyphBrush> { self.glyph_brush.borrow_mut() }

    // TODO: consider not re-adding default font
    pub fn add_font(&mut self, font: RawFont) -> FontId {
        let font = Font::try_from_vec(font.0).unwrap();
        let id = self.glyph_brush.get_mut().add_font(font);
        FontId(id)
    }

    /// Allows clearing out the fonts when switching languages
    pub fn clear_fonts(&mut self, default_font: Font) {
        self.glyph_brush = RefCell::new(
            self.glyph_brush
                .get_mut()
                .to_builder()
                .replace_fonts(|mut fonts| {
                    fonts.clear();
                    fonts.push(default_font);
                    fonts
                })
                .build(),
        );
    }

    pub fn graphic_cache(&self) -> &GraphicCache { &self.graphic_cache }

    pub fn graphic_cache_mut(&mut self) -> &mut GraphicCache { &mut self.graphic_cache }

    pub fn add_graphic(&mut self, graphic: Graphic) -> GraphicId {
        self.graphic_cache.add_graphic(graphic)
    }

    pub fn replace_graphic(&mut self, id: GraphicId, graphic: Graphic) {
        self.graphic_cache.replace_graphic(id, graphic)
    }

    // TODO: combine resize functions
    // Resizes and clears the GraphicCache
    pub fn resize_graphic_cache(&mut self, renderer: &mut Renderer) {
        self.graphic_cache.clear_cache(renderer);
    }

    // Resizes and clears the GlyphCache
    pub fn resize_glyph_cache(&mut self, renderer: &mut Renderer) -> Result<(), Error> {
        let max_texture_size = renderer.max_texture_size();
        let cache_dims = renderer
            .resolution()
            .map(|e| (e * GLYPH_CACHE_SIZE).clamp(512, max_texture_size));
        let glyph_brush = self.glyph_brush.get_mut();
        *glyph_brush = glyph_brush
            .to_builder()
            .initial_cache_size((cache_dims.x, cache_dims.y))
            .build();

        self.glyph_cache_tex = {
            let tex = renderer.create_dynamic_texture(cache_dims);
            let bind = renderer.ui_bind_texture(&tex);
            (tex, bind)
        };

        Ok(())
    }
}

// TODO: use font type instead of raw vec once we convert to full iced
#[derive(Clone)]
pub struct RawFont(pub Vec<u8>);

impl From<Vec<u8>> for RawFont {
    fn from(raw: Vec<u8>) -> RawFont { RawFont(raw) }
}

impl assets::Asset for RawFont {
    type Loader = assets::LoadFrom<Vec<u8>, assets::BytesLoader>;

    const EXTENSION: &'static str = "ttf";
}
