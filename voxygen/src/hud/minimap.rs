use super::{img_ids::Imgs, Fonts, Show, HP_COLOR, TEXT_COLOR};
use client::{self, Client};
use common::comp;
use conrod_core::{
    color,
    image::Id,
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
    _world_map: Id,
    fonts: &'a Fonts,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    pulse: f32,
    zoom: f32,
}

impl<'a> MiniMap<'a> {
    pub fn new(
        show: &'a Show,
        client: &'a Client,
        imgs: &'a Imgs,
        world_map: Id,
        fonts: &'a Fonts,
        pulse: f32,
        zoom: f32,
    ) -> Self {
        Self {
            show,
            client,
            imgs,
            _world_map: world_map,
            fonts: fonts,
            common: widget::CommonBuilder::default(),
            pulse,
            zoom,
        }
    }
}

pub struct State {
    ids: Ids,

    last_region_name: Option<String>,
    last_update: Instant,
}

pub enum Event {
    Toggle,
}

impl<'a> Widget for MiniMap<'a> {
    type State = State;
    type Style = ();
    type Event = Option<Event>;

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),

            last_region_name: None,
            last_update: Instant::now(),
        }
    }

    fn style(&self) -> Self::Style {
        ()
    }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;
        let zoom = self.zoom as f64;
        if self.show.mini_map {
            Image::new(self.imgs.mmap_frame)
                .w_h(100.0 * 4.0 * zoom, 100.0 * 4.0 * zoom)
                .top_right_with_margins_on(ui.window, 5.0, 5.0)
                .set(state.ids.mmap_frame, ui);

            Rectangle::fill_with([92.0 * 4.0, 82.0 * 4.0], color::TRANSPARENT)
                .mid_top_with_margin_on(state.ids.mmap_frame, 13.0 * 4.0 + 4.0 * zoom)
                .set(state.ids.mmap_frame_bg, ui);
            // Zoom Buttons
            // TODO: Add zoomable minimap

            /*if Button::image(self.imgs.mmap_plus)
                .w_h(100.0 * 0.2 * zoom, 100.0 * 0.2 * zoom)
                .hover_image(self.imgs.mmap_plus_hover)
                .press_image(self.imgs.mmap_plus_press)
                .top_left_with_margins_on(state.ids.mmap_frame, 0.0, 0.0)
                .set(state.ids.mmap_plus, ui)
                .was_clicked()
            {
                if zoom > 0.0 {
                    zoom = zoom + 1.0
                } else if zoom == 5.0 {
                }
            }
            if Button::image(self.imgs.mmap_minus)
                .w_h(100.0 * 0.2 * zoom, 100.0 * 0.2 * zoom)
                .hover_image(self.imgs.mmap_minus_hover)
                .press_image(self.imgs.mmap_minus_press)
                .down_from(state.ids.mmap_plus, 0.0)
                .set(state.ids.mmap_minus, ui)
                .was_clicked()
            {
                if zoom < 6.0 {
                    zoom = zoom - 1.0
                } else if zoom == 0.0 {
                }
            }*/
            // Map Image
            Image::new(/*self.world_map*/ self.imgs.map_placeholder)
                .middle_of(state.ids.mmap_frame_bg)
                .w_h(92.0 * 4.0 * zoom, 82.0 * 4.0 * zoom)
                .parent(state.ids.mmap_frame_bg)
                .set(state.ids.grid, ui);
            // Coordinates
            let player_pos = self
                .client
                .state()
                .ecs()
                .read_storage::<comp::Pos>()
                .get(self.client.entity())
                .map_or(Vec3::zero(), |pos| pos.0);

            let worldsize = 32768.0; // TODO This has to get the actual world size and not be hardcoded
            let x = player_pos.x as f64 / worldsize * 92.0 * 4.0;
            let y = (/*1.0X-*/player_pos.y as f64 / worldsize) * 82.0 * 4.0;
            let indic_ani = (self.pulse * 6.0).cos() * 0.5 + 0.5; //Animation timer
            let indic_scale = 0.8;
            // Indicator
            Image::new(if indic_ani <= 0.5 {
                self.imgs.indicator_mmap
            } else {
                self.imgs.indicator_mmap_2
            })
            .bottom_left_with_margins_on(state.ids.grid, y, x - 5.0)
            .w_h(
                // Animation frames depening on timer value from 0.0 to 1.0
                22.0 * 0.8,
                if indic_ani <= 0.5 {
                    18.0 * indic_scale
                } else {
                    23.0 * indic_scale
                },
            )
            .color(Some(Color::Rgba(1.0, 1.0, 1.0, 1.0)))
            .floating(true)
            .parent(ui.window)
            .set(state.ids.indicator, ui);
        } else {
            Image::new(self.imgs.mmap_frame_closed)
                .w_h(100.0 * 2.0, 11.0 * 2.0)
                .top_right_with_margins_on(ui.window, 5.0, 5.0)
                .set(state.ids.mmap_frame, ui);
        }

        if Button::image(if self.show.mini_map {
            self.imgs.mmap_open
        } else {
            self.imgs.mmap_closed
        })
        .wh(if self.show.mini_map {
            [100.0 * 0.4; 2]
        } else {
            [100.0 * 0.2 * zoom; 2]
        })
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
                    .font_size(70)
                    .font_id(self.fonts.alkhemi)
                    .color(Color::Rgba(0.0, 0.0, 0.0, fade))
                    .set(state.ids.zone_display_bg, ui);
                Text::new(state.last_region_name.as_ref().unwrap_or(&"".to_owned()))
                    .top_left_with_margins_on(state.ids.zone_display_bg, -2.5, -2.5)
                    .font_size(70)
                    .font_id(self.fonts.alkhemi)
                    .color(Color::Rgba(1.0, 1.0, 1.0, fade))
                    .set(state.ids.zone_display, ui);
            }
            None => Text::new(" ")
                .middle_of(ui.window)
                .font_size(14)
                .color(HP_COLOR)
                .set(state.ids.zone_display, ui),
        }

        // TODO: Subregion name display

        // Title
        match self.client.current_chunk() {
            Some(chunk) => Text::new(chunk.meta().name())
                .mid_top_with_margin_on(
                    state.ids.mmap_frame,
                    if self.show.mini_map { 6.0 } else { 0.0 },
                )
                .font_size(if self.show.mini_map { 30 } else { 18 })
                .font_id(self.fonts.cyri)
                .color(TEXT_COLOR)
                .set(state.ids.mmap_location, ui),
            None => Text::new(" ")
                .mid_top_with_margin_on(state.ids.mmap_frame, 0.0)
                .font_size(18)
                .color(TEXT_COLOR)
                .set(state.ids.mmap_location, ui),
        }

        None
    }
}
