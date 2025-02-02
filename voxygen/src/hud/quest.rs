use client::{Client, EcsEntity};
use common::{comp::ItemKey, rtsim};
use conrod_core::{
    Color, Colorable, Positionable, Sizeable, Widget, WidgetCommon, color,
    widget::{self, Button, Image, Rectangle, Scrollbar, Text},
    widget_ids,
};
use i18n::Localization;

use crate::ui::{TooltipManager, fonts::Fonts};
use inline_tweak::*;

use super::{
    Show, TEXT_COLOR, UI_HIGHLIGHT_0, UI_MAIN, animate_by_pulse,
    img_ids::{Imgs, ImgsRot},
    item_imgs::ItemImgs,
};

pub struct State {
    ids: Ids,
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
        content_align,
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
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;
        let mut event = None;

        // Window BG
        Image::new(self.imgs.quest_bg)
            .bottom_left_with_margins_on(ui.window, tweak!(308.0), tweak!(500.0))
            .color(Some(UI_MAIN))
            .w_h(280.0, 460.0)
            .set(state.ids.bg, ui);
        // Window frame
        Image::new(self.imgs.quest_frame)
            .middle_of(state.ids.bg)
            .color(Some(UI_HIGHLIGHT_0))
            .w_h(280.0, 460.0)
            .set(state.ids.frame, ui);

        // Icon
        Image::new(self.imgs.quest_ico)
            .w_h(30.0, 30.0)
            .top_left_with_margins_on(state.ids.frame, 6.0, 6.0)
            .set(state.ids.icon, ui);
        // X-Button
        if Button::image(self.imgs.close_button)
            .w_h(24.0, 25.0)
            .hover_image(self.imgs.close_button_hover)
            .press_image(self.imgs.close_button_press)
            .top_right_with_margins_on(state.ids.frame, 0.0, 0.0)
            .set(state.ids.close, ui)
            .was_clicked()
        {
            event = Some(Event::Close);
        }

        // Title
        Rectangle::fill_with([212.0, 42.0], color::TRANSPARENT)
            .top_left_with_margins_on(state.ids.frame, 2.0, 44.0)
            .set(state.ids.title_align, ui);
        Text::new(&self.localized_strings.get_msg("hud-dialogue"))
            .middle_of(state.ids.title_align)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(20))
            .color(TEXT_COLOR)
            .set(state.ids.title, ui);

        // Content Alignment
        Rectangle::fill_with([tweak!(270.0), tweak!(395.0)], color::TRANSPARENT)
            .mid_top_with_margin_on(state.ids.frame, tweak!(55.0))
            .scroll_kids_vertically()
            .set(state.ids.content_align, ui);
        Scrollbar::y_axis(state.ids.content_align)
            .thickness(4.0)
            .color(Color::Rgba(0.79, 1.09, 1.09, 0.0))
            .set(state.ids.scrollbar, ui);

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
            Text::new(&msg_text)
                .top_left_with_margins_on(state.ids.content_align, tweak!(0.0), tweak!(4.0))
                .w(250.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(tweak!(20)))
                .color(TEXT_COLOR)
                .set(state.ids.desc_txt_0, ui);
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
                let frame = Image::new(self.imgs.button).w_h(40.0, 40.0);
                let frame = if i == 0 {
                    frame.down_from(state.ids.desc_txt_0, tweak!(10.0))
                } else {
                    frame.down_from(state.ids.quest_responses_frames[i - 1], tweak!(10.0))
                };
                frame.set(state.ids.quest_responses_frames[i], ui);

                // Slot BG
                if Button::image(self.imgs.button)
                    .w_h(40.0, 40.0)
                    .hover_image(self.imgs.button_hover)
                    .press_image(self.imgs.button_press)
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
                    .right_from(state.ids.quest_responses_frames[i], tweak!(10.0))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(Color::Rgba(1.0, 1.0, 1.0, 1.0))
                    .font_size(self.fonts.cyri.scale(tweak!(18)))
                    .set(state.ids.quest_rewards_txts[i], ui);
            }
        }

        event
    }
}
