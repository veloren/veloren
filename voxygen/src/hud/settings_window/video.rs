use super::{RESET_BUTTONS_HEIGHT, RESET_BUTTONS_WIDTH};

use crate::{
    hud::{
        img_ids::Imgs, CRITICAL_HP_COLOR, HP_COLOR, LOW_HP_COLOR, MENU_BG, STAMINA_COLOR,
        TEXT_COLOR, UI_HIGHLIGHT_0, UI_MAIN, UI_SUBTLE,
    },
    render::{
        AaMode, BloomConfig, BloomFactor, BloomMode, CloudMode, FluidMode, LightingMode,
        PresentMode, ReflectionMode, RenderMode, ShadowMapMode, ShadowMode, UpscaleMode,
    },
    session::settings_change::Graphics as GraphicsChange,
    settings::{Fps, GraphicsSettings},
    ui::{fonts::Fonts, ImageSlider, ToggleButton},
    window::{FullScreenSettings, FullscreenMode},
    GlobalState,
};
use conrod_core::{
    color,
    position::Relative,
    widget::{self, Button, DropDownList, Rectangle, Scrollbar, Text},
    widget_ids, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};
use core::convert::TryFrom;
use i18n::Localization;

use itertools::Itertools;
use std::{iter::once, rc::Rc};
use winit::monitor::VideoMode;

widget_ids! {
    struct Ids {
        window,
        window_r,
        window_scrollbar,
        reset_graphics_button,
        minimal_graphics_button,
        low_graphics_button,
        medium_graphics_button,
        high_graphics_button,
        ultra_graphics_button,
        fps_counter,
        pipeline_recreation_text,
        terrain_vd_slider,
        terrain_vd_text,
        terrain_vd_value,
        entity_vd_slider,
        entity_vd_text,
        entity_vd_value,
        ld_slider,
        ld_text,
        ld_value,
        lod_detail_slider,
        lod_detail_text,
        lod_detail_value,
        sprite_dist_slider,
        sprite_dist_text,
        sprite_dist_value,
        figure_dist_slider,
        figure_dist_text,
        figure_dist_value,
        max_fps_slider,
        max_fps_text,
        max_fps_value,
        max_background_fps_slider,
        max_background_fps_text,
        max_background_fps_value,
        present_mode_text,
        present_mode_list,
        fov_slider,
        fov_text,
        fov_value,
        gamma_slider,
        gamma_text,
        gamma_value,
        exposure_slider,
        exposure_text,
        exposure_value,
        ambiance_slider,
        ambiance_text,
        ambiance_value,
        aa_mode_text,
        aa_mode_list,
        //
        bloom_intensity_text,
        bloom_intensity_slider,
        bloom_intensity_value,
        point_glow_text,
        point_glow_slider,
        point_glow_value,
        //
        upscale_factor_text,
        upscale_factor_list,
        cloud_mode_text,
        cloud_mode_list,
        fluid_mode_text,
        fluid_mode_list,
        reflection_mode_text,
        reflection_mode_list,
        fullscreen_mode_text,
        fullscreen_mode_list,
        //
        resolution,
        resolution_label,
        bit_depth,
        bit_depth_label,
        refresh_rate,
        refresh_rate_label,
        //
        gpu_profiler_button,
        gpu_profiler_label,
        //
        particles_button,
        particles_label,
        weapon_trails_button,
        weapon_trails_label,
        flashing_lights_button,
        flashing_lights_label,
        flashing_lights_info_label,
        //
        fullscreen_button,
        fullscreen_label,
        lighting_mode_text,
        lighting_mode_list,
        shadow_mode_text,
        shadow_mode_list,
        shadow_mode_map_resolution_text,
        shadow_mode_map_resolution_slider,
        shadow_mode_map_resolution_value,
        rain_map_resolution_text,
        rain_map_resolution_slider,
        rain_map_resolution_value,
        save_window_size_button,

    }
}

#[derive(WidgetCommon)]
pub struct Video<'a> {
    global_state: &'a GlobalState,
    imgs: &'a Imgs,
    fonts: &'a Fonts,
    localized_strings: &'a Localization,
    server_view_distance_limit: Option<u32>,
    fps: f32,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}
impl<'a> Video<'a> {
    pub fn new(
        global_state: &'a GlobalState,
        imgs: &'a Imgs,
        fonts: &'a Fonts,
        localized_strings: &'a Localization,
        server_view_distance_limit: Option<u32>,
        fps: f32,
    ) -> Self {
        Self {
            global_state,
            imgs,
            fonts,
            localized_strings,
            server_view_distance_limit,
            fps,
            common: widget::CommonBuilder::default(),
        }
    }
}

pub struct State {
    ids: Ids,
    // Resolution, Bit Depth and Refresh Rate
    video_modes: Vec<VideoMode>,
}
const FPS_CHOICES: [Fps; 17] = [
    Fps::Max(15),
    Fps::Max(30),
    Fps::Max(40),
    Fps::Max(50),
    Fps::Max(60),
    Fps::Max(75),
    Fps::Max(90),
    Fps::Max(100),
    Fps::Max(120),
    Fps::Max(144),
    Fps::Max(165),
    Fps::Max(200),
    Fps::Max(240),
    Fps::Max(280),
    Fps::Max(360),
    Fps::Max(500),
    Fps::Unlimited,
];
const BG_FPS_CHOICES: [Fps; 20] = [
    Fps::Max(5),
    Fps::Max(10),
    Fps::Max(15),
    Fps::Max(20),
    Fps::Max(30),
    Fps::Max(40),
    Fps::Max(50),
    Fps::Max(60),
    Fps::Max(75),
    Fps::Max(90),
    Fps::Max(100),
    Fps::Max(120),
    Fps::Max(144),
    Fps::Max(165),
    Fps::Max(200),
    Fps::Max(240),
    Fps::Max(280),
    Fps::Max(360),
    Fps::Max(500),
    Fps::Unlimited,
];

impl<'a> Widget for Video<'a> {
    type Event = Vec<GraphicsChange>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        let video_modes = self
            .global_state
            .window
            .window()
            .current_monitor()
            .map(|monitor| monitor.video_modes().collect())
            .unwrap_or_default();

        State {
            ids: Ids::new(id_gen),
            video_modes,
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        common_base::prof_span!("Video::update");
        let widget::UpdateArgs { state, ui, .. } = args;

        let mut events = Vec::new();

        Rectangle::fill_with(args.rect.dim(), color::TRANSPARENT)
            .xy(args.rect.xy())
            .graphics_for(args.id)
            .scroll_kids()
            .scroll_kids_vertically()
            .set(state.ids.window, ui);
        Rectangle::fill_with([args.rect.w() / 2.0, args.rect.h()], color::TRANSPARENT)
            .top_right()
            .parent(state.ids.window)
            .set(state.ids.window_r, ui);
        Scrollbar::y_axis(state.ids.window)
            .thickness(5.0)
            .rgba(0.33, 0.33, 0.33, 1.0)
            .set(state.ids.window_scrollbar, ui);

        // FPS/TPS Counter
        //let text_col = match
        let fps_col = match self.fps as i32 {
            0..=14 => CRITICAL_HP_COLOR,
            15..=29 => LOW_HP_COLOR,
            30..=50 => HP_COLOR,
            _ => STAMINA_COLOR,
        };
        Text::new(&format!("FPS: {:.0}", self.fps))
            .color(fps_col)
            .top_right_with_margins_on(state.ids.window_r, 10.0, 10.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(18))
            .set(state.ids.fps_counter, ui);

        // Pipeline recreation status
        if let Some((total, complete)) = self
            .global_state
            .window
            .renderer()
            .pipeline_recreation_status()
        {
            Text::new(&format!("Rebuilding pipelines: ({}/{})", complete, total))
                .down_from(state.ids.fps_counter, 10.0)
                .align_right_of(state.ids.fps_counter)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                // TODO: make color pulse or something
                .color(TEXT_COLOR)
                .set(state.ids.pipeline_recreation_text, ui);
        }

        // Reset the graphics settings to the default settings
        if Button::image(self.imgs.button)
            .w_h(RESET_BUTTONS_WIDTH, RESET_BUTTONS_HEIGHT)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .top_left_with_margins_on(state.ids.window, 10.0, 10.0)
            .label(
                &self
                    .localized_strings
                    .get_msg("hud-settings-reset_graphics"),
            )
            .label_font_size(self.fonts.cyri.scale(14))
            .label_color(TEXT_COLOR)
            .label_font_id(self.fonts.cyri.conrod_id)
            .label_y(Relative::Scalar(2.0))
            .set(state.ids.reset_graphics_button, ui)
            .was_clicked()
        {
            events.push(GraphicsChange::ResetGraphicsSettings);
        }

        // Graphics presets buttons
        let preset_buttons: [(_, _, fn(_) -> _); 5] = [
            (
                "hud-settings-minimal_graphics",
                state.ids.minimal_graphics_button,
                GraphicsSettings::into_minimal,
            ),
            (
                "hud-settings-low_graphics",
                state.ids.low_graphics_button,
                GraphicsSettings::into_low,
            ),
            (
                "hud-settings-medium_graphics",
                state.ids.medium_graphics_button,
                GraphicsSettings::into_medium,
            ),
            (
                "hud-settings-high_graphics",
                state.ids.high_graphics_button,
                GraphicsSettings::into_high,
            ),
            (
                "hud-settings-ultra_graphics",
                state.ids.ultra_graphics_button,
                GraphicsSettings::into_ultra,
            ),
        ];

        let mut lhs = state.ids.reset_graphics_button;

        for (msg, id, change_fn) in preset_buttons {
            if Button::new()
                .label(&self.localized_strings.get_msg(msg))
                .w_h(80.0, 34.0)
                .color(UI_SUBTLE)
                .hover_color(UI_MAIN)
                .press_color(UI_HIGHLIGHT_0)
                .right_from(lhs, 12.0)
                .label_font_size(self.fonts.cyri.scale(14))
                .label_color(TEXT_COLOR)
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_y(Relative::Scalar(2.0))
                .set(id, ui)
                .was_clicked()
            {
                events.push(GraphicsChange::ChangeGraphicsSettings(Rc::new(change_fn)));
            }
            lhs = id;
        }

        // View Distance
        Text::new(&self.localized_strings.get_msg("hud-settings-view_distance"))
            .down_from(state.ids.reset_graphics_button, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.terrain_vd_text, ui);

        let terrain_view_distance = self.global_state.settings.graphics.terrain_view_distance;
        let server_view_distance_limit = self.server_view_distance_limit.unwrap_or(u32::MAX);
        if let Some(new_val) = ImageSlider::discrete(
            terrain_view_distance,
            1,
            client::MAX_SELECTABLE_VIEW_DISTANCE,
            self.imgs.slider_indicator,
            self.imgs.slider,
        )
        .w_h(104.0, 22.0)
        .down_from(state.ids.terrain_vd_text, 8.0)
        .track_breadth(12.0)
        .slider_length(10.0)
        .soft_max(server_view_distance_limit)
        .pad_track((5.0, 5.0))
        .set(state.ids.terrain_vd_slider, ui)
        {
            events.push(GraphicsChange::AdjustTerrainViewDistance(new_val));
        }

        Text::new(&if terrain_view_distance <= server_view_distance_limit {
            format!("{terrain_view_distance}")
        } else {
            format!("{terrain_view_distance} ({server_view_distance_limit})")
        })
        .right_from(state.ids.terrain_vd_slider, 8.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.terrain_vd_value, ui);

        // Entity View Distance
        let soft_entity_vd_max = self
            .server_view_distance_limit
            .unwrap_or(u32::MAX)
            .min(terrain_view_distance);
        let entity_view_distance = self.global_state.settings.graphics.entity_view_distance;
        if let Some(new_val) = ImageSlider::discrete(
            entity_view_distance,
            1,
            client::MAX_SELECTABLE_VIEW_DISTANCE,
            self.imgs.slider_indicator,
            self.imgs.slider,
        )
        .w_h(104.0, 22.0)
        .right_from(state.ids.terrain_vd_slider, 70.0)
        .track_breadth(12.0)
        .slider_length(10.0)
        .soft_max(soft_entity_vd_max)
        .pad_track((5.0, 5.0))
        .set(state.ids.entity_vd_slider, ui)
        {
            events.push(GraphicsChange::AdjustEntityViewDistance(new_val));
        }

        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-entity_view_distance"),
        )
        .up_from(state.ids.entity_vd_slider, 10.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.entity_vd_text, ui);

        Text::new(&if entity_view_distance <= soft_entity_vd_max {
            format!("{entity_view_distance}")
        } else {
            format!("{entity_view_distance} ({soft_entity_vd_max})")
        })
        .right_from(state.ids.entity_vd_slider, 8.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.entity_vd_value, ui);

        // Sprites VD
        if let Some(new_val) = ImageSlider::discrete(
            self.global_state.settings.graphics.sprite_render_distance,
            50,
            500,
            self.imgs.slider_indicator,
            self.imgs.slider,
        )
        .w_h(104.0, 22.0)
        .right_from(state.ids.entity_vd_slider, 70.0)
        .track_breadth(12.0)
        .slider_length(10.0)
        .pad_track((5.0, 5.0))
        .set(state.ids.sprite_dist_slider, ui)
        {
            events.push(GraphicsChange::AdjustSpriteRenderDistance(new_val));
        }
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-sprites_view_distance"),
        )
        .up_from(state.ids.sprite_dist_slider, 10.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.sprite_dist_text, ui);

        Text::new(&format!(
            "{}",
            self.global_state.settings.graphics.sprite_render_distance
        ))
        .right_from(state.ids.sprite_dist_slider, 8.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.sprite_dist_value, ui);

        // LoD Distance
        Text::new(&self.localized_strings.get_msg("hud-settings-lod_distance"))
            .down_from(state.ids.terrain_vd_slider, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.ld_text, ui);

        if let Some(new_val) = ImageSlider::discrete(
            self.global_state.settings.graphics.lod_distance,
            0,
            500,
            self.imgs.slider_indicator,
            self.imgs.slider,
        )
        .w_h(104.0, 22.0)
        .down_from(state.ids.ld_text, 8.0)
        .track_breadth(12.0)
        .slider_length(10.0)
        .pad_track((5.0, 5.0))
        .set(state.ids.ld_slider, ui)
        {
            events.push(GraphicsChange::AdjustLodDistance(new_val));
        }

        Text::new(&format!(
            "{}",
            self.global_state.settings.graphics.lod_distance
        ))
        .right_from(state.ids.ld_slider, 8.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.ld_value, ui);

        // Figure LOD distance
        if let Some(new_val) = ImageSlider::discrete(
            self.global_state
                .settings
                .graphics
                .figure_lod_render_distance,
            50,
            500,
            self.imgs.slider_indicator,
            self.imgs.slider,
        )
        .w_h(104.0, 22.0)
        .right_from(state.ids.ld_slider, 70.0)
        .track_breadth(12.0)
        .slider_length(10.0)
        .pad_track((5.0, 5.0))
        .set(state.ids.figure_dist_slider, ui)
        {
            events.push(GraphicsChange::AdjustFigureLoDRenderDistance(new_val));
        }
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-entities_detail_distance"),
        )
        .up_from(state.ids.figure_dist_slider, 8.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.figure_dist_text, ui);

        Text::new(&format!(
            "{}",
            self.global_state
                .settings
                .graphics
                .figure_lod_render_distance
        ))
        .right_from(state.ids.figure_dist_slider, 8.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.figure_dist_value, ui);

        // Max FPS
        Text::new(&self.localized_strings.get_msg("hud-settings-maximum_fps"))
            .down_from(state.ids.ld_slider, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.max_fps_text, ui);

        if let Some(which) = ImageSlider::discrete(
            FPS_CHOICES
                .iter()
                .position(|&x| x == self.global_state.settings.graphics.max_fps)
                .unwrap_or(5),
            0,
            FPS_CHOICES.len() - 1,
            self.imgs.slider_indicator,
            self.imgs.slider,
        )
        .w_h(104.0, 22.0)
        .down_from(state.ids.max_fps_text, 8.0)
        .track_breadth(12.0)
        .slider_length(10.0)
        .pad_track((5.0, 5.0))
        .set(state.ids.max_fps_slider, ui)
        {
            events.push(GraphicsChange::ChangeMaxFPS(FPS_CHOICES[which]));
        }

        Text::new(&self.global_state.settings.graphics.max_fps.to_string())
            .right_from(state.ids.max_fps_slider, 8.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.max_fps_value, ui);

        // Max Background FPS
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-background_fps"),
        )
        .down_from(state.ids.ld_slider, 10.0)
        .right_from(state.ids.max_fps_value, 44.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.max_background_fps_text, ui);

        if let Some(which) = ImageSlider::discrete(
            BG_FPS_CHOICES
                .iter()
                .position(|&x| x == self.global_state.settings.graphics.max_background_fps)
                .unwrap_or(5),
            0,
            BG_FPS_CHOICES.len() - 1,
            self.imgs.slider_indicator,
            self.imgs.slider,
        )
        .w_h(104.0, 22.0)
        .down_from(state.ids.max_background_fps_text, 8.0)
        .track_breadth(12.0)
        .slider_length(10.0)
        .pad_track((5.0, 5.0))
        .set(state.ids.max_background_fps_slider, ui)
        {
            events.push(GraphicsChange::ChangeMaxBackgroundFPS(
                BG_FPS_CHOICES[which],
            ));
        }

        Text::new(
            &self
                .global_state
                .settings
                .graphics
                .max_background_fps
                .to_string(),
        )
        .right_from(state.ids.max_background_fps_slider, 8.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.max_background_fps_value, ui);

        // Get render mode
        let render_mode = &self.global_state.settings.graphics.render_mode;

        // Present Mode
        Text::new(&self.localized_strings.get_msg("hud-settings-present_mode"))
            .down_from(state.ids.ld_slider, 10.0)
            .right_from(state.ids.max_background_fps_value, 40.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.present_mode_text, ui);

        let mode_list = [
            PresentMode::Fifo,
            PresentMode::Mailbox,
            PresentMode::Immediate,
        ];
        let mode_label_list = [
            "hud-settings-present_mode-vsync_capped",
            "hud-settings-present_mode-vsync_uncapped",
            "hud-settings-present_mode-vsync_off",
        ]
        .map(|k| self.localized_strings.get_msg(k));

        // Get which present mode is currently active
        let selected = mode_list
            .iter()
            .position(|x| *x == render_mode.present_mode);

        if let Some(clicked) = DropDownList::new(&mode_label_list, selected)
            .w_h(150.0, 26.0)
            .color(MENU_BG)
            .label_color(TEXT_COLOR)
            .label_font_id(self.fonts.cyri.conrod_id)
            .down_from(state.ids.present_mode_text, 8.0)
            .align_middle_x()
            .set(state.ids.present_mode_list, ui)
        {
            events.push(GraphicsChange::ChangeRenderMode(Box::new(RenderMode {
                present_mode: mode_list[clicked],
                ..render_mode.clone()
            })));
        }

        // FOV
        Text::new(&self.localized_strings.get_msg("hud-settings-fov"))
            .down_from(state.ids.max_fps_slider, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.fov_text, ui);

        if let Some(new_val) = ImageSlider::discrete(
            self.global_state.settings.graphics.fov,
            30,
            120,
            self.imgs.slider_indicator,
            self.imgs.slider,
        )
        .w_h(104.0, 22.0)
        .down_from(state.ids.fov_text, 8.0)
        .track_breadth(12.0)
        .slider_length(10.0)
        .pad_track((5.0, 5.0))
        .set(state.ids.fov_slider, ui)
        {
            events.push(GraphicsChange::ChangeFOV(new_val));
        }

        Text::new(&format!("{}", self.global_state.settings.graphics.fov))
            .right_from(state.ids.fov_slider, 8.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.fov_value, ui);

        // LoD detail
        Text::new(&self.localized_strings.get_msg("hud-settings-lod_detail"))
            .down_from(state.ids.fov_slider, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.lod_detail_text, ui);

        if let Some(new_val) = ImageSlider::discrete(
            ((self.global_state.settings.graphics.lod_detail as f32 / 100.0).log(5.0) * 10.0)
                .round() as i32,
            0,
            20,
            self.imgs.slider_indicator,
            self.imgs.slider,
        )
        .w_h(104.0, 22.0)
        .down_from(state.ids.lod_detail_text, 8.0)
        .track_breadth(12.0)
        .slider_length(10.0)
        .pad_track((5.0, 5.0))
        .set(state.ids.lod_detail_slider, ui)
        {
            events.push(GraphicsChange::AdjustLodDetail(
                (5.0f32.powf(new_val as f32 / 10.0) * 100.0) as u32,
            ));
        }

        Text::new(&format!(
            "{}",
            self.global_state.settings.graphics.lod_detail
        ))
        .right_from(state.ids.lod_detail_slider, 8.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.lod_detail_value, ui);

        // Gamma
        Text::new(&self.localized_strings.get_msg("hud-settings-gamma"))
            .down_from(state.ids.lod_detail_slider, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.gamma_text, ui);

        if let Some(new_val) = ImageSlider::discrete(
            (self.global_state.settings.graphics.gamma.log2() * 8.0).round() as i32,
            8,
            -8,
            self.imgs.slider_indicator,
            self.imgs.slider,
        )
        .w_h(104.0, 22.0)
        .down_from(state.ids.gamma_text, 8.0)
        .track_breadth(12.0)
        .slider_length(10.0)
        .pad_track((5.0, 5.0))
        .set(state.ids.gamma_slider, ui)
        {
            events.push(GraphicsChange::ChangeGamma(
                2.0f32.powf(new_val as f32 / 8.0),
            ));
        }

        Text::new(&format!("{:.2}", self.global_state.settings.graphics.gamma))
            .right_from(state.ids.gamma_slider, 8.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.gamma_value, ui);

        // Exposure
        if let Some(new_val) = ImageSlider::discrete(
            (self.global_state.settings.graphics.exposure * 16.0) as i32,
            0,
            32,
            self.imgs.slider_indicator,
            self.imgs.slider,
        )
        .w_h(104.0, 22.0)
        .right_from(state.ids.gamma_slider, 50.0)
        .track_breadth(12.0)
        .slider_length(10.0)
        .pad_track((5.0, 5.0))
        .set(state.ids.exposure_slider, ui)
        {
            events.push(GraphicsChange::ChangeExposure(new_val as f32 / 16.0));
        }

        Text::new(&self.localized_strings.get_msg("hud-settings-exposure"))
            .up_from(state.ids.exposure_slider, 8.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.exposure_text, ui);

        Text::new(&format!(
            "{:.2}",
            self.global_state.settings.graphics.exposure
        ))
        .right_from(state.ids.exposure_slider, 8.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.exposure_value, ui);

        //Ambiance Brightness
        if let Some(new_val) = ImageSlider::discrete(
            (self.global_state.settings.graphics.ambiance * 100.0).round() as i32,
            0,
            100,
            self.imgs.slider_indicator,
            self.imgs.slider,
        )
        .w_h(104.0, 22.0)
        .right_from(state.ids.exposure_slider, 50.0)
        .track_breadth(12.0)
        .slider_length(10.0)
        .pad_track((5.0, 5.0))
        .set(state.ids.ambiance_slider, ui)
        {
            events.push(GraphicsChange::ChangeAmbiance(new_val as f32 / 100.0));
        }
        Text::new(&self.localized_strings.get_msg("hud-settings-ambiance"))
            .up_from(state.ids.ambiance_slider, 8.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.ambiance_text, ui);
        Text::new(&format!(
            "{:.0}%",
            (self.global_state.settings.graphics.ambiance * 100.0).round()
        ))
        .right_from(state.ids.ambiance_slider, 8.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.ambiance_value, ui);

        // AaMode
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-antialiasing_mode"),
        )
        .down_from(state.ids.gamma_slider, 8.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.aa_mode_text, ui);

        // NOTE: MSAA modes are currently disabled from the UI due to poor
        // interaction with greedy meshing, and may eventually be removed.
        let mode_list = [
            AaMode::None,
            AaMode::Fxaa,
            /* AaMode::MsaaX4,
            AaMode::MsaaX8,
            AaMode::MsaaX16, */
            AaMode::FxUpscale,
            AaMode::Hqx,
        ];
        let mode_label_list = [
            "No anti-aliasing",
            "FXAA",
            /* "MSAA x4",
            "MSAA x8",
            "MSAA x16 (experimental)", */
            "FXUpscale",
            "HQX",
        ];

        // Get which AA mode is currently active
        let selected = mode_list.iter().position(|x| *x == render_mode.aa);

        if let Some(clicked) = DropDownList::new(&mode_label_list, selected)
            .w_h(400.0, 22.0)
            .color(MENU_BG)
            .label_color(TEXT_COLOR)
            .label_font_id(self.fonts.cyri.conrod_id)
            .down_from(state.ids.aa_mode_text, 8.0)
            .set(state.ids.aa_mode_list, ui)
        {
            events.push(GraphicsChange::ChangeRenderMode(Box::new(RenderMode {
                aa: mode_list[clicked],
                ..render_mode.clone()
            })));
        }

        // Bloom
        let bloom_intensity = match render_mode.bloom {
            BloomMode::Off => 0.0,
            BloomMode::On(bloom) => bloom.factor.fraction(),
        };
        let max_bloom = 0.3;

        Text::new(&self.localized_strings.get_msg("hud-settings-bloom"))
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .down_from(state.ids.aa_mode_list, 10.0)
            .color(TEXT_COLOR)
            .set(state.ids.bloom_intensity_text, ui);
        if let Some(new_val) = ImageSlider::continuous(
            bloom_intensity,
            0.0,
            max_bloom,
            self.imgs.slider_indicator,
            self.imgs.slider,
        )
        .w_h(104.0, 22.0)
        .down_from(state.ids.bloom_intensity_text, 8.0)
        .track_breadth(12.0)
        .slider_length(10.0)
        .pad_track((5.0, 5.0))
        .set(state.ids.bloom_intensity_slider, ui)
        {
            if new_val > f32::EPSILON {
                // Toggle Bloom On and set Custom value to new_val
                events.push(GraphicsChange::ChangeRenderMode(Box::new(RenderMode {
                    bloom: {
                        BloomMode::On(BloomConfig {
                            factor: BloomFactor::Custom(new_val),
                            // TODO: Decide if this should be a separate setting
                            uniform_blur: false,
                        })
                    },
                    ..render_mode.clone()
                })))
            } else {
                // Toggle Bloom Off if the value is near 0
                events.push(GraphicsChange::ChangeRenderMode(Box::new(RenderMode {
                    bloom: { BloomMode::Off },
                    ..render_mode.clone()
                })))
            }
        }
        Text::new(&if bloom_intensity <= f32::EPSILON {
            "Off".to_string()
        } else {
            format!("{}%", (bloom_intensity * 100.0 / max_bloom) as i32)
        })
        .right_from(state.ids.bloom_intensity_slider, 8.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.bloom_intensity_value, ui);

        // Point Glow
        Text::new(&self.localized_strings.get_msg("hud-settings-point_glow"))
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .down_from(state.ids.aa_mode_list, 10.0)
            .right_from(state.ids.bloom_intensity_value, 10.0)
            .color(TEXT_COLOR)
            .set(state.ids.point_glow_text, ui);
        if let Some(new_val) = ImageSlider::continuous(
            render_mode.point_glow,
            0.0,
            1.0,
            self.imgs.slider_indicator,
            self.imgs.slider,
        )
        .w_h(104.0, 22.0)
        .down_from(state.ids.point_glow_text, 8.0)
        .track_breadth(12.0)
        .slider_length(10.0)
        .pad_track((5.0, 5.0))
        .set(state.ids.point_glow_slider, ui)
        {
            // Toggle Bloom On and set Custom value to new_val
            events.push(GraphicsChange::ChangeRenderMode(Box::new(RenderMode {
                point_glow: new_val,
                ..render_mode.clone()
            })));
        }
        Text::new(&if render_mode.point_glow <= f32::EPSILON {
            "Off".to_string()
        } else {
            format!("{}%", (render_mode.point_glow * 100.0) as i32)
        })
        .right_from(state.ids.point_glow_slider, 8.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.point_glow_value, ui);

        // Upscaling factor
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-upscale_factor"),
        )
        .down_from(state.ids.bloom_intensity_slider, 8.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.upscale_factor_text, ui);

        let upscale_factors = [
            // Upscaling
            0.1, 0.15, 0.2, 0.25, 0.35, 0.5, 0.65, 0.75, 0.85, 1.0,
            // Downscaling (equivalent to SSAA)
            1.25, 1.5, 1.75, 2.0,
        ];

        // Get which upscale factor is currently active
        let selected = upscale_factors
            .iter()
            .position(|factor| (*factor - render_mode.upscale_mode.factor).abs() < 0.001);

        if let Some(clicked) = DropDownList::new(
            &upscale_factors
                .iter()
                .map(|factor| format!("{n:.*}", 3, n = factor))
                .collect::<Vec<String>>(),
            selected,
        )
        .w_h(400.0, 22.0)
        .color(MENU_BG)
        .label_color(TEXT_COLOR)
        .label_font_id(self.fonts.cyri.conrod_id)
        .down_from(state.ids.upscale_factor_text, 8.0)
        .set(state.ids.upscale_factor_list, ui)
        {
            events.push(GraphicsChange::ChangeRenderMode(Box::new(RenderMode {
                upscale_mode: UpscaleMode {
                    factor: upscale_factors[clicked],
                },
                ..render_mode.clone()
            })));
        }

        // CloudMode
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-cloud_rendering_mode"),
        )
        .down_from(state.ids.upscale_factor_list, 8.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.cloud_mode_text, ui);

        let mode_list = [
            CloudMode::None,
            CloudMode::Minimal,
            CloudMode::Low,
            CloudMode::Medium,
            CloudMode::High,
            CloudMode::Ultra,
        ];
        let mode_label_list = [
            self.localized_strings.get_msg("common-none"),
            self.localized_strings
                .get_msg("hud-settings-cloud_rendering_mode-minimal"),
            self.localized_strings
                .get_msg("hud-settings-cloud_rendering_mode-low"),
            self.localized_strings
                .get_msg("hud-settings-cloud_rendering_mode-medium"),
            self.localized_strings
                .get_msg("hud-settings-cloud_rendering_mode-high"),
            self.localized_strings
                .get_msg("hud-settings-cloud_rendering_mode-ultra"),
        ];

        // Get which cloud rendering mode is currently active
        let selected = mode_list.iter().position(|x| *x == render_mode.cloud);

        if let Some(clicked) = DropDownList::new(&mode_label_list, selected)
            .w_h(400.0, 22.0)
            .color(MENU_BG)
            .label_color(TEXT_COLOR)
            .label_font_id(self.fonts.cyri.conrod_id)
            .down_from(state.ids.cloud_mode_text, 8.0)
            .set(state.ids.cloud_mode_list, ui)
        {
            events.push(GraphicsChange::ChangeRenderMode(Box::new(RenderMode {
                cloud: mode_list[clicked],
                ..render_mode.clone()
            })));
        }

        // FluidMode
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-fluid_rendering_mode"),
        )
        .down_from(state.ids.cloud_mode_list, 8.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.fluid_mode_text, ui);

        let mode_list = [FluidMode::Low, FluidMode::Medium, FluidMode::High];
        let mode_label_list = [
            self.localized_strings
                .get_msg("hud-settings-fluid_rendering_mode-low"),
            self.localized_strings
                .get_msg("hud-settings-fluid_rendering_mode-medium"),
            self.localized_strings
                .get_msg("hud-settings-fluid_rendering_mode-high"),
        ];

        // Get which fluid rendering mode is currently active
        let selected = mode_list.iter().position(|x| *x == render_mode.fluid);

        if let Some(clicked) = DropDownList::new(&mode_label_list, selected)
            .w_h(400.0, 22.0)
            .color(MENU_BG)
            .label_color(TEXT_COLOR)
            .label_font_id(self.fonts.cyri.conrod_id)
            .down_from(state.ids.fluid_mode_text, 8.0)
            .set(state.ids.fluid_mode_list, ui)
        {
            events.push(GraphicsChange::ChangeRenderMode(Box::new(RenderMode {
                fluid: mode_list[clicked],
                ..render_mode.clone()
            })));
        }

        // ReflectionMode
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-reflection_rendering_mode"),
        )
        .down_from(state.ids.fluid_mode_list, 8.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.reflection_mode_text, ui);

        let mode_list = [
            ReflectionMode::Low,
            ReflectionMode::Medium,
            ReflectionMode::High,
        ];
        let mode_label_list = [
            self.localized_strings
                .get_msg("hud-settings-reflection_rendering_mode-low"),
            self.localized_strings
                .get_msg("hud-settings-reflection_rendering_mode-medium"),
            self.localized_strings
                .get_msg("hud-settings-reflection_rendering_mode-high"),
        ];

        // Get which fluid rendering mode is currently active
        let selected = mode_list.iter().position(|x| *x == render_mode.reflection);

        if let Some(clicked) = DropDownList::new(&mode_label_list, selected)
            .w_h(400.0, 22.0)
            .color(MENU_BG)
            .label_color(TEXT_COLOR)
            .label_font_id(self.fonts.cyri.conrod_id)
            .down_from(state.ids.reflection_mode_text, 8.0)
            .set(state.ids.reflection_mode_list, ui)
        {
            events.push(GraphicsChange::ChangeRenderMode(Box::new(RenderMode {
                reflection: mode_list[clicked],
                ..render_mode.clone()
            })));
        }

        // LightingMode
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-lighting_rendering_mode"),
        )
        .down_from(state.ids.reflection_mode_list, 8.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.lighting_mode_text, ui);

        let mode_list = [
            LightingMode::Ashikhmin,
            LightingMode::BlinnPhong,
            LightingMode::Lambertian,
        ];
        let mode_label_list = [
            self.localized_strings
                .get_msg("hud-settings-lighting_rendering_mode-ashikhmin"),
            self.localized_strings
                .get_msg("hud-settings-lighting_rendering_mode-blinnphong"),
            self.localized_strings
                .get_msg("hud-settings-lighting_rendering_mode-lambertian"),
        ];

        // Get which lighting rendering mode is currently active
        let selected = mode_list.iter().position(|x| *x == render_mode.lighting);

        if let Some(clicked) = DropDownList::new(&mode_label_list, selected)
            .w_h(400.0, 22.0)
            .color(MENU_BG)
            .label_color(TEXT_COLOR)
            .label_font_id(self.fonts.cyri.conrod_id)
            .down_from(state.ids.lighting_mode_text, 8.0)
            .set(state.ids.lighting_mode_list, ui)
        {
            events.push(GraphicsChange::ChangeRenderMode(Box::new(RenderMode {
                lighting: mode_list[clicked],
                ..render_mode.clone()
            })));
        }

        // ShadowMode
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-shadow_rendering_mode"),
        )
        .down_from(state.ids.lighting_mode_list, 8.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.shadow_mode_text, ui);

        let shadow_map_mode = ShadowMapMode::try_from(render_mode.shadow).ok();
        let mode_list = [
            ShadowMode::None,
            ShadowMode::Cheap,
            ShadowMode::Map(shadow_map_mode.unwrap_or_default()),
        ];
        let mode_label_list = [
            self.localized_strings
                .get_msg("hud-settings-shadow_rendering_mode-none"),
            self.localized_strings
                .get_msg("hud-settings-shadow_rendering_mode-cheap"),
            self.localized_strings
                .get_msg("hud-settings-shadow_rendering_mode-map"),
        ];

        // Get which shadow rendering mode is currently active
        let selected = mode_list.iter().position(|x| *x == render_mode.shadow);

        if let Some(clicked) = DropDownList::new(&mode_label_list, selected)
            .w_h(400.0, 22.0)
            .color(MENU_BG)
            .label_color(TEXT_COLOR)
            .label_font_id(self.fonts.cyri.conrod_id)
            .down_from(state.ids.shadow_mode_text, 8.0)
            .set(state.ids.shadow_mode_list, ui)
        {
            events.push(GraphicsChange::ChangeRenderMode(Box::new(RenderMode {
                shadow: mode_list[clicked],
                ..render_mode.clone()
            })));
        }

        if let Some(shadow_map_mode) = shadow_map_mode {
            // Display the shadow map mode if selected.
            Text::new(
                &self
                    .localized_strings
                    .get_msg("hud-settings-shadow_rendering_mode-map-resolution"),
            )
            .right_from(state.ids.shadow_mode_list, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.shadow_mode_map_resolution_text, ui);

            if let Some(new_val) = ImageSlider::discrete(
                (shadow_map_mode.resolution.log2() * 4.0).round() as i8,
                -8,
                8,
                self.imgs.slider_indicator,
                self.imgs.slider,
            )
            .w_h(104.0, 22.0)
            .right_from(state.ids.shadow_mode_map_resolution_text, 8.0)
            .track_breadth(12.0)
            .slider_length(10.0)
            .pad_track((5.0, 5.0))
            .set(state.ids.shadow_mode_map_resolution_slider, ui)
            {
                events.push(GraphicsChange::ChangeRenderMode(Box::new(RenderMode {
                    shadow: ShadowMode::Map(ShadowMapMode {
                        resolution: 2.0f32.powf(f32::from(new_val) / 4.0),
                    }),
                    ..render_mode.clone()
                })));
            }

            // TODO: Consider fixing to avoid allocation (it's probably not a bottleneck but
            // there's no reason to allocate for numbers).
            Text::new(&format!("{}", shadow_map_mode.resolution))
                .right_from(state.ids.shadow_mode_map_resolution_slider, 8.0)
                .font_size(self.fonts.cyri.scale(14))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.shadow_mode_map_resolution_value, ui);
        }

        // Rain occlusion texture size
        // Display the shadow map mode if selected.
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-rain_occlusion-resolution"),
        )
        .down_from(state.ids.shadow_mode_list, 10.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.rain_map_resolution_text, ui);

        if let Some(new_val) = ImageSlider::discrete(
            (render_mode.rain_occlusion.resolution.log2() * 4.0).round() as i8,
            -8,
            8,
            self.imgs.slider_indicator,
            self.imgs.slider,
        )
        .w_h(104.0, 22.0)
        .right_from(state.ids.rain_map_resolution_text, 8.0)
        .track_breadth(12.0)
        .slider_length(10.0)
        .pad_track((5.0, 5.0))
        .set(state.ids.rain_map_resolution_slider, ui)
        {
            events.push(GraphicsChange::ChangeRenderMode(Box::new(RenderMode {
                rain_occlusion: ShadowMapMode {
                    resolution: 2.0f32.powf(f32::from(new_val) / 4.0),
                },
                ..render_mode.clone()
            })));
        }
        Text::new(&format!("{}", render_mode.rain_occlusion.resolution))
            .right_from(state.ids.rain_map_resolution_slider, 8.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.rain_map_resolution_value, ui);

        // GPU Profiler
        Text::new(&self.localized_strings.get_msg("hud-settings-gpu_profiler"))
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .down_from(state.ids.rain_map_resolution_text, 8.0)
            .color(TEXT_COLOR)
            .set(state.ids.gpu_profiler_label, ui);

        let gpu_profiler_enabled = ToggleButton::new(
            render_mode.profiler_enabled,
            self.imgs.checkbox,
            self.imgs.checkbox_checked,
        )
        .w_h(18.0, 18.0)
        .right_from(state.ids.gpu_profiler_label, 10.0)
        .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
        .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
        .set(state.ids.gpu_profiler_button, ui);

        if render_mode.profiler_enabled != gpu_profiler_enabled {
            events.push(GraphicsChange::ChangeRenderMode(Box::new(RenderMode {
                profiler_enabled: gpu_profiler_enabled,
                ..render_mode.clone()
            })));
        }

        // Particles
        Text::new(&self.localized_strings.get_msg("hud-settings-particles"))
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .down_from(state.ids.gpu_profiler_label, 8.0)
            .color(TEXT_COLOR)
            .set(state.ids.particles_label, ui);

        let particles_enabled = ToggleButton::new(
            self.global_state.settings.graphics.particles_enabled,
            self.imgs.checkbox,
            self.imgs.checkbox_checked,
        )
        .w_h(18.0, 18.0)
        .right_from(state.ids.particles_label, 10.0)
        .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
        .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
        .set(state.ids.particles_button, ui);

        if self.global_state.settings.graphics.particles_enabled != particles_enabled {
            events.push(GraphicsChange::ToggleParticlesEnabled(particles_enabled));
        }

        // Weapon trails
        Text::new(&self.localized_strings.get_msg("hud-settings-weapon_trails"))
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .right_from(state.ids.particles_label, 64.0)
            .color(TEXT_COLOR)
            .set(state.ids.weapon_trails_label, ui);

        let weapon_trails_enabled = ToggleButton::new(
            self.global_state.settings.graphics.weapon_trails_enabled,
            self.imgs.checkbox,
            self.imgs.checkbox_checked,
        )
        .w_h(18.0, 18.0)
        .right_from(state.ids.weapon_trails_label, 10.0)
        .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
        .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
        .set(state.ids.weapon_trails_button, ui);

        if self.global_state.settings.graphics.weapon_trails_enabled != weapon_trails_enabled {
            events.push(GraphicsChange::ToggleWeaponTrailsEnabled(
                weapon_trails_enabled,
            ));
        }

        // Disable flashing lights
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-flashing_lights"),
        )
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .down_from(state.ids.particles_label, 25.0)
        .color(TEXT_COLOR)
        .set(state.ids.flashing_lights_label, ui);

        let flashing_lights_enabled = ToggleButton::new(
            self.global_state
                .settings
                .graphics
                .render_mode
                .flashing_lights_enabled,
            self.imgs.checkbox,
            self.imgs.checkbox_checked,
        )
        .w_h(18.0, 18.0)
        .right_from(state.ids.flashing_lights_label, 10.0)
        .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
        .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
        .set(state.ids.flashing_lights_button, ui);

        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-flashing_lights_info"),
        )
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .right_from(state.ids.flashing_lights_label, 32.0)
        .color(TEXT_COLOR)
        .set(state.ids.flashing_lights_info_label, ui);

        if self
            .global_state
            .settings
            .graphics
            .render_mode
            .flashing_lights_enabled
            != flashing_lights_enabled
        {
            events.push(GraphicsChange::ChangeRenderMode(Box::new(RenderMode {
                flashing_lights_enabled,
                ..render_mode.clone()
            })));
        }

        // Resolution
        let resolutions: Vec<[u16; 2]> = state
            .video_modes
            .iter()
            .sorted_by_key(|mode| mode.size().height)
            .sorted_by_key(|mode| mode.size().width)
            .map(|mode| [mode.size().width as u16, mode.size().height as u16])
            .dedup()
            .collect();

        Text::new(&self.localized_strings.get_msg("hud-settings-resolution"))
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .down_from(state.ids.flashing_lights_label, 25.0)
            .color(TEXT_COLOR)
            .set(state.ids.resolution_label, ui);

        if let Some(clicked) = DropDownList::new(
            resolutions
                .iter()
                .map(|res| format!("{}x{}", res[0], res[1]))
                .collect::<Vec<String>>()
                .as_slice(),
            resolutions
                .iter()
                .position(|res| res == &self.global_state.settings.graphics.fullscreen.resolution),
        )
        .w_h(128.0, 22.0)
        .color(MENU_BG)
        .label_color(TEXT_COLOR)
        .label_font_id(self.fonts.opensans.conrod_id)
        .down_from(state.ids.resolution_label, 10.0)
        .set(state.ids.resolution, ui)
        {
            events.push(GraphicsChange::ChangeFullscreenMode(FullScreenSettings {
                resolution: resolutions[clicked],
                ..self.global_state.settings.graphics.fullscreen
            }));
        }

        // Bit Depth and Refresh Rate
        let correct_res: Vec<&VideoMode> = state
            .video_modes
            .iter()
            .filter(|mode| {
                mode.size().width
                    == self.global_state.settings.graphics.fullscreen.resolution[0] as u32
            })
            .filter(|mode| {
                mode.size().height
                    == self.global_state.settings.graphics.fullscreen.resolution[1] as u32
            })
            .collect();

        // Bit Depth
        let bit_depths: Vec<u16> = correct_res
            .iter()
            .filter(
                |mode| match self.global_state.settings.graphics.fullscreen.refresh_rate {
                    Some(refresh_rate) => mode.refresh_rate() == refresh_rate,
                    None => true,
                },
            )
            .sorted_by_key(|mode| mode.bit_depth())
            .map(|mode| mode.bit_depth())
            .rev()
            .dedup()
            .collect();

        Text::new(&self.localized_strings.get_msg("hud-settings-bit_depth"))
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .down_from(state.ids.flashing_lights_label, 25.0)
            .right_from(state.ids.resolution, 8.0)
            .color(TEXT_COLOR)
            .set(state.ids.bit_depth_label, ui);

        if let Some(clicked) = DropDownList::new(
            once(String::from(
                self.localized_strings.get_msg("common-automatic"),
            ))
            .chain(bit_depths.iter().map(|depth| format!("{}", depth)))
            .collect::<Vec<String>>()
            .as_slice(),
            match self.global_state.settings.graphics.fullscreen.bit_depth {
                Some(bit_depth) => bit_depths
                    .iter()
                    .position(|depth| depth == &bit_depth)
                    .map(|index| index + 1),
                None => Some(0),
            },
        )
        .w_h(128.0, 22.0)
        .color(MENU_BG)
        .label_color(TEXT_COLOR)
        .label_font_id(self.fonts.opensans.conrod_id)
        .down_from(state.ids.bit_depth_label, 10.0)
        .right_from(state.ids.resolution, 8.0)
        .set(state.ids.bit_depth, ui)
        {
            events.push(GraphicsChange::ChangeFullscreenMode(FullScreenSettings {
                bit_depth: if clicked == 0 {
                    None
                } else {
                    Some(bit_depths[clicked - 1])
                },
                ..self.global_state.settings.graphics.fullscreen
            }));
        }

        // Refresh Rate
        let refresh_rates: Vec<u16> = correct_res
            .into_iter()
            .filter(
                |mode| match self.global_state.settings.graphics.fullscreen.bit_depth {
                    Some(bit_depth) => mode.bit_depth() == bit_depth,
                    None => true,
                },
            )
            .sorted_by_key(|mode| mode.refresh_rate())
            .map(|mode| mode.refresh_rate())
            .rev()
            .dedup()
            .collect();

        Text::new(&self.localized_strings.get_msg("hud-settings-refresh_rate"))
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .down_from(state.ids.flashing_lights_label, 25.0)
            .right_from(state.ids.bit_depth, 8.0)
            .color(TEXT_COLOR)
            .set(state.ids.refresh_rate_label, ui);

        if let Some(clicked) = DropDownList::new(
            once(String::from(
                self.localized_strings.get_msg("common-automatic"),
            ))
            .chain(refresh_rates.iter().map(|rate| format!("{}", rate)))
            .collect::<Vec<String>>()
            .as_slice(),
            match self.global_state.settings.graphics.fullscreen.refresh_rate {
                Some(refresh_rate) => refresh_rates
                    .iter()
                    .position(|rate| rate == &refresh_rate)
                    .map(|index| index + 1),
                None => Some(0),
            },
        )
        .w_h(128.0, 22.0)
        .color(MENU_BG)
        .label_color(TEXT_COLOR)
        .label_font_id(self.fonts.opensans.conrod_id)
        .down_from(state.ids.refresh_rate_label, 10.0)
        .right_from(state.ids.bit_depth, 8.0)
        .set(state.ids.refresh_rate, ui)
        {
            events.push(GraphicsChange::ChangeFullscreenMode(FullScreenSettings {
                refresh_rate: if clicked == 0 {
                    None
                } else {
                    Some(refresh_rates[clicked - 1])
                },
                ..self.global_state.settings.graphics.fullscreen
            }));
        }

        // Fullscreen
        Text::new(&self.localized_strings.get_msg("hud-settings-fullscreen"))
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .down_from(state.ids.resolution, 8.0)
            .color(TEXT_COLOR)
            .set(state.ids.fullscreen_label, ui);

        let enabled = ToggleButton::new(
            self.global_state.settings.graphics.fullscreen.enabled,
            self.imgs.checkbox,
            self.imgs.checkbox_checked,
        )
        .w_h(18.0, 18.0)
        .right_from(state.ids.fullscreen_label, 10.0)
        .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
        .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
        .set(state.ids.fullscreen_button, ui);

        if self.global_state.settings.graphics.fullscreen.enabled != enabled {
            events.push(GraphicsChange::ChangeFullscreenMode(FullScreenSettings {
                enabled,
                ..self.global_state.settings.graphics.fullscreen
            }));
        }

        // Fullscreen Mode
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-fullscreen_mode"),
        )
        .down_from(state.ids.fullscreen_label, 8.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.fullscreen_mode_text, ui);

        let mode_list = [FullscreenMode::Exclusive, FullscreenMode::Borderless];
        let mode_label_list = [
            &self
                .localized_strings
                .get_msg("hud-settings-fullscreen_mode-exclusive"),
            &self
                .localized_strings
                .get_msg("hud-settings-fullscreen_mode-borderless"),
        ];

        // Get which fullscreen mode is currently active
        let selected = mode_list
            .iter()
            .position(|x| *x == self.global_state.settings.graphics.fullscreen.mode);

        if let Some(clicked) = DropDownList::new(&mode_label_list, selected)
            .w_h(400.0, 22.0)
            .color(MENU_BG)
            .label_color(TEXT_COLOR)
            .label_font_id(self.fonts.cyri.conrod_id)
            .down_from(state.ids.fullscreen_mode_text, 8.0)
            .set(state.ids.fullscreen_mode_list, ui)
        {
            events.push(GraphicsChange::ChangeFullscreenMode(FullScreenSettings {
                mode: mode_list[clicked],
                ..self.global_state.settings.graphics.fullscreen
            }));
        }

        // Save current screen size
        if Button::image(self.imgs.button)
            .w_h(RESET_BUTTONS_WIDTH, RESET_BUTTONS_HEIGHT)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .down_from(state.ids.fullscreen_mode_list, 12.0)
            .label(
                &self
                    .localized_strings
                    .get_msg("hud-settings-save_window_size"),
            )
            .label_font_size(self.fonts.cyri.scale(14))
            .label_color(TEXT_COLOR)
            .label_font_id(self.fonts.cyri.conrod_id)
            .label_y(Relative::Scalar(2.0))
            .set(state.ids.save_window_size_button, ui)
            .was_clicked()
        {
            events.push(GraphicsChange::AdjustWindowSize(
                self.global_state
                    .window
                    .logical_size()
                    .map(|e| e as u16)
                    .into_array(),
            ));
        }

        events
    }
}
