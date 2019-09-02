use super::{
    img_ids::Imgs, Fonts, BROADCAST_COLOR, FACTION_COLOR, GAME_UPDATE_COLOR, GROUP_COLOR,
    KILL_COLOR, META_COLOR, PRIVATE_COLOR, SAY_COLOR, TELL_COLOR, TEXT_COLOR,
};
use client::Event as ClientEvent;
use common::ChatType;
use conrod_core::{
    input::Key,
    position::Dimension,
    text::cursor::Index,
    widget::{self, Button, Id, List, Rectangle, Text, TextEdit},
    widget_ids, Colorable, Positionable, Sizeable, UiCell, Widget, WidgetCommon,
};
use std::collections::VecDeque;

widget_ids! {
    struct Ids {
        message_box,
        message_box_bg,
        input,
        input_bg,
        chat_arrow,
    }
}

const MAX_MESSAGES: usize = 100;

#[derive(WidgetCommon)]
pub struct Chat<'a> {
    new_messages: &'a mut VecDeque<ClientEvent>,
    force_input: Option<String>,
    force_cursor: Option<Index>,

    imgs: &'a Imgs,
    fonts: &'a Fonts,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,

    // TODO: add an option to adjust this
    history_max: usize,
}

impl<'a> Chat<'a> {
    pub fn new(
        new_messages: &'a mut VecDeque<ClientEvent>,
        imgs: &'a Imgs,
        fonts: &'a Fonts,
    ) -> Self {
        Self {
            new_messages,
            force_input: None,
            force_cursor: None,
            imgs,
            fonts,
            common: widget::CommonBuilder::default(),
            history_max: 32,
        }
    }

    pub fn input(mut self, input: String) -> Self {
        self.force_input = Some(input);
        self
    }

    pub fn cursor_pos(mut self, index: Index) -> Self {
        self.force_cursor = Some(index);
        self
    }

    fn scrolled_to_bottom(state: &State, ui: &UiCell) -> bool {
        // Might be more efficient to cache result and update it when a scroll event has occurred
        // instead of every frame.
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
}

pub enum Event {
    SendMessage(String),
    Focus(Id),
}

impl<'a> Widget for Chat<'a> {
    type State = State;
    type Style = ();
    type Event = Option<Event>;

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            input: "".to_owned(),
            messages: VecDeque::new(),
            history: VecDeque::new(),
            history_pos: 0,
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {
        ()
    }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { id, state, ui, .. } = args;

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

        // If up or down are pressed move through history
        // TODO: move cursor to the end of the last line
        match ui.widget_input(state.ids.input).presses().key().fold(
            (false, false),
            |(up, down), key_press| match key_press.key {
                Key::Up => (true, down),
                Key::Down => (up, true),
                _ => (up, down),
            },
        ) {
            (true, false) => {
                if state.history_pos < state.history.len() {
                    state.update(|s| {
                        s.history_pos += 1;
                        s.input = s.history.get(s.history_pos - 1).unwrap().to_owned();
                    });
                }
            }
            (false, true) => {
                if state.history_pos > 0 {
                    state.update(|s| {
                        s.history_pos -= 1;
                        if s.history_pos > 0 {
                            s.input = s.history.get(s.history_pos - 1).unwrap().to_owned();
                        } else {
                            s.input.clear();
                        }
                    });
                }
            }
            _ => {}
        }

        let keyboard_capturer = ui.global_input().current.widget_capturing_keyboard;

        if let Some(input) = &self.force_input {
            state.update(|s| s.input = input.clone());
        }

        let input_focused =
            keyboard_capturer == Some(state.ids.input) || keyboard_capturer == Some(id);

        // Only show if it has the keyboard captured.
        // Chat input uses a rectangle as its background.
        if input_focused {
            let mut text_edit = TextEdit::new(&state.input)
                .w(460.0)
                .restrict_to_height(false)
                .color(TEXT_COLOR)
                .line_spacing(2.0)
                .font_size(15)
                .font_id(self.fonts.opensans);

            if let Some(pos) = self.force_cursor {
                text_edit = text_edit.cursor_pos(pos);
            }

            let y = match text_edit.get_y_dimension(ui) {
                Dimension::Absolute(y) => y + 6.0,
                _ => 0.0,
            };
            Rectangle::fill([470.0, y])
                .rgba(0.0, 0.0, 0.0, 0.8)
                .bottom_left_with_margins_on(ui.window, 10.0, 10.0)
                .w(470.0)
                .set(state.ids.input_bg, ui);

            if let Some(str) = text_edit
                .top_left_with_margins_on(state.ids.input_bg, 1.0, 1.0)
                .set(state.ids.input, ui)
            {
                let mut input = str.to_owned();
                input.retain(|c| c != '\n');
                state.update(|s| s.input = input);
            }
        }

        // Message box
        Rectangle::fill([470.0, 174.0])
            .rgba(0.0, 0.0, 0.0, 0.4)
            .and(|r| {
                if input_focused {
                    r.up_from(state.ids.input_bg, 0.0)
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
                            .font_size(15)
                            .font_id(self.fonts.opensans)
                            .w(470.0)
                            .color(color)
                            .line_spacing(2.0);
                        // Add space between messages.
                        let y = match text.get_y_dimension(ui) {
                            Dimension::Absolute(y) => y + 2.0,
                            _ => 0.0,
                        };
                        Some(text.h(y))
                    }
                    _ => None,
                }
            } else {
                // Spacer at bottom of the last message so that it is not cut off.
                // Needs to be larger than the space above.
                Some(
                    Text::new("")
                        .font_size(6)
                        .font_id(self.fonts.opensans)
                        .w(470.0),
                )
            };
            match widget {
                Some(widget) => {
                    item.set(widget, ui);
                }
                None => {}
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

        // If the chat widget is focused, return a focus event to pass the focus to the input box.
        if keyboard_capturer == Some(id) {
            Some(Event::Focus(state.ids.input))
        }
        // If enter is pressed and the input box is not empty, send the current message.
        else if ui
            .widget_input(state.ids.input)
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
