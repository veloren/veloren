use super::{
    cr_color,
    img_ids::{Imgs, ImgsRot},
    Show, BLACK, BUFF_COLOR, DEBUFF_COLOR, ERROR_COLOR, GROUP_COLOR, HP_COLOR, KILL_COLOR,
    LOW_HP_COLOR, QUALITY_EPIC, STAMINA_COLOR, TEXT_COLOR, TEXT_COLOR_GREY, UI_HIGHLIGHT_0,
    UI_MAIN,
};

use crate::{
    game_input::GameInput,
    hud::BuffIcon,
    settings::Settings,
    ui::{fonts::Fonts, ImageFrame, Tooltip, TooltipManager, Tooltipable},
    GlobalState,
};
use client::{self, Client};
use common::{
    combat,
    comp::{group::Role, inventory::item::MaterialStatManifest, invite::InviteKind, Stats},
    resources::Time,
    uid::{Uid, UidAllocator},
};
use common_net::sync::WorldSyncExt;
use conrod_core::{
    color,
    position::{Place, Relative},
    widget::{self, Button, Image, Rectangle, Scrollbar, Text},
    widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};
use i18n::Localization;
use specs::{saveload::MarkerAllocator, WorldExt};

widget_ids! {
    pub struct Ids {
        group_button,
        bg,
        title,
        title_bg,
        btn_bg,
        btn_friend,
        btn_leader,
        btn_link,
        btn_kick,
        btn_leave,
        scroll_area,
        scrollbar,
        members[],
        bubble_frame,
        btn_accept,
        btn_decline,
        member_panels_bg[],
        member_panels_frame[],
        member_panels_txt_bg[],
        member_panels_txt[],
        member_health[],
        member_health_decay[],
        member_energy[],
        buffs[],
        buff_timers[],
        dead_txt[],
        health_txt[],
        combat_rating_indicators[],
        timeout_bg,
        timeout,
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
    rot_imgs: &'a ImgsRot,
    fonts: &'a Fonts,
    localized_strings: &'a Localization,
    pulse: f32,
    global_state: &'a GlobalState,
    tooltip_manager: &'a mut TooltipManager,
    msm: &'a MaterialStatManifest,
    time: &'a Time,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> Group<'a> {
    pub fn new(
        show: &'a mut Show,
        client: &'a Client,
        settings: &'a Settings,
        imgs: &'a Imgs,
        rot_imgs: &'a ImgsRot,
        fonts: &'a Fonts,
        localized_strings: &'a Localization,
        pulse: f32,
        global_state: &'a GlobalState,
        tooltip_manager: &'a mut TooltipManager,
        msm: &'a MaterialStatManifest,
        time: &'a Time,
    ) -> Self {
        Self {
            show,
            client,
            settings,
            imgs,
            rot_imgs,
            fonts,
            localized_strings,
            pulse,
            global_state,
            tooltip_manager,
            msm,
            time,
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

    fn style(&self) -> Self::Style {}

    //TODO: Disband groups when there's only one member in them
    //TODO: Always send health, energy, level and position of group members to the
    // client
    #[allow(clippy::blocks_in_if_conditions)] // TODO: Pending review in #587
    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        common_base::prof_span!("Group::update");
        let widget::UpdateArgs { state, ui, .. } = args;
        let mut events = Vec::new();
        let localized_strings = self.localized_strings;
        let key_layout = &self.global_state.window.key_layout;
        let buff_ani = ((self.pulse * 4.0/* speed factor */).cos() * 0.5 + 0.8) + 0.5; //Animation timer
        let debug_on = self.global_state.settings.interface.toggle_debug;
        let offset = if debug_on { 270.0 } else { 0.0 };
        let buffs_tooltip = Tooltip::new({
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
        if !in_group {
            self.show.group_menu = false;
            self.show.group = false;
        }

        // Helper
        let uid_to_name_text = |uid, client: &Client| match client.player_list().get(&uid) {
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

        let open_invite = self.client.invite();

        let my_uid = self.client.uid();

        // TODO show something to the player when they click on the group button while
        // they are not in a group so that it doesn't look like the button is
        // broken
        if self.show.group_menu || open_invite.is_some() {
            // Frame
            Rectangle::fill_with([220.0, 140.0], Color::Rgba(0.0, 0.0, 0.0, 0.8))
                .bottom_left_with_margins_on(ui.window, 108.0, 490.0)
                .crop_kids()
                .set(state.ids.bg, ui);
        }
        if let Some((_, timeout_start, timeout_dur, _)) = open_invite {
            // Group Menu button
            Button::image(self.imgs.group_icon)
                .w_h(49.0, 26.0)
                .bottom_left_with_margins_on(ui.window, 10.0, 490.0)
                .set(state.ids.group_button, ui);
            // Show timeout bar
            let timeout_progress =
                1.0 - timeout_start.elapsed().as_secs_f32() / timeout_dur.as_secs_f32();
            Image::new(self.imgs.progress_frame)
                .w_h(100.0, 10.0)
                .middle_of(state.ids.bg)
                .color(Some(UI_MAIN))
                .set(state.ids.timeout_bg, ui);
            Image::new(self.imgs.progress)
                .w_h(98.0 * timeout_progress as f64, 8.0)
                .top_left_with_margins_on(state.ids.timeout_bg, 1.0, 1.0)
                .color(Some(UI_HIGHLIGHT_0))
                .set(state.ids.timeout, ui);
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
            .bottom_left_with_margins_on(ui.window, 10.0, 490.0)
            .hover_image(self.imgs.group_icon_hover)
            .press_image(self.imgs.group_icon_press)
            .set(state.ids.group_button, ui)
            .was_clicked()
            {
                self.show.group_menu = !self.show.group_menu;
            };
            Text::new(&group_name)
                .up_from(state.ids.group_button, 5.0)
                .font_size(14)
                .font_id(self.fonts.cyri.conrod_id)
                .color(BLACK)
                .set(state.ids.title_bg, ui);
            Text::new(&group_name)
                .bottom_right_with_margins_on(state.ids.title_bg, 1.0, 1.0)
                .font_size(14)
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.title, ui);
            // Member panels
            let group_size = group_members.len();
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
                        .resize(group_size, &mut ui.widget_id_generator());
                })
            };
            if state.ids.member_health_decay.len() < group_size {
                state.update(|s| {
                    s.ids
                        .member_health_decay
                        .resize(group_size, &mut ui.widget_id_generator());
                })
            };
            if state.ids.member_energy.len() < group_size {
                state.update(|s| {
                    s.ids
                        .member_energy
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
            if state.ids.combat_rating_indicators.len() < group_size {
                state.update(|s| {
                    s.ids
                        .combat_rating_indicators
                        .resize(group_size, &mut ui.widget_id_generator())
                })
            };
            let client_state = self.client.state();
            let stats = client_state.ecs().read_storage::<Stats>();
            let skill_sets = client_state.ecs().read_storage::<common::comp::SkillSet>();
            let healths = client_state.ecs().read_storage::<common::comp::Health>();
            let energy = client_state.ecs().read_storage::<common::comp::Energy>();
            let buffs = client_state.ecs().read_storage::<common::comp::Buffs>();
            let inventory = client_state.ecs().read_storage::<common::comp::Inventory>();
            let uid_allocator = client_state.ecs().read_resource::<UidAllocator>();
            let bodies = client_state.ecs().read_storage::<common::comp::Body>();
            let poises = client_state.ecs().read_storage::<common::comp::Poise>();
            let stances = client_state.ecs().read_storage::<common::comp::Stance>();

            // Keep track of the total number of widget ids we are using for buffs
            let mut total_buff_count = 0;
            for (i, &uid) in group_members.iter().copied().enumerate() {
                self.show.group = true;
                let entity = uid_allocator.retrieve_entity_internal(uid.into());
                let stats = entity.and_then(|entity| stats.get(entity));
                let skill_set = entity.and_then(|entity| skill_sets.get(entity));
                let health = entity.and_then(|entity| healths.get(entity));
                let energy = entity.and_then(|entity| energy.get(entity));
                let buffs = entity.and_then(|entity| buffs.get(entity));
                let inventory = entity.and_then(|entity| inventory.get(entity));
                let is_leader = uid == leader;
                let body = entity.and_then(|entity| bodies.get(entity));
                let poise = entity.and_then(|entity| poises.get(entity));
                let stance = entity.and_then(|entity| stances.get(entity));

                if let (
                    Some(stats),
                    Some(skill_set),
                    Some(inventory),
                    Some(health),
                    Some(energy),
                    Some(body),
                    Some(poise),
                ) = (stats, skill_set, inventory, health, energy, body, poise)
                {
                    let combat_rating = combat::combat_rating(
                        inventory, health, energy, poise, skill_set, *body, self.msm,
                    );
                    let char_name = stats.name.to_string();
                    let health_perc = health.current() / health.base_max().max(health.maximum());
                    // change panel positions when debug info is shown
                    let x = if debug_on { i / 8 } else { i / 11 };
                    let y = if debug_on { i % 8 } else { i % 11 };
                    let back = Image::new(self.imgs.member_bg).top_left_with_margins_on(
                        ui.window,
                        50.0 + offset + y as f64 * 77.0,
                        10.0 + x as f64 * 180.0,
                    );
                    let hp_ani = (self.pulse * 4.0/* speed factor */).cos() * 0.5 + 0.8; //Animation timer
                    let crit_hp_color: Color = Color::Rgba(0.79, 0.19, 0.17, hp_ani);
                    let health_col = match (health_perc * 100.0) as u8 {
                        0..=20 => crit_hp_color,
                        21..=40 => LOW_HP_COLOR,
                        _ => HP_COLOR,
                    };
                    // Don't show panel for the player!
                    // Panel BG
                    back.w_h(152.0, 36.0)
                        .color(if is_leader {
                            Some(ERROR_COLOR)
                        } else {
                            Some(TEXT_COLOR)
                        })
                        .set(state.ids.member_panels_bg[i], ui);
                    // Health
                    Image::new(self.imgs.bar_content)
                        .w_h(148.0 * f64::from(health_perc), 22.0)
                        .color(Some(health_col))
                        .top_left_with_margins_on(state.ids.member_panels_bg[i], 2.0, 2.0)
                        .set(state.ids.member_health[i], ui);
                    // Health Decay
                    let decayed_health = f64::from(1.0 - health.maximum() / health.base_max());
                    if decayed_health > 0.0 {
                        let decay_bar_len = 148.0 * decayed_health;
                        Image::new(self.imgs.bar_content)
                            .w_h(decay_bar_len, 22.0)
                            .color(Some(QUALITY_EPIC))
                            .top_right_with_margins_on(state.ids.member_panels_bg[i], 2.0, 2.0)
                            .set(state.ids.member_health_decay[i], ui);
                    }
                    if health.is_dead {
                        // Death Text
                        Text::new(&self.localized_strings.get_msg("hud-group-dead"))
                            .mid_top_with_margin_on(state.ids.member_panels_bg[i], 1.0)
                            .font_size(20)
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(KILL_COLOR)
                            .set(state.ids.dead_txt[i], ui);
                    } else {
                        // Health Text
                        let txt = format!(
                            "{}/{}",
                            health.current().round() as u32,
                            health.maximum().round() as u32,
                        );
                        // Change font size depending on health amount
                        let font_size = match health.maximum() {
                            x if (0.0..100.0).contains(&x) => 14,
                            x if (100.0..=1000.0).contains(&x) => 13,
                            x if (1000.0..=10000.0).contains(&x) => 12,
                            _ => 11,
                        };
                        // Change text offset depending on health amount
                        let txt_offset = match health.maximum() {
                            x if (0.0..=100.0).contains(&x) => 4.0,
                            x if (100.0..=1000.0).contains(&x) => 4.5,
                            x if (1000.0..=10000.0).contains(&x) => 5.0,
                            _ => 5.5,
                        };
                        Text::new(&txt)
                            .mid_top_with_margin_on(state.ids.member_panels_bg[i], txt_offset)
                            .font_size(font_size)
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(Color::Rgba(1.0, 1.0, 1.0, 0.5))
                            .set(state.ids.health_txt[i], ui);
                    };

                    // Panel Frame
                    Image::new(self.imgs.member_frame)
                        .w_h(152.0, 36.0)
                        .middle_of(state.ids.member_panels_bg[i])
                        .color(Some(UI_HIGHLIGHT_0))
                        .set(state.ids.member_panels_frame[i], ui);

                    let indicator_col = cr_color(combat_rating);
                    Image::new(self.imgs.combat_rating_ico_shadow)
                        .w_h(18.0, 18.0)
                        .top_left_with_margins_on(state.ids.member_panels_frame[i], -20.0, 2.0)
                        .color(Some(indicator_col))
                        .set(state.ids.combat_rating_indicators[i], ui);
                    // Panel Text
                    Text::new(&char_name)
                     .top_left_with_margins_on(state.ids.member_panels_frame[i], -22.0, 22.0)
                     .font_size(20)
                     .font_id(self.fonts.cyri.conrod_id)
                     .color(BLACK)
                     .w(300.0) // limit name length display
                     .set(state.ids.member_panels_txt_bg[i], ui);
                    Text::new(&char_name)
                            .bottom_left_with_margins_on(state.ids.member_panels_txt_bg[i], 2.0, 2.0)
                            .font_size(20)
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(if is_leader { ERROR_COLOR } else { GROUP_COLOR })
                            .w(300.0) // limit name length display
                            .set(state.ids.member_panels_txt[i], ui);
                    let stam_perc = energy.current() / energy.maximum();
                    // Energy
                    Image::new(self.imgs.bar_content)
                        .w_h(100.0 * f64::from(stam_perc), 8.0)
                        .color(Some(STAMINA_COLOR))
                        .top_left_with_margins_on(state.ids.member_panels_bg[i], 26.0, 2.0)
                        .set(state.ids.member_energy[i], ui);
                    if let Some(buffs) = buffs {
                        let buff_icons = BuffIcon::icons_vec(buffs, stance);
                        // Limit displayed buffs to 11
                        let buff_count = buff_icons.len().min(11);
                        total_buff_count += buff_count;
                        let gen = &mut ui.widget_id_generator();
                        if state.ids.buffs.len() < total_buff_count {
                            state.update(|state| state.ids.buffs.resize(total_buff_count, gen));
                        }
                        if state.ids.buff_timers.len() < total_buff_count {
                            state.update(|state| {
                                state.ids.buff_timers.resize(total_buff_count, gen)
                            });
                        }
                        // Create Buff Widgets
                        let mut prev_id = None;
                        state
                            .ids
                            .buffs
                            .iter()
                            .copied()
                            .zip(state.ids.buff_timers.iter().copied())
                            .skip(total_buff_count - buff_count)
                            .zip(buff_icons.iter())
                            .for_each(|((id, timer_id), buff)| {
                                let max_duration = buff.kind.max_duration();
                                let pulsating_col = Color::Rgba(1.0, 1.0, 1.0, buff_ani);
                                let norm_col = Color::Rgba(1.0, 1.0, 1.0, 1.0);
                                let current_duration = buff.end_time.map(|end| end - self.time.0);
                                let duration_percentage = current_duration.map_or(1000.0, |cur| {
                                    max_duration.map_or(1000.0, |max| cur / max.0 * 1000.0)
                                }) as u32; // Percentage to determine which frame of the timer overlay is displayed
                                let buff_img = buff.kind.image(self.imgs);
                                let buff_widget = Image::new(buff_img).w_h(15.0, 15.0);
                                let buff_widget = if let Some(id) = prev_id {
                                    buff_widget.right_from(id, 1.0)
                                } else {
                                    buff_widget.bottom_left_with_margins_on(
                                        state.ids.member_panels_frame[i],
                                        -16.0,
                                        1.0,
                                    )
                                };
                                prev_id = Some(id);
                                buff_widget
                                    .color(if current_duration.map_or(false, |cur| cur < 10.0) {
                                        Some(pulsating_col)
                                    } else {
                                        Some(norm_col)
                                    })
                                    .set(id, ui);
                                // Create Buff tooltip
                                let (title, desc_txt) =
                                    buff.kind.title_description(localized_strings);
                                let remaining_time = buff.get_buff_time(*self.time);
                                let desc = format!("{}\n\n{}", desc_txt, remaining_time);
                                Image::new(match duration_percentage as u64 {
                                    875..=1000 => self.imgs.nothing, // 8/8
                                    750..=874 => self.imgs.buff_0,   // 7/8
                                    625..=749 => self.imgs.buff_1,   // 6/8
                                    500..=624 => self.imgs.buff_2,   // 5/8
                                    375..=499 => self.imgs.buff_3,   // 4/8
                                    250..=374 => self.imgs.buff_4,   // 3/8
                                    125..=249 => self.imgs.buff_5,   // 2/8
                                    0..=124 => self.imgs.buff_6,     // 1/8
                                    _ => self.imgs.nothing,
                                })
                                .w_h(15.0, 15.0)
                                .middle_of(id)
                                .with_tooltip(
                                    self.tooltip_manager,
                                    &title,
                                    &desc,
                                    &buffs_tooltip,
                                    if buff.is_buff {
                                        BUFF_COLOR
                                    } else {
                                        DEBUFF_COLOR
                                    },
                                )
                                .set(timer_id, ui);
                            });
                    } else {
                        // Values N.A.
                        Text::new(&stats.name.to_string())
                            .top_left_with_margins_on(state.ids.member_panels_frame[i], -22.0, 0.0)
                            .font_size(20)
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(GROUP_COLOR)
                            .set(state.ids.member_panels_txt[i], ui);
                        let back = if i == 0 {
                            Image::new(self.imgs.member_bg)
                                .top_left_with_margins_on(ui.window, offset, 20.0)
                        } else {
                            Image::new(self.imgs.member_bg)
                                .down_from(state.ids.member_panels_bg[i - 1], 40.0)
                        };
                        back.w_h(152.0, 36.0)
                            .color(Some(TEXT_COLOR))
                            .set(state.ids.member_panels_bg[i], ui);
                        // Panel Frame
                        Image::new(self.imgs.member_frame)
                            .w_h(152.0, 36.0)
                            .middle_of(state.ids.member_panels_bg[i])
                            .color(Some(UI_HIGHLIGHT_0))
                            .set(state.ids.member_panels_frame[i], ui);
                        // Panel Text
                        Text::new(&self.localized_strings.get_msg("hud-group-out_of_range"))
                            .mid_top_with_margin_on(state.ids.member_panels_bg[i], 3.0)
                            .font_size(16)
                            .font_id(self.fonts.cyri.conrod_id)
                            .color(TEXT_COLOR)
                            .set(state.ids.dead_txt[i], ui);
                    }
                }
            }

            if self.show.group_menu {
                let selected = state.selected_member;
                if Button::image(self.imgs.button) // Change button behaviour and style when the friendslist is working
                    .w_h(90.0, 22.0)
                    .top_right_with_margins_on(state.ids.bg, 5.0, 5.0)
                    .hover_image(self.imgs.button)
                    .press_image(self.imgs.button)
                    .label_color(TEXT_COLOR_GREY)
                    .image_color(TEXT_COLOR_GREY)
                    .label(&self.localized_strings.get_msg("hud-group-add_friend"))
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
                    .label(&self.localized_strings.get_msg("hud-group-leave"))
                    .label_color(TEXT_COLOR)
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .label_font_size(self.fonts.cyri.scale(10))
                    .set(state.ids.btn_leave, ui)
                    .was_clicked()
                {
                    self.show.group_menu = false;
                    self.show.group = !self.show.group;
                    events.push(Event::LeaveGroup);
                };
                // Group leader functions
                if my_uid == Some(leader) {
                    if Button::image(self.imgs.button)
                        .w_h(90.0, 22.0)
                        .mid_bottom_with_margin_on(state.ids.btn_friend, -27.0)
                        .hover_image(self.imgs.button_hover)
                        .press_image(self.imgs.button_press)
                        .label(&self.localized_strings.get_msg("hud-group-assign_leader"))
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
                        .label(&self.localized_strings.get_msg("hud-group-link_group"))
                        .hover_image(self.imgs.button)
                        .press_image(self.imgs.button)
                        .label_color(TEXT_COLOR_GREY)
                        .image_color(TEXT_COLOR_GREY)
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
                        .label(&self.localized_strings.get_msg("hud-group-kick"))
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
                let group_size = group_members.len();
                if state.ids.members.len() < group_size {
                    state.update(|s| {
                        s.ids
                            .members
                            .resize(group_size, &mut ui.widget_id_generator())
                    })
                }
                // Scrollable area for group member names
                Rectangle::fill_with([110.0, 135.0], color::TRANSPARENT)
                    .top_left_with_margins_on(state.ids.bg, 5.0, 5.0)
                    .crop_kids()
                    .scroll_kids_vertically()
                    .set(state.ids.scroll_area, ui);
                Scrollbar::y_axis(state.ids.scroll_area)
                    .thickness(5.0)
                    .rgba(0.33, 0.33, 0.33, 1.0)
                    .set(state.ids.scrollbar, ui);
                // List member names
                for (i, &uid) in group_members.iter().copied().enumerate() {
                    let selected = state.selected_member.map_or(false, |u| u == uid);
                    let char_name = uid_to_name_text(uid, self.client);
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
                            w.down_from(state.ids.members[i - 1], 5.0)
                        }
                    })
                    .hover_image(self.imgs.selection_hover)
                    .press_image(self.imgs.selection_press)
                    .image_color(color::rgba(1.0, 0.82, 0.27, 1.0))
                    .crop_kids()
                    .label_x(Relative::Place(Place::Start(Some(4.0))))
                    .label(&char_name)
                    .label_color(if uid == leader {
                        ERROR_COLOR
                    } else {
                        TEXT_COLOR
                    })
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
        if let Some((invite_uid, _, _, kind)) = open_invite {
            self.show.group = true; // Auto open group menu
            // TODO: add group name here too
            // Invite text

            let name = uid_to_name_text(invite_uid, self.client);
            let invite_text = match kind {
                InviteKind::Group => self.localized_strings.get_msg_ctx(
                    "hud-group-invite_to_join",
                    &i18n::fluent_args! {
                        "name" => name,
                    },
                ),
                InviteKind::Trade => self.localized_strings.get_msg_ctx(
                    "hud-group-invite_to_trade",
                    &i18n::fluent_args! {
                        "name" => &name,
                    },
                ),
            };
            Text::new(&invite_text)
                .mid_top_with_margin_on(state.ids.bg, 5.0)
                .font_size(12)
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .w(165.0) // Text stays within frame
                .set(state.ids.title, ui);
            // Accept Button
            let accept_key = self
                .settings
                .controls
                .get_binding(GameInput::AcceptGroupInvite)
                .map_or_else(|| "".into(), |key| key.display_string(key_layout));
            if Button::image(self.imgs.button)
                .w_h(90.0, 22.0)
                .bottom_left_with_margins_on(state.ids.bg, 15.0, 15.0)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .label(&format!(
                    "[{}] {}",
                    &accept_key,
                    self.localized_strings.get_msg("common-accept")
                ))
                .label_color(TEXT_COLOR)
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_font_size(self.fonts.cyri.scale(12))
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
                .map_or_else(|| "".into(), |key| key.display_string(key_layout));
            if Button::image(self.imgs.button)
                .w_h(90.0, 22.0)
                .bottom_right_with_margins_on(state.ids.bg, 15.0, 15.0)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .label(&format!(
                    "[{}] {}",
                    &decline_key,
                    self.localized_strings.get_msg("common-decline")
                ))
                .label_color(TEXT_COLOR)
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_font_size(self.fonts.cyri.scale(12))
                .set(state.ids.btn_decline, ui)
                .was_clicked()
            {
                events.push(Event::Decline);
            };
        }

        events
    }
}
