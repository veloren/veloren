use client::Client;
use common::comp::{inventory::item::item_key::ItemKey, Stats};
use conrod_core::{
    color,
    widget::{self, Button, Image, Rectangle, Scrollbar, Text},
    widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};
use i18n::Localization;

use crate::ui::{fonts::Fonts, TooltipManager};
use inline_tweak::*;

use super::{
    img_ids::{Imgs, ImgsRot},
    item_imgs::{animate_by_pulse, ItemImgs},
    Show, HP_COLOR, TEXT_COLOR, TEXT_DULL_RED_COLOR, TEXT_VELORITE, UI_HIGHLIGHT_0, UI_MAIN,
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
        quest_reward_txt,
        objective_text,
        quest_rewards_frames[],
        quest_rewards_icons[],
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
    stats: &'a Stats,
    item_imgs: &'a ItemImgs,
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
        stats: &'a Stats,
        item_imgs: &'a ItemImgs,
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
            stats,
            item_imgs,
            pulse,
            common: widget::CommonBuilder::default(),
        }
    }
}

pub enum Event {
    Close,
}

impl<'a> Widget for Quest<'a> {
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
        Text::new(&self.localized_strings.get_msg("hud-quest"))
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

        // Quest Text

        // Introduction

        Text::new(
            &self
                .localized_strings
                .get_msg_ctx("hud-quest-intro", &i18n::fluent_args! {
                    "playername" => self.stats.name.to_string(),
                }),
        )
        .top_left_with_margins_on(state.ids.content_align, tweak!(0.0), tweak!(2.0))
        .font_id(self.fonts.cyri.conrod_id)
        .font_size(self.fonts.cyri.scale(tweak!(20)))
        .color(TEXT_COLOR)
        .set(state.ids.intro_txt, ui);

        enum QuestType {
            FetchQuest,
            // KillQuest,
        }

        // Define type of quest to change introduction text

        let quest_type = QuestType::FetchQuest;

        let q_desc0 = match quest_type {
            QuestType::FetchQuest => "hud-quest-desc-fetch",
            // QuestType::KillQuest => "hud-quest-desc-kill",
        };

        Text::new(&self.localized_strings.get_msg(q_desc0))
            .top_left_with_margins_on(state.ids.intro_txt, tweak!(40.0), tweak!(0.0))
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(tweak!(20)))
            .color(TEXT_COLOR)
            .set(state.ids.desc_txt_0, ui);

        // Objective(s)
        let objective_amount = 20.0;
        let objective_name = "Flower";
        let objective_txt = format!("{}x {}", objective_amount, objective_name);

        Text::new(&objective_txt)
            .down_from(state.ids.desc_txt_0, tweak!(10.0))
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(tweak!(20)))
            .color(TEXT_VELORITE)
            .set(state.ids.objective_text, ui);

        Text::new(&self.localized_strings.get_msg("hud-quest-reward"))
            .down_from(state.ids.objective_text, tweak!(30.0))
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(tweak!(20)))
            .color(TEXT_COLOR)
            .set(state.ids.quest_reward_txt, ui);

        // insert reward item data here
        // [amount, item_desc]

        //("common.items.weapons.sword.caladbolg");
        let rewards = vec![
            (1, "common.items.weapons.dagger.starter_dagger", "Dagger"),
            (4, "common.items.crafting_ing.seashells", "Seashell"),
            (
                8,
                "common.items.crafting_ing.animal_misc.raptor_feather",
                "Raptor Feather",
            ),
        ];
        let rewards_amount = rewards.len();

        if state.ids.quest_rewards_frames.len() < rewards_amount {
            state.update(|s| {
                s.ids
                    .quest_rewards_frames
                    .resize(rewards.len(), &mut ui.widget_id_generator())
            })
        };
        if state.ids.quest_rewards_icons.len() < rewards_amount {
            state.update(|s| {
                s.ids
                    .quest_rewards_icons
                    .resize(rewards.len(), &mut ui.widget_id_generator())
            })
        };
        if state.ids.quest_rewards_txts.len() < rewards_amount {
            state.update(|s| {
                s.ids
                    .quest_rewards_txts
                    .resize(rewards.len(), &mut ui.widget_id_generator())
            })
        };

        for (i, item) in rewards.iter().enumerate() {
            // Slot BG
            let mut frame_img = Image::new(self.imgs.skillbar_slot)
                .w_h(40.0, 40.0)
                .color(Some(Color::Rgba(1.0, 1.0, 1.0, 1.0)));

            if i == 0 {
                frame_img = frame_img.down_from(state.ids.quest_reward_txt, tweak!(10.0))
            } else {
                frame_img = frame_img.down_from(state.ids.quest_rewards_frames[i - 1], tweak!(5.0))
            }
            frame_img.set(state.ids.quest_rewards_frames[i], ui);

            // Item amount and text
            let item_txt = if item.0 == 1 {
                item.2.to_string()
            } else {
                format!("{}x {}", item.0, item.2)
            };
            //INPUT QUALITY HERE TO CHANGE COLOR
            let item_quality = TEXT_VELORITE;
            Text::new(&item_txt)
                .right_from(state.ids.quest_rewards_frames[i], tweak!(10.0))
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(tweak!(18)))
                .color(item_quality)
                .set(state.ids.quest_rewards_txts[i], ui);
            // Item image
            Image::new(animate_by_pulse(
                &self
                    .item_imgs
                    .img_ids_or_not_found_img(ItemKey::Simple(item.1.to_string())),
                self.pulse,
            ))
            .w_h(38.0, 38.0)
            .middle_of(state.ids.quest_rewards_frames[i])
            .color(Some(Color::Rgba(1.0, 1.0, 1.0, 1.0)))
            .set(state.ids.quest_rewards_icons[i], ui);
        }

        // Accept/Decline Buttons

        if Button::image(self.imgs.button)
            .bottom_left_with_margins_on(state.ids.content_align, tweak!(5.0), tweak!(5.0))
            .w_h(tweak!(120.0), tweak!(50.0))
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .label(&self.localized_strings.get_msg("hud-quest-accept"))
            .label_y(conrod_core::position::Relative::Scalar(3.0))
            .label_color(TEXT_COLOR)
            .label_font_size(self.fonts.cyri.scale(20))
            .label_font_id(self.fonts.cyri.conrod_id)
            .image_color(HP_COLOR)
            .set(state.ids.accept_btn, ui)
            .was_clicked()
        {
            event = Some(Event::Close);
        };

        if Button::image(self.imgs.button)
            .bottom_right_with_margins_on(state.ids.content_align, tweak!(5.0), tweak!(5.0))
            .w_h(tweak!(120.0), tweak!(50.0))
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .label(&self.localized_strings.get_msg("hud-quest-decline"))
            .label_y(conrod_core::position::Relative::Scalar(3.0))
            .label_color(TEXT_COLOR)
            .label_font_size(self.fonts.cyri.scale(20))
            .label_font_id(self.fonts.cyri.conrod_id)
            .image_color(TEXT_DULL_RED_COLOR)
            .set(state.ids.decline_btn, ui)
            .was_clicked()
        {
            event = Some(Event::Close);
        };

        event
    }
}
