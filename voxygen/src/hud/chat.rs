use super::{
    img_ids::Imgs, BROADCAST_COLOR, FACTION_COLOR, GAME_UPDATE_COLOR, GROUP_COLOR, KILL_COLOR,
    META_COLOR, PRIVATE_COLOR, SAY_COLOR, TELL_COLOR, TEXT_COLOR,
};
use crate::{ui::fonts::ConrodVoxygenFonts, GlobalState};
use client::Event as ClientEvent;
use common::{msg::validate_chat_msg, ChatType};
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
// Maximum completions shown at once
const MAX_COMPLETIONS: usize = 10;

#[derive(WidgetCommon)]
pub struct Chat<'a> {
    new_messages: &'a mut VecDeque<ClientEvent>,
    force_input: Option<String>,
    force_cursor: Option<Index>,

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
        new_messages: &'a mut VecDeque<ClientEvent>,
        global_state: &'a GlobalState,
        imgs: &'a Imgs,
        fonts: &'a ConrodVoxygenFonts,
    ) -> Self {
        Self {
            new_messages,
            force_input: None,
            force_cursor: None,
            imgs,
            fonts,
            global_state,
            common: widget::CommonBuilder::default(),
            history_max: 32,
        }
    }

    pub fn input(mut self, input: String) -> Self {
        if let Ok(()) = validate_chat_msg(&input) {
            if input.contains('\t') {
                println!("Contains tab: '{}'", input);
            }
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
    messages: VecDeque<ClientEvent>,
    input: String,
    ids: Ids,
    history: VecDeque<String>,
    // Index into the history Vec, history_pos == 0 is history not in use
    // otherwise index is history_pos -1
    history_pos: usize,
    completions: Vec<String>,
    // Index into the completion Vec, completions_pos == 0 means not in use
    // otherwise index is completions_pos -1
    completions_pos: usize,
}

pub enum Event {
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
            completions_pos: 0,
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style { () }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { id, state, ui, .. } = args;
        let transp = self.global_state.settings.gameplay.chat_transp;
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

        let mut force_cursor = self.force_cursor;

        // If up or down are pressed move through history
        let history_move =
            ui.widget_input(state.ids.chat_input)
                .presses()
                .key()
                .fold(0, |n, key_press| match key_press.key {
                    Key::Up => n + 1,
                    Key::Down => n - 1,
                    _ => n,
                });
        if history_move != 0 {
            state.update(|s| {
                if history_move > 0 {
                    if s.history_pos < s.history.len() {
                        s.history_pos += 1;
                    }
                } else {
                    if s.history_pos > 0 {
                        s.history_pos -= 1;
                    }
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

        // Handle tab-completion
        if let Some(cursor) = state.input.find('\t') {
            state.update(|s| {
                if s.completions_pos > 0 {
                    if s.completions_pos >= s.completions.len() {
                        s.completions_pos = 1;
                    } else {
                        s.completions_pos += 1;
                    }
                } else {
                    // TODO FIXME pull completions from common::cmd
                    s.completions = "a,bc,def,ghi,jklm,nop,qr,stu,v,w,xyz"
                        .split(",")
                        .map(|x| x.to_string())
                        .collect();
                    s.completions_pos = 1;
                }
                //let index = force_cursor;
                //let cursor = index.and_then(|index| cursor_index_to_offset(index, &s.input,
                // ui, &self.fonts)).unwrap_or(0);
                let replacement = &s.completions[s.completions_pos - 1];
                let (completed, offset) = do_tab_completion(cursor, &s.input, replacement);
                force_cursor = cursor_offset_to_index(offset, &completed, &ui, &self.fonts);
                s.input = completed;
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
                    if input.contains('\t') {
                        println!("Contains tab: '{}'", input);
                    }
                    state.update(|s| s.input = input);
                }
            }
        }

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
            let widget = if item.i < state.messages.len() {
                let msg = &state.messages[item.i];
                match msg {
                    ClientEvent::Chat { chat_type, message } => {
                        let color = match chat_type {
                            ChatType::Meta => META_COLOR,
                            ChatType::Tell => TELL_COLOR,
                            ChatType::Chat => TEXT_COLOR,
                            ChatType::Private => PRIVATE_COLOR,
                            ChatType::Broadcast => BROADCAST_COLOR,
                            ChatType::GameUpdate => GAME_UPDATE_COLOR,
                            ChatType::Say => SAY_COLOR,
                            ChatType::Group => GROUP_COLOR,
                            ChatType::Faction => FACTION_COLOR,
                            ChatType::Kill => KILL_COLOR,
                        };
                        let text = Text::new(&message)
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
                        Some(text.h(y))
                    },
                    _ => None,
                }
            } else {
                // Spacer at bottom of the last message so that it is not cut off.
                // Needs to be larger than the space above.
                Some(
                    Text::new("")
                        .font_size(self.fonts.opensans.scale(6))
                        .font_id(self.fonts.opensans.conrod_id)
                        .w(470.0),
                )
            };
            match widget {
                Some(widget) => {
                    item.set(widget, ui);
                },
                None => {},
            }
        }

        // Chat Arrow
        // Check if already at bottom.
        if !Self::scrolled_to_bottom(state, ui) {
            if Button::image(self.imgs.chat_arrow)
                .w_h(20.0, 20.0)
                .hover_image(self.imgs.chat_arrow_mo)
                .press_image(self.imgs.chat_arrow_press)
                .bottom_right_with_margins_on(state.ids.message_box_bg, 0.0, -22.0)
                .set(state.ids.chat_arrow, ui)
                .was_clicked()
            {
                ui.scroll_widget(state.ids.message_box, [0.0, std::f64::MAX]);
            }
        }

        // If the chat widget is focused, return a focus event to pass the focus to the
        // input box.
        if keyboard_capturer == Some(id) {
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

fn cursor_index_to_offset(
    index: text::cursor::Index,
    text: &str,
    ui: &Ui,
    fonts: &ConrodVoxygenFonts,
) -> Option<usize> {
    // Width and font must match that of the chat TextEdit
    let width = 460.0;
    let font = ui.fonts.get(fonts.opensans.conrod_id)?;
    let font_size = fonts.opensans.scale(15);
    let infos = text::line::infos(&text, &font, font_size).wrap_by_whitespace(width);

    text::glyph::index_after_cursor(infos, index)
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
