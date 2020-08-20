use super::{
    img_ids::{Imgs, ImgsRot},
    Show, TEXT_COLOR, UI_HIGHLIGHT_0, UI_MAIN,
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
use vek::*;

widget_ids! {
    struct Ids {
        mmap_frame,
        mmap_frame_2,
        mmap_frame_bg,
        mmap_location,
        mmap_button,
        mmap_plus,
        mmap_minus,
        grid,
        indicator,
        mmap_north,
        mmap_east,
        mmap_south,
        mmap_west,
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
    ori: Vec3<f32>,
}

impl<'a> MiniMap<'a> {
    pub fn new(
        show: &'a Show,
        client: &'a Client,
        imgs: &'a Imgs,
        rot_imgs: &'a ImgsRot,
        world_map: &'a (img_ids::Rotations, Vec2<u32>),
        fonts: &'a ConrodVoxygenFonts,
        ori: Vec3<f32>,
    ) -> Self {
        Self {
            show,
            client,
            imgs,
            rot_imgs,
            world_map,
            fonts,
            common: widget::CommonBuilder::default(),
            ori,
        }
    }
}

pub struct State {
    ids: Ids,

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

    #[allow(clippy::unused_unit)] // TODO: Pending review in #587
    fn style(&self) -> Self::Style { () }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;
        let zoom = state.zoom;
        const SCALE: f64 = 1.5;
        if self.show.mini_map {
            Image::new(self.imgs.mmap_frame)
                .w_h(174.0 * SCALE, 190.0 * SCALE)
                .top_right_with_margins_on(ui.window, 5.0, 5.0)
                .color(Some(UI_MAIN))
                .set(state.ids.mmap_frame, ui);
            Image::new(self.imgs.mmap_frame_2)
                .w_h(174.0 * SCALE, 190.0 * SCALE)
                .middle_of(state.ids.mmap_frame)
                .color(Some(UI_HIGHLIGHT_0))
                .set(state.ids.mmap_frame_2, ui);
            Rectangle::fill_with([170.0 * SCALE, 170.0 * SCALE], color::TRANSPARENT)
                .mid_top_with_margin_on(state.ids.mmap_frame_2, 18.0 * SCALE)
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
                .reduce_partial_max()/*.min(f64::MAX)*/;

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
                .image_color(UI_HIGHLIGHT_0)
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
                .image_color(UI_HIGHLIGHT_0)
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
            let w_src = max_zoom / zoom;
            let h_src = max_zoom / zoom;

            // Set map image to be centered around player coordinates.
            let rect_src = position::Rect::from_xy_dim(
                [
                    player_pos.x as f64 / TerrainChunkSize::RECT_SIZE.x as f64,
                    (worldsize.y - player_pos.y as f64) / TerrainChunkSize::RECT_SIZE.y as f64,
                ],
                [w_src, h_src],
            );

            let map_size = Vec2::new(170.0, 170.0);

            // Map Image
            Image::new(world_map.source_north)
                .middle_of(state.ids.mmap_frame_bg)
                .w_h(map_size.x * SCALE, map_size.y * SCALE)
                .parent(state.ids.mmap_frame_bg)
                .source_rectangle(rect_src)
                .set(state.ids.grid, ui);

            // Indicator
            let ind_scale = 0.4;
            Image::new(self.rot_imgs.indicator_mmap_small.none)
                .middle_of(state.ids.grid)
                .w_h(32.0 * ind_scale, 37.0 * ind_scale)
                .color(Some(UI_HIGHLIGHT_0))
                .floating(true)
                .parent(ui.window)
                .set(state.ids.indicator, ui);

            // Compass directions
            let dirs = [
                (Vec2::new(0.0, 1.0), state.ids.mmap_north, "N", true),
                (Vec2::new(1.0, 0.0), state.ids.mmap_east, "E", false),
                (Vec2::new(0.0, -1.0), state.ids.mmap_south, "S", false),
                (Vec2::new(-1.0, 0.0), state.ids.mmap_west, "W", false),
            ];
            for (dir, id, name, bold) in dirs.iter() {
                let cardinal_dir = Vec2::unit_x().rotated_z(self.ori.x as f64) * dir.x
                    + Vec2::unit_y().rotated_z(self.ori.x as f64) * dir.y;
                let clamped = (cardinal_dir * 3.0)
                    / (cardinal_dir * 3.0).map(|e| e.abs()).reduce_partial_max();
                let pos = clamped * (map_size * 0.73 - 10.0);
                Text::new(name)
                    .x_y_position_relative_to(
                        state.ids.grid,
                        position::Relative::Scalar(pos.x),
                        position::Relative::Scalar(pos.y),
                    )
                    .font_size(self.fonts.cyri.scale(18))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(if *bold {
                        Color::Rgba(0.75, 0.0, 0.0, 1.0)
                    } else {
                        TEXT_COLOR
                    })
                    .floating(true)
                    .parent(ui.window)
                    .set(*id, ui);
            }
        } else {
            Image::new(self.imgs.mmap_frame_closed)
                .w_h(174.0 * SCALE, 18.0 * SCALE)
                .color(Some(UI_MAIN))
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
        .image_color(UI_HIGHLIGHT_0)
        .set(state.ids.mmap_button, ui)
        .was_clicked()
        {
            return Some(Event::Toggle);
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
