// TODO: figure out proper way to propagate events down to the ui

// Library
use image::DynamicImage;
use conrod_core::{
    Ui as CrUi,
    UiBuilder,
    UiCell,
    text::{
        Font,
        GlyphCache,
        font::Id as FontId,
    },
    image::{Map, Id as ImgId},
    widget::{Id as WidgId, id::Generator},
    render::Primitive,
    event::Input,
    input::Widget,
};

// Crate
use crate::{
    Error,
    render::{
        RenderError,
        Renderer,
        Model,
        Mesh,
        Texture,
        UiPipeline,
        UiMode,
        push_ui_quad_to_mesh,
    },
    window::Window,
};

#[derive(Debug)]
pub enum UiError {
    RenderError(RenderError),
}

pub struct Cache {
    blank_texture: Texture<UiPipeline>,
    glyph_cache: GlyphCache<'static>,
    glyph_cache_tex: Texture<UiPipeline>,
}

// TODO: Should functions be returning UiError instead of Error?
impl Cache {
    pub fn new(renderer: &mut Renderer) -> Result<Self, Error> {
        let (w, h) = renderer.get_resolution().into_tuple();
        const SCALE_TOLERANCE: f32 = 0.1;
        const POSITION_TOLERANCE: f32 = 0.1;

        Ok(Self {
            blank_texture: renderer.create_texture(&DynamicImage::new_rgba8(1, 1))?,
            glyph_cache: GlyphCache::builder()
                .dimensions(w as u32, h as u32)
                .scale_tolerance(SCALE_TOLERANCE)
                .position_tolerance(POSITION_TOLERANCE)
                .build(),
            glyph_cache_tex: renderer.create_dynamic_texture((w, h).into())?,
        })
    }
    pub fn blank_texture(&self) -> &Texture<UiPipeline> { &self.blank_texture }
    pub fn glyph_cache_tex(&self) -> &Texture<UiPipeline> { &self.glyph_cache_tex }
    pub fn glyph_cache_mut_and_tex(&mut self) -> (&mut GlyphCache<'static>, &Texture<UiPipeline>) { (&mut self.glyph_cache, &self.glyph_cache_tex) }
}

pub enum DrawCommand {
    Image(Model<UiPipeline>, ImgId),
    // Text and non-textured geometry
    Plain(Model<UiPipeline>),
}

pub struct Ui {
    ui: CrUi,
    image_map: Map<Texture<UiPipeline>>,
    cache: Cache,
    // Draw commands for the next render
    draw_commands: Vec<DrawCommand>,
}

impl Ui {
    pub fn new(window: &mut Window) -> Result<Self, Error> {
        // Retrieve the logical size of the window content
        let (w, h) = window.logical_size();
        Ok(Self {
            ui: UiBuilder::new([w, h]).build(),
            image_map: Map::new(),
            cache: Cache::new(window.renderer_mut())?,
            draw_commands: vec![],
        })
    }

    pub fn new_image(&mut self, renderer: &mut Renderer, image: &DynamicImage) -> Result<ImgId, Error> {
        Ok(self.image_map.insert(renderer.create_texture(image)?))
    }

    pub fn new_font(&mut self, font: Font) -> FontId {
        self.ui.fonts.insert(font)
    }

    pub fn id_generator(&mut self) -> Generator {
        self.ui.widget_id_generator()
    }

    pub fn set_widgets(&mut self) -> UiCell {
        self.ui.set_widgets()
    }

    pub fn handle_event(&mut self, event: Input) {
        self.ui.handle_event(event);
    }

    pub fn widget_input(&self, id: WidgId) -> Widget {
        self.ui.widget_input(id)
    }

    pub fn maintain(&mut self, renderer: &mut Renderer) {
        let ref mut ui = self.ui;
        // Regenerate draw commands and associated models only if ui_changed
        if let Some(mut primitives) = ui.draw_if_changed() {
            self.draw_commands.clear();
            let mut mesh = Mesh::new();

            let mut current_img = None;

            // Switches to the `Plain` state and completes the previous `Command` if not already in the
            // `Plain` state.
            macro_rules! switch_to_plain_state {
                () => {
                    if let Some(image_id) = current_img.take() {
                        self.draw_commands.push(DrawCommand::Image(renderer.create_model(&mesh).unwrap(), image_id));
                        mesh.clear();
                    }
                };
            }

            while let Some(prim) = primitives.next() {
                // TODO: Use scizzor
                let Primitive {kind, scizzor, id, rect} = prim;

                use conrod_core::render::PrimitiveKind;
                match kind {
                    // TODO: use source_rect
                    PrimitiveKind::Image { image_id, color, source_rect } => {

                        // Switch to the `Image` state for this image if we're not in it already.
                        let new_image_id = image_id;
                        match current_img {
                            // If we're already in the drawing mode for this image, we're done.
                            Some(image_id) if image_id == new_image_id => (),
                            // If we were in the `Plain` drawing state, switch to Image drawing state.
                            None => {
                                self.draw_commands.push(DrawCommand::Plain(renderer.create_model(&mesh).unwrap()));
                                mesh.clear();
                                current_img = Some(new_image_id);
                            }
                            // If we were drawing a different image, switch state to draw *this* image.
                            Some(image_id) => {
                                self.draw_commands.push(DrawCommand::Image(renderer.create_model(&mesh).unwrap(), image_id));
                                mesh.clear();
                                current_img = Some(new_image_id);
                            }
                        }

                        let color = color.unwrap_or(conrod_core::color::WHITE).to_fsa();

                        // Transform the source rectangle into uv coordinates
                        let (image_w, image_h) = self.image_map
                            .get(&image_id)
                            .expect("Image does not exist in image map")
                            .get_dimensions()
                            .map(|e| e as f64)
                            .into_tuple();
                        let (uv_l, uv_r, uv_t, uv_b) = match source_rect {
                            Some(src_rect) => {
                                let (l, r, b, t) = src_rect.l_r_b_t();
                                ((l / image_w) as f32,
                                (r / image_w) as f32,
                                (b / image_h) as f32,
                                (t / image_h) as f32)
                            }
                            None => (0.0, 1.0, 0.0, 1.0),
                        };
                        // Convert from conrod Scalar range to GL range -1.0 to 1.0.
                        let (l, r, b, t) = rect.l_r_b_t();
                        let (l, r, b, t) = (
                            (l / ui.win_w * 2.0) as f32,
                            (r / ui.win_w * 2.0) as f32,
                            (b / ui.win_h * 2.0) as f32,
                            (t / ui.win_h * 2.0) as f32,
                        );
                        push_ui_quad_to_mesh(
                            &mut mesh,
                            [l, t , r, b],
                            [uv_l, uv_t, uv_r, uv_b],
                            color,
                            UiMode::Image,
                        );

                    }
                    PrimitiveKind::Text { color, text, font_id } => {
                        switch_to_plain_state!();
                        // Get screen width
                        let (screen_w, screen_h) = renderer.get_resolution().map(|e| e as f32).into_tuple();
                        // Calculate dpi factor
                        let dpi_factor = screen_w / ui.win_w as f32;

                        let positioned_glyphs = text.positioned_glyphs(dpi_factor);
                        let (glyph_cache, cache_tex) = self.cache.glyph_cache_mut_and_tex();
                        // Queue the glyphs to be cached
                        for glyph in positioned_glyphs {
                            glyph_cache.queue_glyph(font_id.index(), glyph.clone());
                        }

                        glyph_cache.cache_queued(|rect, data| {
                            let offset = [rect.min.x as u16, rect.min.y as u16];
                            let size = [rect.width() as u16, rect.height() as u16];

                            let new_data = data.iter().map(|x| [255, 255, 255, *x]).collect::<Vec<[u8; 4]>>();

                            renderer.update_texture(cache_tex, offset, size, &new_data);
                        }).unwrap();

                        // TODO: consider gamma....
                        let color = color.to_fsa();

                        for g in positioned_glyphs {
                            if let Ok(Some((uv_rect, screen_rect))) = glyph_cache.rect_for(font_id.index(), g) {
                                let (uv_l, uv_r, uv_t, uv_b) = (
                                    uv_rect.min.x,
                                    uv_rect.max.x,
                                    uv_rect.min.y,
                                    uv_rect.max.y,
                                );
                                let (l, t, r, b) = (
                                    (screen_rect.min.x as f32 / screen_w - 0.5) *  2.0,
                                    (screen_rect.min.y as f32 / screen_h - 0.5) * -2.0,
                                    (screen_rect.max.x as f32 / screen_w - 0.5) *  2.0,
                                    (screen_rect.max.y as f32 / screen_h - 0.5) * -2.0,
                                );
                                push_ui_quad_to_mesh(
                                    &mut mesh,
                                    [l, t , r, b],
                                    [uv_l, uv_t, uv_r, uv_b],
                                    color,
                                    UiMode::Text,
                                );
                            }
                        }
                    }
                    _ => {}
                    // TODO: Add these
                    //PrimitiveKind::Other {..} => {println!("primitive kind other with id {:?}", id);}
                    //PrimitiveKind::Rectangle { color } => {println!("primitive kind rect[x:{},y:{},w:{},h:{}] with color {:?} and id {:?}", x, y, w, h, color, id);}
                    //PrimitiveKind::TrianglesMultiColor {..} => {println!("primitive kind multicolor with id {:?}", id);}
                    //PrimitiveKind::TrianglesSingleColor {..} => {println!("primitive kind singlecolor with id {:?}", id);}
                }
            }
            // Enter the final command.
            match current_img {
                None =>
                    self.draw_commands.push(DrawCommand::Plain(renderer.create_model(&mesh).unwrap())),
                Some(image_id) =>
                    self.draw_commands.push(DrawCommand::Image(renderer.create_model(&mesh).unwrap(), image_id)),
            }
        }
    }

    pub fn render(&self, renderer: &mut Renderer) {
        for draw_command in self.draw_commands.iter() {
            match draw_command {
                DrawCommand::Image(model, image_id) => {
                    let tex = self.image_map.get(&image_id).expect("Image does not exist in image map");
                    renderer.render_ui_element(&model, &tex);
                },
                DrawCommand::Plain(model) => {
                    let tex = self.cache.glyph_cache_tex();
                    renderer.render_ui_element(&model, &tex);
                },
            }
        }
    }
}
