mod defaults;
mod primitive;
pub mod style;
mod widget;

pub use defaults::Defaults;

pub(self) use primitive::Primitive;

use super::{
    super::graphic::{self, Graphic, TexId},
    cache::Cache,
    widget::image,
    Font, FontId, RawFont, Rotation,
};
use crate::{
    error::Error,
    render::{
        create_ui_quad, create_ui_quad_vert_gradient, DynamicModel, Mesh, Renderer, UiBoundLocals,
        UiDrawer, UiLocals, UiMode, UiVertex,
    },
};
use common::{slowjob::SlowJobPool, util::srgba_to_linear};
use common_base::span;
use std::{convert::TryInto, ops::Range};
use vek::*;

enum DrawKind {
    Image(TexId),
    // Text and non-textured geometry
    Plain,
}

#[allow(dead_code)] // TODO: remove once WorldPos is used
enum DrawCommand {
    Draw { kind: DrawKind, verts: Range<u32> },
    Scissor(Aabr<u16>),
    WorldPos(Option<usize>),
}
impl DrawCommand {
    fn image(verts: Range<usize>, id: TexId) -> DrawCommand {
        DrawCommand::Draw {
            kind: DrawKind::Image(id),
            // TODO: move conversion into helper method so we don't have to write it out so many
            // times
            verts: verts
                .start
                .try_into()
                .expect("Vertex count for UI rendering does not fit in a u32!")
                ..verts
                    .end
                    .try_into()
                    .expect("Vertex count for UI rendering does not fit in a u32!"),
        }
    }

    fn plain(verts: Range<usize>) -> DrawCommand {
        DrawCommand::Draw {
            kind: DrawKind::Plain,
            verts: verts
                .start
                .try_into()
                .expect("Vertex count for UI rendering does not fit in a u32!")
                ..verts
                    .end
                    .try_into()
                    .expect("Vertex count for UI rendering does not fit in a u32!"),
        }
    }
}

#[derive(PartialEq)]
enum State {
    Image(TexId),
    Plain,
}

// Optimization idea inspired by what I think iced wgpu renderer may be doing:
// Could have layers of things which don't intersect and thus can be reordered
// arbitrarily

pub struct IcedRenderer {
    //image_map: Map<(Image, Rotation)>,
    cache: Cache,
    // Model for drawing the ui
    model: DynamicModel<UiVertex>,
    // Consts to specify positions of ingame elements (e.g. Nametags)
    ingame_locals: Vec<UiBoundLocals>,
    // Consts for default ui drawing position (ie the interface)
    interface_locals: UiBoundLocals,

    // Used to delay cache resizing until after current frame is drawn
    //need_cache_resize: bool,
    // Half of physical resolution
    half_res: Vec2<f32>,
    // Pixel perfection alignment
    align: Vec2<f32>,
    // Scale factor between physical and win dims
    p_scale: f32,
    // Pretend dims :) (i.e. scaled)
    win_dims: Vec2<f32>,
    // Scissor for the whole window
    window_scissor: Aabr<u16>,

    // Per-frame/update
    current_state: State,
    mesh: Mesh<UiVertex>,
    glyphs: Vec<(usize, usize, Rgba<f32>, Vec2<u32>)>,
    // Output from glyph_brush in the previous frame
    // It can sometimes ask you to redraw with these instead (idk if that is done with
    // pre-positioned glyphs)
    last_glyph_verts: Vec<(Aabr<f32>, Aabr<f32>)>,
    start: usize,
    // Draw commands for the next render
    draw_commands: Vec<DrawCommand>,
}
impl IcedRenderer {
    pub fn new(
        renderer: &mut Renderer,
        scaled_resolution: Vec2<f32>,
        physical_resolution: Vec2<u32>,
        default_font: Font,
    ) -> Result<Self, Error> {
        let (half_res, align, p_scale) =
            Self::calculate_resolution_dependents(physical_resolution, scaled_resolution);

        let interface_locals = renderer.create_ui_bound_locals(&[UiLocals::default()]);

        Ok(Self {
            cache: Cache::new(renderer, default_font)?,
            draw_commands: Vec::new(),
            model: renderer.create_dynamic_model(100),
            interface_locals,
            ingame_locals: Vec::new(),
            mesh: Mesh::new(),
            glyphs: Vec::new(),
            last_glyph_verts: Vec::new(),
            current_state: State::Plain,
            half_res,
            align,
            p_scale,
            win_dims: scaled_resolution,
            window_scissor: default_scissor(physical_resolution),
            start: 0,
        })
    }

    pub fn add_font(&mut self, font: RawFont) -> FontId { self.cache.add_font(font) }

    /// Allows clearing out the fonts when switching languages
    pub fn clear_fonts(&mut self, default_font: Font) { self.cache.clear_fonts(default_font); }

    pub fn add_graphic(&mut self, graphic: Graphic) -> graphic::Id {
        self.cache.add_graphic(graphic)
    }

    pub fn replace_graphic(&mut self, id: graphic::Id, graphic: Graphic) {
        self.cache.replace_graphic(id, graphic);
    }

    fn image_dims(&self, handle: image::Handle) -> (u32, u32) {
        self
            .cache
            .graphic_cache()
            .get_graphic_dims((handle, Rotation::None))
            // TODO: don't unwrap
            .unwrap()
    }

    pub fn resize(
        &mut self,
        scaled_resolution: Vec2<f32>,
        physical_resolution: Vec2<u32>,
        renderer: &mut Renderer,
    ) {
        self.win_dims = scaled_resolution;
        self.window_scissor = default_scissor(physical_resolution);

        self.update_resolution_dependents(physical_resolution);

        // Resize graphic cache
        self.cache.resize_graphic_cache(renderer);
        // Resize glyph cache
        self.cache.resize_glyph_cache(renderer).unwrap();
    }

    pub fn draw(
        &mut self,
        primitive: Primitive,
        renderer: &mut Renderer,
        pool: Option<&SlowJobPool>,
    ) {
        span!(_guard, "draw", "IcedRenderer::draw");
        // Re-use memory
        self.draw_commands.clear();
        self.mesh.clear();
        self.glyphs.clear();

        self.current_state = State::Plain;
        self.start = 0;

        self.draw_primitive(primitive, Vec2::zero(), 1.0, renderer, pool);

        // Enter the final command.
        self.draw_commands.push(match self.current_state {
            State::Plain => DrawCommand::plain(self.start..self.mesh.vertices().len()),
            State::Image(id) => DrawCommand::image(self.start..self.mesh.vertices().len(), id),
        });

        // Draw glyph cache (use for debugging).
        /*self.draw_commands
            .push(DrawCommand::Scissor(self.window_scissor));
        self.start = self.mesh.vertices().len();
        self.mesh.push_quad(create_ui_quad(
            Aabr {
                min: (-1.0, -1.0).into(),
                max: (1.0, 1.0).into(),
            },
            Aabr {
                min: (0.0, 1.0).into(),
                max: (1.0, 0.0).into(),
            },
            Rgba::new(1.0, 1.0, 1.0, 0.3),
            UiMode::Text,
        ));
        self.draw_commands
            .push(DrawCommand::plain(self.start..self.mesh.vertices().len()));*/

        // Fill in placeholder glyph quads
        let (glyph_cache, (cache_tex, _)) = self.cache.glyph_cache_mut_and_tex();
        let half_res = self.half_res;

        let brush_result = glyph_cache.process_queued(
            |rect, tex_data| {
                let offset = rect.min;
                let size = [rect.width(), rect.height()];

                let new_data = tex_data
                    .iter()
                    .map(|x| [255, 255, 255, *x])
                    .collect::<Vec<[u8; 4]>>();

                renderer.update_texture(cache_tex, offset, size, &new_data);
            },
            // Urgh more allocation we don't need
            |vertex_data| {
                let uv_rect = vertex_data.tex_coords;
                let uv = Aabr {
                    min: Vec2::new(uv_rect.min.x, uv_rect.max.y),
                    max: Vec2::new(uv_rect.max.x, uv_rect.min.y),
                };
                let pixel_coords = vertex_data.pixel_coords;
                let rect = Aabr {
                    min: Vec2::new(
                        pixel_coords.min.x / half_res.x - 1.0,
                        1.0 - pixel_coords.max.y / half_res.y,
                    ),
                    max: Vec2::new(
                        pixel_coords.max.x / half_res.x - 1.0,
                        1.0 - pixel_coords.min.y / half_res.y,
                    ),
                };
                (uv, rect)
            },
        );

        match brush_result {
            Ok(brush_action) => {
                match brush_action {
                    glyph_brush::BrushAction::Draw(verts) => self.last_glyph_verts = verts,
                    glyph_brush::BrushAction::ReDraw => {},
                }

                let glyphs = &self.glyphs;
                let mesh = &mut self.mesh;
                let p_scale = self.p_scale;
                let half_res = self.half_res;

                glyphs
                    .iter()
                    .flat_map(|(mesh_index, glyph_count, linear_color, offset)| {
                        let mesh_index = *mesh_index;
                        let linear_color = *linear_color;
                        // Could potentially pass this in as part of the extras
                        let offset =
                            offset.map(|e| e as f32 * p_scale) / half_res * Vec2::new(-1.0, 1.0);
                        (0..*glyph_count).map(move |i| (mesh_index + i * 6, linear_color, offset))
                    })
                    .zip(self.last_glyph_verts.iter())
                    .for_each(|((mesh_index, linear_color, offset), (uv, rect))| {
                        // TODO: add function to vek for this
                        let rect = Aabr {
                            min: rect.min + offset,
                            max: rect.max + offset,
                        };

                        mesh.replace_quad(
                            mesh_index,
                            create_ui_quad(rect, *uv, linear_color, UiMode::Text),
                        )
                    });
            },
            Err(glyph_brush::BrushError::TextureTooSmall { suggested: (x, y) }) => {
                tracing::error!(
                    "Texture to small for all glyphs, would need one of the size: ({}, {})",
                    x,
                    y
                );
            },
        }

        // Create a larger dynamic model if the mesh is larger than the current model
        // size.
        if self.model.len() < self.mesh.vertices().len() {
            self.model = renderer.create_dynamic_model(self.mesh.vertices().len() * 4 / 3);
        }
        // Update model with new mesh.
        renderer.update_model(&self.model, &self.mesh, 0);
    }

    // Returns (half_res, align)
    fn calculate_resolution_dependents(
        res: Vec2<u32>,
        win_dims: Vec2<f32>,
    ) -> (Vec2<f32>, Vec2<f32>, f32) {
        let half_res = res.map(|e| e as f32 / 2.0);
        let align = align(res);
        // Assume to be the same in x and y for now...
        let p_scale = res.x as f32 / win_dims.x;

        (half_res, align, p_scale)
    }

    fn update_resolution_dependents(&mut self, res: Vec2<u32>) {
        let (half_res, align, p_scale) = Self::calculate_resolution_dependents(res, self.win_dims);
        self.half_res = half_res;
        self.align = align;
        self.p_scale = p_scale;
    }

    fn gl_aabr(&self, bounds: iced::Rectangle) -> Aabr<f32> {
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

    fn position_glyphs(
        &mut self,
        bounds: iced::Rectangle,
        horizontal_alignment: iced::HorizontalAlignment,
        vertical_alignment: iced::VerticalAlignment,
        text: &str,
        size: u16,
        font: FontId,
    ) -> Vec<glyph_brush::SectionGlyph> {
        use glyph_brush::{GlyphCruncher, HorizontalAlign, VerticalAlign};
        // TODO: add option to align based on the geometry of the rendered glyphs
        // instead of all possible glyphs
        let (x, h_align) = match horizontal_alignment {
            iced::HorizontalAlignment::Left => (bounds.x, HorizontalAlign::Left),
            iced::HorizontalAlignment::Center => (bounds.center_x(), HorizontalAlign::Center),
            iced::HorizontalAlignment::Right => (bounds.x + bounds.width, HorizontalAlign::Right),
        };

        let (y, v_align) = match vertical_alignment {
            iced::VerticalAlignment::Top => (bounds.y, VerticalAlign::Top),
            iced::VerticalAlignment::Center => (bounds.center_y(), VerticalAlign::Center),
            iced::VerticalAlignment::Bottom => (bounds.y + bounds.height, VerticalAlign::Bottom),
        };

        let p_scale = self.p_scale;

        let section = glyph_brush::Section {
            screen_position: (x * p_scale, y * p_scale),
            bounds: (bounds.width * p_scale, bounds.height * p_scale),
            layout: glyph_brush::Layout::Wrap {
                line_breaker: Default::default(),
                h_align,
                v_align,
            },
            text: vec![glyph_brush::Text {
                text,
                scale: (size as f32 * p_scale).into(),
                font_id: font.0,
                extra: (),
            }],
        };

        self
            .cache
            .glyph_cache_mut()
            .glyphs(section)
            // We would still have to generate vertices for these even if they have no pixels
            // Note: this is somewhat hacky and could fail if there is a non-whitespace character
            // that is not visible (to solve this we could use the extra values in
            // queue_pre_positioned to keep track of which glyphs are actually returned by
            // proccess_queued)
            .filter(|g| {
                !text[g.byte_index..]
                    .chars()
                    .next()
                    .unwrap()
                    .is_whitespace()
            })
            .cloned()
            .collect()
    }

    fn draw_primitive(
        &mut self,
        primitive: Primitive,
        offset: Vec2<u32>,
        alpha: f32,
        renderer: &mut Renderer,
        pool: Option<&SlowJobPool>,
    ) {
        match primitive {
            Primitive::Group { primitives } => {
                primitives
                    .into_iter()
                    .for_each(|p| self.draw_primitive(p, offset, alpha, renderer, pool));
            },
            Primitive::Image {
                handle,
                bounds,
                color,
                source_rect,
            } => {
                let color = srgba_to_linear(color.map(|e| e as f32 / 255.0));
                let color = apply_alpha(color, alpha);
                // Don't draw a transparent image.
                if color.a == 0.0 {
                    return;
                }

                let (graphic_id, rotation) = handle;
                let gl_aabr = self.gl_aabr(iced::Rectangle {
                    x: bounds.x - offset.x as f32,
                    y: bounds.y - offset.y as f32,
                    ..bounds
                });

                let graphic_cache = self.cache.graphic_cache_mut();
                let half_res = self.half_res; // Make borrow checker happy by avoiding self in closure
                let (source_aabr, gl_size) = {
                    // Transform the source rectangle into uv coordinate.
                    // TODO: Make sure this is right.  Especially the conversions.
                    let ((uv_l, uv_r, uv_b, uv_t), gl_size) = match graphic_cache
                        .get_graphic(graphic_id)
                    {
                        Some(Graphic::Blank) | None => return,
                        Some(Graphic::Image(image, ..)) => {
                            source_rect.and_then(|src_rect| {
                                #[rustfmt::skip] use ::image::GenericImageView;
                                let (image_w, image_h) = image.dimensions();
                                let (source_w, source_h) = src_rect.size().into_tuple();
                                let gl_size = gl_aabr.size();
                                if image_w == 0
                                    || image_h == 0
                                    || source_w < 1.0
                                    || source_h < 1.0
                                    || gl_size.reduce_partial_min() < f32::EPSILON
                                {
                                    None
                                } else {
                                    // TODO: do this earlier
                                    // Multiply drawn image size by ratio of original image
                                    // size to
                                    // source rectangle size (since as the proportion of the
                                    // image gets
                                    // smaller, the drawn size should get bigger), up to the
                                    // actual
                                    // size of the original image.
                                    let ratio_x = (image_w as f32 / source_w)
                                        .min((image_w as f32 / (gl_size.w * half_res.x)).max(1.0));
                                    let ratio_y = (image_h as f32 / source_h)
                                        .min((image_h as f32 / (gl_size.h * half_res.y)).max(1.0));
                                    let (l, b) = src_rect.min.into_tuple();
                                    let (r, t) = src_rect.max.into_tuple();
                                    Some((
                                        (
                                            l / image_w as f32, /* * ratio_x */
                                            r / image_w as f32, /* * ratio_x */
                                            b / image_h as f32, /* * ratio_y */
                                            t / image_h as f32, /* * ratio_y */
                                        ),
                                        Extent2::new(gl_size.w * ratio_x, gl_size.h * ratio_y),
                                    ))
                                    /* ((l / image_w as f32),
                                    (r / image_w as f32),
                                    (b / image_h as f32),
                                    (t / image_h as f32)) */
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
                    (gl_size.w * self.half_res.x).round() as u16,
                    (gl_size.h * self.half_res.y).round() as u16,
                );

                // Don't do anything if resolution is zero
                if resolution.map(|e| e == 0).reduce_or() {
                    return;
                    // TODO: consider logging uneeded elements
                }

                // Cache graphic at particular resolution.
                let (uv_aabr, tex_id) = match graphic_cache.cache_res(
                    renderer,
                    pool,
                    graphic_id,
                    resolution,
                    // TODO: take f32 here
                    source_aabr.map(|e| e as f64),
                    rotation,
                ) {
                    // TODO: get dims from graphic_cache (or have it return floats directly)
                    Some((aabr, tex_id)) => {
                        let cache_dims = graphic_cache
                            .get_tex(tex_id)
                            .0
                            .get_dimensions()
                            .xy()
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
            Primitive::Gradient {
                bounds,
                top_linear_color,
                bottom_linear_color,
            } => {
                // Don't draw a transparent rectangle.
                let top_linear_color = apply_alpha(top_linear_color, alpha);
                let bottom_linear_color = apply_alpha(bottom_linear_color, alpha);
                if top_linear_color.a == 0.0 && bottom_linear_color.a == 0.0 {
                    return;
                }

                self.switch_state(State::Plain);

                let gl_aabr = self.gl_aabr(iced::Rectangle {
                    x: bounds.x - offset.x as f32,
                    y: bounds.y - offset.y as f32,
                    ..bounds
                });

                self.mesh.push_quad(create_ui_quad_vert_gradient(
                    gl_aabr,
                    Aabr {
                        min: Vec2::zero(),
                        max: Vec2::zero(),
                    },
                    top_linear_color,
                    bottom_linear_color,
                    UiMode::Geometry,
                ));
            },

            Primitive::Rectangle {
                bounds,
                linear_color,
            } => {
                let linear_color = apply_alpha(linear_color, alpha);
                // Don't draw a transparent rectangle.
                if linear_color.a == 0.0 {
                    return;
                }

                self.switch_state(State::Plain);

                let gl_aabr = self.gl_aabr(iced::Rectangle {
                    x: bounds.x - offset.x as f32,
                    y: bounds.y - offset.y as f32,
                    ..bounds
                });

                self.mesh.push_quad(create_ui_quad(
                    gl_aabr,
                    Aabr {
                        min: Vec2::zero(),
                        max: Vec2::zero(),
                    },
                    linear_color,
                    UiMode::Geometry,
                ));
            },
            Primitive::Text {
                glyphs,
                bounds: _, // iced::Rectangle
                linear_color,
            } => {
                let linear_color = apply_alpha(linear_color, alpha);
                self.switch_state(State::Plain);

                // TODO: makes sure we are not doing all this work for hidden text
                // e.g. in chat
                let glyph_cache = self.cache.glyph_cache_mut();

                // Count glyphs
                let glyph_count = glyphs.len();

                // Queue the glyphs to be cached.
                glyph_cache.queue_pre_positioned(
                    glyphs,
                    // TODO: glyph_brush should document that these need to be the same length
                    vec![(); glyph_count],
                    // Since we already passed in `bounds` to position the glyphs some of this
                    // seems redundant...
                    // Note: we can't actually use this because dropping glyphs messeses up the
                    // counting and there is not a method provided to drop out of bounds
                    // glyphs while positioning them
                    // Note: keeping commented code in case how we handle text changes
                    glyph_brush::ab_glyph::Rect {
                        min: glyph_brush::ab_glyph::point(
                            -10000.0, //bounds.x * self.p_scale,
                            -10000.0, //bounds.y * self.p_scale,
                        ),
                        max: glyph_brush::ab_glyph::point(
                            10000.0, //(bounds.x + bounds.width) * self.p_scale,
                            10000.0, //(bounds.y + bounds.height) * self.p_scale,
                        ),
                    },
                );

                // Leave ui and verts blank to fill in when processing cached glyphs
                let zero_aabr = Aabr {
                    min: Vec2::broadcast(0.0),
                    max: Vec2::broadcast(0.0),
                };
                self.glyphs.push((
                    self.mesh.vertices().len(),
                    glyph_count,
                    linear_color,
                    offset,
                ));
                for _ in 0..glyph_count {
                    // Push placeholder quad
                    // Note: moving to some sort of layering / z based system would be an
                    // alternative to this (and might help with reducing draw
                    // calls)
                    self.mesh.push_quad(create_ui_quad(
                        zero_aabr,
                        zero_aabr,
                        linear_color,
                        UiMode::Text,
                    ));
                }
            },
            Primitive::Clip {
                bounds,
                offset: clip_offset,
                content,
            } => {
                let new_scissor = {
                    // TODO: incorporate current offset for nested Clips
                    let intersection = Aabr {
                        min: Vec2 {
                            x: (bounds.x * self.p_scale) as u16,
                            y: (bounds.y * self.p_scale) as u16,
                        },
                        max: Vec2 {
                            x: ((bounds.x + bounds.width) * self.p_scale) as u16,
                            y: ((bounds.y + bounds.height) * self.p_scale) as u16,
                        },
                    }
                    .intersection(self.window_scissor);

                    if intersection.is_valid() && intersection.size().map(|s| s > 0).reduce_and() {
                        intersection
                    } else {
                        // If the intersection is invalid or zero sized we don't need to process
                        // the content primitive
                        return;
                    }
                };
                // Not expecting this case: new_scissor == current_scissor
                // So not optimizing for it

                // Finish the current command.
                // TODO: ensure we never push empty commands
                self.draw_commands.push(match self.current_state {
                    State::Plain => DrawCommand::plain(self.start..self.mesh.vertices().len()),
                    State::Image(id) => {
                        DrawCommand::image(self.start..self.mesh.vertices().len(), id)
                    },
                });
                self.start = self.mesh.vertices().len();

                self.draw_commands.push(DrawCommand::Scissor(new_scissor));

                // TODO: support nested clips?
                // TODO: if previous command was a clip changing back to the default replace it
                // with this
                // TODO: cull primitives outside the current scissor

                // Renderer child
                self.draw_primitive(*content, offset + clip_offset, alpha, renderer, pool);

                // Reset scissor
                self.draw_commands.push(match self.current_state {
                    State::Plain => DrawCommand::plain(self.start..self.mesh.vertices().len()),
                    State::Image(id) => {
                        DrawCommand::image(self.start..self.mesh.vertices().len(), id)
                    },
                });
                self.start = self.mesh.vertices().len();

                self.draw_commands
                    .push(DrawCommand::Scissor(self.window_scissor));
            },
            Primitive::Opacity { alpha: a, content } => {
                self.draw_primitive(*content, offset, alpha * a, renderer, pool);
            },
            Primitive::Nothing => {},
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

    pub fn render<'a>(&'a self, drawer: &mut UiDrawer<'_, 'a>) {
        span!(_guard, "render", "IcedRenderer::render");
        let mut drawer = drawer.prepare(&self.interface_locals, &self.model, self.window_scissor);
        for draw_command in self.draw_commands.iter() {
            match draw_command {
                DrawCommand::Scissor(new_scissor) => {
                    drawer.set_scissor(*new_scissor);
                },
                DrawCommand::WorldPos(index) => {
                    drawer.set_locals(
                        index.map_or(&self.interface_locals, |i| &self.ingame_locals[i]),
                    );
                },
                DrawCommand::Draw { kind, verts } => {
                    // TODO: don't make these: assert!(!verts.is_empty());
                    let tex = match kind {
                        DrawKind::Image(tex_id) => self.cache.graphic_cache().get_tex(*tex_id),
                        DrawKind::Plain => self.cache.glyph_cache_tex(),
                    };
                    drawer.draw(&tex.1, verts.clone()); // Note: trivial clone
                },
            }
        }
    }
}

// Given the the resolution determines the offset needed to align integer
// offsets from the center of the sceen to pixels
#[inline(always)]
fn align(res: Vec2<u32>) -> Vec2<f32> {
    // TODO: does this logic still apply in iced's coordinate system?
    // If the resolution is odd then the center of the screen will be within the
    // middle of a pixel so we need to offset by 0.5 pixels to be on the edge of
    // a pixel
    res.map(|e| (e & 1) as f32 * 0.5)
}

fn default_scissor(physical_resolution: Vec2<u32>) -> Aabr<u16> {
    let (screen_w, screen_h) = physical_resolution.into_tuple();
    Aabr {
        min: Vec2 { x: 0, y: 0 },
        max: Vec2 {
            x: screen_w as u16,
            y: screen_h as u16,
        },
    }
}

impl iced::Renderer for IcedRenderer {
    // Default styling
    type Defaults = Defaults;
    // TODO: use graph of primitives to enable diffing???
    type Output = (Primitive, iced::mouse::Interaction);

    fn layout<M>(
        &mut self,
        element: &iced::Element<'_, M, Self>,
        limits: &iced::layout::Limits,
    ) -> iced::layout::Node {
        span!(_guard, "layout", "IcedRenderer::layout");

        // Trim text measurements cache?

        element.layout(self, limits)
    }

    fn overlay(
        &mut self,
        (base_primitive, base_interaction): Self::Output,
        (overlay_primitive, overlay_interaction): Self::Output,
        overlay_bounds: iced::Rectangle,
    ) -> Self::Output {
        span!(_guard, "overlay", "IcedRenderer::overlay");
        (
            Primitive::Group {
                primitives: vec![base_primitive, Primitive::Clip {
                    bounds: iced::Rectangle {
                        // TODO: do we need this + 0.5?
                        width: overlay_bounds.width + 0.5,
                        height: overlay_bounds.height + 0.5,
                        ..overlay_bounds
                    },
                    offset: Vec2::new(0, 0),
                    content: Box::new(overlay_primitive),
                }],
            },
            base_interaction.max(overlay_interaction),
        )
    }
}

fn apply_alpha(color: Rgba<f32>, alpha: f32) -> Rgba<f32> {
    Rgba {
        a: alpha * color.a,
        ..color
    }
}
// TODO: impl Debugger
