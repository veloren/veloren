use super::graphic::{Graphic, GraphicCache, Id as GraphicId};
use crate::{
    render::{Renderer, Texture, UiPipeline},
    Error,
};
use conrod_core::text::GlyphCache;
use vek::*;

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
        const SCALE_TOLERANCE: f32 = 0.1;
        const POSITION_TOLERANCE: f32 = 0.1;

        let graphic_cache_dims = Vec2::new(w * 4, h * 4);
        Ok(Self {
            glyph_cache: GlyphCache::builder()
                .dimensions(w as u32, h as u32)
                .scale_tolerance(SCALE_TOLERANCE)
                .position_tolerance(POSITION_TOLERANCE)
                .build(),
            glyph_cache_tex: renderer.create_dynamic_texture((w, h).into())?,
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
    pub fn clear_graphic_cache(&mut self, renderer: &mut Renderer, new_size: Vec2<u16>) {
        self.graphic_cache.clear_cache(new_size);
        self.graphic_cache_tex = renderer.create_dynamic_texture(new_size).unwrap();
    }
}
