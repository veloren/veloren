use super::{
    img_ids::Imgs, ERROR_COLOR, FACTION_COLOR, GROUP_COLOR, INFO_COLOR, KILL_COLOR, LOOT_COLOR,
    OFFLINE_COLOR, ONLINE_COLOR, REGION_COLOR, SAY_COLOR, TELL_COLOR, WORLD_COLOR,
};
use crate::{i18n::Localization, ui::fonts::Fonts, GlobalState};
use client::{cmd, Client};
use common::comp::{
    chat::{KillSource, KillType},
    ChatMode, ChatMsg, ChatType,
};
use common_net::msg::validate_chat_msg;
use conrod_core::{
    input::Key,
    position::Dimension,
    text::{
        self,
        cursor::{self, Index},
    },
    widget::{self, Button, Id, Image, List, Rectangle, Text, TextEdit},
    widget_ids, Color, Colorable, Positionable, Sizeable, Ui, UiCell, Widget, WidgetCommon,
};
use std::collections::VecDeque;

widget_ids! {
    struct Ids {
        message_box,
        message_box_bg,
        chat_input,
        chat_input_bg,
        chat_input_icon,
        chat_arrow,
        chat_icons[],
    }
}
/*#[const_tweaker::tweak(min = 0.0, max = 60.0, step = 1.0)]
const X: f64 = 18.0;*/

const MAX_MESSAGES: usize = 100;

const CHAT_ICON_WIDTH: f64 = 16.0;
const CHAT_ICON_HEIGHT: f64 = 16.0;
const CHAT_BOX_WIDTH: f64 = 470.0;
const CHAT_BOX_INPUT_WIDTH: f64 = 460.0 - CHAT_ICON_WIDTH - 1.0;
const CHAT_BOX_HEIGHT: f64 = 174.0;

#[derive(WidgetCommon)]
pub struct Chat<'a> {
    new_messages: &'a mut VecDeque<ChatMsg>,
    client: &'a Client,
    force_input: Option<String>,
    force_cursor: Option<Index>,
    force_completions: Option<Vec<String>>,

    global_state: &'a GlobalState,
    imgs: &'a Imgs,
    fonts: &'a Fonts,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,

    // TODO: add an option to adjust this
    history_max: usize,

    localized_strings: &'a Localization,
}

impl<'a> Chat<'a> {
    pub fn new(
        new_messages: &'a mut VecDeque<ChatMsg>,
        client: &'a Client,
        global_state: &'a GlobalState,
        imgs: &'a Imgs,
        fonts: &'a Fonts,
        localized_strings: &'a Localization,
    ) -> Self {
        Self {
            new_messages,
            client,
            force_input: None,
            force_cursor: None,
            force_completions: None,
            imgs,
            fonts,
            global_state,
            common: widget::CommonBuilder::default(),
            history_max: 32,
            localized_strings,
        }
    }

    pub fn prepare_tab_completion(mut self, input: String) -> Self {
        if let Some(index) = input.find('\t') {
            self.force_completions = Some(cmd::complete(&input[..index], &self.client));
        } else {
            self.force_completions = None;
        }
        self
    }

    pub fn input(mut self, input: String) -> Self {
        if let Ok(()) = validate_chat_msg(&input) {
            self.force_input = Some(input);
        }
        self
    }

    pub fn cursor_pos(mut self, index: Index) -> Self {
        self.force_cursor = Some(index);
        self
    }

    fn scrolled_to_bottom(state: &State, ui: &UiCell) -> bool {
        // Might be more efficient to cache result and update it when a scroll event has
        // occurred instead of every frame.
        if let Some(scroll) = ui
            .widget_graph()
            .widget(state.ids.message_box)
            .and_then(|widget| widget.maybe_y_scroll_state)
        {
            scroll.offset >= scroll.offset_bounds.start
        } else {
            false
        }
    }
}

struct InputState {
    message: String,
    mode: ChatMode,
}

pub struct State {
    messages: VecDeque<ChatMsg>,
    input: InputState,
    ids: Ids,
    history: VecDeque<String>,
    // Index into the history Vec, history_pos == 0 is history not in use
    // otherwise index is history_pos -1
    history_pos: usize,
    completions: Vec<String>,
    // Index into the completion Vec
    completions_index: Option<usize>,
    // At which character is tab completion happening
    completion_cursor: Option<usize>,
}

pub enum Event {
    TabCompletionStart(String),
    SendMessage(String),
    Focus(Id),
}

impl<'a> Widget for Chat<'a> {
    type Event = Option<Event>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            input: InputState {
                message: "".to_owned(),
                mode: ChatMode::default(),
            },
            messages: VecDeque::new(),
            history: VecDeque::new(),
            history_pos: 0,
            completions: Vec::new(),
            completions_index: None,
            completion_cursor: None,
            ids: Ids::new(id_gen),
        }
    }

    #[allow(clippy::unused_unit)] // TODO: Pending review in #587
    fn style(&self) -> Self::Style { () }

    #[allow(clippy::redundant_clone)] // TODO: Pending review in #587
    #[allow(clippy::single_match)] // TODO: Pending review in #587
    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { id, state, ui, .. } = args;
        let transp = self.global_state.settings.interface.chat_transp;
        // Maintain scrolling.
        if !self.new_messages.is_empty() {
            state.update(|s| s.messages.extend(self.new_messages.drain(..)));
            ui.scroll_widget(state.ids.message_box, [0.0, std::f64::MAX]);
        }

        // Empty old messages
        state.update(|s| {
            while s.messages.len() > MAX_MESSAGES {
                s.messages.pop_front();
            }
        });

        if let Some(comps) = &self.force_completions {
            state.update(|s| s.completions = comps.clone());
        }

        let mut force_cursor = self.force_cursor;

        // If up or down are pressed: move through history
        // If any key other than up, down, or tab is pressed: stop completion.
        let (history_dir, tab_dir, stop_tab_completion) =
            ui.widget_input(state.ids.chat_input).presses().key().fold(
                (0isize, 0isize, false),
                |(n, m, tc), key_press| match key_press.key {
                    Key::Up => (n + 1, m - 1, tc),
                    Key::Down => (n - 1, m + 1, tc),
                    Key::Tab => (n, m + 1, tc),
                    _ => (n, m, true),
                },
            );

        // Handle tab completion
        let request_tab_completions = if stop_tab_completion {
            // End tab completion
            state.update(|s| {
                if s.completion_cursor.is_some() {
                    s.completion_cursor = None;
                }
                s.completions_index = None;
            });
            false
        } else if let Some(cursor) = state.completion_cursor {
            // Cycle through tab completions of the current word
            if state.input.message.contains('\t') {
                state.update(|s| s.input.message.retain(|c| c != '\t'));
                //tab_dir + 1
            }
            if !state.completions.is_empty() && (tab_dir != 0 || state.completions_index.is_none())
            {
                state.update(|s| {
                    let len = s.completions.len();
                    s.completions_index = Some(
                        (s.completions_index.unwrap_or(0) + (tab_dir + len as isize) as usize)
                            % len,
                    );
                    if let Some(replacement) = &s.completions.get(s.completions_index.unwrap()) {
                        let (completed, offset) =
                            do_tab_completion(cursor, &s.input.message, replacement);
                        force_cursor = cursor_offset_to_index(offset, &completed, &ui, &self.fonts);
                        s.input.message = completed;
                    }
                });
            }
            false
        } else if let Some(cursor) = state.input.message.find('\t') {
            // Begin tab completion
            state.update(|s| s.completion_cursor = Some(cursor));
            true
        } else {
            // Not tab completing
            false
        };

        // Move through history
        if history_dir != 0 && state.completion_cursor.is_none() {
            state.update(|s| {
                if history_dir > 0 {
                    if s.history_pos < s.history.len() {
                        s.history_pos += 1;
                    }
                } else if s.history_pos > 0 {
                    s.history_pos -= 1;
                }
                if s.history_pos > 0 {
                    s.input.message = s.history.get(s.history_pos - 1).unwrap().to_owned();
                    force_cursor = cursor_offset_to_index(
                        s.input.message.len(),
                        &s.input.message,
                        &ui,
                        &self.fonts,
                    );
                } else {
                    s.input.message.clear();
                }
            });
        }

        let keyboard_capturer = ui.global_input().current.widget_capturing_keyboard;

        if let Some(input) = &self.force_input {
            state.update(|s| s.input.message = input.to_string());
        }

        let input_focused =
            keyboard_capturer == Some(state.ids.chat_input) || keyboard_capturer == Some(id);

        // Only show if it has the keyboard captured.
        // Chat input uses a rectangle as its background.
        if input_focused {
            // Shallow comparison of ChatMode.
            let discrim = |x| std::mem::discriminant(x);
            if discrim(&state.input.mode) != discrim(&self.client.chat_mode) {
                state.update(|s| {
                    s.input.mode = self.client.chat_mode.clone();
                });
            }

            let (color, icon) = render_chat_mode(&state.input.mode, &self.imgs);
            Image::new(icon)
                .w_h(CHAT_ICON_WIDTH, CHAT_ICON_HEIGHT)
                .top_left_with_margin_on(state.ids.chat_input_bg, 2.0)
                .set(state.ids.chat_input_icon, ui);

            // Any changes to this TextEdit's width and font size must be reflected in
            // `cursor_offset_to_index` below.
            let mut text_edit = TextEdit::new(&state.input.message)
                .w(CHAT_BOX_INPUT_WIDTH)
                .restrict_to_height(false)
                .color(color)
                .line_spacing(2.0)
                .font_size(self.fonts.opensans.scale(15))
                .font_id(self.fonts.opensans.conrod_id);

            if let Some(pos) = force_cursor {
                text_edit = text_edit.cursor_pos(pos);
            }

            let y = match text_edit.get_y_dimension(ui) {
                Dimension::Absolute(y) => y + 6.0,
                _ => 0.0,
            };
            Rectangle::fill([CHAT_BOX_WIDTH, y])
                .rgba(0.0, 0.0, 0.0, transp + 0.1)
                .bottom_left_with_margins_on(ui.window, 10.0, 10.0)
                .w(CHAT_BOX_WIDTH)
                .set(state.ids.chat_input_bg, ui);

            if let Some(str) = text_edit
                .right_from(state.ids.chat_input_icon, 1.0)
                .set(state.ids.chat_input, ui)
            {
                let mut input = str.to_owned();
                input.retain(|c| c != '\n');
                if let Ok(()) = validate_chat_msg(&input) {
                    state.update(|s| s.input.message = input);
                }
            }
        }

        // Message box
        Rectangle::fill([CHAT_BOX_WIDTH, CHAT_BOX_HEIGHT])
            .rgba(0.0, 0.0, 0.0, transp)
            .and(|r| {
                if input_focused {
                    r.up_from(state.ids.chat_input_bg, 0.0)
                } else {
                    r.bottom_left_with_margins_on(ui.window, 10.0, 10.0)
                }
            })
            .crop_kids()
            .set(state.ids.message_box_bg, ui);
        let (mut items, _) = List::flow_down(state.messages.len() + 1)
            .top_left_with_margins_on(state.ids.message_box_bg, 0.0, 16.0)
            .w_h(CHAT_BOX_WIDTH - 16.0, CHAT_BOX_HEIGHT)
            .scroll_kids_vertically()
            .set(state.ids.message_box, ui);
        if state.ids.chat_icons.len() < state.messages.len() {
            state.update(|s| {
                s.ids
                    .chat_icons
                    .resize(s.messages.len(), &mut ui.widget_id_generator())
            });
        }

        let show_char_name = self.global_state.settings.interface.chat_character_name;
        while let Some(item) = items.next(ui) {
            // This would be easier if conrod used the v-metrics from rusttype.
            if item.i < state.messages.len() {
                let mut message = state.messages[item.i].clone();
                let (color, icon) = render_chat_line(&message.chat_type, &self.imgs);
                let ChatMsg { chat_type, .. } = &message;
                // For each ChatType needing localization get/set matching pre-formatted
                // localized string. This string will be formatted with the data
                // provided in ChatType in the client/src/mod.rs
                // fn format_message called below
                message.message = match chat_type {
                    ChatType::Online(_) => self
                        .localized_strings
                        .get("hud.chat.online_msg")
                        .to_string(),
                    ChatType::Offline(_) => self
                        .localized_strings
                        .get("hud.chat.offline_msg")
                        .to_string(),
                    ChatType::Kill(kill_source, _) => match kill_source {
                        KillSource::Player(_, KillType::Buff(_)) => self
                            .localized_strings
                            .get("hud.chat.pvp_buff_kill_msg")
                            .to_string(),
                        KillSource::Player(_, KillType::Melee) => self
                            .localized_strings
                            .get("hud.chat.pvp_melee_kill_msg")
                            .to_string(),
                        KillSource::Player(_, KillType::Projectile) => self
                            .localized_strings
                            .get("hud.chat.pvp_ranged_kill_msg")
                            .to_string(),
                        KillSource::Player(_, KillType::Explosion) => self
                            .localized_strings
                            .get("hud.chat.pvp_explosion_kill_msg")
                            .to_string(),
                        KillSource::Player(_, KillType::Energy) => self
                            .localized_strings
                            .get("hud.chat.pvp_energy_kill_msg")
                            .to_string(),
                        KillSource::Player(_, KillType::Other) => self
                            .localized_strings
                            .get("hud.chat.pvp_other_kill_msg")
                            .to_string(),
                        KillSource::NonExistent(KillType::Buff(_)) => self
                            .localized_strings
                            .get("hud.chat.nonexistent_buff_kill_msg")
                            .to_string(),
                        KillSource::NonPlayer(_, KillType::Buff(_)) => self
                            .localized_strings
                            .get("hud.chat.npc_buff_kill_msg")
                            .to_string(),
                        KillSource::NonPlayer(_, KillType::Melee) => self
                            .localized_strings
                            .get("hud.chat.npc_melee_kill_msg")
                            .to_string(),
                        KillSource::NonPlayer(_, KillType::Projectile) => self
                            .localized_strings
                            .get("hud.chat.npc_ranged_kill_msg")
                            .to_string(),
                        KillSource::NonPlayer(_, KillType::Explosion) => self
                            .localized_strings
                            .get("hud.chat.npc_explosion_kill_msg")
                            .to_string(),
                        KillSource::NonPlayer(_, KillType::Energy) => self
                            .localized_strings
                            .get("hud.chat.npc_energy_kill_msg")
                            .to_string(),
                        KillSource::NonPlayer(_, KillType::Other) => self
                            .localized_strings
                            .get("hud.chat.npc_other_kill_msg")
                            .to_string(),
                        KillSource::Environment(_) => self
                            .localized_strings
                            .get("hud.chat.environmental_kill_msg")
                            .to_string(),
                        KillSource::FallDamage => self
                            .localized_strings
                            .get("hud.chat.fall_kill_msg")
                            .to_string(),
                        KillSource::Suicide => self
                            .localized_strings
                            .get("hud.chat.suicide_msg")
                            .to_string(),
                        KillSource::NonExistent(_) | KillSource::Other => self
                            .localized_strings
                            .get("hud.chat.default_death_msg")
                            .to_string(),
                    },
                    _ => message.message,
                };
                let msg = self.client.format_message(&message, show_char_name);
                let text = Text::new(&msg)
                    .font_size(self.fonts.opensans.scale(15))
                    .font_id(self.fonts.opensans.conrod_id)
                    .w(CHAT_BOX_WIDTH - 17.0)
                    .color(color)
                    .line_spacing(2.0);
                // Add space between messages.
                let y = match text.get_y_dimension(ui) {
                    Dimension::Absolute(y) => y + 2.0,
                    _ => 0.0,
                };
                item.set(text.h(y), ui);
                let icon_id = state.ids.chat_icons[item.i];
                Image::new(icon)
                    .w_h(CHAT_ICON_WIDTH, CHAT_ICON_HEIGHT)
                    .top_left_with_margins_on(item.widget_id, 2.0, -CHAT_ICON_WIDTH)
                    .parent(state.ids.message_box_bg)
                    .set(icon_id, ui);
            } else {
                // Spacer at bottom of the last message so that it is not cut off.
                // Needs to be larger than the space above.
                item.set(
                    Text::new("")
                        .font_size(self.fonts.opensans.scale(6))
                        .font_id(self.fonts.opensans.conrod_id)
                        .w(CHAT_BOX_WIDTH),
                    ui,
                );
            };
        }

        // Chat Arrow
        // Check if already at bottom.
        if !Self::scrolled_to_bottom(state, ui)
            && Button::image(self.imgs.chat_arrow)
                .w_h(20.0, 20.0)
                .hover_image(self.imgs.chat_arrow_mo)
                .press_image(self.imgs.chat_arrow_press)
                .top_right_with_margins_on(state.ids.message_box_bg, 0.0, -22.0)
                .parent(id)
                .set(state.ids.chat_arrow, ui)
                .was_clicked()
        {
            ui.scroll_widget(state.ids.message_box, [0.0, std::f64::MAX]);
        }

        // We've started a new tab completion. Populate tab completion suggestions.
        if request_tab_completions {
            Some(Event::TabCompletionStart(state.input.message.to_string()))
        // If the chat widget is focused, return a focus event to pass the focus
        // to the input box.
        } else if keyboard_capturer == Some(id) {
            Some(Event::Focus(state.ids.chat_input))
        }
        // If enter is pressed and the input box is not empty, send the current message.
        else if ui.widget_input(state.ids.chat_input).presses().key().any(
            |key_press| matches!(key_press.key, Key::Return if !state.input.message.is_empty()),
        ) {
            let msg = state.input.message.clone();
            state.update(|s| {
                s.input.message.clear();
                // Update the history
                // Don't add if this is identical to the last message in the history
                s.history_pos = 0;
                if s.history.get(0).map_or(true, |h| h != &msg) {
                    s.history.push_front(msg.clone());
                    s.history.truncate(self.history_max);
                }
            });
            Some(Event::SendMessage(msg))
        } else {
            None
        }
    }
}

fn do_tab_completion(cursor: usize, input: &str, word: &str) -> (String, usize) {
    let mut pre_ws = None;
    let mut post_ws = None;
    for (char_i, (byte_i, c)) in input.char_indices().enumerate() {
        if c.is_whitespace() && c != '\t' {
            if char_i < cursor {
                pre_ws = Some(byte_i);
            } else {
                post_ws = Some(byte_i);
                break;
            }
        }
    }

    match (pre_ws, post_ws) {
        (None, None) => (word.to_string(), word.chars().count()),
        (None, Some(i)) => (
            format!("{}{}", word, input.split_at(i).1),
            word.chars().count(),
        ),
        (Some(i), None) => {
            let l_split = input.split_at(i).0;
            let completed = format!("{} {}", l_split, word);
            (
                completed,
                l_split.chars().count() + 1 + word.chars().count(),
            )
        },
        (Some(i), Some(j)) => {
            let l_split = input.split_at(i).0;
            let r_split = input.split_at(j).1;
            let completed = format!("{} {}{}", l_split, word, r_split);
            (
                completed,
                l_split.chars().count() + 1 + word.chars().count(),
            )
        },
    }
}

fn cursor_offset_to_index(offset: usize, text: &str, ui: &Ui, fonts: &Fonts) -> Option<Index> {
    // This moves the cursor to the given offset. Conrod is a pain.
    //
    // Width and font must match that of the chat TextEdit
    let font = ui.fonts.get(fonts.opensans.conrod_id)?;
    let font_size = fonts.opensans.scale(15);
    let infos = text::line::infos(&text, &font, font_size).wrap_by_whitespace(CHAT_BOX_INPUT_WIDTH);

    cursor::index_before_char(infos, offset)
}

/// Get the color and icon for a client's ChatMode.
fn render_chat_mode(chat_mode: &ChatMode, imgs: &Imgs) -> (Color, conrod_core::image::Id) {
    match chat_mode {
        ChatMode::World => (WORLD_COLOR, imgs.chat_world_small),
        ChatMode::Say => (SAY_COLOR, imgs.chat_say_small),
        ChatMode::Region => (REGION_COLOR, imgs.chat_region_small),
        ChatMode::Faction(_) => (FACTION_COLOR, imgs.chat_faction_small),
        ChatMode::Group(_) => (GROUP_COLOR, imgs.chat_group_small),
        ChatMode::Tell(_) => (TELL_COLOR, imgs.chat_tell_small),
    }
}

/// Get the color and icon for the current line in the chat box
fn render_chat_line(chat_type: &ChatType<String>, imgs: &Imgs) -> (Color, conrod_core::image::Id) {
    match chat_type {
        ChatType::Online(_) => (ONLINE_COLOR, imgs.chat_online_small),
        ChatType::Offline(_) => (OFFLINE_COLOR, imgs.chat_offline_small),
        ChatType::CommandError => (ERROR_COLOR, imgs.chat_command_error_small),
        ChatType::CommandInfo => (INFO_COLOR, imgs.chat_command_info_small),
        ChatType::Loot => (LOOT_COLOR, imgs.chat_loot_small),
        ChatType::GroupMeta(_) => (GROUP_COLOR, imgs.chat_group_small),
        ChatType::FactionMeta(_) => (FACTION_COLOR, imgs.chat_faction_small),
        ChatType::Kill(_, _) => (KILL_COLOR, imgs.chat_kill_small),
        ChatType::Tell(_from, _to) => (TELL_COLOR, imgs.chat_tell_small),
        ChatType::Say(_uid) => (SAY_COLOR, imgs.chat_say_small),
        ChatType::Group(_uid, _s) => (GROUP_COLOR, imgs.chat_group_small),
        ChatType::Faction(_uid, _s) => (FACTION_COLOR, imgs.chat_faction_small),
        ChatType::Region(_uid) => (REGION_COLOR, imgs.chat_region_small),
        ChatType::World(_uid) => (WORLD_COLOR, imgs.chat_world_small),
        ChatType::Npc(_uid, _r) => panic!("NPCs can't talk"), // Should be filtered by hud/mod.rs
        ChatType::Meta => (INFO_COLOR, imgs.chat_command_info_small),
    }
}
