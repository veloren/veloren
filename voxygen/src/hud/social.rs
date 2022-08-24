use super::{
    img_ids::{Imgs, ImgsRot},
    Show, TEXT_COLOR, TEXT_COLOR_3, UI_HIGHLIGHT_0, UI_MAIN,
};
use crate::ui::{fonts::Fonts, ImageFrame, Tooltip, TooltipManager, Tooltipable};
use client::{self, Client};
use common::{comp::group, uid::Uid};
use conrod_core::{
    color,
    widget::{self, Button, Image, Rectangle, Scrollbar, Text, TextEdit},
    widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};
use i18n::Localization;
use itertools::Itertools;
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
        player_names[],
        online_txt,
        online_no,
        invite_button,
        player_search_icon,
        player_search_input,
        player_search_input_bg,
        player_search_input_overlay,
    }
}

pub struct State {
    ids: Ids,
    // Holds the time when selection is made since this selection can be overridden
    // by selecting an entity in-game
    selected_uid: Option<(Uid, Instant)>,
}

#[derive(WidgetCommon)]
pub struct Social<'a> {
    show: &'a Show,
    client: &'a Client,
    imgs: &'a Imgs,
    fonts: &'a Fonts,
    localized_strings: &'a Localization,
    selected_entity: Option<(specs::Entity, Instant)>,
    rot_imgs: &'a ImgsRot,
    tooltip_manager: &'a mut TooltipManager,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> Social<'a> {
    pub fn new(
        show: &'a Show,
        client: &'a Client,
        imgs: &'a Imgs,
        fonts: &'a Fonts,
        localized_strings: &'a Localization,
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
    Focus(widget::Id),
    SearchPlayers(Option<String>),
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

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        common_base::prof_span!("Social::update");
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
        .font_id(self.fonts.cyri.conrod_id)
        .desc_text_color(TEXT_COLOR);

        // Window BG
        Image::new(self.imgs.social_bg_on)
            .bottom_left_with_margins_on(ui.window, 308.0, 25.0)
            .color(Some(UI_MAIN))
            .w_h(280.0, 460.0)
            .set(state.ids.bg, ui);
        // Window frame
        Image::new(self.imgs.social_frame_on)
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
        Text::new(&self.localized_strings.get_msg("hud-social"))
            .middle_of(state.ids.title_align)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(20))
            .color(TEXT_COLOR)
            .set(state.ids.title, ui);

        let players = self
            .client
            .player_list()
            .iter()
            .filter(|(_, p)| p.is_online);
        let player_count = players.clone().count();

        // Content Alignment
        Rectangle::fill_with([270.0, 346.0], color::TRANSPARENT)
            .mid_top_with_margin_on(state.ids.frame, 74.0)
            .scroll_kids_vertically()
            .set(state.ids.online_align, ui);
        Scrollbar::y_axis(state.ids.online_align)
            .thickness(4.0)
            .color(Color::Rgba(0.79, 1.09, 1.09, 0.0))
            .set(state.ids.scrollbar, ui);

        // Online Text
        Text::new(&self.localized_strings.get_msg("hud-social-online"))
            .bottom_left_with_margins_on(state.ids.frame, 18.0, 10.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(14))
            .color(TEXT_COLOR)
            .set(state.ids.online_txt, ui);
        Text::new(&player_count.to_string())
            .right_from(state.ids.online_txt, 5.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(14))
            .color(TEXT_COLOR)
            .set(state.ids.online_no, ui);
        // Adjust widget_id struct vec length to player count
        if state.ids.player_names.len() < player_count {
            state.update(|s| {
                s.ids
                    .player_names
                    .resize(player_count, &mut ui.widget_id_generator())
            })
        };

        // Filter out yourself from the online list and perform search
        let my_uid = self.client.uid();
        let mut player_list = players
            .filter(|(uid, _)| Some(**uid) != my_uid)
            .filter(|(_, player)| {
                self.show
                    .social_search_key
                    .as_ref()
                    .map(|search_key| {
                        search_key
                            .to_lowercase()
                            .split_whitespace()
                            .all(|substring| {
                                let player_alias = &player.player_alias.to_lowercase();
                                let character_name = player
                                    .character
                                    .as_ref()
                                    .map(|character| character.name.to_lowercase());
                                player_alias.contains(substring)
                                    || character_name
                                        .map(|cn| cn.contains(substring))
                                        .unwrap_or(false)
                            })
                    })
                    .unwrap_or(true)
            })
            .collect_vec();
        player_list.sort_by_key(|(_, player)| {
            player
                .character
                .as_ref()
                .map(|character| &character.name)
                .unwrap_or(&player.player_alias)
                .to_lowercase()
        });
        for (i, (&uid, player_info)) in player_list.into_iter().enumerate() {
            let hide_username = true;
            let selected = state.selected_uid.map_or(false, |u| u.0 == uid);
            let alias = &player_info.player_alias;
            let name_text = match &player_info.character {
                Some(character) => {
                    if hide_username {
                        character.name.to_string()
                    } else {
                        format!("[{}] {}", alias, &character.name)
                    }
                },
                None => format!(
                    "{} [{}]",
                    alias.clone(),
                    self.localized_strings.get_msg("hud-group-in_menu")
                ), // character select or spectating
            };
            let acc_name_txt = format!(
                "{}: {}",
                &self.localized_strings.get_msg("hud-social-account"),
                alias
            );
            // Player name widget
            let button = Button::image(if !selected {
                self.imgs.nothing
            } else {
                self.imgs.selection
            })
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
            .w_h(260.0, 20.0)
            .image_color(color::rgba(1.0, 0.82, 0.27, 1.0));
            let button = if i == 0 {
                button.mid_top_with_margin_on(state.ids.online_align, 1.0)
            } else {
                button.down_from(state.ids.player_names[i - 1], 1.0)
            };
            if button
                .label(&name_text)
                .label_font_size(self.fonts.cyri.scale(14))
                .label_y(conrod_core::position::Relative::Scalar(1.0))
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_color(TEXT_COLOR)
                .with_tooltip(
                    self.tooltip_manager,
                    &acc_name_txt,
                    "",
                    &button_tooltip,
                    TEXT_COLOR,
                )
                .set(state.ids.player_names[i], ui)
                .was_clicked()
            {
                state.update(|s| s.selected_uid = Some((uid, Instant::now())));
            }
        }

        // Invite Button
        let is_leader_or_not_in_group = self
            .client
            .group_info()
            .map_or(true, |(_, l_uid)| self.client.uid() == Some(l_uid));

        let current_members = self
            .client
            .group_members()
            .iter()
            .filter(|(_, role)| matches!(role, group::Role::Member))
            .count()
            + 1;
        let current_invites = self.client.pending_invites().len();
        let max_members = self.client.max_group_size() as usize;
        let group_not_full = current_members + current_invites < max_members;
        let selected_to_invite = (is_leader_or_not_in_group && group_not_full)
            .then(|| {
                state
                    .selected_uid
                    .as_ref()
                    .map(|(s, _)| *s)
                    .filter(|selected| {
                        self.client
                            .player_list()
                            .get(selected)
                            .map_or(false, |selected_player| {
                                selected_player.is_online && selected_player.character.is_some()
                            })
                    })
                    .or_else(|| {
                        self.selected_entity
                            .and_then(|s| self.client.state().read_component_copied(s.0))
                    })
                    .filter(|selected| {
                        // Prevent inviting entities already in the same group
                        !self.client.group_members().contains_key(selected)
                    })
            })
            .flatten();

        let invite_text = self.localized_strings.get_msg("hud-group-invite");
        let invite_button = Button::image(self.imgs.button)
            .w_h(106.0, 26.0)
            .bottom_right_with_margins_on(state.ids.frame, 9.0, 7.0)
            .hover_image(if selected_to_invite.is_some() {
                self.imgs.button_hover
            } else {
                self.imgs.button
            })
            .press_image(if selected_to_invite.is_some() {
                self.imgs.button_press
            } else {
                self.imgs.button
            })
            .label(&invite_text)
            .label_y(conrod_core::position::Relative::Scalar(3.0))
            .label_color(if selected_to_invite.is_some() {
                TEXT_COLOR
            } else {
                TEXT_COLOR_3
            })
            .image_color(if selected_to_invite.is_some() {
                TEXT_COLOR
            } else {
                TEXT_COLOR_3
            })
            .label_font_size(self.fonts.cyri.scale(15))
            .label_font_id(self.fonts.cyri.conrod_id);

        if if self.client.group_info().is_some() {
            let tooltip_txt = format!(
                "{}/{} {}",
                current_members + current_invites,
                max_members,
                &self.localized_strings.get_msg("hud-group-members")
            );
            invite_button
                .with_tooltip(
                    self.tooltip_manager,
                    &tooltip_txt,
                    "",
                    &button_tooltip,
                    TEXT_COLOR,
                )
                .set(state.ids.invite_button, ui)
        } else {
            invite_button.set(state.ids.invite_button, ui)
        }
        .was_clicked()
        {
            if let Some(uid) = selected_to_invite {
                events.push(Event::Invite(uid));
                state.update(|s| {
                    s.selected_uid = None;
                });
            }
        }

        // Player Search
        if Button::image(self.imgs.search_btn)
            .top_left_with_margins_on(state.ids.frame, 54.0, 9.0)
            .hover_image(self.imgs.search_btn_hover)
            .press_image(self.imgs.search_btn_press)
            .w_h(16.0, 16.0)
            .set(state.ids.player_search_icon, ui)
            .was_clicked()
        {
            events.push(Event::Focus(state.ids.player_search_input));
        }
        Rectangle::fill([248.0, 20.0])
            .top_left_with_margins_on(state.ids.player_search_icon, -2.0, 18.0)
            .hsla(0.0, 0.0, 0.0, 0.7)
            .depth(1.0)
            .parent(state.ids.bg)
            .set(state.ids.player_search_input_bg, ui);
        if let Some(string) =
            TextEdit::new(self.show.social_search_key.as_deref().unwrap_or_default())
                .top_left_with_margins_on(state.ids.player_search_icon, -1.0, 22.0)
                .w_h(215.0, 20.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .color(TEXT_COLOR)
                .set(state.ids.player_search_input, ui)
        {
            events.push(Event::SearchPlayers(Some(string)));
        }
        Rectangle::fill_with([266.0, 20.0], color::TRANSPARENT)
            .top_left_with_margins_on(state.ids.player_search_icon, -1.0, 0.0)
            .graphics_for(state.ids.player_search_icon)
            .set(state.ids.player_search_input_overlay, ui);

        events
    }
}
