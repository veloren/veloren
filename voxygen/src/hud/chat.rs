use conrod_core::{
    builder_methods, color,
    input::Key,
    text::font,
    widget::{self, Button, Id, List, Rectangle, Text, TextEdit},
    position::Dimension,
    UiCell, widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};
use super::{
    img_ids::Imgs,
    font_ids::Fonts,
    TEXT_COLOR,
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

#[derive(WidgetCommon)]
pub struct Chat<'a> {
    new_messages: &'a mut VecDeque<String>,

    imgs: &'a Imgs,
    fonts: &'a Fonts,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    style: (),
}

impl<'a> Chat<'a> {
    pub fn new(new_messages: &'a mut VecDeque<String>, imgs: &'a Imgs, fonts: &'a Fonts) -> Self {
        Self {
            new_messages,
            imgs,
            fonts,
            common: widget::CommonBuilder::default(),
            style: (),
        }
    }

    fn scrolled_to_bottom(state: &State, ui: &UiCell) -> bool {
        // could be more efficient to cache result and update it when a scroll event has occurred instead of every frame
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
    messages: VecDeque<String>,
    input: String,

    ids: Ids,
}

pub enum Event {
    SendMessage(String),
}

impl<'a> Widget for Chat<'a> {
    type State = State;
    type Style = ();
    type Event = Option<Event>;

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            messages: VecDeque::new(),
            input: "".to_owned(),
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {
        ()
    }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs {
            id,
            state,
            ui,
            style,
            ..
        } = args;

        // Maintain scrolling
        if !self.new_messages.is_empty() {
            state.update(|s| s.messages.extend(self.new_messages.drain(..)));
            ui.scroll_widget(state.ids.message_box, [0.0, std::f64::MAX]);
        }

        // Only show if it has the keyboard captured
        // Chat input with rectangle as background
        let keyboard_captured = ui.global_input().current.widget_capturing_keyboard == Some(id);
        if keyboard_captured {
            let text_edit = TextEdit::new(&state.input)
                .w(460.0)
                .restrict_to_height(false)
                .line_spacing(2.0)
                .font_size(15)
                .font_id(self.fonts.opensans);
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
                if keyboard_captured {
                    r.up_from(self.ids.input_bg, 0.0)
                } else {
                    r.bottom_left_with_margins_on(ui_widgets.window, 10.0, 10.0)
                }
            })
            .set(state.ids.message_box_bg, ui);
        let (mut items, _) = List::flow_down(state.messages.len() + 1)
            .top_left_of(state.ids.message_box_bg)
            .w_h(470.0, 174.0)
            .scroll_kids_vertically()
            .set(state.ids.message_box, ui);
        while let Some(item) = items.next(ui) {
            // This would be easier if conrod used the v-metrics from rusttype
            let widget = if item.i < state.messages.len() {
                let text = Text::new(&state.messages[item.i])
                    .font_size(15)
                    .font_id(self.fonts.opensans)
                    .w(470.0)
                    .rgba(1.0, 1.0, 1.0, 1.0)
                    .line_spacing(2.0);
                // Add space between messages
                let y = match text.get_y_dimension(ui) {
                    Dimension::Absolute(y) => y + 2.0,
                    _ => 0.0,
                };
                text.h(y)
            } else {
                // Spacer at bottom of the last message so that it is not cut off
                // Needs to be larger than the space above
                Text::new("").font_size(6).font_id(self.fonts.opensans).w(470.0)
            };
            item.set(widget, ui);
        }

        // Chat Arrow
        // Check if already at bottom
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

        // If enter is pressed and the input box is not empty send the current message
        if ui
            .widget_input(state.ids.input)
            .presses()
            .key()
            .any(|key_press| match key_press.key {
                Key::Return if !state.input.is_empty() => true,
                _ => false,
            })
        {
            let msg = state.input.clone();
            state.update(|s| s.input.clear());
            return Some(Event::SendMessage(msg))
        }

        None
    }
}
