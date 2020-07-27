mod cache;
mod event;
mod graphic;
mod scale;
mod widgets;
#[macro_use]
pub mod img_ids;
#[macro_use]
pub mod fonts;

pub use event::Event;
pub use graphic::{Graphic, SampleStrat, Transform};
pub use scale::{Scale, ScaleMode};
pub use widgets::{
    image_frame::ImageFrame,
    image_slider::ImageSlider,
    ingame::{Ingame, Ingameable},
    radio_list::RadioList,
    slot,
    toggle_button::ToggleButton,
    tooltip::{Tooltip, TooltipManager, Tooltipable},
};

use crate::{
    render::{
        create_ui_quad, create_ui_tri, Consts, DynamicModel, Globals, Mesh, RenderError, Renderer,
        UiLocals, UiMode, UiPipeline,
    },
    window::Window,
    Error,
};
#[rustfmt::skip]
use ::image::GenericImageView;
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
use graphic::{Rotation, TexId};
use std::{
    f32, f64,
    fs::File,
    io::{BufReader, Read},
    ops::Range,
    sync::Arc,
    time::Duration,
};
use tracing::{error, warn};
use vek::*;

#[derive(Debug)]
pub enum UiError {
    RenderError(RenderError),
}

enum DrawKind {
    Image(TexId),
    // Text and non-textured geometry
    Plain,
}
enum DrawCommand {
    Draw { kind: DrawKind, verts: Range<usize> },
    Scissor(Aabr<u16>),
    WorldPos(Option<usize>),
}
impl DrawCommand {
    fn image(verts: Range<usize>, id: TexId) -> DrawCommand {
        DrawCommand::Draw {
            kind: DrawKind::Image(id),
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

    #[allow(clippy::redundant_clone)] // TODO: Pending review in #587
    fn parse(mut buf_reader: BufReader<File>) -> Result<Self, assets::Error> {
        let mut buf = Vec::new();
        buf_reader.read_to_end(&mut buf)?;
        Ok(Font(text::Font::from_bytes(buf.clone()).unwrap()))
    }
}

pub struct Ui {
    pub ui: conrod_core::Ui,
    image_map: Map<(graphic::Id, Rotation)>,
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
            Duration::from_millis(1),
            Duration::from_millis(100),
            scale.scale_factor_logical(),
        );

        Ok(Self {
            ui,
            image_map: Map::new(),
            cache: Cache::new(renderer)?,
            draw_commands: Vec::new(),
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
    pub fn scale(&self) -> Scale { self.scale }

    pub fn add_graphic(&mut self, graphic: Graphic) -> image::Id {
        self.image_map
            .insert((self.cache.add_graphic(graphic), Rotation::None))
    }

    pub fn add_graphic_with_rotations(&mut self, graphic: Graphic) -> img_ids::Rotations {
        let graphic_id = self.cache.add_graphic(graphic);
        img_ids::Rotations {
            none: self.image_map.insert((graphic_id, Rotation::None)),
            cw90: self.image_map.insert((graphic_id, Rotation::Cw90)),
            cw180: self.image_map.insert((graphic_id, Rotation::Cw180)),
            cw270: self.image_map.insert((graphic_id, Rotation::Cw270)),
            // Hacky way to make sure a source rectangle always faces north regardless of player
            // orientation.
            // This is an easy way to get around Conrod's lack of rotation data for images (for this
            // specific use case).
            source_north: self.image_map.insert((graphic_id, Rotation::SourceNorth)),
            // Hacky way to make sure a target rectangle always faces north regardless of player
            // orientation.
            // This is an easy way to get around Conrod's lack of rotation data for images (for this
            // specific use case).
            target_north: self.image_map.insert((graphic_id, Rotation::TargetNorth)),
        }
    }

    pub fn replace_graphic(&mut self, id: image::Id, graphic: Graphic) {
        let graphic_id = if let Some((graphic_id, _)) = self.image_map.get(&id) {
            *graphic_id
        } else {
            error!("Failed to replace graphic the provided id is not in use");
            return;
        };
        self.cache.replace_graphic(graphic_id, graphic);
        self.image_map.replace(id, (graphic_id, Rotation::None));
    }

    pub fn new_font(&mut self, font: Arc<Font>) -> font::Id {
        self.ui.fonts.insert(font.as_ref().0.clone())
    }

    pub fn id_generator(&mut self) -> Generator { self.ui.widget_id_generator() }

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
    pub fn widget_graph(&self) -> &Graph { self.ui.widget_graph() }

    pub fn handle_event(&mut self, event: Event) {
        match event.0 {
            Input::Resize(w, h) => {
                if w > 1.0 && h > 1.0 {
                    self.window_resized = Some(Vec2::new(w, h))
                }
            },
            Input::Touch(touch) => self.ui.handle_event(Input::Touch(Touch {
                xy: self.scale.scale_point(touch.xy.into()).into_array(),
                ..touch
            })),
            Input::Motion(motion) => self.ui.handle_event(Input::Motion(match motion {
                Motion::MouseCursor { x, y } => {
                    let (x, y) = self.scale.scale_point(Vec2::new(x, y)).into_tuple();
                    Motion::MouseCursor { x, y }
                },
                Motion::MouseRelative { x, y } => {
                    let (x, y) = self.scale.scale_point(Vec2::new(x, y)).into_tuple();
                    Motion::MouseRelative { x, y }
                },
                Motion::Scroll { x, y } => {
                    let (x, y) = self.scale.scale_point(Vec2::new(x, y)).into_tuple();
                    Motion::Scroll { x, y }
                },
                _ => motion,
            })),
            _ => self.ui.handle_event(event.0),
        }
    }

    pub fn widget_input(&self, id: widget::Id) -> Widget { self.ui.widget_input(id) }

    #[allow(clippy::float_cmp)] // TODO: Pending review in #587
    pub fn maintain(&mut self, renderer: &mut Renderer, view_projection_mat: Option<Mat4<f32>>) {
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
            self.cache.resize_graphic_cache(renderer);
            // Resize glyph cache
            self.cache.resize_glyph_cache(renderer).unwrap();

            self.need_cache_resize = false;
        }

        self.draw_commands.clear();
        let mut mesh = Mesh::new();

        let (half_res, x_align, y_align) = {
            let res = renderer.get_resolution();
            (
                res.map(|e| e as f32 / 2.0),
                (res.x & 1) as f32 * 0.5,
                (res.y & 1) as f32 * 0.5,
            )
        };

        enum State {
            Image(TexId),
            Plain,
        };

        let mut current_state = State::Plain;
        let mut start = 0;

        let window_scissor = default_scissor(renderer);
        let mut current_scissor = window_scissor;

        let mut ingame_local_index = 0;

        enum Placement {
            Interface,
            // Number of primitives left to render ingame and visibility
            InWorld(usize, bool),
        };

        let mut placement = Placement::Interface;
        let p_scale_factor = self.scale.scale_factor_physical();

        // Switches to the `Plain` state and completes the previous `Command` if not
        // already in the `Plain` state.
        macro_rules! switch_to_plain_state {
            () => {
                if let State::Image(id) = current_state {
                    self.draw_commands
                        .push(DrawCommand::image(start..mesh.vertices().len(), id));
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
                let intersection = Aabr {
                    min: Vec2 {
                        x: (min_x * scale_factor) as u16,
                        y: (min_y * scale_factor) as u16,
                    },
                    max: Vec2 {
                        x: ((min_x + w) * scale_factor) as u16,
                        y: ((min_y + h) * scale_factor) as u16,
                    },
                }
                .intersection(window_scissor);

                if intersection.is_valid() {
                    intersection
                } else {
                    Aabr::new_empty(Vec2::zero())
                }
            };
            if new_scissor != current_scissor {
                // Finish the current command.
                self.draw_commands.push(match current_state {
                    State::Plain => DrawCommand::plain(start..mesh.vertices().len()),
                    State::Image(id) => DrawCommand::image(start..mesh.vertices().len(), id),
                });
                start = mesh.vertices().len();

                // Update the scissor and produce a command.
                current_scissor = new_scissor;
                self.draw_commands.push(DrawCommand::Scissor(new_scissor));
            }

            match placement {
                // No primitives left to place in the world at the current position, go back to
                // drawing the interface
                Placement::InWorld(0, _) => {
                    placement = Placement::Interface;
                    // Finish current state
                    self.draw_commands.push(match current_state {
                        State::Plain => DrawCommand::plain(start..mesh.vertices().len()),
                        State::Image(id) => DrawCommand::image(start..mesh.vertices().len(), id),
                    });
                    start = mesh.vertices().len();
                    // Push new position command
                    self.draw_commands.push(DrawCommand::WorldPos(None));
                },
                // Primitives still left to draw ingame
                Placement::InWorld(num_prims, visible) => match kind {
                    // Other types aren't drawn & shouldn't decrement the number of primitives left
                    // to draw ingame
                    PrimitiveKind::Other(_) => {},
                    // Decrement the number of primitives left
                    _ => {
                        placement = Placement::InWorld(num_prims - 1, visible);
                        // Behind the camera
                        if !visible {
                            continue;
                        }
                    },
                },
                Placement::Interface => {},
            }

            // Functions for converting for conrod scalar coords to GL vertex coords (-1.0
            // to 1.0).
            let (ui_win_w, ui_win_h) = (self.ui.win_w, self.ui.win_h);
            let vx = |x: f64| (x / ui_win_w * 2.0) as f32;
            let vy = |y: f64| (y / ui_win_h * 2.0) as f32;
            let gl_aabr = |rect: Rect| {
                let (l, r, b, t) = rect.l_r_b_t();
                let min = Vec2::new(
                    ((vx(l) * half_res.x + x_align).round() - x_align) / half_res.x,
                    ((vy(b) * half_res.y + y_align).round() - y_align) / half_res.y,
                );
                let max = Vec2::new(
                    ((vx(r) * half_res.x + x_align).round() - x_align) / half_res.x,
                    ((vy(t) * half_res.y + y_align).round() - y_align) / half_res.y,
                );
                Aabr { min, max }
            };

            match kind {
                PrimitiveKind::Image {
                    image_id,
                    color,
                    source_rect,
                } => {
                    let (graphic_id, rotation) = self
                        .image_map
                        .get(&image_id)
                        .expect("Image does not exist in image map");
                    let graphic_cache = self.cache.graphic_cache_mut();
                    let gl_aabr = gl_aabr(rect);
                    let (source_aabr, gl_size) = {
                        // Transform the source rectangle into uv coordinate.
                        // TODO: Make sure this is right.  Especially the conversions.
                        let ((uv_l, uv_r, uv_b, uv_t), gl_size) =
                            match graphic_cache.get_graphic(*graphic_id) {
                                Some(Graphic::Blank) | None => continue,
                                Some(Graphic::Image(image)) => {
                                    source_rect.and_then(|src_rect| {
                                        let (image_w, image_h) = image.dimensions();
                                        let (source_w, source_h) = src_rect.w_h();
                                        let gl_size = gl_aabr.size();
                                        if image_w == 0
                                            || image_h == 0
                                            || source_w < 1.0
                                            || source_h < 1.0
                                            || gl_size.reduce_partial_min() < f32::EPSILON
                                        {
                                            None
                                        } else {
                                            // Multiply drawn image size by ratio of original image
                                            // size to
                                            // source rectangle size (since as the proportion of the
                                            // image gets
                                            // smaller, the drawn size should get bigger), up to the
                                            // actual
                                            // size of the original image.
                                            let ratio_x = (image_w as f64 / source_w).min(
                                                (image_w as f64 / (gl_size.w * half_res.x) as f64)
                                                    .max(1.0),
                                            );
                                            let ratio_y = (image_h as f64 / source_h).min(
                                                (image_h as f64 / (gl_size.h * half_res.y) as f64)
                                                    .max(1.0),
                                            );
                                            let (l, r, b, t) = src_rect.l_r_b_t();
                                            Some((
                                                (
                                                    l / image_w as f64, /* * ratio_x*/
                                                    r / image_w as f64, /* * ratio_x*/
                                                    b / image_h as f64, /* * ratio_y*/
                                                    t / image_h as f64, /* * ratio_y*/
                                                ),
                                                Extent2::new(
                                                    (gl_size.w as f64 * ratio_x) as f32,
                                                    (gl_size.h as f64 * ratio_y) as f32,
                                                ),
                                            ))
                                            /* ((l / image_w as f64),
                                            (r / image_w as f64),
                                            (b / image_h as f64),
                                            (t / image_h as f64)) */
                                        }
                                    })
                                },
                                // No easy way to interpret source_rect for voxels...
                                Some(Graphic::Voxel(..)) => None,
                            }
                            .unwrap_or_else(|| ((0.0, 1.0, 0.0, 1.0), gl_aabr.size()));
                        (
                            Aabr {
                                min: Vec2::new(uv_l, uv_b),
                                max: Vec2::new(uv_r, uv_t),
                            },
                            gl_size,
                        )
                    };

                    let resolution = Vec2::new(
                        (gl_size.w * half_res.x).round() as u16,
                        (gl_size.h * half_res.y).round() as u16,
                    );

                    // Don't do anything if resolution is zero
                    if resolution.map(|e| e == 0).reduce_or() {
                        continue;
                        // TODO: consider logging uneeded elements
                    }

                    let color =
                        srgba_to_linear(color.unwrap_or(conrod_core::color::WHITE).to_fsa().into());

                    // Cache graphic at particular resolution.
                    let (uv_aabr, tex_id) = match graphic_cache.cache_res(
                        renderer,
                        *graphic_id,
                        resolution,
                        source_aabr,
                        *rotation,
                    ) {
                        // TODO: get dims from graphic_cache (or have it return floats directly)
                        Some((aabr, tex_id)) => {
                            let cache_dims = graphic_cache
                                .get_tex(tex_id)
                                .get_dimensions()
                                .map(|e| e as f32);
                            let min = Vec2::new(aabr.min.x as f32, aabr.max.y as f32) / cache_dims;
                            let max = Vec2::new(aabr.max.x as f32, aabr.min.y as f32) / cache_dims;
                            (Aabr { min, max }, tex_id)
                        },
                        None => continue,
                    };

                    match current_state {
                        // Switch to the image state if we are not in it already.
                        State::Plain => {
                            self.draw_commands
                                .push(DrawCommand::plain(start..mesh.vertices().len()));
                            start = mesh.vertices().len();
                            current_state = State::Image(tex_id);
                        },
                        // If the image is cached in a different texture switch to the new one
                        State::Image(id) if id != tex_id => {
                            self.draw_commands
                                .push(DrawCommand::image(start..mesh.vertices().len(), id));
                            start = mesh.vertices().len();
                            current_state = State::Image(tex_id);
                        },
                        State::Image(_) => {},
                    }

                    mesh.push_quad(create_ui_quad(gl_aabr, uv_aabr, color, match *rotation {
                        Rotation::None | Rotation::Cw90 | Rotation::Cw180 | Rotation::Cw270 => {
                            UiMode::Image
                        },
                        Rotation::SourceNorth => UiMode::ImageSourceNorth,
                        Rotation::TargetNorth => UiMode::ImageTargetNorth,
                    }));
                },
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
                },
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
                            min: Vec2::zero(),
                            max: Vec2::zero(),
                        },
                        color,
                        UiMode::Geometry,
                    ));
                },
                PrimitiveKind::TrianglesSingleColor { color, triangles } => {
                    // Don't draw transparent triangle or switch state if there are actually no
                    // triangles.
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
                        /* // If triangle is counter-clockwise, reverse it.
                        let (v1, v2): (Vec3<f32>, Vec3<f32>) = ((p2 - p1).into(), (p3 - p1).into());
                        let triangle = if v1.cross(v2).z > 0.0 {
                            [p2.into_array(), p1.into_array(), p3.into_array()]
                        } else {
                            [p1.into_array(), p2.into_array(), p3.into_array()]
                        }; */
                        mesh.push_tri(create_ui_tri(
                            triangle,
                            [[0.0; 2]; 3],
                            color,
                            UiMode::Geometry,
                        ));
                    }
                },
                PrimitiveKind::Other(container) => {
                    if container.type_id == std::any::TypeId::of::<widgets::ingame::State>() {
                        // Calculate the scale factor to pixels at this 3d point using the camera.
                        if let Some(view_projection_mat) = view_projection_mat {
                            // Retrieve world position
                            let parameters = container
                                .state_and_style::<widgets::ingame::State, widgets::ingame::Style>()
                                .unwrap()
                                .state
                                .parameters;

                            let pos_on_screen = (view_projection_mat
                                * Vec4::from_point(parameters.pos))
                            .homogenized();
                            let visible = if pos_on_screen.z > -1.0 && pos_on_screen.z < 1.0 {
                                let x = pos_on_screen.x;
                                let y = pos_on_screen.y;
                                let (w, h) = parameters.dims.into_tuple();
                                let (half_w, half_h) = (w / ui_win_w as f32, h / ui_win_h as f32);
                                (x - half_w < 1.0 && x + half_w > -1.0)
                                    && (y - half_h < 1.0 && y + half_h > -1.0)
                            } else {
                                false
                            };
                            // Don't process ingame elements outside the frustum
                            placement = if visible {
                                // Finish current state
                                self.draw_commands.push(match current_state {
                                    State::Plain => {
                                        DrawCommand::plain(start..mesh.vertices().len())
                                    },
                                    State::Image(id) => {
                                        DrawCommand::image(start..mesh.vertices().len(), id)
                                    },
                                });
                                start = mesh.vertices().len();

                                // Push new position command
                                let world_pos = Vec4::from_point(parameters.pos);
                                if self.ingame_locals.len() > ingame_local_index {
                                    renderer
                                        .update_consts(
                                            &mut self.ingame_locals[ingame_local_index],
                                            &[world_pos.into()],
                                        )
                                        .unwrap();
                                } else {
                                    self.ingame_locals
                                        .push(renderer.create_consts(&[world_pos.into()]).unwrap());
                                }
                                self.draw_commands
                                    .push(DrawCommand::WorldPos(Some(ingame_local_index)));
                                ingame_local_index += 1;

                                Placement::InWorld(parameters.num, true)
                            } else {
                                Placement::InWorld(parameters.num, false)
                            };
                        }
                    }
                },
                _ => {}, /* TODO: Add this.
                          *PrimitiveKind::TrianglesMultiColor {..} => {println!("primitive kind
                          * multicolor with id {:?}", id);} */
            }
        }
        // Enter the final command.
        self.draw_commands.push(match current_state {
            State::Plain => DrawCommand::plain(start..mesh.vertices().len()),
            State::Image(id) => DrawCommand::image(start..mesh.vertices().len(), id),
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

        // Create a larger dynamic model if the mesh is larger than the current model
        // size.
        if self.model.vbuf.len() < mesh.vertices().len() {
            self.model = renderer
                .create_dynamic_model(mesh.vertices().len() * 4 / 3)
                .unwrap();
        }
        // Update model with new mesh.
        renderer.update_model(&self.model, &mesh, 0).unwrap();

        // Handle window resizing.
        if let Some(new_dims) = self.window_resized.take() {
            let (old_w, old_h) = self.scale.scaled_window_size().into_tuple();
            self.scale.window_resized(new_dims, renderer);
            let (w, h) = self.scale.scaled_window_size().into_tuple();
            self.ui.handle_event(Input::Resize(w, h));

            // Avoid panic in graphic cache when minimizing.
            // Avoid resetting cache if window size didn't change
            // Somewhat inefficient for elements that won't change size after a window
            // resize
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
                },
                DrawCommand::WorldPos(index) => {
                    locals = index.map_or(&self.interface_locals, |i| &self.ingame_locals[i]);
                },
                DrawCommand::Draw { kind, verts } => {
                    let tex = match kind {
                        DrawKind::Image(tex_id) => self.cache.graphic_cache().get_tex(*tex_id),
                        DrawKind::Plain => self.cache.glyph_cache_tex(),
                    };
                    let model = self.model.submodel(verts.clone());
                    renderer.render_ui_element(&model, tex, scissor, globals, locals);
                },
            }
        }
    }
}

fn default_scissor(renderer: &Renderer) -> Aabr<u16> {
    let (screen_w, screen_h) = renderer.get_resolution().into_tuple();
    Aabr {
        min: Vec2 { x: 0, y: 0 },
        max: Vec2 {
            x: screen_w,
            y: screen_h,
        },
    }
}
