use super::graphic::{Graphic, GraphicCache, Id as GraphicId};
use crate::{
    render::{Renderer, Texture, UiPipeline},
    Error,
};
use conrod_core::text::GlyphCache;
use vek::*;

// Multiplied by current window size
const GLYPH_CACHE_SIZE: u16 = 1;
const GRAPHIC_CACHE_SIZE: u16 = 2;
// Glyph cache tolerances
const SCALE_TOLERANCE: f32 = 0.1;
const POSITION_TOLERANCE: f32 = 0.1;

pub struct Cache {
    glyph_cache: GlyphCache<'static>,
    glyph_cache_tex: Texture<UiPipeline>,
    graphic_cache: GraphicCache,
    graphic_cache_tex: Texture<UiPipeline>,
}

// TODO: Should functions be returning UiError instead of Error?
impl Cache {
    pub fn new(renderer: &mut Renderer) -> Result<Self, Error> {
        let (w, h) = renderer.get_resolution().into_tuple();

        let max_texture_size = renderer.max_texture_size();

        let graphic_cache_dims = Vec2::new(w, h).map(|e| {
            (e * GRAPHIC_CACHE_SIZE)
                .min(max_texture_size as u16)
                .max(512)
        });
        let glyph_cache_dims =
            Vec2::new(w, h).map(|e| (e * GLYPH_CACHE_SIZE).min(max_texture_size as u16).max(512));

        Ok(Self {
            glyph_cache: GlyphCache::builder()
                .dimensions(glyph_cache_dims.x as u32, glyph_cache_dims.y as u32)
                .scale_tolerance(SCALE_TOLERANCE)
                .position_tolerance(POSITION_TOLERANCE)
                .build(),
            glyph_cache_tex: renderer.create_dynamic_texture(glyph_cache_dims.map(|e| e as u16))?,
            graphic_cache: GraphicCache::new(graphic_cache_dims),
            graphic_cache_tex: renderer.create_dynamic_texture(graphic_cache_dims)?,
        })
    }
    pub fn glyph_cache_tex(&self) -> &Texture<UiPipeline> {
        &self.glyph_cache_tex
    }
    pub fn glyph_cache_mut_and_tex(&mut self) -> (&mut GlyphCache<'static>, &Texture<UiPipeline>) {
        (&mut self.glyph_cache, &self.glyph_cache_tex)
    }
    pub fn graphic_cache_tex(&self) -> &Texture<UiPipeline> {
        &self.graphic_cache_tex
    }
    pub fn graphic_cache_mut_and_tex(&mut self) -> (&mut GraphicCache, &Texture<UiPipeline>) {
        (&mut self.graphic_cache, &self.graphic_cache_tex)
    }
    pub fn add_graphic(&mut self, graphic: Graphic) -> GraphicId {
        self.graphic_cache.add_graphic(graphic)
    }
    pub fn replace_graphic(&mut self, id: GraphicId, graphic: Graphic) {
        self.graphic_cache.replace_graphic(id, graphic)
    }
    // Resizes and clears the GraphicCache
    pub fn resize_graphic_cache(&mut self, renderer: &mut Renderer) -> Result<(), Error> {
        let max_texture_size = renderer.max_texture_size();
        let cache_dims = renderer.get_resolution().map(|e| {
            (e * GRAPHIC_CACHE_SIZE)
                .min(max_texture_size as u16)
                .max(512)
        });
        self.graphic_cache.clear_cache(cache_dims);
        self.graphic_cache_tex = renderer.create_dynamic_texture(cache_dims)?;
        Ok(())
    }
    // Resizes and clears the GlyphCache
    pub fn resize_glyph_cache(&mut self, renderer: &mut Renderer) -> Result<(), Error> {
        let max_texture_size = renderer.max_texture_size();
        let cache_dims = renderer
            .get_resolution()
            .map(|e| (e * GLYPH_CACHE_SIZE).min(max_texture_size as u16).max(512));
        self.glyph_cache = GlyphCache::builder()
            .dimensions(cache_dims.x as u32, cache_dims.y as u32)
            .scale_tolerance(SCALE_TOLERANCE)
            .position_tolerance(POSITION_TOLERANCE)
            .build();
        self.glyph_cache_tex = renderer.create_dynamic_texture(cache_dims.map(|e| e as u16))?;
        Ok(())
    }
}
