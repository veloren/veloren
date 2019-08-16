mod cache;
mod event;
mod graphic;
mod scale;
mod widgets;
#[macro_use]
pub mod img_ids;
#[macro_use]
mod font_ids;

pub use event::Event;
pub use graphic::Graphic;
pub use scale::{Scale, ScaleMode};
pub use widgets::{
    image_slider::ImageSlider,
    ingame::{Ingame, IngameAnchor, Ingameable},
    toggle_button::ToggleButton,
    tooltip::{Tooltip, Tooltipable},
};

use crate::{
    render::{
        create_ui_quad, create_ui_tri, Consts, DynamicModel, Globals, Mesh, RenderError, Renderer,
        UiLocals, UiMode, UiPipeline,
    },
    window::Window,
    Error,
};
use cache::Cache;
use common::{assets, util::srgba_to_linear};
use conrod_core::{
    event::Input,
    graph::Graph,
    image::{self, Map},
    input::{touch::Touch, Motion, Widget},
    render::{Primitive, PrimitiveKind},
    text::{self, font},
    widget::{self, id::Generator},
    Rect, UiBuilder, UiCell,
};
use graphic::Id as GraphicId;
use log::warn;
use std::{
    fs::File,
    io::{BufReader, Read},
    ops::Range,
    sync::Arc,
    time::Duration,
};
use vek::*;
use widgets::tooltip::TooltipManager;

#[derive(Debug)]
pub enum UiError {
    RenderError(RenderError),
}

enum DrawKind {
    Image,
    // Text and non-textured geometry
    Plain,
}
enum DrawCommand {
    Draw { kind: DrawKind, verts: Range<usize> },
    Scissor(Aabr<u16>),
    WorldPos(Option<usize>),
}
impl DrawCommand {
    fn image(verts: Range<usize>) -> DrawCommand {
        DrawCommand::Draw {
            kind: DrawKind::Image,
            verts,
        }
    }
    fn plain(verts: Range<usize>) -> DrawCommand {
        DrawCommand::Draw {
            kind: DrawKind::Plain,
            verts,
        }
    }
}

pub struct Font(text::Font);
impl assets::Asset for Font {
    const ENDINGS: &'static [&'static str] = &["ttf"];
    fn parse(mut buf_reader: BufReader<File>) -> Result<Self, assets::Error> {
        let mut buf = Vec::new();
        buf_reader.read_to_end(&mut buf)?;
        Ok(Font(text::Font::from_bytes(buf.clone()).unwrap()))
    }
}

pub struct Ui {
    ui: conrod_core::Ui,
    image_map: Map<GraphicId>,
    cache: Cache,
    // Draw commands for the next render
    draw_commands: Vec<DrawCommand>,
    // Model for drawing the ui
    model: DynamicModel<UiPipeline>,
    // Consts for default ui drawing position (ie the interface)
    interface_locals: Consts<UiLocals>,
    default_globals: Consts<Globals>,
    // Consts to specify positions of ingame elements (e.g. Nametags)
    ingame_locals: Vec<Consts<UiLocals>>,
    // Window size for updating scaling
    window_resized: Option<Vec2<f64>>,
    // Used to delay cache resizing until after current frame is drawn
    need_cache_resize: bool,
    // Scaling of the ui
    scale: Scale,
    // Tooltips
    tooltip_manager: TooltipManager,
}

impl Ui {
    pub fn new(window: &mut Window) -> Result<Self, Error> {
        let scale = Scale::new(window, ScaleMode::Absolute(1.0));
        let win_dims = scale.scaled_window_size().into_array();

        let renderer = window.renderer_mut();

        let mut ui = UiBuilder::new(win_dims).build();
        let tooltip_manager = TooltipManager::new(
            ui.widget_id_generator(),
            Duration::from_millis(1000),
            Duration::from_millis(1000),
            scale.scale_factor_logical(),
        );

        Ok(Self {
            ui,
            image_map: Map::new(),
            cache: Cache::new(renderer)?,
            draw_commands: vec![],
            model: renderer.create_dynamic_model(100)?,
            interface_locals: renderer.create_consts(&[UiLocals::default()])?,
            default_globals: renderer.create_consts(&[Globals::default()])?,
            ingame_locals: Vec::new(),
            window_resized: None,
            need_cache_resize: false,
            scale,
            tooltip_manager,
        })
    }

    // Set the scaling mode of the ui.
    pub fn set_scaling_mode(&mut self, mode: ScaleMode) {
        self.scale.set_scaling_mode(mode);
        // To clear the cache (it won't be resized in this case)
        self.need_cache_resize = true;
        // Give conrod the new size.
        let (w, h) = self.scale.scaled_window_size().into_tuple();
        self.ui.handle_event(Input::Resize(w, h));
    }

    // Get a copy of Scale
    pub fn scale(&self) -> Scale {
        self.scale
    }

    pub fn add_graphic(&mut self, graphic: Graphic) -> image::Id {
        self.image_map.insert(self.cache.add_graphic(graphic))
    }

    pub fn new_font(&mut self, font: Arc<Font>) -> font::Id {
        self.ui.fonts.insert(font.as_ref().0.clone())
    }

    pub fn id_generator(&mut self) -> Generator {
        self.ui.widget_id_generator()
    }

    pub fn set_widgets(&mut self) -> (UiCell, &mut TooltipManager) {
        (self.ui.set_widgets(), &mut self.tooltip_manager)
    }

    // Accepts Option so widget can be unfocused.
    pub fn focus_widget(&mut self, id: Option<widget::Id>) {
        self.ui.keyboard_capture(match id {
            Some(id) => id,
            None => self.ui.window,
        });
    }

    // Get id of current widget capturing keyboard.
    pub fn widget_capturing_keyboard(&self) -> Option<widget::Id> {
        self.ui.global_input().current.widget_capturing_keyboard
    }

    // Get whether a widget besides the window is capturing the mouse.
    pub fn no_widget_capturing_mouse(&self) -> bool {
        self.ui
            .global_input()
            .current
            .widget_capturing_mouse
            .filter(|id| id != &self.ui.window)
            .is_none()
    }

    // Get the widget graph.
    pub fn widget_graph(&self) -> &Graph {
        self.ui.widget_graph()
    }
    pub fn handle_event(&mut self, event: Event) {
        match event.0 {
            Input::Resize(w, h) if w > 1.0 && h > 1.0 => {
                self.window_resized = Some(Vec2::new(w, h))
            }
            Input::Touch(touch) => self.ui.handle_event(Input::Touch(Touch {
                xy: self.scale.scale_point(touch.xy.into()).into_array(),
                ..touch
            })),
            Input::Motion(motion) => self.ui.handle_event(Input::Motion(match motion {
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
            })),
            _ => self.ui.handle_event(event.0),
        }
    }

    pub fn widget_input(&self, id: widget::Id) -> Widget {
        self.ui.widget_input(id)
    }

    pub fn maintain(&mut self, renderer: &mut Renderer, cam_params: Option<(Mat4<f32>, f32)>) {
        // Maintain tooltip manager
        self.tooltip_manager
            .maintain(self.ui.global_input(), self.scale.scale_factor_logical());

        // Regenerate draw commands and associated models only if the ui changed
        let mut primitives = match self.ui.draw_if_changed() {
            Some(primitives) => primitives,
            None => return,
        };

        if self.need_cache_resize {
            // Resize graphic cache
            self.cache.resize_graphic_cache(renderer).unwrap();
            // Resize glyph cache
            self.cache.resize_glyph_cache(renderer).unwrap();

            self.need_cache_resize = false;
        }

        self.draw_commands.clear();
        let mut mesh = Mesh::new();

        // TODO: this could be removed entirely if the draw call just used both textures,
        //  however this allows for flexibility if we want to interweave other draw calls later.
        enum State {
            Image,
            Plain,
        };

        let mut current_state = State::Plain;
        let mut start = 0;

        let window_scissor = default_scissor(renderer);
        let mut current_scissor = window_scissor;

        let mut ingame_local_index = 0;

        enum Placement {
            Interface,
            // Number of primitives left to render ingame and relative scaling/resolution
            InWorld(usize, Option<f32>),
        };

        let mut placement = Placement::Interface;
        // TODO: maybe mutate an ingame scale factor instead of this, depends on if we want them to scale with other ui scaling or not
        let mut p_scale_factor = self.scale.scale_factor_physical();

        // Switches to the `Plain` state and completes the previous `Command` if not already in the
        // `Plain` state.
        macro_rules! switch_to_plain_state {
            () => {
                if let State::Image = current_state {
                    self.draw_commands
                        .push(DrawCommand::image(start..mesh.vertices().len()));
                    start = mesh.vertices().len();
                    current_state = State::Plain;
                }
            };
        }

        while let Some(prim) = primitives.next() {
            let Primitive {
                kind,
                scizzor,
                rect,
                ..
            } = prim;

            // Check for a change in the scissor.
            let new_scissor = {
                let (l, b, w, h) = scizzor.l_b_w_h();
                let scale_factor = self.scale.scale_factor_physical();
                // Calculate minimum x and y coordinates while
                // flipping y axis (from +up to +down) and
                // moving origin to top-left corner (from middle).
                let min_x = self.ui.win_w / 2.0 + l;
                let min_y = self.ui.win_h / 2.0 - b - h;
                Aabr {
                    min: Vec2 {
                        x: (min_x * scale_factor) as u16,
                        y: (min_y * scale_factor) as u16,
                    },
                    max: Vec2 {
                        x: ((min_x + w) * scale_factor) as u16,
                        y: ((min_y + h) * scale_factor) as u16,
                    },
                }
                .intersection(window_scissor)
            };
            if new_scissor != current_scissor {
                // Finish the current command.
                self.draw_commands.push(match current_state {
                    State::Plain => DrawCommand::plain(start..mesh.vertices().len()),
                    State::Image => DrawCommand::image(start..mesh.vertices().len()),
                });
                start = mesh.vertices().len();

                // Update the scissor and produce a command.
                current_scissor = new_scissor;
                self.draw_commands.push(DrawCommand::Scissor(new_scissor));
            }

            match placement {
                // No primitives left to place in the world at the current position, go back to drawing the interface
                Placement::InWorld(0, _) => {
                    placement = Placement::Interface;
                    p_scale_factor = self.scale.scale_factor_physical();
                    // Finish current state
                    self.draw_commands.push(match current_state {
                        State::Plain => DrawCommand::plain(start..mesh.vertices().len()),
                        State::Image => DrawCommand::image(start..mesh.vertices().len()),
                    });
                    start = mesh.vertices().len();
                    // Push new position command
                    self.draw_commands.push(DrawCommand::WorldPos(None));
                }
                // Primitives still left to draw ingame
                Placement::InWorld(num_prims, res) => match kind {
                    // Other types aren't drawn & shouldn't decrement the number of primitives left to draw ingame
                    PrimitiveKind::Other(_) => {}
                    // Decrement the number of primitives left
                    _ => placement = Placement::InWorld(num_prims - 1, res),
                },
                Placement::Interface => {}
            }

            // Functions for converting for conrod scalar coords to GL vertex coords (-1.0 to 1.0).
            let (ui_win_w, ui_win_h) = match placement {
                Placement::InWorld(_, Some(res)) => (res as f64, res as f64),
                // Behind the camera or far away
                Placement::InWorld(_, None) => continue,
                Placement::Interface => (self.ui.win_w, self.ui.win_h),
            };
            let vx = |x: f64| (x / ui_win_w * 2.0) as f32;
            let vy = |y: f64| (y / ui_win_h * 2.0) as f32;
            let gl_aabr = |rect: Rect| {
                let (l, r, b, t) = rect.l_r_b_t();
                Aabr {
                    min: Vec2::new(vx(l), vy(b)),
                    max: Vec2::new(vx(r), vy(t)),
                }
            };

            match kind {
                PrimitiveKind::Image {
                    image_id,
                    color,
                    source_rect: _, // TODO: <-- use this
                } => {
                    let graphic_id = self
                        .image_map
                        .get(&image_id)
                        .expect("Image does not exist in image map");
                    let (graphic_cache, cache_tex) = self.cache.graphic_cache_mut_and_tex();

                    match graphic_cache.get_graphic(*graphic_id) {
                        Some(Graphic::Blank) | None => continue,
                        _ => {}
                    }

                    // Switch to the image state if we are not in it already.
                    if let State::Plain = current_state {
                        self.draw_commands
                            .push(DrawCommand::plain(start..mesh.vertices().len()));
                        start = mesh.vertices().len();
                        current_state = State::Image;
                    }

                    let color =
                        srgba_to_linear(color.unwrap_or(conrod_core::color::WHITE).to_fsa().into());

                    let resolution = Vec2::new(
                        (rect.w() * p_scale_factor).round() as u16,
                        (rect.h() * p_scale_factor).round() as u16,
                    );
                    // Transform the source rectangle into uv coordinate.
                    // TODO: Make sure this is right.
                    let source_aabr = {
                        let (uv_l, uv_r, uv_b, uv_t) = (0.0, 1.0, 0.0, 1.0);
                        /*match source_rect {
                            Some(src_rect) => {
                                let (l, r, b, t) = src_rect.l_r_b_t();
                                ((l / image_w) as f32,
                                (r / image_w) as f32,
                                (b / image_h) as f32,
                                (t / image_h) as f32)
                            }
                            None => (0.0, 1.0, 0.0, 1.0),
                        };*/
                        Aabr {
                            min: Vec2::new(uv_l, uv_b),
                            max: Vec2::new(uv_r, uv_t),
                        }
                    };
                    // TODO: get dims from graphic_cache (or have it return floats directly)
                    let (cache_w, cache_h) =
                        cache_tex.get_dimensions().map(|e| e as f32).into_tuple();

                    // Cache graphic at particular resolution.
                    let uv_aabr =
                        match graphic_cache.queue_res(*graphic_id, resolution, source_aabr) {
                            Some(aabr) => Aabr {
                                min: Vec2::new(
                                    aabr.min.x as f32 / cache_w,
                                    aabr.max.y as f32 / cache_h,
                                ),
                                max: Vec2::new(
                                    aabr.max.x as f32 / cache_w,
                                    aabr.min.y as f32 / cache_h,
                                ),
                            },
                            None => continue,
                        };

                    mesh.push_quad(create_ui_quad(gl_aabr(rect), uv_aabr, color, UiMode::Image));
                }
                PrimitiveKind::Text {
                    color,
                    text,
                    font_id,
                } => {
                    switch_to_plain_state!();

                    let positioned_glyphs = text.positioned_glyphs(p_scale_factor as f32);
                    let (glyph_cache, cache_tex) = self.cache.glyph_cache_mut_and_tex();
                    // Queue the glyphs to be cached.
                    for glyph in positioned_glyphs {
                        glyph_cache.queue_glyph(font_id.index(), glyph.clone());
                    }

                    glyph_cache
                        .cache_queued(|rect, data| {
                            let offset = [rect.min.x as u16, rect.min.y as u16];
                            let size = [rect.width() as u16, rect.height() as u16];

                            let new_data = data
                                .iter()
                                .map(|x| [255, 255, 255, *x])
                                .collect::<Vec<[u8; 4]>>();

                            if let Err(err) =
                                renderer.update_texture(cache_tex, offset, size, &new_data)
                            {
                                warn!("Failed to update texture: {:?}", err);
                            }
                        })
                        .unwrap();

                    let color = srgba_to_linear(color.to_fsa().into());

                    for g in positioned_glyphs {
                        if let Ok(Some((uv_rect, screen_rect))) =
                            glyph_cache.rect_for(font_id.index(), g)
                        {
                            let uv = Aabr {
                                min: Vec2::new(uv_rect.min.x, uv_rect.max.y),
                                max: Vec2::new(uv_rect.max.x, uv_rect.min.y),
                            };
                            let rect = Aabr {
                                min: Vec2::new(
                                    vx(screen_rect.min.x as f64 / p_scale_factor
                                        - self.ui.win_w / 2.0),
                                    vy(self.ui.win_h / 2.0
                                        - screen_rect.max.y as f64 / p_scale_factor),
                                ),
                                max: Vec2::new(
                                    vx(screen_rect.max.x as f64 / p_scale_factor
                                        - self.ui.win_w / 2.0),
                                    vy(self.ui.win_h / 2.0
                                        - screen_rect.min.y as f64 / p_scale_factor),
                                ),
                            };
                            mesh.push_quad(create_ui_quad(rect, uv, color, UiMode::Text));
                        }
                    }
                }
                PrimitiveKind::Rectangle { color } => {
                    let color = srgba_to_linear(color.to_fsa().into());
                    // Don't draw a transparent rectangle.
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
                    // Don't draw transparent triangle or switch state if there are actually no triangles.
                    let color = srgba_to_linear(Rgba::from(Into::<[f32; 4]>::into(color)));
                    if triangles.is_empty() || color[3] == 0.0 {
                        continue;
                    }

                    switch_to_plain_state!();

                    for tri in triangles {
                        let p1 = Vec2::new(vx(tri[0][0]), vy(tri[0][1]));
                        let p2 = Vec2::new(vx(tri[1][0]), vy(tri[1][1]));
                        let p3 = Vec2::new(vx(tri[2][0]), vy(tri[2][1]));
                        // If triangle is clockwise, reverse it.
                        let (v1, v2): (Vec3<f32>, Vec3<f32>) = ((p2 - p1).into(), (p3 - p1).into());
                        let triangle = if v1.cross(v2).z > 0.0 {
                            [p1.into_array(), p2.into_array(), p3.into_array()]
                        } else {
                            [p2.into_array(), p1.into_array(), p3.into_array()]
                        };
                        mesh.push_tri(create_ui_tri(
                            triangle,
                            [[0.0; 2]; 3],
                            color,
                            UiMode::Geometry,
                        ));
                    }
                }
                PrimitiveKind::Other(container) => {
                    if container.type_id == std::any::TypeId::of::<widgets::ingame::State>() {
                        // Calculate the scale factor to pixels at this 3d point using the camera.
                        if let Some((view_mat, fov)) = cam_params {
                            // Retrieve world position
                            let parameters = container
                                .state_and_style::<widgets::ingame::State, widgets::ingame::Style>()
                                .unwrap()
                                .state
                                .parameters;

                            let pos_in_view = view_mat * Vec4::from_point(parameters.pos);
                            let scale_factor = self.ui.win_w as f64
                                / (-2.0
                                    * pos_in_view.z as f64
                                    * (0.5 * fov as f64).tan()
                                    * parameters.res as f64);
                            // Don't process ingame elements behind the camera or very far away
                            placement = if scale_factor > 0.2 {
                                // Finish current state
                                self.draw_commands.push(match current_state {
                                    State::Plain => {
                                        DrawCommand::plain(start..mesh.vertices().len())
                                    }
                                    State::Image => {
                                        DrawCommand::image(start..mesh.vertices().len())
                                    }
                                });
                                start = mesh.vertices().len();
                                // Push new position command
                                if self.ingame_locals.len() > ingame_local_index {
                                    renderer
                                        .update_consts(
                                            &mut self.ingame_locals[ingame_local_index],
                                            &[parameters.pos.into()],
                                        )
                                        .unwrap();
                                } else {
                                    self.ingame_locals.push(
                                        renderer.create_consts(&[parameters.pos.into()]).unwrap(),
                                    );
                                }
                                self.draw_commands
                                    .push(DrawCommand::WorldPos(Some(ingame_local_index)));
                                ingame_local_index += 1;

                                p_scale_factor = ((scale_factor * 10.0).log2().round().powi(2)
                                    / 10.0)
                                    .min(1.6)
                                    .max(0.2);

                                // Scale down ingame elements that are close to the camera
                                let res = if scale_factor > 3.2 {
                                    parameters.res * scale_factor as f32 / 3.2
                                } else {
                                    parameters.res
                                };

                                Placement::InWorld(parameters.num, Some(res))
                            } else {
                                Placement::InWorld(parameters.num, None)
                            };
                        }
                    }
                }
                _ => {} // TODO: Add this.
                        //PrimitiveKind::TrianglesMultiColor {..} => {println!("primitive kind multicolor with id {:?}", id);}
            }
        }
        // Enter the final command.
        self.draw_commands.push(match current_state {
            State::Plain => DrawCommand::plain(start..mesh.vertices().len()),
            State::Image => DrawCommand::image(start..mesh.vertices().len()),
        });

        // Draw glyph cache (use for debugging).
        /*self.draw_commands
            .push(DrawCommand::Scissor(default_scissor(renderer)));
        start = mesh.vertices().len();
        mesh.push_quad(create_ui_quad(
            Aabr {
                min: (-1.0, -1.0).into(),
                max: (1.0, 1.0).into(),
            },
            Aabr {
                min: (0.0, 1.0).into(),
                max: (1.0, 0.0).into(),
            },
            Rgba::new(1.0, 1.0, 1.0, 0.8),
            UiMode::Text,
        ));
        self.draw_commands
            .push(DrawCommand::plain(start..mesh.vertices().len()));*/

        // Create a larger dynamic model if the mesh is larger than the current model size.
        if self.model.vbuf.len() < mesh.vertices().len() {
            self.model = renderer
                .create_dynamic_model(mesh.vertices().len() * 4 / 3)
                .unwrap();
        }
        // Update model with new mesh.
        renderer.update_model(&self.model, &mesh, 0).unwrap();

        // Move cached graphics to the gpu
        let (graphic_cache, cache_tex) = self.cache.graphic_cache_mut_and_tex();
        graphic_cache.cache_queued(|aabr, data| {
            let offset = aabr.min.into_array();
            let size = aabr.size().into_array();
            if let Err(err) = renderer.update_texture(cache_tex, offset, size, data) {
                warn!("Failed to update texture: {:?}", err);
            }
        });

        // Handle window resizing.
        if let Some(new_dims) = self.window_resized.take() {
            let (old_w, old_h) = self.scale.scaled_window_size().into_tuple();
            self.scale.window_resized(new_dims, renderer);
            let (w, h) = self.scale.scaled_window_size().into_tuple();
            self.ui.handle_event(Input::Resize(w, h));

            // Avoid panic in graphic cache when minimizing.
            // Avoid resetting cache if window size didn't change
            // Somewhat inefficient for elements that won't change size after a window resize
            let res = renderer.get_resolution();
            self.need_cache_resize = res.x > 0 && res.y > 0 && !(old_w == w && old_h == h);
        }
    }

    pub fn render(&self, renderer: &mut Renderer, maybe_globals: Option<&Consts<Globals>>) {
        let mut scissor = default_scissor(renderer);
        let globals = maybe_globals.unwrap_or(&self.default_globals);
        let mut locals = &self.interface_locals;
        for draw_command in self.draw_commands.iter() {
            match draw_command {
                DrawCommand::Scissor(new_scissor) => {
                    scissor = *new_scissor;
                }
                DrawCommand::WorldPos(index) => {
                    locals = index.map_or(&self.interface_locals, |i| &self.ingame_locals[i]);
                }
                DrawCommand::Draw { kind, verts } => {
                    let tex = match kind {
                        DrawKind::Image => self.cache.graphic_cache_tex(),
                        DrawKind::Plain => self.cache.glyph_cache_tex(),
                    };
                    let model = self.model.submodel(verts.clone());
                    renderer.render_ui_element(&model, &tex, scissor, globals, locals);
                }
            }
        }
    }
}

fn default_scissor(renderer: &Renderer) -> Aabr<u16> {
    let (screen_w, screen_h) = renderer.get_resolution().map(|e| e as u16).into_tuple();
    Aabr {
        min: Vec2 { x: 0, y: 0 },
        max: Vec2 {
            x: screen_w,
            y: screen_h,
        },
    }
}
