use super::{
    img_ids::{Imgs, ImgsRot},
    Show, TEXT_COLOR, TEXT_COLOR_3, UI_HIGHLIGHT_0, UI_MAIN,
};

use crate::{
    i18n::VoxygenLocalization,
    ui::{fonts::ConrodVoxygenFonts, ImageFrame, Tooltip, TooltipManager, Tooltipable},
};
use client::{self, Client};
use common::sync::Uid;
use conrod_core::{
    color,
    widget::{self, Button, Image, Rectangle, Scrollbar, Text},
    widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};
use std::time::Instant;

widget_ids! {
    pub struct Ids {
        frame,
        close,
        title_align,
        title,
        bg,
        icon,
        scrollbar,
        online_align,
        online_tab,
        online_tab_icon,
        names_align,
        name_txt,
        player_levels[],
        player_names[],
        player_zones[],
        online_txt,
        online_no,
        levels_align,
        level_txt,
        zones_align,
        zone_txt,
        friends_tab,
        //friends_tab_icon,
        faction_tab,
        //faction_tab_icon,
        friends_test,
        faction_test,
        invite_button,
    }
}

pub struct State {
    ids: Ids,
    // Holds the time when selection is made since this selection can be overriden
    // by selecting an entity in-game
    selected_uid: Option<(Uid, Instant)>,
}

pub enum SocialTab {
    Online,
    Friends,
    Faction,
}

#[derive(WidgetCommon)]
pub struct Social<'a> {
    show: &'a Show,
    client: &'a Client,
    imgs: &'a Imgs,
    fonts: &'a ConrodVoxygenFonts,
    localized_strings: &'a std::sync::Arc<VoxygenLocalization>,
    selected_entity: Option<(specs::Entity, Instant)>,
    rot_imgs: &'a ImgsRot,
    tooltip_manager: &'a mut TooltipManager,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> Social<'a> {
    #[allow(clippy::too_many_arguments)] // TODO: Pending review in #587
    pub fn new(
        show: &'a Show,
        client: &'a Client,
        imgs: &'a Imgs,
        fonts: &'a ConrodVoxygenFonts,
        localized_strings: &'a std::sync::Arc<VoxygenLocalization>,
        selected_entity: Option<(specs::Entity, Instant)>,
        rot_imgs: &'a ImgsRot,
        tooltip_manager: &'a mut TooltipManager,
    ) -> Self {
        Self {
            show,
            client,
            imgs,
            rot_imgs,
            fonts,
            localized_strings,
            tooltip_manager,
            selected_entity,
            common: widget::CommonBuilder::default(),
        }
    }
}

pub enum Event {
    Close,
    Invite(Uid),
    ChangeSocialTab(SocialTab),
}

impl<'a> Widget for Social<'a> {
    type Event = Vec<Event>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        Self::State {
            ids: Ids::new(id_gen),
            selected_uid: None,
        }
    }

    #[allow(clippy::unused_unit)] // TODO: Pending review in #587
    fn style(&self) -> Self::Style { () }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;

        let mut events = Vec::new();
        let button_tooltip = Tooltip::new({
            // Edge images [t, b, r, l]
            // Corner images [tr, tl, br, bl]
            let edge = &self.rot_imgs.tt_side;
            let corner = &self.rot_imgs.tt_corner;
            ImageFrame::new(
                [edge.cw180, edge.none, edge.cw270, edge.cw90],
                [corner.none, corner.cw270, corner.cw90, corner.cw180],
                Color::Rgba(0.08, 0.07, 0.04, 1.0),
                5.0,
            )
        })
        .title_font_size(self.fonts.cyri.scale(15))
        .parent(ui.window)
        .desc_font_size(self.fonts.cyri.scale(12))
        .title_text_color(TEXT_COLOR)
        .font_id(self.fonts.cyri.conrod_id)
        .desc_text_color(TEXT_COLOR);

        // Window frame and BG
        let pos = if self.show.group || self.show.group_menu {
            200.0
        } else {
            25.0
        };
        // TODO: Different window visuals depending on the selected tab
        let window_bg = match &self.show.social_tab {
            SocialTab::Online => self.imgs.social_bg_on,
            SocialTab::Friends => self.imgs.social_bg_friends,
            SocialTab::Faction => self.imgs.social_bg_fact,
        };
        let window_frame = match &self.show.social_tab {
            SocialTab::Online => self.imgs.social_frame_on,
            SocialTab::Friends => self.imgs.social_frame_friends,
            SocialTab::Faction => self.imgs.social_frame_fact,
        };
        Image::new(window_bg)
            .bottom_left_with_margins_on(ui.window, 308.0, pos)
            .color(Some(UI_MAIN))
            .w_h(280.0, 460.0)
            .set(state.ids.bg, ui);
        Image::new(window_frame)
            .middle_of(state.ids.bg)
            .color(Some(UI_HIGHLIGHT_0))
            .w_h(280.0, 460.0)
            .set(state.ids.frame, ui);
        // Icon
        Image::new(self.imgs.social)
            .w_h(30.0, 30.0)
            .top_left_with_margins_on(state.ids.frame, 6.0, 6.0)
            .set(state.ids.icon, ui);
        // X-Button
        if Button::image(self.imgs.close_button)
            .w_h(24.0, 25.0)
            .hover_image(self.imgs.close_button_hover)
            .press_image(self.imgs.close_button_press)
            .top_right_with_margins_on(state.ids.frame, 0.0, 0.0)
            .set(state.ids.close, ui)
            .was_clicked()
        {
            events.push(Event::Close);
        }

        // Title
        Rectangle::fill_with([212.0, 42.0], color::TRANSPARENT)
            .top_left_with_margins_on(state.ids.frame, 2.0, 44.0)
            .set(state.ids.title_align, ui);
        Text::new(&self.localized_strings.get("hud.social"))
            .middle_of(state.ids.title_align)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(20))
            .color(TEXT_COLOR)
            .set(state.ids.title, ui);

        // Tabs Buttons
        // Online Tab Button
        if Button::image(match &self.show.social_tab {
            SocialTab::Online => self.imgs.social_tab_act,
            _ => self.imgs.social_tab_inact,
        })
        .w_h(30.0, 44.0)
        .hover_image(match &self.show.social_tab {
            SocialTab::Online => self.imgs.social_tab_act,
            _ => self.imgs.social_tab_inact_hover,
        })
        .press_image(match &self.show.social_tab {
            SocialTab::Online => self.imgs.social_tab_act,
            _ => self.imgs.social_tab_inact_press,
        })
        .image_color(match &self.show.social_tab {
            SocialTab::Online => UI_MAIN,
            _ => Color::Rgba(1.0, 1.0, 1.0, 0.6),
        })
        .top_right_with_margins_on(state.ids.frame, 50.0, -27.0)
        .set(state.ids.online_tab, ui)
        .was_clicked()
        {
            events.push(Event::ChangeSocialTab(SocialTab::Online));
        }
        Image::new(self.imgs.chat_online_small)
            .w_h(20.0, 20.0)
            .top_right_with_margins_on(state.ids.online_tab, 12.0, 7.0)
            .color(match &self.show.social_tab {
                SocialTab::Online => Some(TEXT_COLOR),
                _ => Some(UI_MAIN),
            })
            .set(state.ids.online_tab_icon, ui);
        // Friends Tab Button
        if Button::image(match &self.show.social_tab {
            SocialTab::Friends => self.imgs.social_tab_act,
            _ => self.imgs.social_tab_inact,
        })
        .w_h(30.0, 44.0)
        .hover_image(match &self.show.social_tab {
            SocialTab::Friends => self.imgs.social_tab_act,
            _ => self.imgs.social_tab_inact_hover,
        })
        .press_image(match &self.show.social_tab {
            SocialTab::Friends => self.imgs.social_tab_act,
            _ => self.imgs.social_tab_inact_press,
        })
        .down_from(state.ids.online_tab, 0.0)
        .image_color(match &self.show.social_tab {
            SocialTab::Friends => UI_MAIN,
            _ => Color::Rgba(1.0, 1.0, 1.0, 0.6),
        })
        .set(state.ids.friends_tab, ui)
        .was_clicked()
        {
            events.push(Event::ChangeSocialTab(SocialTab::Friends));
        }
        // Faction Tab Button
        if Button::image(match &self.show.social_tab {
            SocialTab::Friends => self.imgs.social_tab_act,
            _ => self.imgs.social_tab_inact,
        })
        .w_h(30.0, 44.0)
        .hover_image(match &self.show.social_tab {
            SocialTab::Faction => self.imgs.social_tab_act,
            _ => self.imgs.social_tab_inact_hover,
        })
        .press_image(match &self.show.social_tab {
            SocialTab::Faction => self.imgs.social_tab_act,
            _ => self.imgs.social_tab_inact_press,
        })
        .down_from(state.ids.friends_tab, 0.0)
        .image_color(match &self.show.social_tab {
            SocialTab::Faction => UI_MAIN,
            _ => Color::Rgba(1.0, 1.0, 1.0, 0.6),
        })
        .set(state.ids.faction_tab, ui)
        .was_clicked()
        {
            events.push(Event::ChangeSocialTab(SocialTab::Faction));
        }
        // Online Tab
        if let SocialTab::Online = self.show.social_tab {
            // Content Alignments
            Rectangle::fill_with([270.0, 346.0], color::TRANSPARENT)
                .mid_top_with_margin_on(state.ids.frame, 74.0)
                .scroll_kids_vertically()
                .set(state.ids.online_align, ui);
            Rectangle::fill_with([133.0, 346.0], color::TRANSPARENT)
                .top_left_with_margins_on(state.ids.online_align, 0.0, 0.0)
                .crop_kids()
                .set(state.ids.names_align, ui);
            Rectangle::fill_with([39.0, 346.0], color::TRANSPARENT)
                .right_from(state.ids.names_align, 2.0)
                .crop_kids()
                .set(state.ids.levels_align, ui);
            Rectangle::fill_with([94.0, 346.0], color::TRANSPARENT)
                .right_from(state.ids.levels_align, 2.0)
                .crop_kids()
                .set(state.ids.zones_align, ui);
            Scrollbar::y_axis(state.ids.online_align)
                .thickness(4.0)
                .color(UI_HIGHLIGHT_0)
                .set(state.ids.scrollbar, ui);
            //
            // Headlines
            //
            if Button::image(self.imgs.nothing)
                .w_h(133.0, 18.0)
                .top_left_with_margins_on(state.ids.frame, 52.0, 7.0)
                .label(&self.localized_strings.get("hud.social.name"))
                .label_font_size(self.fonts.cyri.scale(14))
                .label_y(conrod_core::position::Relative::Scalar(0.0))
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_color(TEXT_COLOR)
                .set(state.ids.name_txt, ui)
                .was_clicked()
            {
                // Sort widgets by name alphabetically
            }
            if Button::image(self.imgs.nothing)
                .w_h(39.0, 18.0)
                .right_from(state.ids.name_txt, 2.0)
                .label(&self.localized_strings.get("hud.social.level"))
                .label_font_size(self.fonts.cyri.scale(14))
                .label_y(conrod_core::position::Relative::Scalar(0.0))
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_color(TEXT_COLOR)
                .set(state.ids.level_txt, ui)
                .was_clicked()
            {
                // Sort widgets by level (increasing)
            }
            if Button::image(self.imgs.nothing)
                .w_h(93.0, 18.0)
                .right_from(state.ids.level_txt, 2.0)
                .label(&self.localized_strings.get("hud.social.zone"))
                .label_font_size(self.fonts.cyri.scale(14))
                .label_y(conrod_core::position::Relative::Scalar(0.0))
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_color(TEXT_COLOR)
                .set(state.ids.zone_txt, ui)
                .was_clicked()
            {
                // Sort widgets by zone alphabetically
            }
            // Online Text
            let players = self.client.player_list.iter().filter(|(_, p)| p.is_online);
            let count = players.clone().count();
            Text::new(&self.localized_strings.get("hud.social.online"))
                .bottom_left_with_margins_on(state.ids.frame, 18.0, 10.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .color(TEXT_COLOR)
                .set(state.ids.online_txt, ui);
            Text::new(&(count - 1).to_string())
                .right_from(state.ids.online_txt, 5.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .color(TEXT_COLOR)
                .set(state.ids.online_no, ui);
            // Adjust widget_id struct vec length to player count
            if state.ids.player_levels.len() < count {
                state.update(|s| {
                    s.ids
                        .player_levels
                        .resize(count, &mut ui.widget_id_generator())
                })
            };
            if state.ids.player_names.len() < count {
                state.update(|s| {
                    s.ids
                        .player_names
                        .resize(count, &mut ui.widget_id_generator())
                })
            };
            if state.ids.player_zones.len() < count {
                state.update(|s| {
                    s.ids
                        .player_zones
                        .resize(count, &mut ui.widget_id_generator())
                })
            };
            // Create a name, level and zone row for every player in the list
            // Filter out yourself from the online list
            let my_uid = self.client.uid();
            for (i, (&uid, player_info)) in
                players.filter(|(uid, _)| Some(**uid) != my_uid).enumerate()
            {
                let hide_username = true;
                let zone = "Wilderness"; // TODO Add real zone
                let selected = state.selected_uid.map_or(false, |u| u.0 == uid);
                let alias = &player_info.player_alias;
                let name_text = match &player_info.character {
                    Some(character) => {
                        if Some(uid) == my_uid {
                            format!(
                                "{} ({})",
                                &self.localized_strings.get("hud.common.you"),
                                &character.name
                            )
                        } else if hide_username {
                            character.name.clone()
                        } else {
                            format!("[{}] {}", alias, &character.name)
                        }
                    },
                    None => alias.clone(), // character select or spectating
                };
                let level = match &player_info.character {
                    Some(character) => format!("{} ", &character.level),
                    None => "".to_string(), // character select or spectating
                };
                let zone_name = match &player_info.character {
                    None => self.localized_strings.get("hud.group.in_menu").to_string(), /* character select or spectating */
                    _ => format!("{} ", &zone),
                };
                // Player name widgets
                let button = Button::image(if !selected {
                    self.imgs.nothing
                } else {
                    self.imgs.selection
                });
                let button = if i == 0 {
                    button.mid_top_with_margin_on(state.ids.names_align, 1.0)
                } else {
                    button.down_from(state.ids.player_names[i - 1], 1.0)
                };
                if button
                    .w_h(133.0, 20.0)
                    .hover_image(if selected {
                        self.imgs.selection
                    } else {
                        self.imgs.selection_hover
                    })
                    .press_image(if selected {
                        self.imgs.selection
                    } else {
                        self.imgs.selection_press
                    })
                    .label(&name_text)
                    .label_font_size(self.fonts.cyri.scale(14))
                    .label_y(conrod_core::position::Relative::Scalar(0.0))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .label_color(TEXT_COLOR)
                    .set(state.ids.player_names[i], ui)
                    .was_clicked()
                {};
                let level_txt = if i == 0 {
                    Text::new(&level).mid_top_with_margin_on(state.ids.levels_align, 4.0)
                } else {
                    Text::new(&level).down_from(state.ids.player_levels[i - 1], 4.0)
                };
                level_txt
                    .font_size(self.fonts.cyri.scale(14))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(TEXT_COLOR)
                    .set(state.ids.player_levels[i], ui);
                let zone_txt = if i == 0 {
                    Text::new(&zone_name).mid_top_with_margin_on(state.ids.zones_align, 4.0)
                } else {
                    Text::new(&zone_name).down_from(state.ids.player_zones[i - 1], 4.0)
                };
                zone_txt
                    .font_size(self.fonts.cyri.scale(14))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(TEXT_COLOR)
                    .set(state.ids.player_zones[i], ui);

                // Check for click
                if ui
                    .widget_input(state.ids.player_names[i])
                    .clicks()
                    .left()
                    .next()
                    .is_some()
                {
                    state.update(|s| s.selected_uid = Some((uid, Instant::now())));
                }
            }

            // Invite Button
            let selected_ingame = state
                .selected_uid
                .as_ref()
                .map(|(s, _)| *s)
                .filter(|selected| {
                    self.client
                        .player_list
                        .get(selected)
                        .map_or(false, |selected_player| {
                            selected_player.is_online && selected_player.character.is_some()
                        })
                })
                .or_else(|| {
                    self.selected_entity
                        .and_then(|s| self.client.state().read_component_copied(s.0))
                });
            // TODO: Prevent inviting players with the same group uid
            // TODO: Show current amount of group members as a tooltip for the invite button
            // if the player is the group leader TODO: Grey out the invite
            // button if the group has 6/6 members
            let current_members = 4;
            let tooltip_txt = if selected_ingame.is_some() {
                format!(
                    "{}/6 {}",
                    &current_members,
                    &self.localized_strings.get("hud.group.members")
                )
            } else {
                (&self.localized_strings.get("hud.group.members")).to_string()
            };
            if Button::image(self.imgs.button)
                .w_h(106.0, 26.0)
                .bottom_right_with_margins_on(state.ids.frame, 9.0, 7.0)
                .hover_image(if selected_ingame.is_some() {
                    self.imgs.button_hover
                } else {
                    self.imgs.button
                })
                .press_image(if selected_ingame.is_some() {
                    self.imgs.button_press
                } else {
                    self.imgs.button
                })
                .label(&self.localized_strings.get("hud.group.invite"))
                .label_y(conrod_core::position::Relative::Scalar(3.0))
                .label_color(if selected_ingame.is_some() {
                    TEXT_COLOR
                } else {
                    TEXT_COLOR_3
                })
                .image_color(if selected_ingame.is_some() {
                    TEXT_COLOR
                } else {
                    TEXT_COLOR_3
                })
                .label_font_size(self.fonts.cyri.scale(15))
                .label_font_id(self.fonts.cyri.conrod_id)
                .with_tooltip(self.tooltip_manager, &tooltip_txt, "", &button_tooltip)
                .set(state.ids.invite_button, ui)
                .was_clicked()
            {
                if let Some(uid) = selected_ingame {
                    events.push(Event::Invite(uid));
                    state.update(|s| {
                        s.selected_uid = None;
                    });
                }
            }
        } // End of Online Tab       

        events
    }
}
