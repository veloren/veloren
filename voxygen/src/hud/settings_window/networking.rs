use crate::{
    hud::{img_ids::Imgs, MENU_BG, TEXT_COLOR},
    session::settings_change::{Networking as NetworkingChange, Networking::*},
    ui::{fonts::Fonts, ImageSlider, ToggleButton},
    GlobalState,
};
use conrod_core::{
    color,
    widget::{self, DropDownList, Rectangle, Text},
    widget_ids, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};
use i18n::Localization;

widget_ids! {
    struct Ids {
        window,
        window_r,
        terrain_vd_text,
        terrain_vd_slider,
        terrain_vd_value,
        entity_vd_text,
        entity_vd_slider,
        entity_vd_value,
        player_physics_behavior_text,
        player_physics_behavior_list,
        lossy_terrain_compression_button,
        lossy_terrain_compression_label,
        third_party_integrations_title,
        enable_discord_integration_text,
        enable_discord_integration_button
    }
}

#[derive(WidgetCommon)]
pub struct Networking<'a> {
    global_state: &'a GlobalState,
    imgs: &'a Imgs,
    fonts: &'a Fonts,
    localized_strings: &'a Localization,
    server_view_distance_limit: Option<u32>,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}
impl<'a> Networking<'a> {
    pub fn new(
        global_state: &'a GlobalState,
        imgs: &'a Imgs,
        fonts: &'a Fonts,
        localized_strings: &'a Localization,
        server_view_distance_limit: Option<u32>,
    ) -> Self {
        Self {
            global_state,
            imgs,
            fonts,
            localized_strings,
            server_view_distance_limit,
            common: widget::CommonBuilder::default(),
        }
    }
}

pub struct State {
    ids: Ids,
}

impl<'a> Widget for Networking<'a> {
    type Event = Vec<NetworkingChange>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        common_base::prof_span!("Networking::update");
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

        // View Distance
        Text::new(&self.localized_strings.get_msg("hud-settings-view_distance"))
            .top_left_with_margins_on(state.ids.window, 10.0, 10.0)
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
            events.push(NetworkingChange::AdjustTerrainViewDistance(new_val));
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
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-entity_view_distance"),
        )
        .down_from(state.ids.terrain_vd_slider, 10.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.entity_vd_text, ui);

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
        .down_from(state.ids.entity_vd_text, 8.0)
        .track_breadth(12.0)
        .slider_length(10.0)
        .soft_max(soft_entity_vd_max)
        .pad_track((5.0, 5.0))
        .set(state.ids.entity_vd_slider, ui)
        {
            events.push(NetworkingChange::AdjustEntityViewDistance(new_val));
        }

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

        // Player physics behavior
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-player_physics_behavior"),
        )
        .down_from(state.ids.entity_vd_slider, 8.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.player_physics_behavior_text, ui);

        let player_physics_selected = self
            .global_state
            .settings
            .networking
            .player_physics_behavior as usize;

        if let Some(clicked) = DropDownList::new(
            &["Client-authoritative", "Server-authoritative"],
            Some(player_physics_selected),
        )
        .w_h(200.0, 30.0)
        .color(MENU_BG)
        .label_color(TEXT_COLOR)
        .label_font_id(self.fonts.cyri.conrod_id)
        .down_from(state.ids.player_physics_behavior_text, 8.0)
        .set(state.ids.player_physics_behavior_list, ui)
        {
            match clicked {
                0 => events.push(ChangePlayerPhysicsBehavior {
                    server_authoritative: false,
                }),
                _ => events.push(ChangePlayerPhysicsBehavior {
                    server_authoritative: true,
                }),
            }
        }

        // Lossy terrain compression
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-lossy_terrain_compression"),
        )
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .right_from(state.ids.player_physics_behavior_text, 64.0)
        .color(TEXT_COLOR)
        .set(state.ids.lossy_terrain_compression_label, ui);

        let lossy_terrain_compression = ToggleButton::new(
            self.global_state
                .settings
                .networking
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
            .networking
            .lossy_terrain_compression
            != lossy_terrain_compression
        {
            events.push(NetworkingChange::ToggleLossyTerrainCompression(
                lossy_terrain_compression,
            ));
        }

        #[cfg(feature = "discord")]
        {
            // Third party integrations
            Text::new(
                &self
                    .localized_strings
                    .get_msg("hud-settings-third_party_integrations"),
            )
            .down_from(state.ids.player_physics_behavior_list, 16.0)
            .font_size(self.fonts.cyri.scale(18))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.third_party_integrations_title, ui);

            // Toggle Discord integration
            let enable_discord_integration = ToggleButton::new(
                self.global_state
                    .settings
                    .networking
                    .enable_discord_integration,
                self.imgs.checkbox,
                self.imgs.checkbox_checked,
            )
            .w_h(18.0, 18.0)
            .down_from(state.ids.third_party_integrations_title, 8.0)
            .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
            .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
            .set(state.ids.enable_discord_integration_button, ui);

            if self
                .global_state
                .settings
                .networking
                .enable_discord_integration
                != enable_discord_integration
            {
                events.push(ToggleDiscordIntegration(
                    !self
                        .global_state
                        .settings
                        .networking
                        .enable_discord_integration,
                ));
            }

            Text::new(
                &self
                    .localized_strings
                    .get_msg("hud-settings-enable_discord_integration"),
            )
            .right_from(state.ids.enable_discord_integration_button, 10.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .graphics_for(state.ids.enable_discord_integration_button)
            .color(TEXT_COLOR)
            .set(state.ids.enable_discord_integration_text, ui);
        }

        events
    }
}
