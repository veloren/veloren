use super::graphic::{Graphic, GraphicCache, Id as GraphicId};
use crate::{
    render::{Renderer, Texture},
    Error,
};
use glyph_brush::GlyphBrushBuilder;
use std::cell::{RefCell, RefMut};
use vek::*;

// Multiplied by current window size
const GLYPH_CACHE_SIZE: u16 = 1;
// Glyph cache tolerances
const SCALE_TOLERANCE: f32 = 0.1;
const POSITION_TOLERANCE: f32 = 0.1;

type GlyphBrush = glyph_brush::GlyphBrush<'static, (Aabr<f32>, Aabr<f32>)>;

pub type Font = glyph_brush::rusttype::Font<'static>;

pub struct Cache {
    glyph_brush: RefCell<GlyphBrush>,
    glyph_cache_tex: Texture,
    graphic_cache: GraphicCache,
}

// TODO: Should functions be returning UiError instead of Error?
impl Cache {
    pub fn new(renderer: &mut Renderer, default_font: Font) -> Result<Self, Error> {
        let (w, h) = renderer.get_resolution().into_tuple();

        let max_texture_size = renderer.max_texture_size();

        let glyph_cache_dims =
            Vec2::new(w, h).map(|e| (e * GLYPH_CACHE_SIZE).min(max_texture_size as u16).max(512));

        let glyph_brush = GlyphBrushBuilder::using_font(default_font)
            .initial_cache_size((glyph_cache_dims.x as u32, glyph_cache_dims.y as u32))
            .gpu_cache_scale_tolerance(SCALE_TOLERANCE)
            .gpu_cache_position_tolerance(POSITION_TOLERANCE)
            .build();

        Ok(Self {
            glyph_brush: RefCell::new(glyph_brush),
            glyph_cache_tex: renderer.create_dynamic_texture(glyph_cache_dims.map(|e| e as u16))?,
            graphic_cache: GraphicCache::new(renderer),
        })
    }

    pub fn glyph_cache_tex(&self) -> &Texture { &self.glyph_cache_tex }

    pub fn glyph_cache_mut_and_tex(&mut self) -> (&mut GlyphBrush, &Texture) {
        (self.glyph_brush.get_mut(), &self.glyph_cache_tex)
    }

    pub fn glyph_cache_mut(&mut self) -> &mut GlyphBrush { self.glyph_brush.get_mut() }

    pub fn glyph_calculator(&self) -> RefMut<GlyphBrush> { self.glyph_brush.borrow_mut() }

    // TODO: add font fn

    pub fn graphic_cache(&self) -> &GraphicCache { &self.graphic_cache }

    pub fn graphic_cache_mut(&mut self) -> &mut GraphicCache { &mut self.graphic_cache }

    pub fn add_graphic(&mut self, graphic: Graphic) -> GraphicId {
        self.graphic_cache.add_graphic(graphic)
    }

    pub fn replace_graphic(&mut self, id: GraphicId, graphic: Graphic) {
        self.graphic_cache.replace_graphic(id, graphic)
    }

    // Resizes and clears the GraphicCache
    pub fn resize_graphic_cache(&mut self, renderer: &mut Renderer) {
        self.graphic_cache.clear_cache(renderer);
    }

    // Resizes and clears the GlyphCache
    pub fn resize_glyph_cache(&mut self, renderer: &mut Renderer) -> Result<(), Error> {
        let max_texture_size = renderer.max_texture_size();
        let cache_dims = renderer
            .get_resolution()
            .map(|e| (e * GLYPH_CACHE_SIZE).min(max_texture_size as u16).max(512));
        let glyph_brush = self.glyph_brush.get_mut();
        *glyph_brush = glyph_brush
            .to_builder()
            .initial_cache_size((cache_dims.x as u32, cache_dims.y as u32))
            .build();

        self.glyph_cache_tex = renderer.create_dynamic_texture(cache_dims.map(|e| e as u16))?;
        Ok(())
    }
}
