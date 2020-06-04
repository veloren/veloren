use super::{
    img_ids::Imgs, BROADCAST_COLOR, FACTION_COLOR, GROUP_COLOR, KILL_COLOR, PRIVATE_COLOR,
    REGION_COLOR, SAY_COLOR, TELL_COLOR, TEXT_COLOR, WORLD_COLOR,
};
use crate::{ui::fonts::ConrodVoxygenFonts, GlobalState};
use client::{cmd, Client};
use common::{
    comp::{ChatMsg, ChatType},
    msg::validate_chat_msg,
};
use conrod_core::{
    input::Key,
    position::Dimension,
    text::{
        self,
        cursor::{self, Index},
    },
    widget::{self, Button, Id, List, Rectangle, Text, TextEdit},
    widget_ids, Colorable, Positionable, Sizeable, Ui, UiCell, Widget, WidgetCommon,
};
use specs::world::WorldExt;
use std::collections::VecDeque;

widget_ids! {
    struct Ids {
        message_box,
        message_box_bg,
        chat_input,
        chat_input_bg,
        chat_arrow,
        completion_box,
    }
}

const MAX_MESSAGES: usize = 100;

#[derive(WidgetCommon)]
pub struct Chat<'a> {
    new_messages: &'a mut VecDeque<ChatMsg>,
    client: &'a Client,
    force_input: Option<String>,
    force_cursor: Option<Index>,
    force_completions: Option<Vec<String>>,

    global_state: &'a GlobalState,
    imgs: &'a Imgs,
    fonts: &'a ConrodVoxygenFonts,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,

    // TODO: add an option to adjust this
    history_max: usize,
}

impl<'a> Chat<'a> {
    pub fn new(
        new_messages: &'a mut VecDeque<ChatMsg>,
        client: &'a Client,
        global_state: &'a GlobalState,
        imgs: &'a Imgs,
        fonts: &'a ConrodVoxygenFonts,
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

pub struct State {
    messages: VecDeque<ChatMsg>,
    input: String,
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
            input: "".to_owned(),
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
        let transp = self.global_state.settings.gameplay.chat_transp;
        // Maintain scrolling.
        if !self.new_messages.is_empty() {
            state.update(|s| {
                s.messages.extend(
                    self.new_messages
                        .drain(..)
                        .map(|msg| {
                            // TODO format!([{}] {}, name, msg)
                            msg
                        })
                        .collect::<Vec<_>>(),
                )
            });
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
            if state.input.contains('\t') {
                state.update(|s| s.input.retain(|c| c != '\t'));
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
                        let (completed, offset) = do_tab_completion(cursor, &s.input, replacement);
                        force_cursor = cursor_offset_to_index(offset, &completed, &ui, &self.fonts);
                        s.input = completed;
                    }
                });
            }
            false
        } else if let Some(cursor) = state.input.find('\t') {
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
                    s.input = s.history.get(s.history_pos - 1).unwrap().to_owned();
                    force_cursor =
                        cursor_offset_to_index(s.input.len(), &s.input, &ui, &self.fonts);
                } else {
                    s.input.clear();
                }
            });
        }

        let keyboard_capturer = ui.global_input().current.widget_capturing_keyboard;

        if let Some(input) = &self.force_input {
            state.update(|s| s.input = input.to_string());
        }

        let input_focused =
            keyboard_capturer == Some(state.ids.chat_input) || keyboard_capturer == Some(id);

        // Only show if it has the keyboard captured.
        // Chat input uses a rectangle as its background.
        if input_focused {
            // Any changes to this TextEdit's width and font size must be reflected in
            // `cursor_offset_to_index` below.
            let mut text_edit = TextEdit::new(&state.input)
                .w(460.0)
                .restrict_to_height(false)
                .color(TEXT_COLOR)
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
            Rectangle::fill([470.0, y])
                .rgba(0.0, 0.0, 0.0, transp + 0.1)
                .bottom_left_with_margins_on(ui.window, 10.0, 10.0)
                .w(470.0)
                .set(state.ids.chat_input_bg, ui);

            if let Some(str) = text_edit
                .top_left_with_margins_on(state.ids.chat_input_bg, 1.0, 1.0)
                .set(state.ids.chat_input, ui)
            {
                let mut input = str.to_owned();
                input.retain(|c| c != '\n');
                if let Ok(()) = validate_chat_msg(&input) {
                    state.update(|s| s.input = input);
                }
            }
        }

        let alias_of_uid = |uid| {
            self.client
                .player_list
                .get(uid)
                .map_or("<?>".to_string(), |player_info| {
                    if player_info.is_admin {
                        format!("ADMIN - {}", player_info.player_alias)
                    } else {
                        player_info.player_alias.to_string()
                    }
                })
        };
        let message_format = |uid, message, group| {
            if let Some(group) = group {
                format!("{{{}}} [{}]: {}", group, alias_of_uid(uid), message)
            } else {
                format!("[{}]: {}", alias_of_uid(uid), message)
            }
        };
        // Message box
        Rectangle::fill([470.0, 174.0])
            .rgba(0.0, 0.0, 0.0, transp)
            .and(|r| {
                if input_focused {
                    r.up_from(state.ids.chat_input_bg, 0.0)
                } else {
                    r.bottom_left_with_margins_on(ui.window, 10.0, 10.0)
                }
            })
            .set(state.ids.message_box_bg, ui);
        let (mut items, _) = List::flow_down(state.messages.len() + 1)
            .top_left_of(state.ids.message_box_bg)
            .w_h(470.0, 174.0)
            .scroll_kids_vertically()
            .set(state.ids.message_box, ui);
        while let Some(item) = items.next(ui) {
            // This would be easier if conrod used the v-metrics from rusttype.
            if item.i < state.messages.len() {
                let ChatMsg { chat_type, message } = &state.messages[item.i];
                let (color, msg) = match chat_type {
                    ChatType::Private => (PRIVATE_COLOR, message.to_string()),
                    ChatType::Broadcast => (BROADCAST_COLOR, message.to_string()),
                    ChatType::Kill => (KILL_COLOR, message.to_string()),
                    ChatType::Tell(from, to) => {
                        let from_alias = alias_of_uid(&from);
                        let to_alias = alias_of_uid(&to);
                        if Some(from)
                            == self
                                .client
                                .state()
                                .ecs()
                                .read_storage()
                                .get(self.client.entity())
                        {
                            (TELL_COLOR, format!("To [{}]: {}", to_alias, message))
                        } else {
                            (TELL_COLOR, format!("From [{}]: {}", from_alias, message))
                        }
                    },
                    ChatType::Say(uid) => (SAY_COLOR, message_format(uid, message, None)),
                    ChatType::Group(uid, s) => (GROUP_COLOR, message_format(uid, message, Some(s))),
                    ChatType::Faction(uid, s) => {
                        (FACTION_COLOR, message_format(uid, message, Some(s)))
                    },
                    ChatType::Region(uid) => (REGION_COLOR, message_format(uid, message, None)),
                    ChatType::World(uid) => (WORLD_COLOR, message_format(uid, message, None)),
                    ChatType::Npc(_uid, _r) => continue, // Should be filtered by hud/mod.rs
                };
                let text = Text::new(&msg)
                    .font_size(self.fonts.opensans.scale(15))
                    .font_id(self.fonts.opensans.conrod_id)
                    .w(470.0)
                    .color(color)
                    .line_spacing(2.0);
                // Add space between messages.
                let y = match text.get_y_dimension(ui) {
                    Dimension::Absolute(y) => y + 2.0,
                    _ => 0.0,
                };
                let widget = text.h(y);
                item.set(widget, ui);
            } else {
                // Spacer at bottom of the last message so that it is not cut off.
                // Needs to be larger than the space above.
                let widget = Text::new("")
                    .font_size(self.fonts.opensans.scale(6))
                    .font_id(self.fonts.opensans.conrod_id)
                    .w(470.0);
                item.set(widget, ui);
            };
        }

        // Chat Arrow
        // Check if already at bottom.
        if !Self::scrolled_to_bottom(state, ui)
            && Button::image(self.imgs.chat_arrow)
                .w_h(20.0, 20.0)
                .hover_image(self.imgs.chat_arrow_mo)
                .press_image(self.imgs.chat_arrow_press)
                .bottom_right_with_margins_on(state.ids.message_box_bg, 0.0, -22.0)
                .set(state.ids.chat_arrow, ui)
                .was_clicked()
        {
            ui.scroll_widget(state.ids.message_box, [0.0, std::f64::MAX]);
        }

        // We've started a new tab completion. Populate tab completion suggestions.
        if request_tab_completions {
            Some(Event::TabCompletionStart(state.input.to_string()))
        // If the chat widget is focused, return a focus event to pass the focus
        // to the input box.
        } else if keyboard_capturer == Some(id) {
            Some(Event::Focus(state.ids.chat_input))
        }
        // If enter is pressed and the input box is not empty, send the current message.
        else if ui
            .widget_input(state.ids.chat_input)
            .presses()
            .key()
            .any(|key_press| match key_press.key {
                Key::Return if !state.input.is_empty() => true,
                _ => false,
            })
        {
            let msg = state.input.clone();
            state.update(|s| {
                s.input.clear();
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
                assert_eq!(post_ws, None); // TODO debug
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

fn cursor_offset_to_index(
    offset: usize,
    text: &str,
    ui: &Ui,
    fonts: &ConrodVoxygenFonts,
) -> Option<Index> {
    // This moves the cursor to the given offset. Conrod is a pain.
    //let iter = cursor::xys_per_line_from_text(&text, &[], &font, font_size,
    // Justify::Left, Align::Start, 2.0, Rect{x: Range{start: 0.0, end: width}, y:
    // Range{start: 0.0, end: 12.345}});
    // cursor::closest_cursor_index_and_xy([f64::MAX, f64::MAX], iter).map(|(i, _)|
    // i) Width and font must match that of the chat TextEdit
    let width = 460.0;
    let font = ui.fonts.get(fonts.opensans.conrod_id)?;
    let font_size = fonts.opensans.scale(15);
    let infos = text::line::infos(&text, &font, font_size).wrap_by_whitespace(width);

    cursor::index_before_char(infos, offset)
}
