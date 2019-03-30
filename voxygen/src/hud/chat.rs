use crate::ui::Ui;
use conrod_core::{
    input::Key,
    position::Dimension,
    text::font::Id as FontId,
    widget::{Id, List, Rectangle, Text, TextEdit},
    widget_ids, Color, Colorable, Positionable, Sizeable, UiCell, Widget,
};
use std::collections::VecDeque;

widget_ids! {
    struct Ids {
        message_box,
        message_box_bg,
        input,
        input_bg,
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
        self.messages.push_front(msg);
        self.new_messages = true;
    }
    // Determine if the message box is scrolled to the bottom
    // (i.e. the player is viewing new messages)
    // If so scroll down when new messages are added
    fn scroll_new_messages(&mut self, ui_widgets: &mut UiCell) {
        if let Some(scroll) = ui_widgets
            .widget_graph()
            .widget(self.ids.message_box)
            .and_then(|widget| widget.maybe_y_scroll_state)
        {
            // If previously scrolled to the bottom stay there
            if scroll.offset >= scroll.offset_bounds.start {
                ui_widgets.scroll_widget(self.ids.message_box, [0.0, std::f64::MAX]);
            }
        }
    }
    pub fn update_layout(&mut self, ui_widgets: &mut UiCell, font: FontId) -> Option<String> {
        // Maintain scrolling
        if self.new_messages {
            //self.scroll_new_messages(ui_widgets);
            self.new_messages = false;
        }

        // Chat input with rectangle as background
        let text_edit = TextEdit::new(&self.input)
            .w(470.0)
            .restrict_to_height(false)
            .font_size(14)
            .font_id(font)
            .bottom_left_with_margins_on(ui_widgets.window, 10.0, 10.0);
        let dims = match (
            text_edit.get_x_dimension(ui_widgets),
            text_edit.get_y_dimension(ui_widgets),
        ) {
            (Dimension::Absolute(x), Dimension::Absolute(y)) => [x, y],
            _ => [0.0, 0.0],
        };
        Rectangle::fill(dims)
            .rgba(0.0, 0.0, 0.0, 0.8)
            .x_position(text_edit.get_x_position(ui_widgets))
            .y_position(text_edit.get_y_position(ui_widgets))
            .set(self.ids.input_bg, ui_widgets);
        if let Some(str) = text_edit.set(self.ids.input, ui_widgets) {
            self.input = str.to_string();
            self.input.retain(|c| c != '\n');
        }

        // Message box
        Rectangle::fill([470.0, 180.0])
            .rgba(0.0, 0.0, 0.0, 0.4)
            .up_from(self.ids.input, 0.0)
            .set(self.ids.message_box_bg, ui_widgets);
        let (mut items, scrollbar) = List::flow_up(self.messages.len())
            .middle_of(self.ids.message_box_bg)
            .scrollbar_next_to()
            .scrollbar_thickness(18.0)
            .scrollbar_color(Color::Rgba(0.0, 0.0, 0.0, 1.0))
            .set(self.ids.message_box, ui_widgets);
        while let Some(item) = items.next(ui_widgets) {
            item.set(
                Text::new(&self.messages[item.i])
                    .font_size(14)
                    .font_id(font)
                    .rgba(220.0, 220.0, 220.0, 1.0),
                ui_widgets,
            )
        }
        if let Some(s) = scrollbar {
            s.set(ui_widgets)
        }

        // If enter is pressed send the current message
        if ui_widgets
            .widget_input(self.ids.input)
            .presses()
            .key()
            .any(|key_press| match key_press.key {
                Key::Return => true,
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
