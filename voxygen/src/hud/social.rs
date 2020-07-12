use super::{img_ids::Imgs, Show, TEXT_COLOR, TEXT_COLOR_3, UI_MAIN};

use crate::{i18n::VoxygenLocalization, ui::fonts::ConrodVoxygenFonts};
use client::{self, Client};
use common::{
    comp::Stats,
    sync::{Uid, WorldSyncExt},
};
use conrod_core::{
    color,
    widget::{self, Button, Image, Rectangle, Scrollbar, Text},
    widget_ids, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};
use specs::WorldExt;
use std::time::Instant;

widget_ids! {
    pub struct Ids {
        social_frame,
        social_close,
        social_title,
        frame,
        align,
        content_align,
        online_tab,
        friends_tab,
        faction_tab,
        online_title,
        online_no,
        scrollbar,
        friends_test,
        faction_test,
        player_names[],
        group,
        group_invite,
        member_names[],
        accept_invite_button,
        reject_invite_button,
        invite_button,
        kick_button,
        assign_leader_button,
        leave_button,
    }
}

pub struct State {
    ids: Ids,
    // Holds the time when selection is made since this selection can be overriden
    // by selecting an entity in-game
    selected_uid: Option<(Uid, Instant)>,
    // Selected group member
    selected_member: Option<Uid>,
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
        selected_entity: Option<(specs::Entity, Instant)>,
    ) -> Self {
        Self {
            show,
            client,
            imgs,
            fonts,
            localized_strings,
            selected_entity,
            common: widget::CommonBuilder::default(),
        }
    }
}

pub enum Event {
    Close,
    ChangeSocialTab(SocialTab),
    Invite(Uid),
    Accept,
    Reject,
    Kick(Uid),
    LeaveGroup,
    AssignLeader(Uid),
}

impl<'a> Widget for Social<'a> {
    type Event = Vec<Event>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        Self::State {
            ids: Ids::new(id_gen),
            selected_uid: None,
            selected_member: None,
        }
    }

    #[allow(clippy::unused_unit)] // TODO: Pending review in #587
    fn style(&self) -> Self::Style { () }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;

        let mut events = Vec::new();

        Image::new(self.imgs.window_3)
            .top_left_with_margins_on(ui.window, 200.0, 25.0)
            .color(Some(UI_MAIN))
            .w_h(103.0 * 4.0, 122.0 * 4.0)
            .set(state.ids.social_frame, ui);

        // X-Button
        if Button::image(self.imgs.close_button)
            .w_h(28.0, 28.0)
            .hover_image(self.imgs.close_button_hover)
            .press_image(self.imgs.close_button_press)
            .top_right_with_margins_on(state.ids.social_frame, 0.0, 0.0)
            .set(state.ids.social_close, ui)
            .was_clicked()
        {
            events.push(Event::Close);
        }

        // Title
        Text::new(&self.localized_strings.get("hud.social"))
            .mid_top_with_margin_on(state.ids.social_frame, 6.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(14))
            .color(TEXT_COLOR)
            .set(state.ids.social_title, ui);

        // Alignment
        Rectangle::fill_with([99.0 * 4.0, 112.0 * 4.0], color::TRANSPARENT)
            .mid_top_with_margin_on(state.ids.social_frame, 8.0 * 4.0)
            .set(state.ids.align, ui);
        // Content Alignment
        Rectangle::fill_with([94.0 * 4.0, 94.0 * 4.0], color::TRANSPARENT)
            .middle_of(state.ids.frame)
            .scroll_kids()
            .scroll_kids_vertically()
            .set(state.ids.content_align, ui);
        Scrollbar::y_axis(state.ids.content_align)
            .thickness(5.0)
            .rgba(0.33, 0.33, 0.33, 1.0)
            .set(state.ids.scrollbar, ui);
        // Frame
        Image::new(self.imgs.social_frame)
            .w_h(99.0 * 4.0, 100.0 * 4.0)
            .mid_bottom_of(state.ids.align)
            .color(Some(UI_MAIN))
            .set(state.ids.frame, ui);

        // Online Tab

        if Button::image(if let SocialTab::Online = self.show.social_tab {
            self.imgs.social_button_pressed
        } else {
            self.imgs.social_button
        })
        .w_h(30.0 * 4.0, 12.0 * 4.0)
        .hover_image(if let SocialTab::Online = self.show.social_tab {
            self.imgs.social_button_pressed
        } else {
            self.imgs.social_button_hover
        })
        .press_image(if let SocialTab::Online = self.show.social_tab {
            self.imgs.social_button_pressed
        } else {
            self.imgs.social_button_press
        })
        .top_left_with_margins_on(state.ids.align, 4.0, 0.0)
        .label(&self.localized_strings.get("hud.social.online"))
        .label_font_size(self.fonts.cyri.scale(14))
        .label_font_id(self.fonts.cyri.conrod_id)
        .parent(state.ids.frame)
        .color(UI_MAIN)
        .label_color(TEXT_COLOR)
        .set(state.ids.online_tab, ui)
        .was_clicked()
        {
            events.push(Event::ChangeSocialTab(SocialTab::Online));
        }

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

            Text::new(&self.localized_strings.get("hud.group"))
                .down(10.0)
                .font_size(self.fonts.cyri.scale(20))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.group, ui);

            // Helper
            let uid_to_name_text = |uid, client: &Client| match client.player_list.get(&uid) {
                Some(player_info) => {
                    let alias = &player_info.player_alias;
                    let character_name_level = match &player_info.character {
                        Some(character) => format!("{} Lvl {}", &character.name, &character.level),
                        None => "<None>".to_string(), // character select or spectating
                    };
                    format!("[{}] {}", alias, character_name_level)
                },
                None => self
                    .client
                    .state()
                    .ecs()
                    .entity_from_uid(uid.0)
                    .and_then(|entity| {
                        self.client
                            .state()
                            .ecs()
                            .read_storage::<Stats>()
                            .get(entity)
                            .map(|stats| stats.name.clone())
                    })
                    .unwrap_or_else(|| format!("NPC Uid: {}", uid)),
            };

            // Accept/Reject Invite
            if let Some(invite_uid) = self.client.group_invite() {
                let name = uid_to_name_text(invite_uid, &self.client);
                let text = self
                    .localized_strings
                    .get("hud.group.invite_to_join")
                    .replace("{name}", &name);
                Text::new(&text)
                    .down(10.0)
                    .font_size(self.fonts.cyri.scale(15))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(TEXT_COLOR)
                    .set(state.ids.group_invite, ui);
                if Button::image(self.imgs.button)
                    .down(3.0)
                    .w_h(150.0, 30.0)
                    .hover_image(self.imgs.button_hover)
                    .press_image(self.imgs.button_press)
                    .label(&self.localized_strings.get("common.accept"))
                    .label_y(conrod_core::position::Relative::Scalar(3.0))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(15))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .set(state.ids.accept_invite_button, ui)
                    .was_clicked()
                {
                    events.push(Event::Accept);
                }
                if Button::image(self.imgs.button)
                    .down(3.0)
                    .w_h(150.0, 30.0)
                    .hover_image(self.imgs.button_hover)
                    .press_image(self.imgs.button_press)
                    .label(&self.localized_strings.get("common.reject"))
                    .label_y(conrod_core::position::Relative::Scalar(3.0))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(15))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .set(state.ids.reject_invite_button, ui)
                    .was_clicked()
                {
                    events.push(Event::Reject);
                }
            } else if self // Invite Button
                .client
                .group_leader()
                .map_or(true, |l_uid| self.client.uid() == Some(l_uid))
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

            // Show group members
            if let Some(leader) = self.client.group_leader() {
                let group_size = self.client.group_members.len() + 1;
                if state.ids.member_names.len() < group_size {
                    state.update(|s| {
                        s.ids
                            .member_names
                            .resize(group_size, &mut ui.widget_id_generator())
                    })
                }
                // List member names
                for (i, &uid) in self
                    .client
                    .uid()
                    .iter()
                    .chain(self.client.group_members.iter())
                    .enumerate()
                {
                    let selected = state.selected_member.map_or(false, |u| u == uid);
                    let text = uid_to_name_text(uid, &self.client);
                    let text = if selected {
                        format!("-> {}", &text)
                    } else {
                        text
                    };
                    let text = if uid == leader {
                        format!("{} (Leader)", &text)
                    } else {
                        text
                    };
                    Text::new(&text)
                        .down(3.0)
                        .font_size(self.fonts.cyri.scale(15))
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(TEXT_COLOR)
                        .set(state.ids.member_names[i], ui);
                    // Check for click
                    if ui
                        .widget_input(state.ids.member_names[i])
                        .clicks()
                        .left()
                        .next()
                        .is_some()
                    {
                        state.update(|s| {
                            s.selected_member = if selected { None } else { Some(uid) }
                        });
                    }
                }

                // Show more buttons if leader
                if self.client.uid() == Some(leader) {
                    let selected = state.selected_member;
                    // Kick
                    if Button::image(self.imgs.button)
                        .down(3.0)
                        .w_h(150.0, 30.0)
                        .hover_image(self.imgs.button_hover)
                        .press_image(self.imgs.button_press)
                        .label(&self.localized_strings.get("hud.group.kick"))
                        .label_y(conrod_core::position::Relative::Scalar(3.0))
                        .label_color(if selected.is_some() {
                            TEXT_COLOR
                        } else {
                            TEXT_COLOR_3
                        })
                        .label_font_size(self.fonts.cyri.scale(15))
                        .label_font_id(self.fonts.cyri.conrod_id)
                        .set(state.ids.kick_button, ui)
                        .was_clicked()
                    {
                        if let Some(uid) = selected {
                            events.push(Event::Kick(uid));
                            state.update(|s| {
                                s.selected_member = None;
                            });
                        }
                    }
                    // Assign leader
                    if Button::image(self.imgs.button)
                        .down(3.0)
                        .w_h(150.0, 30.0)
                        .hover_image(self.imgs.button_hover)
                        .press_image(self.imgs.button_press)
                        .label(&self.localized_strings.get("hud.group.assign_leader"))
                        .label_y(conrod_core::position::Relative::Scalar(3.0))
                        .label_color(if selected.is_some() {
                            TEXT_COLOR
                        } else {
                            TEXT_COLOR_3
                        })
                        .label_font_size(self.fonts.cyri.scale(15))
                        .label_font_id(self.fonts.cyri.conrod_id)
                        .set(state.ids.assign_leader_button, ui)
                        .was_clicked()
                    {
                        if let Some(uid) = selected {
                            events.push(Event::AssignLeader(uid));
                            state.update(|s| {
                                s.selected_member = None;
                            });
                        }
                    }
                }

                // Leave group button
                if Button::image(self.imgs.button)
                    .down(3.0)
                    .w_h(150.0, 30.0)
                    .hover_image(self.imgs.button_hover)
                    .press_image(self.imgs.button_press)
                    .label(&self.localized_strings.get("hud.group.leave"))
                    .label_y(conrod_core::position::Relative::Scalar(3.0))
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(15))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .set(state.ids.leave_button, ui)
                    .was_clicked()
                {
                    events.push(Event::LeaveGroup);
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
        }

        events
    }
}
