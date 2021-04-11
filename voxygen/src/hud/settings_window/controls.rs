use super::{Event, RESET_BUTTONS_HEIGHT, RESET_BUTTONS_WIDTH};

use crate::{
    hud::{img_ids::Imgs, ERROR_COLOR, TEXT_BIND_CONFLICT_COLOR, TEXT_COLOR},
    i18n::Localization,
    ui::fonts::Fonts,
    window::GameInput,
    GlobalState,
};
use conrod_core::{
    color,
    position::Relative,
    widget::{self, Button, Rectangle, Scrollbar, Text},
    widget_ids, Borderable, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};

widget_ids! {
    struct Ids {
        window,
        window_r,
        window_scrollbar,
        frame,
        tabs_align,
        icon,
        settings_icon,
        settings_button_mo,
        settings_close,
        settings_title,
        settings_r,
        settings_l,
        settings_scrollbar,
        controls_texts[],
        controls_buttons[],
        reset_interface_button,
        reset_gameplay_button,
        reset_controls_button,
        reset_graphics_button,
        reset_sound_button,
        controls_alignment_rectangle,
        button_help,
        button_help2,
        show_help_label,
        ui_scale_label,
        ui_scale_slider,
        ui_scale_button,
        ui_scale_value,
        relative_to_win_button,
        relative_to_win_text,
        absolute_scale_button,
        absolute_scale_text,
        gameplay,
        controls,
        languages,
        language_list[],
        languages_list,
        rectangle,
        general_txt,
        load_tips_button,
        load_tips_button_label,
        debug_button,
        debug_button_label,
        interface,
        language_text,
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
        ch_title,
        ch_transp_slider,
        ch_transp_label,
        ch_transp_value,
        ch_transp_text,
        ch_1_bg,
        ch_2_bg,
        ch_3_bg,
        crosshair_outer_1,
        crosshair_inner_1,
        crosshair_outer_2,
        crosshair_inner_2,
        crosshair_outer_3,
        crosshair_inner_3,
        settings_bg,
        sound,
        test,
        video,
        language,
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
        audio_volume_slider,
        audio_volume_text,
        sfx_volume_slider,
        sfx_volume_text,
        audio_device_list,
        audio_device_text,
        //
        hotbar_title,
        bar_numbers_title,
        show_bar_numbers_none_button,
        show_bar_numbers_none_text,
        show_bar_numbers_values_button,
        show_bar_numbers_values_text,
        show_bar_numbers_percentage_button,
        show_bar_numbers_percentage_text,
        //
        show_shortcuts_button,
        show_shortcuts_text,
        buff_pos_bar_button,
        buff_pos_bar_text,
        buff_pos_map_button,
        buff_pos_map_text,
        //
        chat_transp_title,
        chat_transp_text,
        chat_transp_slider,
        chat_char_name_text,
        chat_char_name_button,
        //
        sct_title,
        sct_show_text,
        sct_show_radio,
        sct_single_dmg_text,
        sct_single_dmg_radio,
        sct_show_batch_text,
        sct_show_batch_radio,
        sct_batched_dmg_text,
        sct_batched_dmg_radio,
        sct_inc_dmg_text,
        sct_inc_dmg_radio,
        sct_batch_inc_text,
        sct_batch_inc_radio,
        sct_num_dur_text,
        sct_num_dur_slider,
        sct_num_dur_value,
        //
        speech_bubble_text,
        speech_bubble_dark_mode_text,
        speech_bubble_dark_mode_button,
        speech_bubble_icon_text,
        speech_bubble_icon_button,
        free_look_behavior_text,
        free_look_behavior_list,
        auto_walk_behavior_text,
        auto_walk_behavior_list,
        camera_clamp_behavior_text,
        camera_clamp_behavior_list,
        stop_auto_walk_on_input_button,
        stop_auto_walk_on_input_label,
        auto_camera_button,
        auto_camera_label,
    }
}

#[derive(WidgetCommon)]
pub struct Controls<'a> {
    global_state: &'a GlobalState,
    imgs: &'a Imgs,
    fonts: &'a Fonts,
    localized_strings: &'a Localization,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}
impl<'a> Controls<'a> {
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

impl<'a> Widget for Controls<'a> {
    type Event = Vec<Event>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
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

        // Used for sequential placement in a flow-down pattern
        let mut previous_element_id = None;
        let mut keybindings_vec: Vec<GameInput> = GameInput::iterator().collect();
        keybindings_vec.sort();

        let controls = &self.global_state.settings.controls;
        if keybindings_vec.len() > state.ids.controls_texts.len()
            || keybindings_vec.len() > state.ids.controls_buttons.len()
        {
            state.update(|s| {
                s.ids
                    .controls_texts
                    .resize(keybindings_vec.len(), &mut ui.widget_id_generator());
                s.ids
                    .controls_buttons
                    .resize(keybindings_vec.len(), &mut ui.widget_id_generator());
            });
        }

        // Loop all existing keybindings and the ids for text and button widgets
        for (game_input, (&text_id, &button_id)) in keybindings_vec.into_iter().zip(
            state
                .ids
                .controls_texts
                .iter()
                .zip(state.ids.controls_buttons.iter()),
        ) {
            let (key_string, key_color) =
                if self.global_state.window.remapping_keybindings == Some(game_input) {
                    (
                        String::from(self.localized_strings.get("hud.settings.awaitingkey")),
                        TEXT_COLOR,
                    )
                } else if let Some(key) = controls.get_binding(game_input) {
                    (
                        key.to_string(),
                        if controls.has_conflicting_bindings(key) {
                            TEXT_BIND_CONFLICT_COLOR
                        } else {
                            TEXT_COLOR
                        },
                    )
                } else {
                    (
                        String::from(self.localized_strings.get("hud.settings.unbound")),
                        ERROR_COLOR,
                    )
                };
            let loc_key = self
                .localized_strings
                .get(game_input.get_localization_key());
            let text_widget = Text::new(loc_key)
                .color(TEXT_COLOR)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(18));
            let button_widget = Button::new()
                .label(&key_string)
                .label_color(key_color)
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_font_size(self.fonts.cyri.scale(15))
                .w(150.0)
                .rgba(0.0, 0.0, 0.0, 0.0)
                .border_rgba(0.0, 0.0, 0.0, 255.0)
                .label_y(Relative::Scalar(3.0));
            // Place top-left if it's the first text, else under the previous one
            let text_widget = match previous_element_id {
                None => text_widget.top_left_with_margins_on(state.ids.window, 10.0, 5.0),
                Some(prev_id) => text_widget.down_from(prev_id, 10.0),
            };
            let text_width = text_widget.get_w(ui).unwrap_or(0.0);
            text_widget.set(text_id, ui);
            if button_widget
                .right_from(text_id, 350.0 - text_width)
                .set(button_id, ui)
                .was_clicked()
            {
                events.push(Event::ChangeBinding(game_input));
            }
            // Set the previous id to the current one for the next cycle
            previous_element_id = Some(text_id);
        }

        // Reset the KeyBindings settings to the default settings
        if let Some(prev_id) = previous_element_id {
            if Button::image(self.imgs.button)
                .w_h(RESET_BUTTONS_WIDTH, RESET_BUTTONS_HEIGHT)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .down_from(prev_id, 20.0)
                .label(&self.localized_strings.get("hud.settings.reset_keybinds"))
                .label_font_size(self.fonts.cyri.scale(14))
                .label_color(TEXT_COLOR)
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_y(Relative::Scalar(2.0))
                .set(state.ids.reset_controls_button, ui)
                .was_clicked()
            {
                events.push(Event::ResetKeyBindings);
            }
            previous_element_id = Some(state.ids.reset_controls_button)
        }

        // Add an empty text widget to simulate some bottom margin, because conrod sucks
        if let Some(prev_id) = previous_element_id {
            Rectangle::fill_with([1.0, 1.0], color::TRANSPARENT)
                .down_from(prev_id, 10.0)
                .set(state.ids.controls_alignment_rectangle, ui);
        }

        events
    }
}
