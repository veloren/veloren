mod widgets;

pub use widgets::toggle_button::ToggleButton;

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
    input::{touch::Touch, Widget, Motion, Button, MouseButton},
};
use vek::*;
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
        create_ui_quad,
        create_ui_tri,
    },
    window::Window,
};

#[derive(Debug)]
pub enum UiError {
    RenderError(RenderError),
}
#[derive(Clone)]
pub struct Event(Input);
impl Event {
    pub fn try_from(event: glutin::Event, window: &glutin::GlWindow) -> Option<Self> {
        use conrod_winit::*;
        use winit;
        // A wrapper around the winit window that allows us to implement the trait necessary for enabling
        // the winit <-> conrod conversion functions.
        struct WindowRef<'a>(&'a winit::Window);

        // Implement the `WinitWindow` trait for `WindowRef` to allow for generating compatible conversion
        // functions.
        impl<'a> conrod_winit::WinitWindow for WindowRef<'a> {
            fn get_inner_size(&self) -> Option<(u32, u32)> {
                winit::Window::get_inner_size(&self.0).map(Into::into)
            }
            fn hidpi_factor(&self) -> f32 {
                winit::Window::get_hidpi_factor(&self.0) as _
            }
        }
        convert_event!(event, &WindowRef(window.window())).map(|input| {
            Self(input)
        })
    }
    pub fn is_keyboard_or_mouse(&self) -> bool {
        match self.0 {
            Input::Press(_) | Input::Release(_) | Input::Motion(_) | Input::Touch(_) | Input::Text(_) => true,
            _ => false,
        }
    }
    pub fn is_keyboard(&self) -> bool {
        match self.0 {
            Input::Press(Button::Keyboard(_)) | Input::Release(Button::Keyboard(_)) | Input::Text(_) => true,
            _ => false,
        }
    }
    pub fn new_resize(dims: Vec2<f64>) -> Self {
        Self(Input::Resize(dims.x, dims.y))
    }
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

enum DrawKind {
    Image(ImgId),
    // Text and non-textured geometry
    Plain,
}
enum DrawCommand {
    Draw {
        kind: DrawKind,
        model: Model<UiPipeline>,
    },
    Scissor(Aabr<u16>),
}
impl DrawCommand {
    fn image(model: Model<UiPipeline>, img_id: ImgId) -> DrawCommand {
        DrawCommand::Draw {
            kind: DrawKind::Image(img_id),
            model,
        }
    }
    fn plain(model: Model<UiPipeline>) -> DrawCommand {
        DrawCommand::Draw {
            kind: DrawKind::Plain,
            model,
        }
    }
}

// How to scale the ui
pub enum ScaleMode {
    // Scale against physical size
    Absolute(f64),
    // Use the dpi factor provided by the windowing system (i.e. use logical size)
    DpiFactor,
    // Scale based on the window's physical size, but maintain aspect ratio of widgets
    // Contains width and height of the "default" window size (ie where there should be no scaling)
    RelativeToWindow(Vec2<f64>),
}

struct Scale {
    // Type of scaling to use
    mode: ScaleMode,
    // Current dpi factor
    dpi_factor: f64,
    // Current logical window size
    window_dims: Vec2<f64>,
}

impl Scale {
    fn new(window: &Window, mode: ScaleMode) -> Self {
        let window_dims = window.logical_size();
        let dpi_factor = window.renderer().get_resolution().x as f64 / window_dims.x;
        Scale {
            mode,
            dpi_factor,
            window_dims,
        }
    }
    // Change the scaling mode
    pub fn scaling_mode(&mut self, mode: ScaleMode) {
        self.mode = mode;
    }
    // Calculate factor to transform between logical coordinates and our scaled coordinates
    fn scale_factor_logical(&self) -> f64 {
        match self.mode {
            ScaleMode::Absolute(scale) => scale / self.dpi_factor,
            ScaleMode::DpiFactor => 1.0,
            ScaleMode::RelativeToWindow(dims) => (self.window_dims.x / dims.x).min(self.window_dims.y / dims.y),
        }
    }
    // Calculate factor to transform between physical coordinates and our scaled coordinates
    fn scale_factor_physical(&self) -> f64 {
        self.scale_factor_logical() * self.dpi_factor
    }
    // Updates internal window size (and/or dpi_factor)
    fn window_resized(&mut self, new_dims: Vec2<f64>, renderer: &Renderer) {
        self.dpi_factor = renderer.get_resolution().x as f64 / new_dims.x;
        self.window_dims = new_dims;
    }
    // Get scaled window size
    fn scaled_window_size(&self) -> Vec2<f64> {
        self.window_dims / self.scale_factor_logical()
    }
    // Transform point from logical to scaled coordinates
    fn scale_point(&self, point: Vec2<f64>) -> Vec2<f64> {
        point / self.scale_factor_logical()
    }
}

pub struct Ui {
    ui: CrUi,
    image_map: Map<Texture<UiPipeline>>,
    cache: Cache,
    // Draw commands for the next render
    draw_commands: Vec<DrawCommand>,
    // Stores new window size for updating scaling
    window_resized: Option<Vec2<f64>>,
    // Scaling of the ui
    scale: Scale,
}

impl Ui {
    pub fn new(window: &mut Window) -> Result<Self, Error> {
        let scale = Scale::new(window, ScaleMode::Absolute(1.0));
        let win_dims = scale.scaled_window_size().into_array();
        Ok(Self {
            ui: UiBuilder::new(win_dims).build(),
            image_map: Map::new(),
            cache: Cache::new(window.renderer_mut())?,
            window_resized: None,
            draw_commands: vec![],
            scale,
        })
    }

    // Set the scaling mode of the ui
    pub fn scaling_mode(&mut self, mode: ScaleMode) {
        self.scale.scaling_mode(mode);
        // Give conrod the new size
        let (w, h) = self.scale.scaled_window_size().into_tuple();
        self.ui.handle_event(Input::Resize(w, h));
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

    // Workaround because conrod currently gives us no way to programmatically set which widget is capturing key input
    // Note the widget must be visible and not covered by other widgets at its center for this to work
    // Accepts option so widget can be "unfocused"
    // TODO: try https://docs.rs/conrod_core/0.63.0/conrod_core/struct.Ui.html#method.keyboard_capture
    pub fn focus_widget(&mut self, id: Option<WidgId>) {
        let (x, y) = match id {
            // get position of widget
            Some(id) => if let Some([x, y]) = self.ui.xy_of(id) {
                (x, y)
            } else {
                return;
            },
            // outside window (origin is center)
            None => (self.ui.win_h, self.ui.win_w)
        };
        // things to consider: does cursor need to be moved back?, should we check if the mouse if already pressed and then trigger a release?
        self.ui.handle_event(
            Input::Motion(Motion::MouseCursor { x, y })
        );
        self.ui.handle_event(
            Input::Press(Button::Mouse(MouseButton::Left))
        );
        self.ui.handle_event(
            Input::Release(Button::Mouse(MouseButton::Left))
        );
    }

    // Get whether the a widget besides the window is capturing the mouse
    pub fn no_widget_capturing_mouse(&self) -> bool {
        self.ui.global_input().current.widget_capturing_mouse.filter(|id| id != &self.ui.window ).is_none()
    }

    pub fn handle_event(&mut self, event: Event) {
        match event.0 {
            Input::Resize(w, h) => self.window_resized = Some(Vec2::new(w, h)),
            Input::Touch(touch) => self.ui.handle_event(
                Input::Touch(Touch {
                    xy: self.scale.scale_point(touch.xy.into()).into_array(),
                    ..touch
                })
            ),
            Input::Motion(motion) => self.ui.handle_event(
                Input::Motion( match motion {
                    Motion::MouseCursor { x, y } => {
                        let (x, y) = self.scale.scale_point(Vec2::new(x, y)).into_tuple();
                        Motion::MouseCursor { x, y }
                    }
                    Motion::MouseRelative { x, y } => {
                        let (x, y) = self.scale.scale_point(Vec2::new(x, y)).into_tuple();
                        Motion::MouseRelative { x, y }
                    }
                    Motion::Scroll { x, y } => {
                        let (x, y) = self.scale.scale_point(Vec2::new(x, y)).into_tuple();
                        Motion::Scroll { x, y }
                    }
                    _ => motion,
                })
            ),
            _ => self.ui.handle_event(event.0),
        }
    }

    pub fn widget_input(&self, id: WidgId) -> Widget {
        self.ui.widget_input(id)
    }

    pub fn maintain(&mut self, renderer: &mut Renderer) {
        let ref mut ui = self.ui;
        // Regenerate draw commands and associated models only if the ui changed
        if let Some(mut primitives) = ui.draw_if_changed() {
            self.draw_commands.clear();
            let mut mesh = Mesh::new();

            let mut current_img = None;

            let window_scizzor = default_scissor(renderer);
            let mut current_scizzor = window_scizzor;

            // Switches to the `Plain` state and completes the previous `Command` if not already in the
            // `Plain` state.
            macro_rules! switch_to_plain_state {
                () => {
                    if let Some(image_id) = current_img.take() {
                        self.draw_commands.push(DrawCommand::image(renderer.create_model(&mesh).unwrap(), image_id));
                        mesh.clear();
                    }
                };
            }

            let p_scale_factor = self.scale.scale_factor_physical();

            while let Some(prim) = primitives.next() {
                let Primitive {kind, scizzor, id, rect} = prim;

                // Check for a change in the scizzor
                let new_scizzor = {
                    let (l, b, w, h) = scizzor.l_b_w_h();
                    // Calculate minimum x and y coordinates while
                    //  - flipping y axis (from +up to +down)
                    //  - moving origin to top-left corner (from middle)
                    let min_x = ui.win_w / 2.0 + l;
                    let min_y = ui.win_h / 2.0 - b - h;
                    Aabr {
                        min: Vec2 {
                            x: (min_x * p_scale_factor) as u16,
                            y: (min_y * p_scale_factor) as u16,
                        },
                        max: Vec2 {
                            x: ((min_x + w) * p_scale_factor) as u16,
                            y: ((min_y + h) * p_scale_factor) as u16,
                        }
                    }
                    .intersection(window_scizzor)
                };
                if new_scizzor != current_scizzor {
                    // Finish the current command
                    match current_img.take() {
                        None =>
                            self.draw_commands.push(DrawCommand::plain(renderer.create_model(&mesh).unwrap())),
                        Some(image_id) =>
                            self.draw_commands.push(DrawCommand::image(renderer.create_model(&mesh).unwrap(), image_id)),
                    }
                    mesh.clear();

                    // Update the scizzor and produce a command.
                    current_scizzor = new_scizzor;
                    self.draw_commands.push(DrawCommand::Scissor(new_scizzor));
                }

                // Functions for converting for conrod scalar coords to GL vertex coords (-1.0 to 1.0)
                let vx = |x: f64| (x / ui.win_w * 2.0) as f32;
                let vy = |y: f64| (y / ui.win_h * 2.0) as f32;
                let gl_aabr = |rect: conrod_core::Rect| {
                    let (l, r, b, t) = rect.l_r_b_t();
                    Aabr {
                        min: Vec2::new(vx(l), vy(b)),
                        max: Vec2::new(vx(r), vy(t)),
                    }
                };

                use conrod_core::render::PrimitiveKind;
                match kind {
                    PrimitiveKind::Image { image_id, color, source_rect } => {

                        // Switch to the `Image` state for this image if we're not in it already.
                        let new_image_id = image_id;
                        match current_img {
                            // If we're already in the drawing mode for this image, we're done.
                            Some(image_id) if image_id == new_image_id => (),
                            // If we were in the `Plain` drawing state, switch to Image drawing state.
                            None => {
                                self.draw_commands.push(DrawCommand::plain(renderer.create_model(&mesh).unwrap()));
                                mesh.clear();
                                current_img = Some(new_image_id);
                            }
                            // If we were drawing a different image, switch state to draw *this* image.
                            Some(image_id) => {
                                self.draw_commands.push(DrawCommand::image(renderer.create_model(&mesh).unwrap(), image_id));
                                mesh.clear();
                                current_img = Some(new_image_id);
                            }
                        }

                        let color = srgb_to_linear(color.unwrap_or(conrod_core::color::WHITE).to_fsa());

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
                        let uv = Aabr {
                            min: Vec2::new(uv_l, uv_b),
                            max: Vec2::new(uv_r, uv_t),
                        };
                        mesh.push_quad(create_ui_quad(
                            gl_aabr(rect),
                            uv,
                            color,
                            UiMode::Image,
                        ));

                    }
                    PrimitiveKind::Text { color, text, font_id } => {
                        switch_to_plain_state!();
                        // Get screen width and height
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

                        let color = srgb_to_linear(color.to_fsa());

                        for g in positioned_glyphs {
                            if let Ok(Some((uv_rect, screen_rect))) = glyph_cache.rect_for(font_id.index(), g) {
                                let uv = Aabr {
                                    min: Vec2::new(uv_rect.min.x, uv_rect.max.y),
                                    max: Vec2::new(uv_rect.max.x, uv_rect.min.y),
                                };
                                let rect = Aabr {
                                    min: Vec2::new(
                                        (screen_rect.min.x as f32 / screen_w - 0.5) *  2.0,
                                        (screen_rect.max.y as f32 / screen_h - 0.5) * -2.0,
                                    ),
                                    max: Vec2::new(
                                        (screen_rect.max.x as f32 / screen_w - 0.5) *  2.0,
                                        (screen_rect.min.y as f32 / screen_h - 0.5) * -2.0,
                                    ),
                                };
                                mesh.push_quad(create_ui_quad(
                                    rect,
                                    uv,
                                    color,
                                    UiMode::Text,
                                ));
                            }
                        }
                    }
                    PrimitiveKind::Rectangle { color } => {
                        let color = srgb_to_linear(color.to_fsa());
                        // Don't draw a transparent rectangle
                        if color[3] == 0.0 {
                            continue;
                        }

                        switch_to_plain_state!();

                        mesh.push_quad(create_ui_quad(
                            gl_aabr(rect),
                            Aabr {
                                min: Vec2::new(0.0, 0.0),
                                max: Vec2::new(0.0, 0.0),
                            },
                            color,
                            UiMode::Geometry,
                        ));
                    }
                    PrimitiveKind::TrianglesSingleColor { color, triangles } => {
                        // Don't draw transparent triangle or switch state if there are actually no triangles
                        let color: [f32; 4] = srgb_to_linear(color.into());
                        if triangles.is_empty() || color[3] == 0.0 {
                            continue;
                        }

                        switch_to_plain_state!();

                        for tri in triangles {
                            let p1 = Vec2::new(vx(tri[0][0]), vy(tri[0][1]));
                            let p2 = Vec2::new(vx(tri[1][0]), vy(tri[1][1]));
                            let p3 = Vec2::new(vx(tri[2][0]), vy(tri[2][1]));
                            // If triangle is clockwise reverse it
                            let (v1, v2): (Vec3<f32>, Vec3<f32>) = ((p2 - p1).into(), (p3 - p1).into());
                            let triangle = if v1.cross(v2).z > 0.0 {[
                                p1.into_array(),
                                p2.into_array(),
                                p3.into_array(),
                            ]} else {[
                                p2.into_array(),
                                p1.into_array(),
                                p3.into_array(),
                            ]};
                            mesh.push_tri(create_ui_tri(
                                triangle,
                                [[0.0; 2]; 3],
                                color,
                                UiMode::Geometry,
                            ));
                        }

                    }
                    _ => {}
                    // TODO: Add these
                    //PrimitiveKind::Other {..} => {println!("primitive kind other with id {:?}", id);}
                    //PrimitiveKind::TrianglesMultiColor {..} => {println!("primitive kind multicolor with id {:?}", id);}
                }
            }
            // Enter the final command
            match current_img {
                None =>
                    self.draw_commands.push(DrawCommand::plain(renderer.create_model(&mesh).unwrap())),
                Some(image_id) =>
                    self.draw_commands.push(DrawCommand::image(renderer.create_model(&mesh).unwrap(), image_id)),
            }

            // Handle window resizing
            if let Some(new_dims) = self.window_resized.take() {
                self.scale.window_resized(new_dims, renderer);
                let (w, h) = self.scale.scaled_window_size().into_tuple();
                self.ui.handle_event(Input::Resize(w, h));
            }
        }
    }

    pub fn render(&self, renderer: &mut Renderer) {
        let mut scissor = default_scissor(renderer);
        for draw_command in self.draw_commands.iter() {
            match draw_command {
                DrawCommand::Scissor(scizzor) => {
                    scissor = *scizzor;
                }
                DrawCommand::Draw { kind, model } => {
                    let tex = match kind {
                        DrawKind::Image(image_id) => {
                            self.image_map.get(&image_id).expect("Image does not exist in image map")
                        }
                        DrawKind::Plain => {
                            self.cache.glyph_cache_tex()
                        }
                    };
                    renderer.render_ui_element(&model, &tex, scissor);
                }
            }
        }
    }
}

fn default_scissor(renderer: &mut Renderer) -> Aabr<u16> {
    let (screen_w, screen_h) = renderer.get_resolution().map(|e| e as u16).into_tuple();
    Aabr {
        min: Vec2 { x: 0, y: 0 },
        max: Vec2 { x: screen_w, y: screen_h }
    }
}

fn srgb_to_linear(color: [f32; 4]) -> [f32; 4] {
    fn linearize(comp: f32) -> f32 {
        if comp <= 0.04045 {
            comp / 12.92
        } else {
            ((comp + 0.055) / 1.055).powf(2.4)
        }
    }

    [linearize(color[0]), linearize(color[1]), linearize(color[2]), color[3]]
}
