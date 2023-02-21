use super::{RESET_BUTTONS_HEIGHT, RESET_BUTTONS_WIDTH};

use crate::{
    hud::{img_ids::Imgs, AutoPressBehavior, PressBehavior, MENU_BG, TEXT_COLOR},
    session::settings_change::{Gameplay as GameplayChange, Gameplay::*},
    ui::{fonts::Fonts, ImageSlider, ToggleButton},
    GlobalState,
};
use conrod_core::{
    color,
    position::Relative,
    widget::{self, Button, DropDownList, Rectangle, Scrollbar, Text},
    widget_ids, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};
use i18n::Localization;

widget_ids! {
    struct Ids {
        window,
        window_r,
        window_scrollbar,
        reset_gameplay_button,
        mouse_pan_slider,
        mouse_pan_label,
        mouse_pan_value,
        mouse_zoom_slider,
        mouse_zoom_label,
        mouse_zoom_value,
        mouse_zoom_invert_button,
        mouse_zoom_invert_label,
        camera_clamp_slider,
        camera_clamp_label,
        camera_clamp_value,
        mouse_y_invert_button,
        mouse_y_invert_label,
        controller_y_invert_button,
        controller_y_invert_label,
        smooth_pan_toggle_button,
        smooth_pan_toggle_label,
        //
        free_look_behavior_text,
        free_look_behavior_list,
        auto_walk_behavior_text,
        auto_walk_behavior_list,
        camera_clamp_behavior_text,
        camera_clamp_behavior_list,
        zoom_lock_behavior_text,
        zoom_lock_behavior_list,
        stop_auto_walk_on_input_button,
        stop_auto_walk_on_input_label,
        auto_camera_button,
        auto_camera_label,
        bow_zoom_button,
        bow_zoom_label,
        zoom_lock_button,
        zoom_lock_label,
    }
}

#[derive(WidgetCommon)]
pub struct Gameplay<'a> {
    global_state: &'a GlobalState,
    imgs: &'a Imgs,
    fonts: &'a Fonts,
    localized_strings: &'a Localization,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}
impl<'a> Gameplay<'a> {
    pub fn new(
        global_state: &'a GlobalState,
        imgs: &'a Imgs,
        fonts: &'a Fonts,
        localized_strings: &'a Localization,
    ) -> Self {
        Self {
            global_state,
            imgs,
            fonts,
            localized_strings,
            common: widget::CommonBuilder::default(),
        }
    }
}

pub struct State {
    ids: Ids,
}

impl<'a> Widget for Gameplay<'a> {
    type Event = Vec<GameplayChange>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        common_base::prof_span!("Gameplay::update");
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

        let display_pan = self.global_state.settings.gameplay.pan_sensitivity;
        let display_zoom = self.global_state.settings.gameplay.zoom_sensitivity;
        let display_clamp = self.global_state.settings.gameplay.camera_clamp_angle;

        // Mouse Pan Sensitivity
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-pan_sensitivity"),
        )
        .top_left_with_margins_on(state.ids.window, 10.0, 10.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.mouse_pan_label, ui);

        if let Some(new_val) = ImageSlider::discrete(
            display_pan,
            1,
            200,
            self.imgs.slider_indicator,
            self.imgs.slider,
        )
        .w_h(550.0, 22.0)
        .down_from(state.ids.mouse_pan_label, 10.0)
        .track_breadth(30.0)
        .slider_length(10.0)
        .pad_track((5.0, 5.0))
        .set(state.ids.mouse_pan_slider, ui)
        {
            events.push(AdjustMousePan(new_val));
        }

        Text::new(&format!("{}", display_pan))
            .right_from(state.ids.mouse_pan_slider, 8.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.mouse_pan_value, ui);

        // Mouse Zoom Sensitivity
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-zoom_sensitivity"),
        )
        .down_from(state.ids.mouse_pan_slider, 10.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.mouse_zoom_label, ui);

        if let Some(new_val) = ImageSlider::discrete(
            display_zoom,
            1,
            300,
            self.imgs.slider_indicator,
            self.imgs.slider,
        )
        .w_h(550.0, 22.0)
        .down_from(state.ids.mouse_zoom_label, 10.0)
        .track_breadth(30.0)
        .slider_length(10.0)
        .pad_track((5.0, 5.0))
        .set(state.ids.mouse_zoom_slider, ui)
        {
            events.push(AdjustMouseZoom(new_val));
        }

        Text::new(&format!("{}", display_zoom))
            .right_from(state.ids.mouse_zoom_slider, 8.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.mouse_zoom_value, ui);

        // Camera clamp angle
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-camera_clamp_angle"),
        )
        .down_from(state.ids.mouse_zoom_slider, 10.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.camera_clamp_label, ui);

        if let Some(new_val) = ImageSlider::discrete(
            display_clamp,
            1,
            90,
            self.imgs.slider_indicator,
            self.imgs.slider,
        )
        .w_h(550.0, 22.0)
        .down_from(state.ids.camera_clamp_label, 10.0)
        .track_breadth(30.0)
        .slider_length(10.0)
        .pad_track((5.0, 5.0))
        .set(state.ids.camera_clamp_slider, ui)
        {
            events.push(AdjustCameraClamp(new_val));
        }

        Text::new(&format!("{}", display_clamp))
            .right_from(state.ids.camera_clamp_slider, 8.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.camera_clamp_value, ui);

        // Zoom Inversion
        let zoom_inverted = ToggleButton::new(
            self.global_state.settings.gameplay.zoom_inversion,
            self.imgs.checkbox,
            self.imgs.checkbox_checked,
        )
        .w_h(18.0, 18.0)
        .down_from(state.ids.camera_clamp_slider, 20.0)
        .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
        .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
        .set(state.ids.mouse_zoom_invert_button, ui);

        if self.global_state.settings.gameplay.zoom_inversion != zoom_inverted {
            events.push(ToggleZoomInvert(
                !self.global_state.settings.gameplay.zoom_inversion,
            ));
        }

        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-invert_scroll_zoom"),
        )
        .right_from(state.ids.mouse_zoom_invert_button, 10.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .graphics_for(state.ids.mouse_zoom_invert_button)
        .color(TEXT_COLOR)
        .set(state.ids.mouse_zoom_invert_label, ui);

        // Mouse Y Inversion
        let mouse_y_inverted = ToggleButton::new(
            self.global_state.settings.gameplay.mouse_y_inversion,
            self.imgs.checkbox,
            self.imgs.checkbox_checked,
        )
        .w_h(18.0, 18.0)
        .right_from(state.ids.mouse_zoom_invert_label, 10.0)
        .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
        .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
        .set(state.ids.mouse_y_invert_button, ui);

        if self.global_state.settings.gameplay.mouse_y_inversion != mouse_y_inverted {
            events.push(ToggleMouseYInvert(
                !self.global_state.settings.gameplay.mouse_y_inversion,
            ));
        }

        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-invert_mouse_y_axis"),
        )
        .right_from(state.ids.mouse_y_invert_button, 10.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .graphics_for(state.ids.mouse_y_invert_button)
        .color(TEXT_COLOR)
        .set(state.ids.mouse_y_invert_label, ui);

        // Controller Y Pan Inversion
        let controller_y_inverted = ToggleButton::new(
            self.global_state.settings.controller.pan_invert_y,
            self.imgs.checkbox,
            self.imgs.checkbox_checked,
        )
        .w_h(18.0, 18.0)
        .right_from(state.ids.mouse_y_invert_label, 10.0)
        .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
        .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
        .set(state.ids.controller_y_invert_button, ui);

        if self.global_state.settings.controller.pan_invert_y != controller_y_inverted {
            events.push(ToggleControllerYInvert(
                !self.global_state.settings.controller.pan_invert_y,
            ));
        }

        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-invert_controller_y_axis"),
        )
        .right_from(state.ids.controller_y_invert_button, 10.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .graphics_for(state.ids.controller_y_invert_button)
        .color(TEXT_COLOR)
        .set(state.ids.controller_y_invert_label, ui);

        // Mouse Smoothing Toggle
        let smooth_pan_enabled = ToggleButton::new(
            self.global_state.settings.gameplay.smooth_pan_enable,
            self.imgs.checkbox,
            self.imgs.checkbox_checked,
        )
        .w_h(18.0, 18.0)
        .right_from(state.ids.controller_y_invert_label, 10.0)
        .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
        .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
        .set(state.ids.smooth_pan_toggle_button, ui);

        if self.global_state.settings.gameplay.smooth_pan_enable != smooth_pan_enabled {
            events.push(ToggleSmoothPan(
                !self.global_state.settings.gameplay.smooth_pan_enable,
            ));
        }

        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-enable_mouse_smoothing"),
        )
        .right_from(state.ids.smooth_pan_toggle_button, 10.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .graphics_for(state.ids.smooth_pan_toggle_button)
        .color(TEXT_COLOR)
        .set(state.ids.smooth_pan_toggle_label, ui);

        // Free look behaviour
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-free_look_behavior"),
        )
        .down_from(state.ids.mouse_zoom_invert_button, 10.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.free_look_behavior_text, ui);

        let mode_label_list = [
            self.localized_strings
                .get_msg("hud-settings-press_behavior-toggle"),
            self.localized_strings
                .get_msg("hud-settings-press_behavior-hold"),
        ];

        // Get which free look behavior is currently active
        let selected = self.global_state.settings.gameplay.free_look_behavior as usize;

        if let Some(clicked) = DropDownList::new(&mode_label_list, Some(selected))
            .w_h(200.0, 30.0)
            .color(MENU_BG)
            .label_color(TEXT_COLOR)
            .label_font_id(self.fonts.cyri.conrod_id)
            .down_from(state.ids.free_look_behavior_text, 8.0)
            .set(state.ids.free_look_behavior_list, ui)
        {
            match clicked {
                0 => events.push(ChangeFreeLookBehavior(PressBehavior::Toggle)),
                1 => events.push(ChangeFreeLookBehavior(PressBehavior::Hold)),
                _ => unreachable!(),
            }
        }

        // Auto walk behavior
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-auto_walk_behavior"),
        )
        .down_from(state.ids.mouse_zoom_invert_button, 10.0)
        .right_from(state.ids.free_look_behavior_text, 150.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.auto_walk_behavior_text, ui);

        let auto_walk_selected = self.global_state.settings.gameplay.auto_walk_behavior as usize;

        if let Some(clicked) = DropDownList::new(&mode_label_list, Some(auto_walk_selected))
            .w_h(200.0, 30.0)
            .color(MENU_BG)
            .label_color(TEXT_COLOR)
            .label_font_id(self.fonts.cyri.conrod_id)
            .down_from(state.ids.auto_walk_behavior_text, 8.0)
            .set(state.ids.auto_walk_behavior_list, ui)
        {
            match clicked {
                0 => events.push(ChangeAutoWalkBehavior(PressBehavior::Toggle)),
                1 => events.push(ChangeAutoWalkBehavior(PressBehavior::Hold)),
                _ => unreachable!(),
            }
        }

        // Camera clamp behavior
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-camera_clamp_behavior"),
        )
        .down_from(state.ids.free_look_behavior_list, 10.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.camera_clamp_behavior_text, ui);

        let camera_clamp_selected =
            self.global_state.settings.gameplay.camera_clamp_behavior as usize;

        if let Some(clicked) = DropDownList::new(&mode_label_list, Some(camera_clamp_selected))
            .w_h(200.0, 30.0)
            .color(MENU_BG)
            .label_color(TEXT_COLOR)
            .label_font_id(self.fonts.cyri.conrod_id)
            .down_from(state.ids.camera_clamp_behavior_text, 8.0)
            .set(state.ids.camera_clamp_behavior_list, ui)
        {
            match clicked {
                0 => events.push(ChangeCameraClampBehavior(PressBehavior::Toggle)),
                1 => events.push(ChangeCameraClampBehavior(PressBehavior::Hold)),
                _ => unreachable!(),
            }
        }

        // Stop autowalk on input toggle
        let stop_auto_walk_on_input_toggle = ToggleButton::new(
            self.global_state.settings.gameplay.stop_auto_walk_on_input,
            self.imgs.checkbox,
            self.imgs.checkbox_checked,
        )
        .w_h(18.0, 18.0)
        .down_from(state.ids.smooth_pan_toggle_button, 8.0)
        .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
        .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
        .set(state.ids.stop_auto_walk_on_input_button, ui);

        if self.global_state.settings.gameplay.stop_auto_walk_on_input
            != stop_auto_walk_on_input_toggle
        {
            events.push(ChangeStopAutoWalkOnInput(
                !self.global_state.settings.gameplay.stop_auto_walk_on_input,
            ));
        }

        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-stop_auto_walk_on_input"),
        )
        .right_from(state.ids.stop_auto_walk_on_input_button, 10.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .graphics_for(state.ids.stop_auto_walk_on_input_button)
        .color(TEXT_COLOR)
        .set(state.ids.stop_auto_walk_on_input_label, ui);

        // Auto-camera toggle
        let auto_camera_toggle = ToggleButton::new(
            self.global_state.settings.gameplay.auto_camera,
            self.imgs.checkbox,
            self.imgs.checkbox_checked,
        )
        .w_h(18.0, 18.0)
        .down_from(state.ids.stop_auto_walk_on_input_button, 8.0)
        .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
        .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
        .set(state.ids.auto_camera_button, ui);

        if self.global_state.settings.gameplay.auto_camera != auto_camera_toggle {
            events.push(ChangeAutoCamera(
                !self.global_state.settings.gameplay.auto_camera,
            ));
        }

        Text::new(&self.localized_strings.get_msg("hud-settings-auto_camera"))
            .right_from(state.ids.auto_camera_button, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .graphics_for(state.ids.auto_camera_button)
            .color(TEXT_COLOR)
            .set(state.ids.auto_camera_label, ui);

        // Charging bow zoom toggle
        let bow_zoom_toggle = ToggleButton::new(
            self.global_state.settings.gameplay.bow_zoom,
            self.imgs.checkbox,
            self.imgs.checkbox_checked,
        )
        .w_h(18.0, 18.0)
        .down_from(state.ids.auto_camera_button, 8.0)
        .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
        .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
        .set(state.ids.bow_zoom_button, ui);

        if self.global_state.settings.gameplay.bow_zoom != bow_zoom_toggle {
            events.push(ChangeBowZoom(!self.global_state.settings.gameplay.bow_zoom));
        }

        Text::new(&self.localized_strings.get_msg("hud-settings-bow_zoom"))
            .right_from(state.ids.bow_zoom_button, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .graphics_for(state.ids.bow_zoom_button)
            .color(TEXT_COLOR)
            .set(state.ids.bow_zoom_label, ui);

        let zoom_lock_label_list = [
            self.localized_strings
                .get_msg("hud-settings-autopress_behavior-toggle"),
            self.localized_strings
                .get_msg("hud-settings-autopress_behavior-auto"),
        ];

        // Camera zoom lock behavior
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-zoom_lock_behavior"),
        )
        .down_from(state.ids.auto_walk_behavior_list, 10.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.zoom_lock_behavior_text, ui);

        let zoom_lock_selected = self.global_state.settings.gameplay.zoom_lock_behavior as usize;

        if let Some(clicked) = DropDownList::new(&zoom_lock_label_list, Some(zoom_lock_selected))
            .w_h(200.0, 30.0)
            .color(MENU_BG)
            .label_color(TEXT_COLOR)
            .label_font_id(self.fonts.cyri.conrod_id)
            .down_from(state.ids.zoom_lock_behavior_text, 8.0)
            .set(state.ids.zoom_lock_behavior_list, ui)
        {
            match clicked {
                0 => events.push(ChangeZoomLockBehavior(AutoPressBehavior::Toggle)),
                1 => events.push(ChangeZoomLockBehavior(AutoPressBehavior::Auto)),
                _ => unreachable!(),
            }
        }

        // Camera zoom lock toggle
        let zoom_lock_toggle = ToggleButton::new(
            self.global_state.settings.gameplay.zoom_lock,
            self.imgs.checkbox,
            self.imgs.checkbox_checked,
        )
        .w_h(18.0, 18.0)
        .down_from(state.ids.bow_zoom_button, 8.0)
        .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
        .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
        .set(state.ids.zoom_lock_button, ui);

        if self.global_state.settings.gameplay.zoom_lock != zoom_lock_toggle {
            events.push(ChangeZoomLock(
                !self.global_state.settings.gameplay.zoom_lock,
            ));
        }

        Text::new(&self.localized_strings.get_msg("hud-settings-zoom_lock"))
            .right_from(state.ids.zoom_lock_button, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .graphics_for(state.ids.zoom_lock_button)
            .color(TEXT_COLOR)
            .set(state.ids.zoom_lock_label, ui);

        // Reset the gameplay settings to the default settings
        if Button::image(self.imgs.button)
            .w_h(RESET_BUTTONS_WIDTH, RESET_BUTTONS_HEIGHT)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .down_from(state.ids.camera_clamp_behavior_list, 12.0)
            .label(
                &self
                    .localized_strings
                    .get_msg("hud-settings-reset_gameplay"),
            )
            .label_font_size(self.fonts.cyri.scale(14))
            .label_color(TEXT_COLOR)
            .label_font_id(self.fonts.cyri.conrod_id)
            .label_y(Relative::Scalar(2.0))
            .set(state.ids.reset_gameplay_button, ui)
            .was_clicked()
        {
            events.push(ResetGameplaySettings);
        }

        events
    }
}
