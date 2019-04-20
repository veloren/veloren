use crate::ui::Ui;
use conrod_core::{
    color,
    input::Key,
    position::Dimension,
    text::font::Id as FontId,
    widget::{Button, Id, List, Rectangle, Text, TextEdit},
    widget_ids, Colorable, Positionable, Sizeable, UiCell, Widget,
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
// Chat Behaviour:
// Input Window is only shown when the player presses Enter (graphical overlay to make it look better?)
// Instead of a Scrollbar it could have 3 Arrows at it's left side
// First two: Scroll the chat up and down
// Last one: Gets back to the bottom of the chat

// Consider making this a custom Widget
pub struct Chat {
    ids: Ids,
    messages: VecDeque<String>,
    input: String,
    new_messages: bool,
}
impl Chat {
    pub fn new(ui: &mut Ui) -> Self {
        Chat {
            ids: Ids::new(ui.id_generator()),
            messages: VecDeque::new(),
            input: String::new(),
            new_messages: false,
        }
    }
    pub fn input_box_id(&self) -> Id {
        self.ids.input
    }
    pub fn new_message(&mut self, msg: String) {
        self.messages.push_back(msg);
        self.new_messages = true;
    }
    // Determine if the message box is scrolled to the bottom
    // (i.e. the player is viewing new messages)
    // If so scroll down when new messages are added
    fn scroll_new_messages(&self, ui_widgets: &mut UiCell) {
        // If previously scrolled to the bottom stay there
        if self.scrolled_to_bottom(ui_widgets) {
            self.scroll_to_bottom(ui_widgets);
        }
    }
    fn scrolled_to_bottom(&self, ui_widgets: &UiCell) -> bool {
        // could be more efficient to cache result and update it when a scroll event has occurred instead of every frame
        if let Some(scroll) = ui_widgets
            .widget_graph()
            .widget(self.ids.message_box)
            .and_then(|widget| widget.maybe_y_scroll_state)
        {
            scroll.offset >= scroll.offset_bounds.start
        } else {
            false
        }
    }
    fn scroll_to_bottom(&self, ui_widgets: &mut UiCell) {
        ui_widgets.scroll_widget(self.ids.message_box, [0.0, std::f64::MAX]);
    }
    pub(super) fn update_layout(
        &mut self,
        ui_widgets: &mut UiCell,
        font: FontId,
        imgs: &super::Imgs,
    ) -> Option<String> {
        // Maintain scrolling
        if self.new_messages {
            self.scroll_new_messages(ui_widgets);
            self.new_messages = false;
        }

        // Only show if it has the keyboard captured
        // Chat input with rectangle as background
        let keyboard_captured = ui_widgets.global_input().current.widget_capturing_keyboard.map_or(false, |id| id == self.ids.input);
        if keyboard_captured {
            let text_edit = TextEdit::new(&self.input)
                .w(460.0)
                .restrict_to_height(false)
                .line_spacing(2.0)
                .font_size(15)
                .font_id(font);
            let y = match text_edit.get_y_dimension(ui_widgets) {
                Dimension::Absolute(y) => y + 6.0,
                _ => 0.0,
            };
            Rectangle::fill([470.0, y])
                .rgba(0.0, 0.0, 0.0, 0.8)
                .bottom_left_with_margins_on(ui_widgets.window, 10.0, 10.0)
                .w(470.0)
                .set(self.ids.input_bg, ui_widgets);
            if let Some(str) = text_edit
                .top_left_with_margins_on(self.ids.input_bg, 1.0, 1.0)
                .set(self.ids.input, ui_widgets)
            {
                self.input = str.to_string();
                self.input.retain(|c| c != '\n');
            }
        }

        // Message box
        Rectangle::fill([470.0, 174.0])
            .rgba(0.0, 0.0, 0.0, 0.4)
            .and(|r| if keyboard_captured {
                r.up_from(self.ids.input_bg, 0.0)
            } else {
                r.bottom_left_with_margins_on(ui_widgets.window, 10.0, 10.0)
            })
            .set(self.ids.message_box_bg, ui_widgets);
        let (mut items, _) = List::flow_down(self.messages.len() + 1)
            .top_left_of(self.ids.message_box_bg)
            .w_h(470.0, 174.0)
            .scroll_kids_vertically()
            .set(self.ids.message_box, ui_widgets);
        while let Some(item) = items.next(ui_widgets) {
            // This would be easier if conrod used the v-metrics from rusttype
            let widget = if item.i < self.messages.len() {
                let text = Text::new(&self.messages[item.i])
                    .font_size(15)
                    .font_id(font)
                    .w(470.0)
                    .rgba(1.0, 1.0, 1.0, 1.0)
                    .line_spacing(2.0);
                // Add space between messages
                let y = match text.get_y_dimension(ui_widgets) {
                    Dimension::Absolute(y) => y + 2.0,
                    _ => 0.0,
                };
                text.h(y)
            } else {
                // Spacer at bottom of the last message so that it is not cut off
                // Needs to be larger than the space above
                Text::new("").font_size(6).font_id(font).w(470.0)
            };
            item.set(widget, ui_widgets);
        }

        // Chat Arrow
        if !self.scrolled_to_bottom(ui_widgets) {
            if Button::image(imgs.chat_arrow)
                .w_h(22.0, 22.0)
                .hover_image(imgs.chat_arrow_mo)
                .press_image(imgs.chat_arrow_press)
                .bottom_right_with_margins_on(self.ids.message_box_bg, 2.0, 2.0)
                .set(self.ids.chat_arrow, ui_widgets)
                .was_clicked()
            {
                self.scroll_to_bottom(ui_widgets);
            }
        }

        // If enter is pressed and the input box is not empty send the current message
        if ui_widgets
            .widget_input(self.ids.input)
            .presses()
            .key()
            .any(|key_press| match key_press.key {
                Key::Return if !self.input.is_empty() => true,
                _ => false,
            })
        {
            let new_message = self.input.clone();
            self.input.clear();
            Some(new_message)
        } else {
            None
        }
    }
}
