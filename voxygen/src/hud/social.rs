use super::{img_ids::Imgs, Show, TEXT_COLOR, TEXT_COLOR_3, UI_HIGHLIGHT_0, UI_MAIN};

use crate::{i18n::VoxygenLocalization, ui::fonts::ConrodVoxygenFonts};
use client::{self, Client};
use common::{comp::Stats, sync::Uid};
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
    stats: &'a Stats,

    selected_entity: Option<(specs::Entity, Instant)>,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> Social<'a> {
    pub fn new(
        show: &'a Show,
        client: &'a Client,
        imgs: &'a Imgs,
        fonts: &'a ConrodVoxygenFonts,
        localized_strings: &'a std::sync::Arc<VoxygenLocalization>,
        stats: &'a Stats,
        selected_entity: Option<(specs::Entity, Instant)>,
    ) -> Self {
        Self {
            show,
            client,
            imgs,
            fonts,
            localized_strings,
            stats,
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

        // Window frame and BG
        let pos = if self.show.group { 180.0 } else { 25.0 };
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
            .top_left_with_margins_on(ui.window, 200.0, pos)
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
        Text::new(&self.localized_strings.get("hud.social"))
            .mid_top_with_margin_on(state.ids.frame, 9.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(22))
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
        .top_right_with_margins_on(state.ids.frame, 50.0, -28.0)
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
            Rectangle::fill_with([133.0, 370.0], color::TRANSPARENT)
                .top_left_with_margins_on(state.ids.online_align, 0.0, 0.0)
                .crop_kids()
                .set(state.ids.names_align, ui);
            Rectangle::fill_with([39.0, 370.0], color::TRANSPARENT)
                .right_from(state.ids.names_align, 2.0)
                .crop_kids()
                .set(state.ids.levels_align, ui);
            Rectangle::fill_with([94.0, 370.0], color::TRANSPARENT)
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
            Text::new(&count.to_string())
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
                let selected = state.selected_uid.map_or(false, |u| u.0 == uid);
                let alias = &player_info.player_alias;
                let name = match &player_info.character {
                    Some(character) => format!("{} ", &character.name),
                    None => "<None>".to_string(), // character select or spectating
                };
                let level = match &player_info.character {
                    Some(character) => format!("{} ", &character.level),
                    None => "<None>".to_string(), // character select or spectating
                };
                let setting = true; // TODO Remove this
                let zone = "Wilderness"; // TODO: Add real zone
                let name_text = if name == self.stats.name {
                    format!("You ({})", name) // TODO: Locale
                } else if setting {
                    format!("{}", name)
                } else {
                    format!("[{}] {}", alias, name)
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
                    Text::new(&level).mid_top_with_margin_on(state.ids.levels_align, 2.0)
                } else {
                    Text::new(&level).down_from(state.ids.player_levels[i - 1], 2.0)
                };
                level_txt
                    .font_size(self.fonts.cyri.scale(14))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(TEXT_COLOR)
                    .set(state.ids.player_levels[i], ui);
                let zone_txt = if i == 0 {
                    Text::new(&zone).mid_top_with_margin_on(state.ids.zones_align, 2.0)
                } else {
                    Text::new(&zone).down_from(state.ids.player_zones[i - 1], 2.0)
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

        // Alignment
        /*
        // Online Tab

        // Contents

        if let SocialTab::Online = self.show.social_tab {
            // Players list
            // TODO: this list changes infrequently enough that it should not have to be
            // recreated every frame
            let players = self.client.player_list.iter().filter(|(_, p)| p.is_online);
            let count = players.clone().count();
            if state.ids.player_names.len() < count {
                state.update(|s| {
                    s.ids
                        .player_names
                        .resize(count, &mut ui.widget_id_generator())
                })
            }
            Text::new(
                &self
                    .localized_strings
                    .get("hud.social.play_online_fmt")
                    .replace("{nb_player}", &format!("{:?}", count)),
            )
            .top_left_with_margins_on(state.ids.content_align, -2.0, 7.0)
            .font_size(self.fonts.cyri.scale(14))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.online_title, ui);

            // Clear selected player if an entity was selected
            if state
                .selected_uid
                .zip(self.selected_entity)
                // Compare instants
                .map_or(false, |(u, e)| u.1 < e.1)
            {
                state.update(|s| s.selected_uid = None);
            }

            for (i, (&uid, player_info)) in players.enumerate() {
                let selected = state.selected_uid.map_or(false, |u| u.0 == uid);
                let alias = &player_info.player_alias;
                let character_name_level = match &player_info.character {
                    Some(character) => format!("{} Lvl {}", &character.name, &character.level),
                    None => "<None>".to_string(), // character select or spectating
                };
                let text = if selected {
                    format!("-> [{}] {}", alias, character_name_level)
                } else {
                    format!("[{}] {}", alias, character_name_level)
                };
                Text::new(&text)
                    .down(3.0)
                    .font_size(self.fonts.cyri.scale(15))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(TEXT_COLOR)
                    .set(state.ids.player_names[i], ui);
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
            if self
                .client
                .group_info()
                .map_or(true, |(_, l_uid)| self.client.uid() == Some(l_uid))
            {
                let selected = state.selected_uid.map(|s| s.0).or_else(|| {
                    self.selected_entity
                        .and_then(|s| self.client.state().read_component_copied(s.0))
                });

                if Button::image(self.imgs.button)
                    .down(3.0)
                    .w_h(150.0, 30.0)
                    .hover_image(self.imgs.button_hover)
                    .press_image(self.imgs.button_press)
                    .label(&self.localized_strings.get("hud.group.invite"))
                    .label_y(conrod_core::position::Relative::Scalar(3.0))
                    .label_color(if selected.is_some() {
                        TEXT_COLOR
                    } else {
                        TEXT_COLOR_3
                    })
                    .label_font_size(self.fonts.cyri.scale(15))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .set(state.ids.invite_button, ui)
                    .was_clicked()
                {
                    if let Some(uid) = selected {
                        events.push(Event::Invite(uid));
                        state.update(|s| {
                            s.selected_uid = None;
                        });
                    }
                }
            }
        }

        // Friends Tab

        if Button::image(if let SocialTab::Friends = self.show.social_tab {
            self.imgs.social_button_pressed
        } else {
            self.imgs.social_button
        })
        .w_h(30.0 * 4.0, 12.0 * 4.0)
        .hover_image(if let SocialTab::Friends = self.show.social_tab {
            self.imgs.social_button_pressed
        } else {
            self.imgs.social_button
        })
        .press_image(if let SocialTab::Friends = self.show.social_tab {
            self.imgs.social_button_pressed
        } else {
            self.imgs.social_button
        })
        .right_from(state.ids.online_tab, 0.0)
        .label(&self.localized_strings.get("hud.social.friends"))
        .label_font_size(self.fonts.cyri.scale(14))
        .label_font_id(self.fonts.cyri.conrod_id)
        .parent(state.ids.frame)
        .color(UI_MAIN)
        .label_color(TEXT_COLOR_3)
        .set(state.ids.friends_tab, ui)
        .was_clicked()
        {
            events.push(Event::ChangeSocialTab(SocialTab::Friends));
        }

        // Contents

        if let SocialTab::Friends = self.show.social_tab {
            Text::new(&self.localized_strings.get("hud.social.not_yet_available"))
                .middle_of(state.ids.content_align)
                .font_size(self.fonts.cyri.scale(18))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR_3)
                .set(state.ids.friends_test, ui);
        }

        // Faction Tab
        let button_img = if let SocialTab::Faction = self.show.social_tab {
            self.imgs.social_button_pressed
        } else {
            self.imgs.social_button
        };
        if Button::image(button_img)
            .w_h(30.0 * 4.0, 12.0 * 4.0)
            .right_from(state.ids.friends_tab, 0.0)
            .label(&self.localized_strings.get("hud.social.faction"))
            .parent(state.ids.frame)
            .label_font_size(self.fonts.cyri.scale(14))
            .label_font_id(self.fonts.cyri.conrod_id)
            .color(UI_MAIN)
            .label_color(TEXT_COLOR_3)
            .set(state.ids.faction_tab, ui)
            .was_clicked()
        {
            events.push(Event::ChangeSocialTab(SocialTab::Faction));
        }

        // Contents

        if let SocialTab::Faction = self.show.social_tab {
            Text::new(&self.localized_strings.get("hud.social.not_yet_available"))
                .middle_of(state.ids.content_align)
                .font_size(self.fonts.cyri.scale(18))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR_3)
                .set(state.ids.faction_test, ui);
        }*/

        events
    }
}
