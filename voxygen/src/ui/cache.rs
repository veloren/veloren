use super::graphic::{Graphic, GraphicCache, Id as GraphicId};
use crate::{
    error::Error,
    render::{Mesh, Renderer, Texture, UiTextureBindGroup, UiVertex},
};
use conrod_core::{text::GlyphCache, widget::Id};
use hashbrown::HashMap;
use vek::*;

// Multiplied by current window size
const GLYPH_CACHE_SIZE: u32 = 1;
// Glyph cache tolerances
const SCALE_TOLERANCE: f32 = 0.5;
const POSITION_TOLERANCE: f32 = 0.5;

type TextCache = HashMap<Id, Mesh<UiVertex>>;

pub struct Cache {
    // Map from text ids to their positioned glyphs.
    text_cache: TextCache,
    glyph_cache: GlyphCache<'static>,
    glyph_cache_tex: (Texture, UiTextureBindGroup),
    graphic_cache: GraphicCache,
}

// TODO: Should functions be returning UiError instead of Error?
impl Cache {
    pub fn new(renderer: &mut Renderer) -> Result<Self, Error> {
        let (w, h) = renderer.resolution().into_tuple();

        let max_texture_size = renderer.max_texture_size();

        let glyph_cache_dims =
            Vec2::new(w, h).map(|e| (e * GLYPH_CACHE_SIZE).clamp(512, max_texture_size));

        let glyph_cache_tex = {
            let tex = renderer.create_dynamic_texture(glyph_cache_dims);
            let bind = renderer.ui_bind_texture(&tex);
            (tex, bind)
        };

        Ok(Self {
            text_cache: Default::default(),
            glyph_cache: GlyphCache::builder()
                .dimensions(glyph_cache_dims.x, glyph_cache_dims.y)
                .scale_tolerance(SCALE_TOLERANCE)
                .position_tolerance(POSITION_TOLERANCE)
                .build(),
            glyph_cache_tex,
            graphic_cache: GraphicCache::new(renderer),
        })
    }

    pub fn glyph_cache_tex(&self) -> &(Texture, UiTextureBindGroup) { &self.glyph_cache_tex }

    pub fn cache_mut_and_tex(
        &mut self,
    ) -> (
        &mut GraphicCache,
        &mut TextCache,
        &mut GlyphCache<'static>,
        &(Texture, UiTextureBindGroup),
    ) {
        (
            &mut self.graphic_cache,
            &mut self.text_cache,
            &mut self.glyph_cache,
            &self.glyph_cache_tex,
        )
    }

    pub fn graphic_cache(&self) -> &GraphicCache { &self.graphic_cache }

    pub fn add_graphic(&mut self, graphic: Graphic) -> GraphicId {
        self.graphic_cache.add_graphic(graphic)
    }

    pub fn replace_graphic(&mut self, id: GraphicId, graphic: Graphic) {
        self.graphic_cache.replace_graphic(id, graphic)
    }

    /// Resizes and clears the various caches.
    ///
    /// To be called when something like the scaling factor changes,
    /// invalidating all existing cached UI state.
    pub fn resize(&mut self, renderer: &mut Renderer) -> Result<(), Error> {
        self.graphic_cache.clear_cache(renderer);
        self.text_cache.clear();
        let max_texture_size = renderer.max_texture_size();
        let cache_dims = renderer
            .resolution()
            .map(|e| (e * GLYPH_CACHE_SIZE).clamp(512, max_texture_size));
        self.glyph_cache = GlyphCache::builder()
            .dimensions(cache_dims.x, cache_dims.y)
            .scale_tolerance(SCALE_TOLERANCE)
            .position_tolerance(POSITION_TOLERANCE)
            .build();
        self.glyph_cache_tex = {
            let tex = renderer.create_dynamic_texture(cache_dims);
            let bind = renderer.ui_bind_texture(&tex);
            (tex, bind)
        };
        Ok(())
    }
}
