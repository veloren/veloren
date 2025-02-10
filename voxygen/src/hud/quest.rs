use client::{Client, EcsEntity};
use common::{comp::ItemKey, rtsim};
use conrod_core::{
    Color, Colorable, Positionable, Sizeable, UiCell, Widget, WidgetCommon, color,
    widget::{self, Button, Image, Rectangle, Scrollbar, Text},
    widget_ids,
};
use i18n::Localization;
use std::time::{Duration, Instant};

use crate::ui::{TooltipManager, fonts::Fonts};
use inline_tweak::*;

use super::{
    Show, TEXT_COLOR, UI_HIGHLIGHT_0, UI_MAIN, animate_by_pulse,
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
        icon,
        close,
        title_align,
        title,
        text_align,
        topics_align,
        scrollbar,
        intro_txt,
        desc_txt_0,
        quest_objectives[],
        quest_response_txt,
        objective_text,
        quest_responses_frames[],
        quest_responses_btn[],
        quest_responses_icons[],
        quest_responses_amounts[],
        quest_rewards_txts[],
        accept_btn,
        decline_btn,
    }
}

#[derive(WidgetCommon)]
pub struct Quest<'a> {
    _show: &'a Show,
    _client: &'a Client,
    imgs: &'a Imgs,
    fonts: &'a Fonts,
    localized_strings: &'a Localization,
    _rot_imgs: &'a ImgsRot,
    _tooltip_manager: &'a mut TooltipManager,
    item_imgs: &'a ItemImgs,
    sender: EcsEntity,
    dialogue: &'a rtsim::Dialogue<true>,
    pulse: f32,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> Quest<'a> {
    pub fn new(
        _show: &'a Show,
        _client: &'a Client,
        imgs: &'a Imgs,
        fonts: &'a Fonts,
        localized_strings: &'a Localization,
        _rot_imgs: &'a ImgsRot,
        _tooltip_manager: &'a mut TooltipManager,
        item_imgs: &'a ItemImgs,
        sender: EcsEntity,
        dialogue: &'a rtsim::Dialogue<true>,
        pulse: f32,
    ) -> Self {
        Self {
            _show,
            _client,
            imgs,
            _rot_imgs,
            fonts,
            localized_strings,
            _tooltip_manager,
            item_imgs,
            sender,
            dialogue,
            pulse,
            common: widget::CommonBuilder::default(),
        }
    }

    fn update_text(&self, state: &mut State, ui: &mut UiCell, msg_text: &str) {
        let now = Instant::now();

        // Check if we have a new message
        let is_new_message = state.text_position == 0
            || state.text_position > msg_text.len()
            || state.last_displayed_text.as_deref() != Some(msg_text);

        if is_new_message {
            state.text_timer = Some(now);
            state.text_position = 1; // Start displaying from the first character
            state.last_displayed_text = Some(msg_text.to_string()); // Store the message
        }

        if state.text_timer.is_none() {
            state.text_timer = Some(now);
        }

        if let Some(start_time) = state.text_timer {
            if now.duration_since(start_time) >= Duration::from_millis(50) {
                if state.text_position < msg_text.len() {
                    state.text_position += 1;
                    state.text_timer = Some(now);
                }
            }
        }

        let display_text = &msg_text[..state.text_position.min(msg_text.len())];
        Text::new(display_text)
            .top_left_with_margins_on(state.ids.text_align, tweak!(8.0), tweak!(8.0))
            .w(500.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(tweak!(20)))
            .color(TEXT_COLOR)
            .set(state.ids.desc_txt_0, ui);
    }
}

pub enum Event {
    Dialogue(EcsEntity, rtsim::Dialogue),
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
        let state = state;
        let mut event = None;

        // Window BG
        Image::new(self.imgs.dialogue_bg)
            .mid_bottom_with_margin_on(ui.window, 80.0)
            .color(Some(UI_MAIN))
            .w_h(720.0, 234.0)
            .set(state.ids.bg, ui);
        // Window frame
        Image::new(self.imgs.dialogue_frame)
            .middle_of(state.ids.bg)
            .color(Some(UI_HIGHLIGHT_0))
            .w_h(720.0, 234.0)
            .set(state.ids.frame, ui);

        // // X-Button
        // if Button::image(self.imgs.close_button)
        //     .w_h(24.0, 25.0)
        //     .hover_image(self.imgs.close_button_hover)
        //     .press_image(self.imgs.close_button_press)
        //     .top_right_with_margins_on(state.ids.frame, 0.0, 0.0)
        //     .set(state.ids.close, ui)
        //     .was_clicked()
        // {
        //     event = Some(Event::Close);
        // }

        // Content Alignment
        // Text Left
        Rectangle::fill_with([tweak!(529.0), tweak!(230.0)], color::TRANSPARENT)
            .top_left_with_margins_on(state.ids.frame, tweak!(2.0), tweak!(2.0))
            .scroll_kids_vertically()
            .set(state.ids.text_align, ui);
        // Topics Right
        Rectangle::fill_with([tweak!(186.0), tweak!(230.0)], color::TRANSPARENT)
            .top_right_with_margins_on(state.ids.frame, tweak!(2.0), tweak!(2.0))
            .scroll_kids_vertically()
            .set(state.ids.topics_align, ui);

        // Define type of quest to change introduction text
        let msg_text = match &self.dialogue.kind {
            rtsim::DialogueKind::Start | rtsim::DialogueKind::End => None,
            rtsim::DialogueKind::Statement(msg) => Some(self.localized_strings.get_content(msg)),
            rtsim::DialogueKind::Question { msg, .. } => {
                Some(self.localized_strings.get_content(msg))
            },
            rtsim::DialogueKind::Response { response, .. } => {
                Some(self.localized_strings.get_content(&response.msg))
            },
        };

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
                let frame = Button::image(self.imgs.nothing).w_h(186.0, 40.0);
                let frame = if i == 0 {
                    frame.top_left_with_margins_on(
                        state.ids.topics_align,
                        tweak!(20.0),
                        tweak!(2.0),
                    )
                } else {
                    frame.down_from(state.ids.quest_responses_frames[i - 1], tweak!(10.0))
                };
                frame.set(state.ids.quest_responses_frames[i], ui);

                // Slot BG
                if Button::image(self.imgs.nothing)
                    .w_h(120.0, 40.0)
                    .hover_image(self.imgs.nothing)
                    .press_image(self.imgs.nothing)
                    .middle_of(state.ids.quest_responses_frames[i])
                    .set(state.ids.quest_responses_btn[i], ui)
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

                // Item image
                if let Some((item, amount)) = &response.given_item {
                    Image::new(animate_by_pulse(
                        &self
                            .item_imgs
                            .img_ids_or_not_found_img(ItemKey::from(&**item)),
                        self.pulse,
                    ))
                    .middle_of(state.ids.quest_responses_btn[i])
                    .w_h(20.0, 20.0)
                    .graphics_for(state.ids.quest_responses_btn[i])
                    .set(state.ids.quest_responses_icons[i], ui);

                    if *amount > 0 {
                        Text::new(&format!("x{amount}"))
                            .mid_bottom_with_margin_on(state.ids.quest_responses_frames[i], 3.0)
                            .font_id(self.fonts.cyri.conrod_id)
                            .font_size(self.fonts.cyri.scale(12))
                            .color(TEXT_COLOR)
                            .wrap_by_word()
                            .set(state.ids.quest_responses_amounts[i], ui);
                    }
                }

                Text::new(&self.localized_strings.get_content(&response.msg))
                    .middle_of(state.ids.quest_responses_btn[i])
                    .graphics_for(state.ids.quest_responses_btn[i])
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(Color::Rgba(1.0, 1.0, 1.0, 1.0))
                    .font_size(self.fonts.cyri.scale(tweak!(14)))
                    .set(state.ids.quest_rewards_txts[i], ui);
            }
        }

        event
    }
}
