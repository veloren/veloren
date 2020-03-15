use super::{
    img_ids::{Imgs, ImgsRot},
    Show, HP_COLOR, TEXT_COLOR,
};
use crate::ui::{fonts::ConrodVoxygenFonts, img_ids};
use client::{self, Client};
use common::{comp, terrain::TerrainChunkSize, vol::RectVolSize};
use conrod_core::{
    color, position,
    widget::{self, Button, Image, Rectangle, Text},
    widget_ids, Color, Colorable, Positionable, Sizeable, Widget, WidgetCommon,
};
use specs::WorldExt;
use std::time::{Duration, Instant};
use vek::*;

widget_ids! {
    struct Ids {
        mmap_frame,
        mmap_frame_bg,
        mmap_location,
        mmap_button,
        mmap_plus,
        mmap_minus,
        zone_display_bg,
        zone_display,
        grid,
        indicator
    }
}

#[derive(WidgetCommon)]
pub struct MiniMap<'a> {
    show: &'a Show,

    client: &'a Client,

    imgs: &'a Imgs,
    rot_imgs: &'a ImgsRot,
    world_map: &'a (img_ids::Rotations, Vec2<u32>),
    fonts: &'a ConrodVoxygenFonts,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> MiniMap<'a> {
    pub fn new(
        show: &'a Show,
        client: &'a Client,
        imgs: &'a Imgs,
        rot_imgs: &'a ImgsRot,
        world_map: &'a (img_ids::Rotations, Vec2<u32>),
        fonts: &'a ConrodVoxygenFonts,
    ) -> Self {
        Self {
            show,
            client,
            imgs,
            rot_imgs,
            world_map,
            fonts,
            common: widget::CommonBuilder::default(),
        }
    }
}

pub struct State {
    ids: Ids,

    last_region_name: Option<String>,
    last_update: Instant,
    zoom: f64,
}

pub enum Event {
    Toggle,
}

impl<'a> Widget for MiniMap<'a> {
    type Event = Option<Event>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),

            last_region_name: None,
            last_update: Instant::now(),
            zoom: {
                let min_world_dim = self.world_map.1.reduce_partial_min() as f64;
                min_world_dim.min(
                    min_world_dim
                        * (TerrainChunkSize::RECT_SIZE.reduce_partial_max() as f64 / 32.0)
                        * (16.0 / 1024.0),
                )
            },
        }
    }

    fn style(&self) -> Self::Style { () }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;
        let zoom = state.zoom;
        const SCALE: f64 = 1.5;
        if self.show.mini_map {
            Image::new(self.imgs.mmap_frame)
                .w_h(174.0 * SCALE, 190.0 * SCALE)
                .top_right_with_margins_on(ui.window, 0.0, 5.0)
                .set(state.ids.mmap_frame, ui);

            Rectangle::fill_with([170.0 * SCALE, 170.0 * SCALE], color::TRANSPARENT)
                .mid_top_with_margin_on(state.ids.mmap_frame, 18.0 * SCALE)
                .set(state.ids.mmap_frame_bg, ui);

            // Map size
            let (world_map, worldsize) = self.world_map;
            let worldsize = worldsize.map2(TerrainChunkSize::RECT_SIZE, |e, f| e as f64 * f as f64);

            // Zoom Buttons

            // Pressing + multiplies, and - divides, zoom by ZOOM_FACTOR.
            const ZOOM_FACTOR: f64 = 2.0;

            // TODO: Either prevent zooming all the way in, *or* see if we can interpolate
            // somehow if you zoom in too far.  Or both.
            let min_zoom = 1.0;
            let max_zoom = (worldsize / TerrainChunkSize::RECT_SIZE.map(|e| e as f64))
                .reduce_partial_min()/*.min(f64::MAX)*/;

            // NOTE: Not sure if a button can be clicked while disabled, but we still double
            // check for both kinds of zoom to make sure that not only was the
            // button clicked, it is also okay to perform the zoom action.
            // Note that since `Button::image` has side effects, we must perform
            // the `can_zoom_in` and `can_zoom_out` checks after the `&&` to avoid
            // undesired early termination.
            let can_zoom_in = zoom < max_zoom;
            let can_zoom_out = zoom > min_zoom;

            if Button::image(self.imgs.mmap_minus)
                .w_h(16.0 * SCALE, 18.0 * SCALE)
                .hover_image(self.imgs.mmap_minus_hover)
                .press_image(self.imgs.mmap_minus_press)
                .top_left_with_margins_on(state.ids.mmap_frame, 0.0, 0.0)
                .enabled(can_zoom_out)
                .set(state.ids.mmap_minus, ui)
                .was_clicked()
                && can_zoom_out
            {
                // Set the image dimensions here, rather than recomputing each time.
                let zoom = min_zoom.max(zoom / ZOOM_FACTOR);
                state.update(|s| s.zoom = zoom);
                // set_image_dims(zoom);
            }
            if Button::image(self.imgs.mmap_plus)
                .w_h(18.0 * SCALE, 18.0 * SCALE)
                .hover_image(self.imgs.mmap_plus_hover)
                .press_image(self.imgs.mmap_plus_press)
                .right_from(state.ids.mmap_minus, 0.0)
                .enabled(can_zoom_in)
                .set(state.ids.mmap_plus, ui)
                .was_clicked()
                && can_zoom_in
            {
                let zoom = max_zoom.min(zoom * ZOOM_FACTOR);
                state.update(|s| s.zoom = zoom);
                // set_image_dims(zoom);
            }

            // Reload zoom in case it changed.
            let zoom = state.zoom;

            // Coordinates
            let player_pos = self
                .client
                .state()
                .ecs()
                .read_storage::<comp::Pos>()
                .get(self.client.entity())
                .map_or(Vec3::zero(), |pos| pos.0);

            // Get map image source rectangle dimensons.
            let w_src = worldsize.x / TerrainChunkSize::RECT_SIZE.x as f64 / zoom;
            let h_src = worldsize.y / TerrainChunkSize::RECT_SIZE.y as f64 / zoom;

            // Set map image to be centered around player coordinates.
            let rect_src = position::Rect::from_xy_dim(
                [
                    player_pos.x as f64 / TerrainChunkSize::RECT_SIZE.x as f64,
                    (worldsize.y - player_pos.y as f64) / TerrainChunkSize::RECT_SIZE.y as f64,
                ],
                [w_src, h_src],
            );

            // Map Image
            Image::new(world_map.source_north)
                .middle_of(state.ids.mmap_frame_bg)
                .w_h(170.0 * SCALE, 170.0 * SCALE)
                .parent(state.ids.mmap_frame_bg)
                .source_rectangle(rect_src)
                .set(state.ids.grid, ui);

            // Indicator
            let ind_scale = 0.4;
            Image::new(self.rot_imgs.indicator_mmap_small.none)
                .middle_of(state.ids.grid)
                .w_h(32.0 * ind_scale, 37.0 * ind_scale)
                .color(Some(Color::Rgba(1.0, 1.0, 1.0, 1.0)))
                .floating(true)
                .parent(ui.window)
                .set(state.ids.indicator, ui);
        } else {
            Image::new(self.imgs.mmap_frame_closed)
                .w_h(174.0 * SCALE, 18.0 * SCALE)
                .top_right_with_margins_on(ui.window, 0.0, 5.0)
                .set(state.ids.mmap_frame, ui);
        }

        if Button::image(if self.show.mini_map {
            self.imgs.mmap_open
        } else {
            self.imgs.mmap_closed
        })
        .w_h(18.0 * SCALE, 18.0 * SCALE)
        .hover_image(if self.show.mini_map {
            self.imgs.mmap_open_hover
        } else {
            self.imgs.mmap_closed_hover
        })
        .press_image(if self.show.mini_map {
            self.imgs.mmap_open_press
        } else {
            self.imgs.mmap_closed_press
        })
        .top_right_with_margins_on(state.ids.mmap_frame, 0.0, 0.0)
        .set(state.ids.mmap_button, ui)
        .was_clicked()
        {
            return Some(Event::Toggle);
        }

        // Display zone name on entry

        const FADE_IN: f32 = 0.5;
        const FADE_HOLD: f32 = 1.0;
        const FADE_OUT: f32 = 3.0;
        match self.client.current_chunk() {
            Some(chunk) => {
                let current = chunk.meta().name();
                // Check if no other popup is displayed and a new one is needed
                if state.last_update.elapsed()
                    > Duration::from_secs_f32(FADE_IN + FADE_HOLD + FADE_OUT)
                    && state
                        .last_region_name
                        .as_ref()
                        .map(|l| l != current)
                        .unwrap_or(true)
                {
                    // Update last_region
                    state.update(|s| s.last_region_name = Some(current.to_owned()));
                    state.update(|s| s.last_update = Instant::now());
                }

                let seconds = state.last_update.elapsed().as_secs_f32();
                let fade = if seconds < FADE_IN {
                    seconds / FADE_IN
                } else if seconds < FADE_IN + FADE_HOLD {
                    1.0
                } else {
                    (1.0 - (seconds - FADE_IN - FADE_HOLD) / FADE_OUT).max(0.0)
                };
                // Region Name
                Text::new(state.last_region_name.as_ref().unwrap_or(&"".to_owned()))
                    .mid_top_with_margin_on(ui.window, 200.0)
                    .font_size(self.fonts.alkhemi.scale(70))
                    .font_id(self.fonts.alkhemi.conrod_id)
                    .color(Color::Rgba(0.0, 0.0, 0.0, fade))
                    .set(state.ids.zone_display_bg, ui);
                Text::new(state.last_region_name.as_ref().unwrap_or(&"".to_owned()))
                    .top_left_with_margins_on(state.ids.zone_display_bg, -2.5, -2.5)
                    .font_size(self.fonts.alkhemi.scale(70))
                    .font_id(self.fonts.alkhemi.conrod_id)
                    .color(Color::Rgba(1.0, 1.0, 1.0, fade))
                    .set(state.ids.zone_display, ui);
            },
            None => Text::new(" ")
                .middle_of(ui.window)
                .font_size(self.fonts.alkhemi.scale(14))
                .color(HP_COLOR)
                .set(state.ids.zone_display, ui),
        }

        // TODO: Subregion name display

        // Title
        match self.client.current_chunk() {
            Some(chunk) => Text::new(chunk.meta().name())
                .mid_top_with_margin_on(state.ids.mmap_frame, 2.0)
                .font_size(self.fonts.cyri.scale(18))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.mmap_location, ui),
            None => Text::new(" ")
                .mid_top_with_margin_on(state.ids.mmap_frame, 0.0)
                .font_size(self.fonts.cyri.scale(18))
                .color(TEXT_COLOR)
                .set(state.ids.mmap_location, ui),
        }

        None
    }
}
