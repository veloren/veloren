mod cache;
mod event;
pub mod graphic;
mod scale;
mod widgets;
#[macro_use]
pub mod img_ids;
#[macro_use]
pub mod fonts;
#[cfg(feature = "egui-ui")] pub mod egui;
pub mod ice;
pub mod keyed_jobs;

pub use event::Event;
pub use graphic::{Graphic, Id as GraphicId, Rotation, SampleStrat, Transform};
pub use keyed_jobs::KeyedJobs;
pub use scale::{Scale, ScaleMode};
pub use widgets::{
    image_frame::ImageFrame,
    image_slider::ImageSlider,
    ingame::{Ingame, Ingameable},
    item_tooltip::{ItemTooltip, ItemTooltipManager, ItemTooltipable},
    outlined_text::OutlinedText,
    radio_list::RadioList,
    slot,
    toggle_button::ToggleButton,
    tooltip::{Tooltip, TooltipManager, Tooltipable},
};

use crate::{
    error::Error,
    render::{
        create_ui_quad, create_ui_tri, DynamicModel, Mesh, RenderError, Renderer, UiBoundLocals,
        UiDrawer, UiLocals, UiMode, UiVertex,
    },
    window::Window,
};
#[rustfmt::skip]
use ::image::GenericImageView;
use cache::Cache;
use common::{slowjob::SlowJobPool, util::srgba_to_linear};
use common_base::span;
use conrod_core::{
    event::Input,
    graph::{self, Graph},
    image::{Id as ImageId, Map},
    input::{touch::Touch, Motion, Widget},
    render::{Primitive, PrimitiveKind},
    text::{self, font},
    widget::{self, id::Generator},
    Rect, Scalar, UiBuilder, UiCell,
};
use core::{convert::TryInto, f64, ops::Range};
use graphic::TexId;
use hashbrown::hash_map::Entry;
use std::time::Duration;
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
    Draw { kind: DrawKind, verts: Range<u32> },
    Scissor(Aabr<u16>),
    WorldPos(Option<usize>),
}
impl DrawCommand {
    fn image(verts: Range<usize>, id: TexId) -> DrawCommand {
        DrawCommand::Draw {
            kind: DrawKind::Image(id),
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

pub struct Ui {
    pub ui: conrod_core::Ui,
    image_map: Map<(graphic::Id, Rotation)>,
    cache: Cache,
    // Draw commands for the next render
    draw_commands: Vec<DrawCommand>,
    // Mesh buffer for UI vertices; we reuse its allocation in order to limit vector reallocations
    // during redrawing.
    mesh: Mesh<UiVertex>,
    // Model for drawing the ui
    model: DynamicModel<UiVertex>,
    // Consts for default ui drawing position (ie the interface)
    interface_locals: UiBoundLocals,
    // Consts to specify positions of ingame elements (e.g. Nametags)
    ingame_locals: Vec<UiBoundLocals>,
    // Whether the window was resized since the last maintain, for updating scaling
    window_resized: bool,
    // Scale factor changed
    scale_factor_changed: Option<f64>,
    // Used to delay cache resizing until after current frame is drawn
    need_cache_resize: bool,
    // Whether a graphic was replaced with replaced_graphic since the last maintain call
    graphic_replaced: bool,
    // Scaling of the ui
    scale: Scale,
    // Tooltips
    tooltip_manager: TooltipManager,
    // Item tooltips manager
    item_tooltip_manager: ItemTooltipManager,
    // Scissor for the whole window
    window_scissor: Aabr<u16>,
}

impl Ui {
    pub fn new(window: &mut Window) -> Result<Self, Error> {
        let scale_factor = window.scale_factor();
        let renderer = window.renderer_mut();
        let physical_resolution = renderer.resolution();
        let scale = Scale::new(
            physical_resolution,
            scale_factor,
            ScaleMode::Absolute(1.0),
            1.0,
        );

        let win_dims = scale.scaled_resolution().into_array();

        let mut ui = UiBuilder::new(win_dims).build();
        // NOTE: Since we redraw the actual frame each time whether or not the UI needs
        // to be updated, there's no reason to set the redraw count higher than
        // 1.
        ui.set_num_redraw_frames(1);

        let item_tooltip_manager = ItemTooltipManager::new(
            Duration::from_millis(1),
            Duration::from_millis(0),
            scale.scale_factor_logical(),
        );

        let tooltip_manager = TooltipManager::new(
            ui.widget_id_generator(),
            Duration::from_millis(1),
            Duration::from_millis(0),
            scale.scale_factor_logical(),
        );

        let interface_locals = renderer.create_ui_bound_locals(&[UiLocals::default()]);

        Ok(Self {
            ui,
            image_map: Map::new(),
            cache: Cache::new(renderer)?,
            draw_commands: Vec::new(),
            mesh: Mesh::new(),
            model: renderer.create_dynamic_model(100),
            interface_locals,
            ingame_locals: Vec::new(),
            window_resized: false,
            scale_factor_changed: None,
            need_cache_resize: false,
            graphic_replaced: false,
            scale,
            tooltip_manager,
            item_tooltip_manager,
            window_scissor: default_scissor(physical_resolution),
        })
    }

    // Set the scaling mode of the ui.
    pub fn set_scaling_mode(&mut self, mode: ScaleMode) {
        self.scale.set_scaling_mode(mode);
        // To clear the cache (it won't be resized in this case)
        self.need_cache_resize = true;
        // Give conrod the new size.
        let (w, h) = self.scale.scaled_resolution().into_tuple();
        self.ui.handle_event(Input::Resize(w, h));
    }

    pub fn scale_factor_changed(&mut self, scale_factor: f64) {
        self.scale_factor_changed = Some(scale_factor);
    }

    // Get a copy of Scale
    pub fn scale(&self) -> Scale { self.scale }

    pub fn add_graphic(&mut self, graphic: Graphic) -> ImageId {
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

    pub fn replace_graphic(&mut self, id: ImageId, graphic: Graphic) {
        if let Some(&(graphic_id, _)) = self.image_map.get(&id) {
            self.cache.replace_graphic(graphic_id, graphic);
            self.image_map.replace(id, (graphic_id, Rotation::None));
            self.graphic_replaced = true;
        } else {
            error!("Failed to replace graphic, the provided id is not in use.");
        };
    }

    pub fn new_font(&mut self, font: ice::RawFont) -> font::Id {
        let font = text::Font::from_bytes(font.0).unwrap();
        self.ui.fonts.insert(font)
    }

    pub fn id_generator(&mut self) -> Generator { self.ui.widget_id_generator() }

    pub fn set_widgets(&mut self) -> (UiCell, &mut ItemTooltipManager, &mut TooltipManager) {
        (
            self.ui.set_widgets(),
            &mut self.item_tooltip_manager,
            &mut self.tooltip_manager,
        )
    }

    pub fn set_item_widgets(&mut self) -> (UiCell, &mut ItemTooltipManager) {
        (self.ui.set_widgets(), &mut self.item_tooltip_manager)
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
                if w > 0.0 && h > 0.0 {
                    self.window_resized = true;
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

    pub fn maintain(
        &mut self,
        renderer: &mut Renderer,
        pool: Option<&SlowJobPool>,
        view_projection_mat: Option<Mat4<f32>>,
    ) {
        span!(_guard, "maintain", "Ui::maintain");
        // Maintain tooltip manager
        self.tooltip_manager
            .maintain(self.ui.global_input(), self.scale.scale_factor_logical());

        // Maintain tooltip manager
        self.item_tooltip_manager
            .maintain(self.ui.global_input(), self.scale.scale_factor_logical());

        // Handle scale factor changing
        let need_resize = if let Some(scale_factor) = self.scale_factor_changed.take() {
            self.scale.scale_factor_changed(scale_factor)
        } else {
            false
        };

        // Used to tell if we need to clear out the draw commands (which contain scissor
        // commands that can be invalidated by this change)
        let physical_resolution_changed = renderer.resolution() != self.scale.physical_resolution();

        // Handle window resizing.
        let need_resize = if self.window_resized {
            self.window_resized = false;
            let surface_resolution = renderer.resolution();
            let (old_w, old_h) = self.scale.scaled_resolution().into_tuple();
            self.scale.surface_resized(surface_resolution);
            let (w, h) = self.scale.scaled_resolution().into_tuple();
            self.ui.handle_event(Input::Resize(w, h));
            self.window_scissor = default_scissor(surface_resolution);

            // Avoid panic in graphic cache when minimizing.
            // Avoid resetting cache if window size didn't change
            // Somewhat inefficient for elements that won't change size after a window
            // resize
            let res = renderer.resolution();
            res.x > 0 && res.y > 0 && !(old_w == w && old_h == h)
        } else {
            false
        } || need_resize;

        if need_resize {
            self.need_cache_resize = true;
        }

        if self.need_cache_resize {
            // Resize graphic cache
            // FIXME: Handle errors here.
            self.cache.resize(renderer).unwrap();

            self.need_cache_resize = false;
        }

        let mut retry = false;
        self.maintain_internal(
            renderer,
            pool,
            view_projection_mat,
            &mut retry,
            physical_resolution_changed,
        );
        if retry {
            // Update the glyph cache and try again.
            self.maintain_internal(renderer, pool, view_projection_mat, &mut retry, false);
        }
    }

    fn maintain_internal(
        &mut self,
        renderer: &mut Renderer,
        pool: Option<&SlowJobPool>,
        view_projection_mat: Option<Mat4<f32>>,
        retry: &mut bool,
        physical_resolution_changed: bool,
    ) {
        span!(_guard, "internal", "Ui::maintain_internal");
        let (graphic_cache, text_cache, glyph_cache, cache_tex) = self.cache.cache_mut_and_tex();

        // If the physical resolution changed draw commands need to be cleared since
        // scissors commands will be invalid. A resize usually means everything
        // needs to be redrawn anyway but certain error cases below can cause an
        // early return.
        if physical_resolution_changed {
            self.draw_commands.clear();
        }

        let mut primitives = if *retry || self.graphic_replaced || physical_resolution_changed {
            // If this is a retry, always redraw.
            //
            // Also redraw if a texture was swapped out by replace_graphic in order to
            // regenerate invalidated textures and clear out any invalid `TexId`s.
            //
            // Also redraw if the physical resolution changed since we need to regenerate
            // the invalid scissor rect commands.
            self.graphic_replaced = false;
            self.ui.draw()
        } else {
            // Otherwise, redraw only if widgets were actually updated.
            match self.ui.draw_if_changed() {
                Some(primitives) => primitives,
                None => return,
            }
        };

        let (half_res, x_align, y_align) = {
            let res = renderer.resolution();
            (
                res.map(|e| e as f32 / 2.0),
                (res.x & 1) as f32 * 0.5,
                (res.y & 1) as f32 * 0.5,
            )
        };

        let ui = &self.ui;
        let p_scale_factor = self.scale.scale_factor_physical();
        // Functions for converting for conrod scalar coords to GL vertex coords (-1.0
        // to 1.0).
        let (ui_win_w, ui_win_h) = (ui.win_w, ui.win_h);
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

        // let window_dim = ui.window_dim();
        let theme = &ui.theme;
        let widget_graph = ui.widget_graph();
        let fonts = &ui.fonts;
        let dpi_factor = p_scale_factor as f32;

        // We can use information about whether a widget was actually updated to more
        // easily track cache invalidations.
        let updated_widgets = ui.updated_widgets();
        let mut glyph_missing = false;

        updated_widgets.iter()
            // Filter out widgets that are either:
            // - not text primitives, or
            // - are text primitives, but were both already in the cache, and not updated this
            //  frame.
            //
            // The reason the second condition is so complicated is that we want to handle cases
            // where we cleared the whole cache, which can result in glyphs from text updated in a
            // previous frame not being present in the text cache.
            .filter_map(|(&widget_id, updated)| {
                widget_graph.widget(widget_id)
                    .and_then(|widget| Some((widget.rect, widget.unique_widget_state::<widget::Text>()?)))
                    .and_then(|(rect, text)| {
                        // NOTE: This fallback is weird and probably shouldn't exist.
                        let font_id = text.style.font_id(theme)/*.or_else(|| fonts.ids().next())*/?;
                        let font = fonts.get(font_id)?;

                        Some((widget_id, updated, rect, text, font_id, font))
                    })
            })
            // Recache the entry.
            .for_each(|(widget_id, updated, rect, graph::UniqueWidgetState { state, style }, font_id, font)| {
                let entry = match text_cache.entry(widget_id) {
                    Entry::Occupied(_) if !updated => return,
                    entry => entry,
                };

                // Retrieve styling.
                let color = style.color(theme);
                let font_size = style.font_size(theme);
                let line_spacing = style.line_spacing(theme);
                let justify = style.justify(theme);
                let y_align = conrod_core::position::Align::End;

                let text = &state.string;
                let line_infos = &state.line_infos;

                // Convert conrod coordinates to pixel coordinates.
                let trans_x = |x: Scalar| (x + ui_win_w / 2.0) * dpi_factor as Scalar;
                let trans_y = |y: Scalar| ((-y) + ui_win_h / 2.0) * dpi_factor as Scalar;

                // Produce the text layout iterators.
                let lines = line_infos.iter().map(|info| &text[info.byte_range()]);
                let line_rects = text::line::rects(line_infos.iter(), font_size, rect,
                                                   justify, y_align, line_spacing);

                // Grab the positioned glyphs from the text primitives.
                let scale = text::f32_pt_to_scale(font_size as f32 * dpi_factor);
                let positioned_glyphs = lines.zip(line_rects).flat_map(|(line, line_rect)| {
                    let (x, y) = (trans_x(line_rect.left()) as f32, trans_y(line_rect.bottom()) as f32);
                    let point = text::rt::Point { x, y };
                    font.layout(line, scale, point)
                });

                // Reuse the mesh allocation if possible at this entry if possible; we
                // then clear it to avoid using stale entries.
                let mesh = entry.or_insert_with(Mesh::new);
                mesh.clear();

                let color = srgba_to_linear(color.to_fsa().into());

                positioned_glyphs.for_each(|g| {
                    match glyph_cache.rect_for(font_id.index(), &g) {
                        Ok(Some((uv_rect, screen_rect))) => {
                            let uv = Aabr {
                                min: Vec2::new(uv_rect.min.x, uv_rect.max.y),
                                max: Vec2::new(uv_rect.max.x, uv_rect.min.y),
                            };
                            let rect = Aabr {
                                min: Vec2::new(
                                    vx(screen_rect.min.x as f64 / p_scale_factor
                                        - ui.win_w / 2.0),
                                    vy(ui.win_h / 2.0
                                        - screen_rect.max.y as f64 / p_scale_factor),
                                ),
                                max: Vec2::new(
                                    vx(screen_rect.max.x as f64 / p_scale_factor
                                        - ui.win_w / 2.0),
                                    vy(ui.win_h / 2.0
                                        - screen_rect.min.y as f64 / p_scale_factor),
                                ),
                            };
                            mesh.push_quad(create_ui_quad(rect, uv, color, UiMode::Text));
                        },
                        // Glyph not found, no-op.
                        Ok(None) => {},
                        // Glyph was found, but was not yet cached; we'll need to add it to the
                        // cache and retry.
                        Err(_) => {
                            // Queue the unknown glyph to be cached.
                            glyph_missing = true;
                        }
                    }

                    // NOTE: Important to do this for *all* glyphs to try to make sure that
                    // any that are uncached are part of the graph.  Because we always
                    // clear the entire cache whenever a new glyph is encountered, by
                    // adding and checking all glyphs as they come in we guarantee that (1)
                    // any glyphs in the text cache are in the queue, and (2) any glyphs
                    // not in the text cache are either in the glyph cache, or (during our
                    // processing here) we set the retry flag to force a glyph cache
                    // update.  Setting the flag causes all glyphs in the current queue to
                    // become part of the glyph cache during the second call to
                    // `maintain_internal`, so as long as the cache refresh succeeded,
                    // during the second call all glyphs will hit this branch as desired.
                    glyph_cache.queue_glyph(font_id.index(), g);
                });
            });

        if glyph_missing {
            if *retry {
                // If a glyph was missing and this was our second try, we know something was
                // messed up during the glyph_cache redraw.  It is possible that
                // the queue contained unneeded glyphs, so we don't necessarily
                // have to give up; a more precise enumeration of the
                // glyphs required to render this frame might work out.  However, this is a
                // pretty remote edge case, so we opt to not care about this
                // frame (we skip rendering it, basically), and just clear the
                // text cache and glyph queue; next frame will then
                // start out with an empty slate, and therefore will enqueue precisely the
                // glyphs needed for that frame.  If *that* frame fails, we're
                // in bigger trouble.
                text_cache.clear();
                glyph_cache.clear();
                glyph_cache.clear_queue();
                self.ui.needs_redraw();
                warn!("Could not recache queued glyphs, skipping frame.");
            } else {
                // NOTE: If this is the first round after encountering a new glyph, we just
                // refresh the whole glyph cache.  Technically this is not necessary since
                // positioned_glyphs should still be accurate, but it's just the easiest way
                // to ensure that all glyph positions get updated.  It also helps keep the glyph
                // cache reasonable by making sure any glyphs that subsequently get rendered are
                // actually in the cache, including glyphs that were mapped to ids but didn't
                // happen to be rendered on the frame where the cache was
                // refreshed.
                text_cache.clear();
                tracing::debug!("Updating glyphs and clearing text cache.");

                if let Err(err) = glyph_cache.cache_queued(|rect, data| {
                    let offset = [rect.min.x, rect.min.y];
                    let size = [rect.width(), rect.height()];

                    let new_data = data
                        .iter()
                        .map(|x| [255, 255, 255, *x])
                        .collect::<Vec<[u8; 4]>>();

                    renderer.update_texture(&cache_tex.0, offset, size, &new_data);
                }) {
                    // FIXME: If we actually hit this error, it's still possible we could salvage
                    // things in various ways (for instance, the current queue might have extra
                    // stuff in it, so we could try calling maintain_internal a
                    // third time with a fully clean queue; or we could try to
                    // increase the glyph texture size, etc.  But hopefully
                    // we will not actually encounter this error.
                    warn!("Failed to cache queued glyphs: {:?}", err);

                    // Clear queued glyphs, so that (hopefully) next time we won't have the
                    // offending glyph or glyph set.  We then exit the loop and don't try to
                    // rerender the frame.
                    glyph_cache.clear_queue();
                    self.ui.needs_redraw();
                } else {
                    // Successfully cached, so repeat the loop.
                    *retry = true;
                }
            }

            return;
        }

        self.draw_commands.clear();
        let mesh = &mut self.mesh;
        mesh.clear();

        enum State {
            Image(TexId),
            Plain,
        }

        let mut current_state = State::Plain;
        let mut start = 0;

        let window_scissor = self.window_scissor;
        let mut current_scissor = window_scissor;

        let mut ingame_local_index = 0;

        enum Placement {
            Interface,
            // Number of primitives left to render ingame and visibility
            InWorld(usize, bool),
        }

        let mut placement = Placement::Interface;

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
                id: widget_id,
            } = prim;
            // Check for a change in the scissor.
            let new_scissor = {
                let (l, b, w, h) = scizzor.l_b_w_h();
                let scale_factor = self.scale.scale_factor_physical();
                // Calculate minimum x and y coordinates while
                // flipping y axis (from +up to +down) and
                // moving origin to top-left corner (from middle).
                let min_x = ui.win_w / 2.0 + l;
                let min_y = ui.win_h / 2.0 - b - h;
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

                if intersection.is_valid() && intersection.size().map(|s| s > 0).reduce_and() {
                    intersection
                } else {
                    // TODO: What should we return here
                    // We used to return a zero sized aabr but it's invalid to
                    // use a zero sized scissor so for now we just don't change
                    // the scissor.
                    current_scissor
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
                    let gl_aabr = gl_aabr(rect);
                    let (source_aabr, gl_size) = {
                        // Transform the source rectangle into uv coordinate.
                        // TODO: Make sure this is right.  Especially the conversions.
                        let ((uv_l, uv_r, uv_b, uv_t), gl_size) =
                            match graphic_cache.get_graphic(*graphic_id) {
                                Some(Graphic::Blank) | None => continue,
                                Some(Graphic::Image(image, ..)) => {
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
                                                    l / image_w as f64, /* * ratio_x */
                                                    r / image_w as f64, /* * ratio_x */
                                                    b / image_h as f64, /* * ratio_y */
                                                    t / image_h as f64, /* * ratio_y */
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
                        // TODO: consider logging unneeded elements
                    }

                    let color =
                        srgba_to_linear(color.unwrap_or(conrod_core::color::WHITE).to_fsa().into());

                    // Cache graphic at particular resolution.
                    let (uv_aabr, tex_id) = match graphic_cache.cache_res(
                        renderer,
                        pool,
                        *graphic_id,
                        resolution,
                        source_aabr,
                        *rotation,
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
                PrimitiveKind::Text { .. } => {
                    switch_to_plain_state!();

                    // Mesh should already be cached.
                    mesh.push_mesh(text_cache.get(&widget_id).unwrap_or(&Mesh::new()));
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
                            let visible = if pos_on_screen.z > 0.0 && pos_on_screen.z < 1.0 {
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
                                    renderer.update_consts(
                                        &mut self.ingame_locals[ingame_local_index],
                                        &[world_pos.into()],
                                    )
                                } else {
                                    self.ingame_locals
                                        .push(renderer.create_ui_bound_locals(&[world_pos.into()]));
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

        /* // Draw glyph cache (use for debugging).
        self.draw_commands
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
            .push(DrawCommand::plain(start..mesh.vertices().len())); */

        // Create a larger dynamic model if the mesh is larger than the current model
        // size.
        if self.model.len() < self.mesh.vertices().len() {
            self.model = renderer.create_dynamic_model(self.mesh.vertices().len() * 4 / 3);
        }
        // Update model with new mesh.
        renderer.update_model(&self.model, &self.mesh, 0);
    }

    pub fn render<'pass, 'data: 'pass>(&'data self, drawer: &mut UiDrawer<'_, 'pass>) {
        span!(_guard, "render", "Ui::render");
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
