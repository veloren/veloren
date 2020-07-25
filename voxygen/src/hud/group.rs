use super::{
    img_ids::{Imgs, ImgsRot},
    Show, BLACK, GROUP_COLOR, HP_COLOR, KILL_COLOR, LOW_HP_COLOR, MANA_COLOR, TEXT_COLOR,
    TEXT_COLOR_GREY, TRANSPARENT, UI_HIGHLIGHT_0,
};

use crate::{
    i18n::VoxygenLocalization,
    settings::Settings,
    ui::{fonts::ConrodVoxygenFonts, ImageFrame, Tooltip, TooltipManager, Tooltipable},
    window::GameInput,
    GlobalState,
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
    widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};
use specs::{saveload::MarkerAllocator, WorldExt};

widget_ids! {
    pub struct Ids {
        group_button,
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
        member_panels_bg[],
        member_panels_frame[],
        member_panels_txt_bg[],
        member_panels_txt[],
        member_health[],
        member_stam[],
        dead_txt[],
        health_txt[],
    }
}

pub struct State {
    ids: Ids,
    // Selected group member
    selected_member: Option<Uid>,
}

const TOOLTIP_UPSHIFT: f64 = 40.0;
#[derive(WidgetCommon)]
pub struct Group<'a> {
    show: &'a mut Show,
    client: &'a Client,
    settings: &'a Settings,
    imgs: &'a Imgs,
    fonts: &'a ConrodVoxygenFonts,
    localized_strings: &'a std::sync::Arc<VoxygenLocalization>,
    tooltip_manager: &'a mut TooltipManager,
    rot_imgs: &'a ImgsRot,
    pulse: f32,
    global_state: &'a GlobalState,

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
        tooltip_manager: &'a mut TooltipManager,
        rot_imgs: &'a ImgsRot,
        pulse: f32,
        global_state: &'a GlobalState,
    ) -> Self {
        Self {
            show,
            client,
            settings,
            imgs,
            rot_imgs,
            tooltip_manager,
            fonts,
            localized_strings,
            pulse,
            global_state,
            common: widget::CommonBuilder::default(),
        }
    }
}

pub enum Event {
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

    //TODO: Disband groups when there's only one member in them
    //TODO: Always send health, energy, level and position of group members to the
    // client

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;

        let mut events = Vec::new();
        let localized_strings = self.localized_strings;

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
        if self.show.group_menu || open_invite.is_some() {
            // Frame
            Rectangle::fill_with([220.0, 230.0], color::Color::Rgba(0.0, 0.0, 0.0, 0.8))
                .bottom_left_with_margins_on(ui.window, 220.0, 10.0)
                .set(state.ids.bg, ui);
        }
        if open_invite.is_some() {
            // Group Menu button
            Button::image(self.imgs.group_icon)
                .w_h(49.0, 26.0)
                .bottom_left_with_margins_on(ui.window, 190.0, 10.0)
                .set(state.ids.group_button, ui);
        }
        // Buttons
        if let Some((group_name, leader)) = self.client.group_info().filter(|_| in_group) {
            // Group Menu Button
            if Button::image(if self.show.group_menu {
                self.imgs.group_icon_press
            } else {
                self.imgs.group_icon
            })
            .w_h(49.0, 26.0)
            .bottom_left_with_margins_on(ui.window, 190.0, 10.0)
            .hover_image(self.imgs.group_icon_hover)
            .press_image(self.imgs.group_icon_press)
            .with_tooltip(
                self.tooltip_manager,
                &localized_strings.get("hud.group"),
                "",
                &button_tooltip,
            )
            .bottom_offset(TOOLTIP_UPSHIFT)
            .set(state.ids.group_button, ui)
            .was_clicked()
            {
                self.show.group_menu = !self.show.group_menu;
            };
            // Member panels
            let group_size = group_members.len() + 1;
            if state.ids.member_panels_bg.len() < group_size {
                state.update(|s| {
                    s.ids
                        .member_panels_bg
                        .resize(group_size, &mut ui.widget_id_generator())
                })
            };
            if state.ids.member_health.len() < group_size {
                state.update(|s| {
                    s.ids
                        .member_health
                        .resize(group_size, &mut ui.widget_id_generator())
                })
            };
            if state.ids.member_stam.len() < group_size {
                state.update(|s| {
                    s.ids
                        .member_stam
                        .resize(group_size, &mut ui.widget_id_generator())
                })
            };
            if state.ids.member_panels_frame.len() < group_size {
                state.update(|s| {
                    s.ids
                        .member_panels_frame
                        .resize(group_size, &mut ui.widget_id_generator())
                })
            };
            if state.ids.member_panels_txt.len() < group_size {
                state.update(|s| {
                    s.ids
                        .member_panels_txt
                        .resize(group_size, &mut ui.widget_id_generator())
                })
            };
            if state.ids.dead_txt.len() < group_size {
                state.update(|s| {
                    s.ids
                        .dead_txt
                        .resize(group_size, &mut ui.widget_id_generator())
                })
            };
            if state.ids.health_txt.len() < group_size {
                state.update(|s| {
                    s.ids
                        .health_txt
                        .resize(group_size, &mut ui.widget_id_generator())
                })
            };
            if state.ids.member_panels_txt_bg.len() < group_size {
                state.update(|s| {
                    s.ids
                        .member_panels_txt_bg
                        .resize(group_size, &mut ui.widget_id_generator())
                })
            };

            let client_state = self.client.state();
            let stats = client_state.ecs().read_storage::<common::comp::Stats>();
            let energy = client_state.ecs().read_storage::<common::comp::Energy>();
            let uid_allocator = client_state
                .ecs()
                .read_resource::<common::sync::UidAllocator>();

            for (i, &uid) in self
                .client
                .uid()
                .iter()
                .chain(group_members.iter().copied())
                .enumerate()
            {
                self.show.group = true;
                let entity = uid_allocator.retrieve_entity_internal(uid.into());
                let stats = entity.and_then(|entity| stats.get(entity));
                let energy = entity.and_then(|entity| energy.get(entity));
                if let Some(stats) = stats {
                    let char_name = stats.name.to_string();
                    let health_perc = stats.health.current() as f64 / stats.health.maximum() as f64;

                    // change panel positions when debug info is shown
                    let offset = if self.global_state.settings.gameplay.toggle_debug {
                        210.0
                    } else {
                        110.0
                    };
                    let pos = if i == 0 {
                        Image::new(self.imgs.member_bg)
                            .top_left_with_margins_on(ui.window, offset, 20.0)
                    } else {
                        Image::new(self.imgs.member_bg)
                            .down_from(state.ids.member_panels_bg[i - 1], 40.0)
                    };
                    let hp_ani = (self.pulse * 4.0/* speed factor */).cos() * 0.5 + 0.8; //Animation timer
                    let crit_hp_color: Color = Color::Rgba(0.79, 0.19, 0.17, hp_ani);
                    let health_col = match (health_perc * 100.0) as u8 {
                        0..=20 => crit_hp_color,
                        21..=40 => LOW_HP_COLOR,
                        _ => HP_COLOR,
                    };
                    // Don't show panel for the player!
                    // Panel BG
                    pos.w_h(152.0, 36.0)
                        .color(if i == 0 {
                            Some(TRANSPARENT)
                        } else {
                            Some(TEXT_COLOR)
                        })
                        .set(state.ids.member_panels_bg[i], ui);
                    // Health
                    Image::new(self.imgs.bar_content)
                        .w_h(148.0 * health_perc, 22.0)
                        .color(if i == 0 {
                            Some(TRANSPARENT)
                        } else {
                            Some(health_col)
                        })
                        .top_left_with_margins_on(state.ids.member_panels_bg[i], 2.0, 2.0)
                        .set(state.ids.member_health[i], ui);
                    if stats.is_dead {
                        // Death Text
                        Text::new(&self.localized_strings.get("hud.group.dead"))
                            .mid_top_with_margin_on(state.ids.member_panels_bg[i], 1.0)
                            .font_size(20)
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(if i == 0 { TRANSPARENT } else { KILL_COLOR })
                            .set(state.ids.dead_txt[i], ui);
                    } else {
                        // Health Text
                        let txt = format!(
                            "{}/{}",
                            stats.health.current() as u32,
                            stats.health.maximum() as u32,
                        );
                        let font_size = match stats.health.maximum() {
                            0..=999 => 14,
                            1000..=9999 => 13,
                            10000..=99999 => 12,
                            _ => 11,
                        };
                        let txt_offset = match stats.health.maximum() {
                            0..=999 => 4.0,
                            1000..=9999 => 4.5,
                            10000..=99999 => 5.0,
                            _ => 5.5,
                        };
                        Text::new(&txt)
                            .mid_top_with_margin_on(state.ids.member_panels_bg[i], txt_offset)
                            .font_size(font_size)
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(if i == 0 {
                                TRANSPARENT
                            } else {
                                Color::Rgba(1.0, 1.0, 1.0, 0.5)
                            })
                            .set(state.ids.health_txt[i], ui);
                    };
                    // Panel Frame
                    Image::new(self.imgs.member_frame)
                        .w_h(152.0, 36.0)
                        .middle_of(state.ids.member_panels_bg[i])
                        .color(if i == 0 {
                            Some(TRANSPARENT)
                        } else {
                            Some(UI_HIGHLIGHT_0)
                        })
                        .set(state.ids.member_panels_frame[i], ui);
                    // Panel Text
                    Text::new(&char_name)
                        .top_left_with_margins_on(state.ids.member_panels_frame[i], -22.0, 0.0)
                        .font_size(20)
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(if i == 0 { TRANSPARENT } else { BLACK })
                        .set(state.ids.member_panels_txt_bg[i], ui);
                    Text::new(&char_name)
                        .bottom_left_with_margins_on(state.ids.member_panels_txt_bg[i], 2.0, 2.0)
                        .font_size(20)
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(if i == 0 { TRANSPARENT } else { GROUP_COLOR })
                        .set(state.ids.member_panels_txt[i], ui);
                    if let Some(energy) = energy {
                        let stam_perc = energy.current() as f64 / energy.maximum() as f64;
                        // Stamina
                        Image::new(self.imgs.bar_content)
                            .w_h(100.0 * stam_perc, 8.0)
                            .color(if i == 0 {
                                Some(TRANSPARENT)
                            } else {
                                Some(MANA_COLOR)
                            })
                            .top_left_with_margins_on(state.ids.member_panels_bg[i], 26.0, 2.0)
                            .set(state.ids.member_stam[i], ui);
                    }
                } else {
                    // Values N.A.
                    if let Some(stats) = stats {
                        Text::new(&stats.name.to_string())
                            .top_left_with_margins_on(state.ids.member_panels_frame[i], -22.0, 0.0)
                            .font_size(20)
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(GROUP_COLOR)
                            .set(state.ids.member_panels_txt[i], ui);
                    };
                    let offset = if self.global_state.settings.gameplay.toggle_debug {
                        210.0
                    } else {
                        110.0
                    };
                    let pos = if i == 0 {
                        Image::new(self.imgs.member_bg)
                            .top_left_with_margins_on(ui.window, offset, 20.0)
                    } else {
                        Image::new(self.imgs.member_bg)
                            .down_from(state.ids.member_panels_bg[i - 1], 40.0)
                    };
                    pos.w_h(152.0, 36.0)
                        .color(if i == 0 {
                            Some(TRANSPARENT)
                        } else {
                            Some(TEXT_COLOR)
                        })
                        .set(state.ids.member_panels_bg[i], ui);
                    // Panel Frame
                    Image::new(self.imgs.member_frame)
                        .w_h(152.0, 36.0)
                        .middle_of(state.ids.member_panels_bg[i])
                        .color(if i == 0 {
                            Some(TRANSPARENT)
                        } else {
                            Some(UI_HIGHLIGHT_0)
                        })
                        .set(state.ids.member_panels_frame[i], ui);
                    // Panel Text
                    Text::new(&self.localized_strings.get("hud.group.out_of_range"))
                        .mid_top_with_margin_on(state.ids.member_panels_bg[i], 3.0)
                        .font_size(16)
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(if i == 0 { TRANSPARENT } else { TEXT_COLOR })
                        .set(state.ids.dead_txt[i], ui);
                }
            }

            if self.show.group_menu {
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
                    self.show.group_menu = false;
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
                .font_size(12)
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
                self.show.group_menu = true;
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
