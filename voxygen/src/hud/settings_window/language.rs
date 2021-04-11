use super::Event;

use crate::{
    hud::{img_ids::Imgs, TEXT_COLOR},
    i18n::list_localizations,
    ui::fonts::Fonts,
    GlobalState,
};
use conrod_core::{
    color,
    widget::{self, Button, Rectangle, Scrollbar},
    widget_ids, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
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
pub struct Language<'a> {
    global_state: &'a GlobalState,
    imgs: &'a Imgs,
    fonts: &'a Fonts,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}
impl<'a> Language<'a> {
    pub fn new(global_state: &'a GlobalState, imgs: &'a Imgs, fonts: &'a Fonts) -> Self {
        Self {
            global_state,
            imgs,
            fonts,
            common: widget::CommonBuilder::default(),
        }
    }
}

pub struct State {
    ids: Ids,
}

impl<'a> Widget for Language<'a> {
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

        // List available languages
        let selected_language = &self.global_state.settings.language.selected_language;
        let language_list = list_localizations();
        if state.ids.language_list.len() < language_list.len() {
            state.update(|state| {
                state
                    .ids
                    .language_list
                    .resize(language_list.len(), &mut ui.widget_id_generator())
            });
        };
        for (i, language) in language_list.iter().enumerate() {
            let button_w = 400.0;
            let button_h = 50.0;
            let button = Button::image(if selected_language == &language.language_identifier {
                self.imgs.selection
            } else {
                self.imgs.nothing
            });
            let button = if i == 0 {
                button.mid_top_with_margin_on(state.ids.window, 20.0)
            } else {
                button.mid_bottom_with_margin_on(state.ids.language_list[i - 1], -button_h)
            };
            if button
                .label(&language.language_name)
                .w_h(button_w, button_h)
                .hover_image(self.imgs.selection_hover)
                .press_image(self.imgs.selection_press)
                .label_color(TEXT_COLOR)
                .label_font_size(self.fonts.cyri.scale(22))
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_y(conrod_core::position::Relative::Scalar(2.0))
                .set(state.ids.language_list[i], ui)
                .was_clicked()
            {
                events.push(Event::ChangeLanguage(Box::new(language.to_owned())));
            }
        }

        events
    }
}
