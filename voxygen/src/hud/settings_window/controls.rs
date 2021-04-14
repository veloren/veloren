use super::{RESET_BUTTONS_HEIGHT, RESET_BUTTONS_WIDTH};

use crate::{
    hud::{img_ids::Imgs, ERROR_COLOR, TEXT_BIND_CONFLICT_COLOR, TEXT_COLOR},
    i18n::Localization,
    session::settings_change::Control as ControlChange,
    ui::fonts::Fonts,
    window::GameInput,
    GlobalState,
};
use conrod_core::{
    color,
    position::Relative,
    widget::{self, Button, Rectangle, Scrollbar, Text},
    widget_ids, Borderable, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};

widget_ids! {
    struct Ids {
        window,
        window_r,
        window_scrollbar,
        reset_controls_button,
        controls_alignment_rectangle,
        controls_texts[],
        controls_buttons[],
    }
}

#[derive(WidgetCommon)]
pub struct Controls<'a> {
    global_state: &'a GlobalState,
    imgs: &'a Imgs,
    fonts: &'a Fonts,
    localized_strings: &'a Localization,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}
impl<'a> Controls<'a> {
    pub fn new(
        global_state: &'a GlobalState,
        imgs: &'a Imgs,
        fonts: &'a Fonts,
        localized_strings: &'a Localization,
    ) -> Self {
        Self {
            global_state,
            imgs,
            fonts,
            localized_strings,
            common: widget::CommonBuilder::default(),
        }
    }
}

pub struct State {
    ids: Ids,
}

impl<'a> Widget for Controls<'a> {
    type Event = Vec<ControlChange>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    #[allow(clippy::unused_unit)] // TODO: Pending review in #587
    fn style(&self) -> Self::Style { () }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;

        let mut events = Vec::new();
        let key_layout = &self.global_state.window.key_layout;

        Rectangle::fill_with(args.rect.dim(), color::TRANSPARENT)
            .xy(args.rect.xy())
            .graphics_for(args.id)
            .scroll_kids()
            .scroll_kids_vertically()
            .set(state.ids.window, ui);
        Rectangle::fill_with([args.rect.w() / 2.0, args.rect.h()], color::TRANSPARENT)
            .top_right()
            .parent(state.ids.window)
            .set(state.ids.window_r, ui);
        Scrollbar::y_axis(state.ids.window)
            .thickness(5.0)
            .rgba(0.33, 0.33, 0.33, 1.0)
            .set(state.ids.window_scrollbar, ui);

        // Used for sequential placement in a flow-down pattern
        let mut previous_element_id = None;
        let mut keybindings_vec: Vec<GameInput> = GameInput::iterator().collect();
        keybindings_vec.sort();

        let controls = &self.global_state.settings.controls;
        if keybindings_vec.len() > state.ids.controls_texts.len()
            || keybindings_vec.len() > state.ids.controls_buttons.len()
        {
            state.update(|s| {
                s.ids
                    .controls_texts
                    .resize(keybindings_vec.len(), &mut ui.widget_id_generator());
                s.ids
                    .controls_buttons
                    .resize(keybindings_vec.len(), &mut ui.widget_id_generator());
            });
        }

        // Loop all existing keybindings and the ids for text and button widgets
        for (game_input, (&text_id, &button_id)) in keybindings_vec.into_iter().zip(
            state
                .ids
                .controls_texts
                .iter()
                .zip(state.ids.controls_buttons.iter()),
        ) {
            let (key_string, key_color) =
                if self.global_state.window.remapping_keybindings == Some(game_input) {
                    (
                        String::from(self.localized_strings.get("hud.settings.awaitingkey")),
                        TEXT_COLOR,
                    )
                } else if let Some(key) = controls.get_binding(game_input) {
                    (
                        key.display_string(key_layout),
                        if controls.has_conflicting_bindings(key) {
                            TEXT_BIND_CONFLICT_COLOR
                        } else {
                            TEXT_COLOR
                        },
                    )
                } else {
                    (
                        String::from(self.localized_strings.get("hud.settings.unbound")),
                        ERROR_COLOR,
                    )
                };
            let loc_key = self
                .localized_strings
                .get(game_input.get_localization_key());
            let text_widget = Text::new(loc_key)
                .color(TEXT_COLOR)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(18));
            let button_widget = Button::new()
                .label(&key_string)
                .label_color(key_color)
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_font_size(self.fonts.cyri.scale(15))
                .w(150.0)
                .rgba(0.0, 0.0, 0.0, 0.0)
                .border_rgba(0.0, 0.0, 0.0, 255.0)
                .label_y(Relative::Scalar(3.0));
            // Place top-left if it's the first text, else under the previous one
            let text_widget = match previous_element_id {
                None => text_widget.top_left_with_margins_on(state.ids.window, 10.0, 5.0),
                Some(prev_id) => text_widget.down_from(prev_id, 10.0),
            };
            let text_width = text_widget.get_w(ui).unwrap_or(0.0);
            text_widget.set(text_id, ui);
            if button_widget
                .right_from(text_id, 350.0 - text_width)
                .set(button_id, ui)
                .was_clicked()
            {
                events.push(ControlChange::ChangeBinding(game_input));
            }
            // Set the previous id to the current one for the next cycle
            previous_element_id = Some(text_id);
        }

        // Reset the KeyBindings settings to the default settings
        if let Some(prev_id) = previous_element_id {
            if Button::image(self.imgs.button)
                .w_h(RESET_BUTTONS_WIDTH, RESET_BUTTONS_HEIGHT)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .down_from(prev_id, 20.0)
                .label(&self.localized_strings.get("hud.settings.reset_keybinds"))
                .label_font_size(self.fonts.cyri.scale(14))
                .label_color(TEXT_COLOR)
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_y(Relative::Scalar(2.0))
                .set(state.ids.reset_controls_button, ui)
                .was_clicked()
            {
                events.push(ControlChange::ResetKeyBindings);
            }
            previous_element_id = Some(state.ids.reset_controls_button)
        }

        // Add an empty text widget to simulate some bottom margin, because conrod sucks
        if let Some(prev_id) = previous_element_id {
            Rectangle::fill_with([1.0, 1.0], color::TRANSPARENT)
                .down_from(prev_id, 10.0)
                .set(state.ids.controls_alignment_rectangle, ui);
        }

        events
    }
}
