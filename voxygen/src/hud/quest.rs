use client::{Client, EcsEntity};
use common::{
    comp::{self, ItemKey},
    rtsim,
};
use conrod_core::{
    Borderable, Color, Colorable, Positionable, Sizeable, UiCell, Widget, WidgetCommon, color,
    widget::{self, Button, Image, Rectangle, Scrollbar, Text},
    widget_ids,
};
use i18n::Localization;
use specs::WorldExt;
use std::{
    borrow::Cow,
    time::{Duration, Instant},
};

use crate::{
    GlobalState,
    ui::{TooltipManager, fonts::Fonts},
};
use inline_tweak::*;

use super::{
    GameInput, Show, TEXT_COLOR, animate_by_pulse,
    img_ids::{Imgs, ImgsRot},
    item_imgs::ItemImgs,
};

pub struct State {
    ids: Ids,
    text_timer: Option<Instant>,
    text_position: usize,
    last_displayed_text: Option<String>, // New field to track the last message
}

widget_ids! {
    pub struct Ids {
        quest_close,
        bg,
        frame,
        close,
        title_align,
        title,
        text_align,
        topics_align,
        scrollbar,
        intro_txt,
        desc_txt_0,
        ack_prompt,
        quest_response_txt,
        objective_text,
        quest_responses_frames[],
        quest_responses_btn[],
        quest_responses_icons[],
        quest_responses_amounts[],
        quest_rewards_txts[],
    }
}

#[derive(WidgetCommon)]
pub struct Quest<'a> {
    _show: &'a Show,
    client: &'a Client,
    _imgs: &'a Imgs,
    fonts: &'a Fonts,
    localized_strings: &'a Localization,
    global_state: &'a GlobalState,
    _rot_imgs: &'a ImgsRot,
    _tooltip_manager: &'a mut TooltipManager,
    item_imgs: &'a ItemImgs,
    sender: EcsEntity,
    dialogue: &'a rtsim::Dialogue<true>,
    recv_time: Instant,
    pulse: f32,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> Quest<'a> {
    pub fn new(
        _show: &'a Show,
        client: &'a Client,
        _imgs: &'a Imgs,
        fonts: &'a Fonts,
        localized_strings: &'a Localization,
        global_state: &'a GlobalState,
        _rot_imgs: &'a ImgsRot,
        _tooltip_manager: &'a mut TooltipManager,
        item_imgs: &'a ItemImgs,
        sender: EcsEntity,
        dialogue: &'a rtsim::Dialogue<true>,
        recv_time: Instant,
        pulse: f32,
    ) -> Self {
        Self {
            _show,
            client,
            _imgs,
            _rot_imgs,
            fonts,
            localized_strings,
            global_state,
            _tooltip_manager,
            item_imgs,
            sender,
            dialogue,
            recv_time,
            pulse,
            common: widget::CommonBuilder::default(),
        }
    }

    fn update_text(&self, state: &mut State, ui: &mut UiCell, msg_text: &str) {
        let now = Instant::now();

        // Check if we have a new message
        let is_new_message = state.text_position == 0
            || state.text_position > msg_text.chars().count()
            || state.last_displayed_text.as_deref() != Some(msg_text);

        if is_new_message {
            state.text_timer = Some(now);
            state.text_position = 1; // Start displaying from the first character
            state.last_displayed_text = Some(msg_text.to_string()); // Store the message
        }

        if state.text_timer.is_none() {
            state.text_timer = Some(now);
        }

        if let Some(start_time) = state.text_timer
            && now.duration_since(start_time) >= Duration::from_millis(10)
            && state.text_position < msg_text.chars().count()
        {
            state.text_position += 1;
            state.text_timer = Some(now);
        }

        let display_text: String = msg_text
            .chars()
            .take(state.text_position.min(msg_text.chars().count()))
            .collect();

        const MARGIN: f64 = 16.0;
        Text::new(&display_text)
            .top_left_with_margins_on(state.ids.text_align, MARGIN, MARGIN)
            .w(429.0 - MARGIN * 2.0)
            .h(200.0 - MARGIN * 2.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(16))
            .color(TEXT_COLOR)
            .set(state.ids.desc_txt_0, ui);
    }
}

pub enum Event {
    Dialogue(EcsEntity, rtsim::Dialogue),
    #[allow(dead_code)]
    Close,
}

impl Widget for Quest<'_> {
    type Event = Option<Event>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        Self::State {
            ids: Ids::new(id_gen),
            text_timer: None,
            text_position: 0,
            last_displayed_text: None,
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;
        let mut event = None;

        // Window BG
        // TODO: It would be nice to use `RoundedRectangle` here, but unfortunately it
        // seems to not propagate scroll events properly!
        Rectangle::fill_with([tweak!(130.0), tweak!(100.0)], color::TRANSPARENT)
            .mid_bottom_with_margin_on(ui.window, 80.0)
            .w_h(749.0, 234.0)
            .set(state.ids.bg, ui);
        // Window frame
        Rectangle::fill_with([tweak!(130.0), tweak!(100.0)], color::TRANSPARENT)
            .middle_of(state.ids.bg)
            .w_h(749.0, tweak!(234.0))
            .set(state.ids.frame, ui);

        const BACKGROUND: Color = Color::Rgba(0.0, 0.0, 0.0, 0.85);

        // Content Alignment
        // Text Left
        Rectangle::fill_with([tweak!(429.0), tweak!(200.0)], BACKGROUND)
            .top_left_with_margins_on(state.ids.frame, tweak!(0.0), tweak!(0.0))
            .scroll_kids_vertically()
            .set(state.ids.text_align, ui);
        // Topics Right
        Rectangle::fill_with([tweak!(315.0), tweak!(200.0)], BACKGROUND)
            .top_right_with_margins_on(state.ids.frame, tweak!(0.0), tweak!(2.0))
            .scroll_kids_vertically()
            .set(state.ids.topics_align, ui);
        Scrollbar::y_axis(state.ids.topics_align)
            .h(tweak!(169.0))
            .top_right_with_margins(29.0, tweak!(4.0))
            .thickness(tweak!(23.0))
            .auto_hide(true)
            .rgba(1.0, 1.0, 1.0, 0.2)
            .set(state.ids.scrollbar, ui);

        // Close Button
        if Button::image(self._imgs.close_btn)
            .w_h(24.0, 25.0)
            .hover_image(self._imgs.close_btn_hover)
            .press_image(self._imgs.close_btn_press)
            .top_right_with_margins_on(state.ids.frame, 2.0, 4.0)
            .set(state.ids.quest_close, ui)
            .was_clicked()
        {
            event = Some(Event::Close);
        }

        if let rtsim::DialogueKind::Statement { .. } = &self.dialogue.kind {
            let recv_time = self.recv_time.elapsed().as_secs_f32();
            Text::new(&if let Some(key) = self
                .global_state
                .settings
                .controls
                .get_binding(GameInput::Interact)
            {
                self.localized_strings.get_msg_ctx(
                    "hud-dialogue-ack",
                    &i18n::fluent_args! { "key" => key.display_string() },
                )
            } else {
                Cow::Borrowed("")
            })
            .bottom_right_with_margins_on(state.ids.text_align, 12.0, 12.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(12))
            .color(Color::Rgba(
                1.0,
                1.0,
                1.0,
                (0.6 + (recv_time * tweak!(5.0)).sin() * 0.4) * (recv_time - 1.0).clamp(0.0, 1.0),
            ))
            .set(state.ids.ack_prompt, ui);
        }

        // Define type of quest to change introduction text
        let msg_text = self
            .dialogue
            .message()
            .map(|msg| self.localized_strings.get_content(msg));

        if let Some(msg_text) = msg_text {
            state.update(|s| {
                self.update_text(s, ui, &msg_text);
            });
        }

        if let rtsim::DialogueKind::Question { responses, tag, .. } = &self.dialogue.kind {
            if state.ids.quest_responses_frames.len() < responses.len() {
                state.update(|s| {
                    s.ids
                        .quest_responses_frames
                        .resize(responses.len(), &mut ui.widget_id_generator())
                })
            };
            if state.ids.quest_responses_icons.len() < responses.len() {
                state.update(|s| {
                    s.ids
                        .quest_responses_icons
                        .resize(responses.len(), &mut ui.widget_id_generator())
                })
            };
            if state.ids.quest_responses_amounts.len() < responses.len() {
                state.update(|s| {
                    s.ids
                        .quest_responses_amounts
                        .resize(responses.len(), &mut ui.widget_id_generator())
                })
            };
            if state.ids.quest_rewards_txts.len() < responses.len() {
                state.update(|s| {
                    s.ids
                        .quest_rewards_txts
                        .resize(responses.len(), &mut ui.widget_id_generator())
                })
            };
            if state.ids.quest_responses_btn.len() < responses.len() {
                state.update(|s| {
                    s.ids
                        .quest_responses_btn
                        .resize(responses.len(), &mut ui.widget_id_generator())
                })
            };

            for (i, (response_id, response)) in responses.iter().enumerate() {
                // Determine whether all requirements for sending the response are met
                let is_valid = if let Some((item, amount)) = &response.given_item {
                    self.client
                        .state()
                        .ecs()
                        .read_storage::<comp::Inventory>()
                        .get(self.client.entity())
                        .is_some_and(|inv| inv.item_count(item) >= *amount as u64)
                } else {
                    true
                };

                let frame = Button::new()
                    .border_color(color::TRANSPARENT)
                    .color(Color::Rgba(1.0, 1.0, 1.0, 0.0))
                    .hover_color(if is_valid {
                        Color::Rgba(1.0, 1.0, 1.0, 0.05)
                    } else {
                        Color::Rgba(1.0, 0.5, 0.5, 0.05)
                    })
                    .press_color(Color::Rgba(1.0, 1.0, 1.0, 0.1))
                    .parent(state.ids.topics_align)
                    .w_h(286.0, 30.0);
                let frame = if i == 0 {
                    frame.top_left_with_margins_on(state.ids.topics_align, tweak!(0.0), tweak!(0.0))
                } else {
                    frame.down_from(state.ids.quest_responses_frames[i - 1], 0.0)
                };
                if frame
                    .set(state.ids.quest_responses_frames[i], ui)
                    .was_clicked()
                {
                    event = Some(Event::Dialogue(self.sender, rtsim::Dialogue {
                        id: self.dialogue.id,
                        kind: rtsim::DialogueKind::Response {
                            tag: *tag,
                            response: response.clone(),
                            response_id: *response_id,
                        },
                    }));
                }

                // Response text
                Text::new(&self.localized_strings.get_content(&response.msg))
                    .middle_of(state.ids.quest_responses_frames[i])
                    .graphics_for(state.ids.quest_responses_frames[i])
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(Color::Rgba(1.0, 1.0, 1.0, if is_valid { 1.0 } else { 0.3 }))
                    .font_size(self.fonts.cyri.scale(tweak!(14)))
                    .set(state.ids.quest_rewards_txts[i], ui);

                // Item image
                if let Some((item, amount)) = &response.given_item {
                    Image::new(animate_by_pulse(
                        &self
                            .item_imgs
                            .img_ids_or_not_found_img(ItemKey::from(&**item)),
                        self.pulse,
                    ))
                    .mid_left_with_margin_on(state.ids.quest_responses_frames[i], 8.0)
                    .w_h(20.0, 20.0)
                    .graphics_for(state.ids.quest_responses_frames[i])
                    .set(state.ids.quest_responses_icons[i], ui);

                    Text::new(&format!("x{amount}"))
                        .mid_left_with_margin_on(state.ids.quest_responses_icons[i], tweak!(24.0))
                        .font_id(self.fonts.cyri.conrod_id)
                        .font_size(self.fonts.cyri.scale(12))
                        .color(if is_valid {
                            TEXT_COLOR
                        } else {
                            // Not enough present!
                            Color::Rgba(1.0, 0.2, 0.2, 0.6 + (self.pulse * 8.0).sin() * 0.4)
                        })
                        .wrap_by_word()
                        .set(state.ids.quest_responses_amounts[i], ui);
                }
            }
        }

        event
    }
}
