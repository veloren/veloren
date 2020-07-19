use super::{img_ids::Imgs, Show, TEXT_COLOR, TEXT_COLOR_3, TEXT_COLOR_GREY, UI_MAIN};

use crate::{
    i18n::VoxygenLocalization, settings::Settings, ui::fonts::ConrodVoxygenFonts, window::GameInput,
};
use client::{self, Client};
use common::{
    comp::{group::Role, Stats},
    sync::{Uid, WorldSyncExt},
};
use conrod_core::{
    color,
    position::{Place, Relative},
    widget::{self, Button, Image, Rectangle, Scrollbar, Text},
    widget_ids, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};
use specs::WorldExt;
use std::time::Instant;

widget_ids! {
    pub struct Ids {
        bg,
        title,
        close,
        btn_bg,
        btn_friend,
        btn_leader,
        btn_link,
        btn_kick,
        btn_leave,
        scroll_area,
        scrollbar,
        members[],
        invite_bubble,
        bubble_frame,
        btn_accept,
        btn_decline,
    }
}

pub struct State {
    ids: Ids,
    // Selected group member
    selected_member: Option<Uid>,
}

#[derive(WidgetCommon)]
pub struct Group<'a> {
    show: &'a mut Show,
    client: &'a Client,
    settings: &'a Settings,
    imgs: &'a Imgs,
    fonts: &'a ConrodVoxygenFonts,
    localized_strings: &'a std::sync::Arc<VoxygenLocalization>,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> Group<'a> {
    pub fn new(
        show: &'a mut Show,
        client: &'a Client,
        settings: &'a Settings,
        imgs: &'a Imgs,
        fonts: &'a ConrodVoxygenFonts,
        localized_strings: &'a std::sync::Arc<VoxygenLocalization>,
    ) -> Self {
        Self {
            show,
            client,
            settings,
            imgs,
            fonts,
            localized_strings,
            common: widget::CommonBuilder::default(),
        }
    }
}

pub enum Event {
    Close,
    Accept,
    Decline,
    Kick(Uid),
    LeaveGroup,
    AssignLeader(Uid),
}

impl<'a> Widget for Group<'a> {
    type Event = Vec<Event>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        Self::State {
            ids: Ids::new(id_gen),
            selected_member: None,
        }
    }

    #[allow(clippy::unused_unit)] // TODO: Pending review in #587
    fn style(&self) -> Self::Style { () }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;

        let mut events = Vec::new();

        // Don't show pets
        let group_members = self
            .client
            .group_members()
            .iter()
            .filter_map(|(u, r)| match r {
                Role::Member => Some(u),
                Role::Pet => None,
            })
            .collect::<Vec<_>>();

        // Not considered in group for ui purposes if it is just pets
        let in_group = !group_members.is_empty();

        // Helper
        let uid_to_name_text = |uid, client: &Client| match client.player_list.get(&uid) {
            Some(player_info) => player_info
                .character
                .as_ref()
                .map_or_else(|| format!("Player<{}>", uid), |c| c.name.clone()),
            None => client
                .state()
                .ecs()
                .entity_from_uid(uid.0)
                .and_then(|entity| {
                    client
                        .state()
                        .ecs()
                        .read_storage::<Stats>()
                        .get(entity)
                        .map(|stats| stats.name.clone())
                })
                .unwrap_or_else(|| format!("Npc<{}>", uid)),
        };

        let open_invite = self.client.group_invite();

        let my_uid = self.client.uid();

        // TODO show something to the player when they click on the group button while
        // they are not in a group so that it doesn't look like the button is
        // broken

        if in_group || open_invite.is_some() {
            // Frame
            Rectangle::fill_with([220.0, 230.0], color::Color::Rgba(0.0, 0.0, 0.0, 0.8))
                .bottom_left_with_margins_on(ui.window, 220.0, 10.0)
                .set(state.ids.bg, ui);
            if open_invite.is_some() {
                // yellow animated border
            }
        }

        // Buttons
        if let Some((group_name, leader)) = self.client.group_info().filter(|_| in_group) {
            let selected = state.selected_member;
            Text::new(&group_name)
                .mid_top_with_margin_on(state.ids.bg, 2.0)
                .font_size(20)
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.title, ui);
            if Button::image(self.imgs.button)
                .w_h(90.0, 22.0)
                .top_right_with_margins_on(state.ids.bg, 30.0, 5.0)
                .hover_image(self.imgs.button)
                .press_image(self.imgs.button)
                .label("Add to Friends")
                .label_color(TEXT_COLOR_GREY) // Change this when the friendslist is working
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_font_size(self.fonts.cyri.scale(10))
                .set(state.ids.btn_friend, ui)
                .was_clicked()
            {};
            if Button::image(self.imgs.button)
                .w_h(90.0, 22.0)
                .bottom_right_with_margins_on(state.ids.bg, 5.0, 5.0)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .label(&self.localized_strings.get("hud.group.leave"))
                .label_color(TEXT_COLOR)
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_font_size(self.fonts.cyri.scale(10))
                .set(state.ids.btn_leave, ui)
                .was_clicked()
            {
                events.push(Event::LeaveGroup);
            };
            // Group leader functions
            if my_uid == Some(leader) {
                if Button::image(self.imgs.button)
                    .w_h(90.0, 22.0)
                    .mid_bottom_with_margin_on(state.ids.btn_friend, -27.0)
                    .hover_image(self.imgs.button_hover)
                    .press_image(self.imgs.button_press)
                    .label(&self.localized_strings.get("hud.group.assign_leader"))
                    .label_color(if state.selected_member.is_some() {
                        TEXT_COLOR
                    } else {
                        TEXT_COLOR_GREY
                    })
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .label_font_size(self.fonts.cyri.scale(10))
                    .set(state.ids.btn_leader, ui)
                    .was_clicked()
                {
                    if let Some(uid) = selected {
                        events.push(Event::AssignLeader(uid));
                        state.update(|s| {
                            s.selected_member = None;
                        });
                    }
                };
                if Button::image(self.imgs.button)
                    .w_h(90.0, 22.0)
                    .mid_bottom_with_margin_on(state.ids.btn_leader, -27.0)
                    .hover_image(self.imgs.button)
                    .press_image(self.imgs.button)
                    .label("Link Group") // TODO: Localize
                    .label_color(TEXT_COLOR_GREY) // Change this when the linking is working
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .label_font_size(self.fonts.cyri.scale(10))
                    .set(state.ids.btn_link, ui)
                    .was_clicked()
                {};
                if Button::image(self.imgs.button)
                    .w_h(90.0, 22.0)
                    .mid_bottom_with_margin_on(state.ids.btn_link, -27.0)
                    .down_from(state.ids.btn_link, 5.0)
                    .hover_image(self.imgs.button_hover)
                    .press_image(self.imgs.button_press)
                    .label(&self.localized_strings.get("hud.group.kick"))
                    .label_color(if state.selected_member.is_some() {
                        TEXT_COLOR
                    } else {
                        TEXT_COLOR_GREY
                    })
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .label_font_size(self.fonts.cyri.scale(10))
                    .set(state.ids.btn_kick, ui)
                    .was_clicked()
                {
                    if let Some(uid) = selected {
                        events.push(Event::Kick(uid));
                        state.update(|s| {
                            s.selected_member = None;
                        });
                    }
                };
            }
            // Group Members, only character names, cut long names when they exceed the
            // button size
            let group_size = group_members.len() + 1;
            if state.ids.members.len() < group_size {
                state.update(|s| {
                    s.ids
                        .members
                        .resize(group_size, &mut ui.widget_id_generator())
                })
            }
            // Scrollable area for group member names
            Rectangle::fill_with([110.0, 192.0], color::TRANSPARENT)
                .top_left_with_margins_on(state.ids.bg, 30.0, 5.0)
                .scroll_kids()
                .scroll_kids_vertically()
                .set(state.ids.scroll_area, ui);
            Scrollbar::y_axis(state.ids.scroll_area)
                .thickness(5.0)
                .rgba(0.33, 0.33, 0.33, 1.0)
                .set(state.ids.scrollbar, ui);
            // List member names
            for (i, &uid) in self
                .client
                .uid()
                .iter()
                .chain(group_members.iter().copied())
                .enumerate()
            {
                let selected = state.selected_member.map_or(false, |u| u == uid);
                let char_name = uid_to_name_text(uid, &self.client);

                // TODO: Do something special visually if uid == leader
                if Button::image(if selected {
                    self.imgs.selection
                } else {
                    self.imgs.nothing
                })
                .w_h(100.0, 22.0)
                .and(|w| {
                    if i == 0 {
                        w.top_left_with_margins_on(state.ids.scroll_area, 5.0, 0.0)
                    } else {
                        w.down_from(state.ids.members[i - 1], 10.0)
                    }
                })
                .hover_image(self.imgs.selection_hover)
                .press_image(self.imgs.selection_press)
                .crop_kids()
                .label_x(Relative::Place(Place::Start(Some(4.0))))
                .label(&char_name)
                .label_color(TEXT_COLOR)
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_font_size(self.fonts.cyri.scale(12))
                .set(state.ids.members[i], ui)
                .was_clicked()
                {
                    // Do nothing when clicking yourself
                    if Some(uid) != my_uid {
                        // Select the group member
                        state.update(|s| {
                            s.selected_member = if selected { None } else { Some(uid) }
                        });
                    }
                };
            }
            // Maximum of 6 Players/Npcs per Group
            // Player pets count as group members, too. They are not counted
            // into the maximum group size.
        }
        if let Some(invite_uid) = open_invite {
            self.show.group = true; // Auto open group menu
            // TODO: add group name here too
            // Invite text
            let name = uid_to_name_text(invite_uid, &self.client);
            let invite_text = self
                .localized_strings
                .get("hud.group.invite_to_join")
                .replace("{name}", &name);
            Text::new(&invite_text)
                .mid_top_with_margin_on(state.ids.bg, 20.0)
                .font_size(20)
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.title, ui);
            // Accept Button
            let accept_key = self
                .settings
                .controls
                .get_binding(GameInput::AcceptGroupInvite)
                .map_or_else(|| "".into(), |key| key.to_string());
            if Button::image(self.imgs.button)
                .w_h(90.0, 22.0)
                .bottom_left_with_margins_on(state.ids.bg, 15.0, 15.0)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .label(&format!(
                    "[{}] {}",
                    &accept_key,
                    &self.localized_strings.get("common.accept")
                ))
                .label_color(TEXT_COLOR)
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_font_size(self.fonts.cyri.scale(15))
                .set(state.ids.btn_accept, ui)
                .was_clicked()
            {
                events.push(Event::Accept);
            };
            // Decline button
            let decline_key = self
                .settings
                .controls
                .get_binding(GameInput::DeclineGroupInvite)
                .map_or_else(|| "".into(), |key| key.to_string());
            if Button::image(self.imgs.button)
                .w_h(90.0, 22.0)
                .bottom_right_with_margins_on(state.ids.bg, 15.0, 15.0)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .label(&format!(
                    "[{}] {}",
                    &decline_key,
                    &self.localized_strings.get("common.decline")
                ))
                .label_color(TEXT_COLOR)
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_font_size(self.fonts.cyri.scale(15))
                .set(state.ids.btn_decline, ui)
                .was_clicked()
            {
                events.push(Event::Decline);
            };
        }
        events
    }
}
