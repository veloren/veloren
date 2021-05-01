use super::{RESET_BUTTONS_HEIGHT, RESET_BUTTONS_WIDTH};

use crate::{
    hud::{
        img_ids::Imgs, CRITICAL_HP_COLOR, HP_COLOR, LOW_HP_COLOR, MENU_BG, STAMINA_COLOR,
        TEXT_COLOR,
    },
    i18n::Localization,
    render::{
        AaMode, CloudMode, FluidMode, LightingMode, RenderMode, ShadowMapMode, ShadowMode,
        UpscaleMode,
    },
    session::settings_change::{Graphics as GraphicsChange, Graphics::*},
    settings::Fps,
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

use itertools::Itertools;
use std::iter::once;
use winit::monitor::VideoMode;

widget_ids! {
    struct Ids {
        window,
        window_r,
        window_scrollbar,
        reset_graphics_button,
        fps_counter,
        vd_slider,
        vd_text,
        vd_value,
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
        upscale_factor_text,
        upscale_factor_list,
        cloud_mode_text,
        cloud_mode_list,
        fluid_mode_text,
        fluid_mode_list,
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
        particles_button,
        particles_label,
        lossy_terrain_compression_button,
        lossy_terrain_compression_label,
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
        save_window_size_button,
    }
}

#[derive(WidgetCommon)]
pub struct Video<'a> {
    global_state: &'a GlobalState,
    imgs: &'a Imgs,
    fonts: &'a Fonts,
    localized_strings: &'a Localization,
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
        fps: f32,
    ) -> Self {
        Self {
            global_state,
            imgs,
            fonts,
            localized_strings,
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
const FPS_CHOICES: [Fps; 12] = [
    Fps::Max(15),
    Fps::Max(30),
    Fps::Max(40),
    Fps::Max(50),
    Fps::Max(60),
    Fps::Max(90),
    Fps::Max(120),
    Fps::Max(144),
    Fps::Max(240),
    Fps::Max(300),
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
            .unwrap()
            .video_modes()
            .collect();

        State {
            ids: Ids::new(id_gen),
            video_modes,
        }
    }

    #[allow(clippy::unused_unit)] // TODO: Pending review in #587
    fn style(&self) -> Self::Style { () }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
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
        // View Distance
        Text::new(&self.localized_strings.get("hud.settings.view_distance"))
            .top_left_with_margins_on(state.ids.window, 10.0, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.vd_text, ui);

        if let Some(new_val) = ImageSlider::discrete(
            self.global_state.settings.graphics.view_distance,
            1,
            // FIXME: Move back to 64 once we support multiple texture atlases, or figure out a
            // way to increase the size of the terrain atlas.
            65,
            self.imgs.slider_indicator,
            self.imgs.slider,
        )
        .w_h(104.0, 22.0)
        .down_from(state.ids.vd_text, 8.0)
        .track_breadth(12.0)
        .slider_length(10.0)
        .pad_track((5.0, 5.0))
        .set(state.ids.vd_slider, ui)
        {
            events.push(AdjustViewDistance(new_val));
        }

        Text::new(&format!(
            "{}",
            self.global_state.settings.graphics.view_distance
        ))
        .right_from(state.ids.vd_slider, 8.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.vd_value, ui);

        // Max FPS
        Text::new(&self.localized_strings.get("hud.settings.maximum_fps"))
            .down_from(state.ids.vd_slider, 10.0)
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
            events.push(ChangeMaxFPS(FPS_CHOICES[which]));
        }

        Text::new(&self.global_state.settings.graphics.max_fps.to_string())
            .right_from(state.ids.max_fps_slider, 8.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.max_fps_value, ui);

        // FOV
        Text::new(&self.localized_strings.get("hud.settings.fov"))
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
            events.push(ChangeFOV(new_val));
        }

        Text::new(&format!("{}", self.global_state.settings.graphics.fov))
            .right_from(state.ids.fov_slider, 8.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.fov_value, ui);

        // LoD detail
        Text::new(&self.localized_strings.get("hud.settings.lod_detail"))
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
            events.push(AdjustLodDetail(
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
        Text::new(&self.localized_strings.get("hud.settings.gamma"))
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
            events.push(ChangeGamma(2.0f32.powf(new_val as f32 / 8.0)));
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
            events.push(ChangeExposure(new_val as f32 / 16.0));
        }

        Text::new(&self.localized_strings.get("hud.settings.exposure"))
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
        // 320.0 = maximum brightness in shaders
        let min_ambiance = 10.0;
        let max_ambiance = 80.0;
        if let Some(new_val) = ImageSlider::discrete(
            self.global_state.settings.graphics.ambiance.round() as i32,
            min_ambiance as i32,
            max_ambiance as i32,
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
            events.push(ChangeAmbiance(new_val as f32));
        }
        Text::new(&self.localized_strings.get("hud.settings.ambiance"))
            .up_from(state.ids.ambiance_slider, 8.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.ambiance_text, ui);
        Text::new(&format!(
            "{:.0}%",
            ((self.global_state.settings.graphics.ambiance - min_ambiance)
                / (max_ambiance - min_ambiance)
                * 100.0)
                .round()
        ))
        .right_from(state.ids.ambiance_slider, 8.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.ambiance_value, ui);

        // Sprites VD
        if let Some(new_val) = ImageSlider::discrete(
            self.global_state.settings.graphics.sprite_render_distance,
            50,
            500,
            self.imgs.slider_indicator,
            self.imgs.slider,
        )
        .w_h(104.0, 22.0)
        .right_from(state.ids.vd_slider, 50.0)
        .track_breadth(12.0)
        .slider_length(10.0)
        .pad_track((5.0, 5.0))
        .set(state.ids.sprite_dist_slider, ui)
        {
            events.push(AdjustSpriteRenderDistance(new_val));
        }
        Text::new(
            &self
                .localized_strings
                .get("hud.settings.sprites_view_distance"),
        )
        .up_from(state.ids.sprite_dist_slider, 8.0)
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
        // Figure VD
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
        .right_from(state.ids.sprite_dist_slider, 50.0)
        .track_breadth(12.0)
        .slider_length(10.0)
        .pad_track((5.0, 5.0))
        .set(state.ids.figure_dist_slider, ui)
        {
            events.push(AdjustFigureLoDRenderDistance(new_val));
        }
        Text::new(
            &self
                .localized_strings
                .get("hud.settings.figures_view_distance"),
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

        let render_mode = &self.global_state.settings.graphics.render_mode;

        // AaMode
        Text::new(&self.localized_strings.get("hud.settings.antialiasing_mode"))
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
        ];
        let mode_label_list = [
            "No AA",
            "FXAA",
            /* "MSAA x4",
            "MSAA x8",
            "MSAA x16 (experimental)", */
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
            events.push(ChangeRenderMode(Box::new(RenderMode {
                aa: mode_list[clicked],
                ..render_mode.clone()
            })));
        }

        // Upscaling factor
        Text::new(&self.localized_strings.get("hud.settings.upscale_factor"))
            .down_from(state.ids.aa_mode_list, 8.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.upscale_factor_text, ui);

        let upscale_factors = [
            // Upscaling
            0.15, 0.2, 0.25, 0.35, 0.5, 0.65, 0.75, 0.85, 1.0,
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
                .map(|factor| format!("{n:.*}", 2, n = factor))
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
            events.push(ChangeRenderMode(Box::new(RenderMode {
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
                .get("hud.settings.cloud_rendering_mode"),
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
            &self.localized_strings.get("common.none"),
            &self
                .localized_strings
                .get("hud.settings.cloud_rendering_mode.minimal"),
            &self
                .localized_strings
                .get("hud.settings.cloud_rendering_mode.low"),
            &self
                .localized_strings
                .get("hud.settings.cloud_rendering_mode.medium"),
            &self
                .localized_strings
                .get("hud.settings.cloud_rendering_mode.high"),
            &self
                .localized_strings
                .get("hud.settings.cloud_rendering_mode.ultra"),
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
            events.push(ChangeRenderMode(Box::new(RenderMode {
                cloud: mode_list[clicked],
                ..render_mode.clone()
            })));
        }

        // FluidMode
        Text::new(
            &self
                .localized_strings
                .get("hud.settings.fluid_rendering_mode"),
        )
        .down_from(state.ids.cloud_mode_list, 8.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.fluid_mode_text, ui);

        let mode_list = [FluidMode::Cheap, FluidMode::Shiny];
        let mode_label_list = [
            &self
                .localized_strings
                .get("hud.settings.fluid_rendering_mode.cheap"),
            &self
                .localized_strings
                .get("hud.settings.fluid_rendering_mode.shiny"),
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
            events.push(ChangeRenderMode(Box::new(RenderMode {
                fluid: mode_list[clicked],
                ..render_mode.clone()
            })));
        }

        // LightingMode
        Text::new(
            &self
                .localized_strings
                .get("hud.settings.lighting_rendering_mode"),
        )
        .down_from(state.ids.fluid_mode_list, 8.0)
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
            &self
                .localized_strings
                .get("hud.settings.lighting_rendering_mode.ashikhmin"),
            &self
                .localized_strings
                .get("hud.settings.lighting_rendering_mode.blinnphong"),
            &self
                .localized_strings
                .get("hud.settings.lighting_rendering_mode.lambertian"),
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
            events.push(ChangeRenderMode(Box::new(RenderMode {
                lighting: mode_list[clicked],
                ..render_mode.clone()
            })));
        }

        // ShadowMode
        Text::new(
            &self
                .localized_strings
                .get("hud.settings.shadow_rendering_mode"),
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
            &self
                .localized_strings
                .get("hud.settings.shadow_rendering_mode.none"),
            &self
                .localized_strings
                .get("hud.settings.shadow_rendering_mode.cheap"),
            &self
                .localized_strings
                .get("hud.settings.shadow_rendering_mode.map"),
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
            events.push(ChangeRenderMode(Box::new(RenderMode {
                shadow: mode_list[clicked],
                ..render_mode.clone()
            })));
        }

        if let Some(shadow_map_mode) = shadow_map_mode {
            // Display the shadow map mode if selected.
            Text::new(
                &self
                    .localized_strings
                    .get("hud.settings.shadow_rendering_mode.map.resolution"),
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
                events.push(ChangeRenderMode(Box::new(RenderMode {
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

        // Particles
        Text::new(&self.localized_strings.get("hud.settings.particles"))
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .down_from(state.ids.shadow_mode_list, 8.0)
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
            events.push(ToggleParticlesEnabled(particles_enabled));
        }

        // Lossy terrain compression
        Text::new(
            &self
                .localized_strings
                .get("hud.settings.lossy_terrain_compression"),
        )
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .right_from(state.ids.particles_label, 64.0)
        .color(TEXT_COLOR)
        .set(state.ids.lossy_terrain_compression_label, ui);

        let lossy_terrain_compression = ToggleButton::new(
            self.global_state
                .settings
                .graphics
                .lossy_terrain_compression,
            self.imgs.checkbox,
            self.imgs.checkbox_checked,
        )
        .w_h(18.0, 18.0)
        .right_from(state.ids.lossy_terrain_compression_label, 10.0)
        .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
        .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
        .set(state.ids.lossy_terrain_compression_button, ui);

        if self
            .global_state
            .settings
            .graphics
            .lossy_terrain_compression
            != lossy_terrain_compression
        {
            events.push(ToggleLossyTerrainCompression(lossy_terrain_compression));
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

        Text::new(&self.localized_strings.get("hud.settings.resolution"))
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .down_from(state.ids.particles_label, 8.0)
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
            events.push(ChangeFullscreenMode(FullScreenSettings {
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

        Text::new(&self.localized_strings.get("hud.settings.bit_depth"))
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .down_from(state.ids.particles_label, 8.0)
            .right_from(state.ids.resolution, 8.0)
            .color(TEXT_COLOR)
            .set(state.ids.bit_depth_label, ui);

        if let Some(clicked) = DropDownList::new(
            once(String::from(self.localized_strings.get("common.automatic")))
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
            events.push(ChangeFullscreenMode(FullScreenSettings {
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

        Text::new(&self.localized_strings.get("hud.settings.refresh_rate"))
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .down_from(state.ids.particles_label, 8.0)
            .right_from(state.ids.bit_depth, 8.0)
            .color(TEXT_COLOR)
            .set(state.ids.refresh_rate_label, ui);

        if let Some(clicked) = DropDownList::new(
            once(String::from(self.localized_strings.get("common.automatic")))
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
            events.push(ChangeFullscreenMode(FullScreenSettings {
                refresh_rate: if clicked == 0 {
                    None
                } else {
                    Some(refresh_rates[clicked - 1])
                },
                ..self.global_state.settings.graphics.fullscreen
            }));
        }

        // Fullscreen
        Text::new(&self.localized_strings.get("hud.settings.fullscreen"))
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
            events.push(ChangeFullscreenMode(FullScreenSettings {
                enabled,
                ..self.global_state.settings.graphics.fullscreen
            }));
        }

        // Fullscreen Mode
        Text::new(&self.localized_strings.get("hud.settings.fullscreen_mode"))
            .down_from(state.ids.fullscreen_label, 8.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.fullscreen_mode_text, ui);

        let mode_list = [FullscreenMode::Exclusive, FullscreenMode::Borderless];
        let mode_label_list = [
            &self
                .localized_strings
                .get("hud.settings.fullscreen_mode.exclusive"),
            &self
                .localized_strings
                .get("hud.settings.fullscreen_mode.borderless"),
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
            events.push(ChangeFullscreenMode(FullScreenSettings {
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
            .label(&self.localized_strings.get("hud.settings.save_window_size"))
            .label_font_size(self.fonts.cyri.scale(14))
            .label_color(TEXT_COLOR)
            .label_font_id(self.fonts.cyri.conrod_id)
            .label_y(Relative::Scalar(2.0))
            .set(state.ids.save_window_size_button, ui)
            .was_clicked()
        {
            events.push(AdjustWindowSize(
                self.global_state
                    .window
                    .logical_size()
                    .map(|e| e as u16)
                    .into_array(),
            ));
        }

        // Reset the graphics settings to the default settings
        if Button::image(self.imgs.button)
            .w_h(RESET_BUTTONS_WIDTH, RESET_BUTTONS_HEIGHT)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .down_from(state.ids.fullscreen_mode_list, 12.0)
            .right_from(state.ids.save_window_size_button, 12.0)
            .label(&self.localized_strings.get("hud.settings.reset_graphics"))
            .label_font_size(self.fonts.cyri.scale(14))
            .label_color(TEXT_COLOR)
            .label_font_id(self.fonts.cyri.conrod_id)
            .label_y(Relative::Scalar(2.0))
            .set(state.ids.reset_graphics_button, ui)
            .was_clicked()
        {
            events.push(ResetGraphicsSettings);
        }

        events
    }
}
