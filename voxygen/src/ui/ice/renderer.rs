mod background_container;
mod column;
mod compound_graphic;
mod container;
mod image;
mod row;

use super::{
    super::{
        cache::Cache,
        graphic::{self, Graphic, TexId},
    },
    widget, Rotation,
};
use crate::{
    render::{
        create_ui_quad, Consts, DynamicModel, Globals, Mesh, Renderer, UiLocals, UiMode, UiPipeline,
    },
    Error,
};
use common::util::srgba_to_linear;
//use log::warn;
use std::ops::Range;
use vek::*;

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

#[derive(PartialEq)]
enum State {
    Image(TexId),
    Plain,
}

pub enum Primitive {
    // Allocation :(
    Group {
        primitives: Vec<Primitive>,
    },
    Image {
        handle: (widget::image::Handle, Rotation),
        bounds: iced::Rectangle,
        color: Rgba<u8>,
    },
    Rectangle {
        bounds: iced::Rectangle,
        color: Rgba<u8>,
    },
}

// Optimization idea inspired by what I think iced wgpu renderer may be doing
// Could have layers of things which don't intersect and thus can be reordered
// arbitrarily

pub struct IcedRenderer {
    //image_map: Map<(Image, Rotation)>,
    cache: Cache,
    // Model for drawing the ui
    model: DynamicModel<UiPipeline>,
    // Consts to specify positions of ingame elements (e.g. Nametags)
    ingame_locals: Vec<Consts<UiLocals>>,
    // Consts for default ui drawing position (ie the interface)
    interface_locals: Consts<UiLocals>,
    default_globals: Consts<Globals>,

    // Used to delay cache resizing until after current frame is drawn
    //need_cache_resize: bool,
    half_res: Vec2<f32>,
    // Pixel perfection alignment
    align: Vec2<f32>,
    // Pretend dims :) (i.e. scaled)
    win_dims: Vec2<f32>,

    // Per-frame/update
    current_state: State,
    mesh: Mesh<UiPipeline>,
    start: usize,
    // Draw commands for the next render
    draw_commands: Vec<DrawCommand>,
    //current_scissor: Aabr<u16>,
}
impl IcedRenderer {
    pub fn new(renderer: &mut Renderer, scaled_dims: Vec2<f32>) -> Result<Self, Error> {
        let (half_res, align) = Self::calculate_resolution_dependents(renderer.get_resolution());

        Ok(Self {
            cache: Cache::new(renderer)?,
            draw_commands: Vec::new(),
            model: renderer.create_dynamic_model(100)?,
            interface_locals: renderer.create_consts(&[UiLocals::default()])?,
            default_globals: renderer.create_consts(&[Globals::default()])?,
            ingame_locals: Vec::new(),
            mesh: Mesh::new(),
            current_state: State::Plain,
            half_res,
            align,
            win_dims: scaled_dims,
            start: 0,
            //current_scissor: default_scissor(renderer),
        })
    }

    pub fn add_graphic(&mut self, graphic: Graphic) -> graphic::Id {
        self.cache.add_graphic(graphic)
    }

    pub fn resize(&mut self, scaled_dims: Vec2<f32>, renderer: &mut Renderer) {
        self.win_dims = scaled_dims;

        self.update_resolution_dependents(renderer.get_resolution());

        // Resize graphic cache
        self.cache.resize_graphic_cache(renderer);
        // Resize glyph cache
        self.cache.resize_glyph_cache(renderer).unwrap();
    }

    pub fn draw(&mut self, primitive: Primitive, renderer: &mut Renderer) {
        // Re-use memory
        self.draw_commands.clear();
        self.mesh.clear();

        self.current_state = State::Plain;
        self.start = 0;

        //self.current_scissor = default_scissor(renderer);

        self.draw_primitive(primitive, renderer);

        // Enter the final command.
        self.draw_commands.push(match self.current_state {
            State::Plain => DrawCommand::plain(self.start..self.mesh.vertices().len()),
            State::Image(id) => DrawCommand::image(self.start..self.mesh.vertices().len(), id),
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
        if self.model.vbuf.len() < self.mesh.vertices().len() {
            self.model = renderer
                .create_dynamic_model(self.mesh.vertices().len() * 4 / 3)
                .unwrap();
        }
        // Update model with new mesh.
        renderer.update_model(&self.model, &self.mesh, 0).unwrap();

        // Handle window resizing.
        /*if let Some(new_dims) = self.window_resized.take() {
            let (old_w, old_h) = self.scale.scaled_window_size().into_tuple();
            self.scale.window_resized(new_dims, renderer);
            let (w, h) = self.scale.scaled_window_size().into_tuple();
            self.ui.handle_event(Input::Resize(w, h));

            // Avoid panic in graphic cache when minimizing.
            // Avoid resetting cache if window size didn't change
            // Somewhat inefficient for elements that won't change size after a window resize
            let res = renderer.get_resolution();
            self.need_cache_resize = res.x > 0 && res.y > 0 && !(old_w == w && old_h == h);
        }*/
    }

    // Returns (half_res, align)
    fn calculate_resolution_dependents(res: Vec2<u16>) -> (Vec2<f32>, Vec2<f32>) {
        let half_res = res.map(|e| e as f32 / 2.0);
        let align = align(res);

        (half_res, align)
    }

    fn update_resolution_dependents(&mut self, res: Vec2<u16>) {
        let (half_res, align) = Self::calculate_resolution_dependents(res);
        self.half_res = half_res;
        self.align = align;
    }

    fn gl_aabr(&self, bounds: iced::Rectangle) -> Aabr<f32> {
        /*let (ui_win_w, ui_win_h) = self.win_dims.into_tuple();
        let (l, b) = aabr.min.into_tuple();
        let (r, t) = aabr.max.into_tuple();
        let vx = |x: f64| (x / ui_win_w * 2.0) as f32;
        let vy = |y: f64| (y / ui_win_h * 2.0) as f32;
        let min = Vec2::new(
            ((vx(l) * half_res.x + x_align).round() - x_align) / half_res.x,
            ((vy(b) * half_res.y + y_align).round() - y_align) / half_res.y,
        );
        let max = Vec2::new(
            ((vx(r) * half_res.x + x_align).round() - x_align) / half_res.x,
            ((vy(t) * half_res.y + y_align).round() - y_align) / half_res.y,
        );*/
        let flipped_y = self.win_dims.y - bounds.y;
        let half_win_dims = self.win_dims.map(|e| e / 2.0);
        let half_res = self.half_res;
        let min = (((Vec2::new(bounds.x, flipped_y - bounds.height) - half_win_dims)
            / half_win_dims
            * half_res
            + self.align)
            .map(|e| e.round())
            - self.align)
            / half_res;
        let max = (((Vec2::new(bounds.x + bounds.width, flipped_y) - half_win_dims)
            / half_win_dims
            * half_res
            + self.align)
            .map(|e| e.round())
            - self.align)
            / half_res;
        Aabr { min, max }
    }

    fn draw_primitive(&mut self, primitive: Primitive, renderer: &mut Renderer) {
        match primitive {
            Primitive::Group { primitives } => {
                primitives
                    .into_iter()
                    .for_each(|p| self.draw_primitive(p, renderer));
            },
            Primitive::Image {
                handle,
                bounds,
                color,
            } => {
                let (graphic_id, rotation) = handle;
                let gl_aabr = self.gl_aabr(bounds);

                let graphic_cache = self.cache.graphic_cache_mut();

                match graphic_cache.get_graphic(graphic_id) {
                    Some(Graphic::Blank) | None => return,
                    _ => {},
                }

                let color = srgba_to_linear(color.map(|e| e as f32 / 255.0));
                // Don't draw a transparent image.
                if color[3] == 0.0 {
                    return;
                }

                let resolution = Vec2::new(
                    (gl_aabr.size().w * self.half_res.x).round() as u16,
                    (gl_aabr.size().h * self.half_res.y).round() as u16,
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

                // Cache graphic at particular resolution.
                let (uv_aabr, tex_id) = match graphic_cache.cache_res(
                    renderer,
                    graphic_id,
                    resolution,
                    source_aabr,
                    rotation,
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
                    None => return,
                };

                // Switch to the image state if we are not in it already or if a different
                // texture id was being used.
                self.switch_state(State::Image(tex_id));

                self.mesh
                    .push_quad(create_ui_quad(gl_aabr, uv_aabr, color, UiMode::Image));
            },
            Primitive::Rectangle { bounds, color } => {
                let color = srgba_to_linear(color.map(|e| e as f32 / 255.0));
                // Don't draw a transparent rectangle.
                if color[3] == 0.0 {
                    return;
                }

                self.switch_state(State::Plain);

                self.mesh.push_quad(create_ui_quad(
                    self.gl_aabr(bounds),
                    Aabr {
                        min: Vec2::zero(),
                        max: Vec2::zero(),
                    },
                    color,
                    UiMode::Geometry,
                ));
            },
        }
    }

    // Switches to the specified state if not already in it
    // If switch occurs current state is converted into a draw command
    fn switch_state(&mut self, state: State) {
        if self.current_state != state {
            let vert_range = self.start..self.mesh.vertices().len();
            let draw_command = match self.current_state {
                State::Plain => DrawCommand::plain(vert_range),
                State::Image(id) => DrawCommand::image(vert_range, id),
            };
            self.draw_commands.push(draw_command);
            self.start = self.mesh.vertices().len();
            self.current_state = state;
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

// Given the the resolution determines the offset needed to align integer
// offsets from the center of the sceen to pixels
#[inline(always)]
fn align(res: Vec2<u16>) -> Vec2<f32> {
    // TODO: does this logic still apply in iced's coordinate system?
    // If the resolution is odd then the center of the screen will be within the
    // middle of a pixel so we need to offset by 0.5 pixels to be on the edge of
    // a pixel
    res.map(|e| (e & 1) as f32 * 0.5)
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

impl iced::Renderer for IcedRenderer {
    // Default styling
    type Defaults = ();
    // TODO: use graph of primitives to enable diffing???
    type Output = (Primitive, iced::MouseCursor);

    fn layout<'a, M>(
        &mut self,
        element: &iced::Element<'a, M, Self>,
        limits: &iced::layout::Limits,
    ) -> iced::layout::Node {
        let node = element.layout(self, limits);

        // Trim text measurements cache?

        node
    }
}

// TODO: impl Debugger
