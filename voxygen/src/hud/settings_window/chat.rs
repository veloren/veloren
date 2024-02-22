use super::{RESET_BUTTONS_HEIGHT, RESET_BUTTONS_WIDTH};

use crate::{
    hud::{img_ids::Imgs, ChatTab, Show, TEXT_COLOR, TEXT_GRAY_COLOR, UI_HIGHLIGHT_0, UI_MAIN},
    session::settings_change::{Chat as ChatChange, Chat::*},
    settings::chat::MAX_CHAT_TABS,
    ui::{fonts::Fonts, ImageSlider, ToggleButton},
    GlobalState,
};
use conrod_core::{
    color,
    position::Relative,
    widget::{self, Button, DropDownList, Image, Rectangle, Text, TextEdit},
    widget_ids, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};
use i18n::Localization;
use std::cmp::Ordering;

widget_ids! {
    struct Ids {
        window,
        window_r,
        general_txt,
        transp_text,
        transp_slider,
        transp_value,
        char_name_text,
        char_name_button,
        reset_chat_button,

        //Tabs
        tabs_frame,
        tabs_bg,
        tabs_text,
        tab_align,
        tab_add,
        tabs[],

        //tab content
        tab_content_align,
        tab_content_align_r,
        tab_label_text,
        tab_label_input,
        tab_label_bg,
        btn_tab_delete,

        text_messages,
        btn_messages_all,
        text_messages_all,
        btn_messages_world,
        text_messages_world,
        icon_messages_world,
        btn_messages_region,
        text_messages_region,
        icon_messages_region,
        btn_messages_faction,
        text_messages_faction,
        icon_messages_faction,
        btn_messages_group,
        text_messages_group,
        icon_messages_group,
        btn_messages_say,
        text_messages_say,
        icon_messages_say,

        text_activity,
        list_activity,

        text_death,
        list_death,
    }
}

#[derive(WidgetCommon)]
pub struct Chat<'a> {
    global_state: &'a GlobalState,
    show: &'a Show,
    imgs: &'a Imgs,
    fonts: &'a Fonts,
    localized_strings: &'a Localization,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}
impl<'a> Chat<'a> {
    pub fn new(
        global_state: &'a GlobalState,
        show: &'a Show,
        imgs: &'a Imgs,
        fonts: &'a Fonts,
        localized_strings: &'a Localization,
    ) -> Self {
        Self {
            global_state,
            show,
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
pub enum Event {
    ChangeChatSettingsTab(Option<usize>),
    ChatChange(ChatChange),
}

impl<'a> Widget for Chat<'a> {
    type Event = Vec<Event>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        common_base::prof_span!("Chat::update");
        let widget::UpdateArgs { state, ui, .. } = args;

        let mut events = Vec::new();
        let chat_settings = &self.global_state.settings.chat;
        // Alignment
        // Settings Window
        Rectangle::fill_with(args.rect.dim(), color::TRANSPARENT)
            .xy(args.rect.xy())
            .graphics_for(args.id)
            .scroll_kids()
            .scroll_kids_vertically()
            .set(state.ids.window, ui);
        // Right Side
        Rectangle::fill_with([args.rect.w() / 2.0, args.rect.h()], color::TRANSPARENT)
            .top_right_of(state.ids.window)
            .set(state.ids.window_r, ui);

        // General Title
        Text::new(&self.localized_strings.get_msg("hud-settings-general"))
            .top_left_with_margins_on(state.ids.window, 5.0, 5.0)
            .font_size(self.fonts.cyri.scale(18))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.general_txt, ui);

        // Chat Transp
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-background_opacity"),
        )
        .down_from(state.ids.general_txt, 20.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.transp_text, ui);
        if let Some(new_val) = ImageSlider::continuous(
            chat_settings.chat_opacity,
            0.0,
            0.9,
            self.imgs.slider_indicator,
            self.imgs.slider,
        )
        .w_h(104.0, 22.0)
        .down_from(state.ids.transp_text, 10.0)
        .track_breadth(12.0)
        .slider_length(10.0)
        .pad_track((5.0, 5.0))
        .set(state.ids.transp_slider, ui)
        {
            events.push(Event::ChatChange(Transp(new_val)));
        }

        Text::new(&format!("{:.2}", chat_settings.chat_opacity,))
            .right_from(state.ids.transp_slider, 8.0)
            .font_size(self.fonts.cyri.scale(14))
            .graphics_for(state.ids.transp_slider)
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.transp_value, ui);

        // "Show character names in chat" toggle button
        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-chat_character_name"),
        )
        .down_from(state.ids.transp_slider, 10.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .color(TEXT_COLOR)
        .set(state.ids.char_name_text, ui);

        if chat_settings.chat_character_name
            != ToggleButton::new(
                chat_settings.chat_character_name,
                self.imgs.checkbox,
                self.imgs.checkbox_checked,
            )
            .w_h(18.0, 18.0)
            .right_from(state.ids.char_name_text, 10.0)
            .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
            .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
            .set(state.ids.char_name_button, ui)
        {
            events.push(Event::ChatChange(CharName(
                !chat_settings.chat_character_name,
            )));
        }

        // Reset the chat settings to the default settings
        if Button::image(self.imgs.button)
            .w_h(RESET_BUTTONS_WIDTH, RESET_BUTTONS_HEIGHT)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .down_from(state.ids.char_name_text, 20.0)
            .label(&self.localized_strings.get_msg("hud-settings-reset_chat"))
            .label_font_size(self.fonts.cyri.scale(14))
            .label_color(TEXT_COLOR)
            .label_font_id(self.fonts.cyri.conrod_id)
            .label_y(Relative::Scalar(2.0))
            .set(state.ids.reset_chat_button, ui)
            .was_clicked()
        {
            events.push(Event::ChatChange(ResetChatSettings));
        }

        // Tabs Title
        Text::new(&self.localized_strings.get_msg("hud-settings-chat_tabs"))
            .top_left_with_margins_on(state.ids.window_r, 5.0, 5.0)
            .font_size(self.fonts.cyri.scale(18))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .set(state.ids.tabs_text, ui);

        // bg and frame
        Image::new(self.imgs.chat_tab_settings_bg)
            .w_h(390.0, 270.0)
            .color(Some(UI_MAIN))
            .down_from(state.ids.tabs_text, 20.0)
            .set(state.ids.tabs_bg, ui);

        Image::new(self.imgs.chat_tab_settings_frame)
            .w_h(390.0, 270.0)
            .color(Some(UI_HIGHLIGHT_0))
            .down_from(state.ids.tabs_text, 20.0)
            .set(state.ids.tabs_frame, ui);

        // Tabs Alignment
        Rectangle::fill_with([390.0, 20.0], color::TRANSPARENT)
            .down_from(state.ids.tabs_text, 20.0)
            .set(state.ids.tab_align, ui);

        // Tabs Settings Alignment
        Rectangle::fill_with([390.0, 250.0], color::TRANSPARENT)
            .down_from(state.ids.tab_align, 0.0)
            .set(state.ids.tab_content_align, ui);
        Rectangle::fill_with([195.0, 250.0], color::TRANSPARENT)
            .top_right_of(state.ids.tab_content_align)
            .set(state.ids.tab_content_align_r, ui);

        let chat_tabs = &chat_settings.chat_tabs;
        if state.ids.tabs.len() < chat_tabs.len() {
            state.update(|s| {
                s.ids
                    .tabs
                    .resize(chat_tabs.len(), &mut ui.widget_id_generator())
            });
        }
        for (i, chat_tab) in chat_tabs.iter().enumerate() {
            let is_selected = self
                .show
                .chat_tab_settings_index
                .map(|index| index == i)
                .unwrap_or(false);

            let button = Button::image(if is_selected {
                self.imgs.selection
            } else {
                self.imgs.nothing
            })
            .w_h(390.0 / (MAX_CHAT_TABS as f64), 19.0)
            .hover_image(self.imgs.selection_hover)
            .press_image(self.imgs.selection_press)
            .image_color(color::rgba(1.0, 0.82, 0.27, 1.0))
            .label(chat_tab.label.as_str())
            .label_font_size(self.fonts.cyri.scale(12))
            .label_font_id(self.fonts.cyri.conrod_id)
            .label_color(TEXT_COLOR)
            .label_y(Relative::Scalar(1.0));

            let button = if i == 0 {
                button.top_left_with_margins_on(state.ids.tab_align, 1.0, 1.0)
            } else {
                button.right_from(state.ids.tabs[i - 1], 0.0)
            };
            if button.set(state.ids.tabs[i], ui).was_clicked() {
                events.push(Event::ChangeChatSettingsTab(if is_selected {
                    None
                } else {
                    Some(i)
                }));
            }
        }
        //Add button
        if chat_tabs.len() < MAX_CHAT_TABS {
            let add_tab_button = Button::image(self.imgs.settings_plus)
                .hover_image(self.imgs.settings_plus_hover)
                .press_image(self.imgs.settings_plus_press)
                .w_h(19.0, 19.0);

            let add_tab_button = if chat_tabs.is_empty() {
                add_tab_button.top_left_with_margins_on(state.ids.tab_align, 1.0, 1.0)
            } else {
                add_tab_button.right_from(state.ids.tabs[chat_tabs.len() - 1], 0.0)
            };

            if add_tab_button.set(state.ids.tab_add, ui).was_clicked() {
                let index = chat_tabs.len();
                events.push(Event::ChatChange(ChatTabInsert(index, ChatTab::default())));
                events.push(Event::ChangeChatSettingsTab(Some(index)));
            }
        }

        //Content
        if let Some((index, chat_tab)) = self
            .show
            .chat_tab_settings_index
            .and_then(|i| chat_tabs.get(i).map(|ct| (i, ct)))
        {
            let mut updated_chat_tab = chat_tab.clone();

            Text::new(&self.localized_strings.get_msg("hud-settings-label"))
                .top_left_with_margins_on(state.ids.tab_content_align, 5.0, 25.0)
                .font_size(self.fonts.cyri.scale(16))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.tab_label_text, ui);

            Rectangle::fill([90.0, 20.0])
                .right_from(state.ids.tab_label_text, 5.0)
                .color(color::rgba(0.0, 0.0, 0.0, 0.7))
                .set(state.ids.tab_label_bg, ui);

            if let Some(label) = TextEdit::new(chat_tab.label.as_str())
                .right_from(state.ids.tab_label_text, 10.0)
                .y_relative_to(state.ids.tab_label_text, -3.0)
                .w_h(75.0, 20.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(14))
                .color(TEXT_COLOR)
                .set(state.ids.tab_label_input, ui)
            {
                updated_chat_tab.label = label;
            }

            if Button::image(self.imgs.button)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .w_h(100.0, 30.0)
                .label(&self.localized_strings.get_msg("hud-settings-delete"))
                .label_font_size(self.fonts.cyri.scale(14))
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_color(TEXT_COLOR)
                .label_y(Relative::Scalar(1.0))
                .bottom_right_with_margins_on(state.ids.tab_content_align, 10.0, 10.0)
                .set(state.ids.btn_tab_delete, ui)
                .was_clicked()
            {
                events.push(Event::ChatChange(ChatTabRemove(index)));
                events.push(Event::ChangeChatSettingsTab(None));

                if let Some(chat_tab_index) = chat_settings.chat_tab_index {
                    match chat_tab_index.cmp(&index) {
                        Ordering::Equal => {
                            events.push(Event::ChatChange(ChangeChatTab(None)));
                        },
                        Ordering::Greater => {
                            events.push(Event::ChatChange(ChangeChatTab(Some(index - 1))));
                        },
                        _ => {},
                    }
                }
            }

            //helper methods to reduce on repeated code
            //(TODO: perhaps introduce a checkbox with label widget)
            let create_toggle = |selected, enabled| {
                ToggleButton::new(selected, self.imgs.checkbox, self.imgs.checkbox_checked)
                    .and(|button| {
                        if enabled {
                            button
                                .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
                                .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
                        } else {
                            button.image_colors(TEXT_GRAY_COLOR, TEXT_GRAY_COLOR)
                        }
                    })
                    .w_h(16.0, 16.0)
            };

            let create_toggle_text = |text, enabled| {
                Text::new(text)
                    .font_id(self.fonts.cyri.conrod_id)
                    .font_size(self.fonts.cyri.scale(14))
                    .color(if enabled { TEXT_COLOR } else { TEXT_GRAY_COLOR })
            };

            let create_toggle_icon = |img, enabled: bool| {
                Image::new(img)
                    .and_if(!enabled, |image| image.color(Some(TEXT_GRAY_COLOR)))
                    .w_h(18.0, 18.0)
            };

            //Messages
            Text::new(&self.localized_strings.get_msg("hud-settings-messages"))
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(16))
                .color(TEXT_COLOR)
                .top_left_with_margins_on(state.ids.tab_content_align, 35.0, 15.0)
                .set(state.ids.text_messages, ui);

            // Toggle all options
            if chat_tab.filter.message_all
                != ToggleButton::new(
                    chat_tab.filter.message_all,
                    self.imgs.checkbox,
                    self.imgs.checkbox_checked,
                )
                .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
                .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
                .w_h(18.0, 18.0)
                .down_from(state.ids.text_messages, 10.0)
                .set(state.ids.btn_messages_all, ui)
            {
                updated_chat_tab.filter.message_all = !chat_tab.filter.message_all;
            };

            Text::new(&self.localized_strings.get_msg("hud-settings-show_all"))
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(16))
                .color(TEXT_COLOR)
                .right_from(state.ids.btn_messages_all, 5.0)
                .set(state.ids.text_messages_all, ui);

            //Messages - group
            if chat_tab.filter.message_group
                != create_toggle(chat_tab.filter.message_group, !chat_tab.filter.message_all)
                    .down_from(state.ids.btn_messages_all, 10.0)
                    .set(state.ids.btn_messages_group, ui)
                && !chat_tab.filter.message_all
            {
                updated_chat_tab.filter.message_group = !chat_tab.filter.message_group;
            }

            let group_text = self.localized_strings.get_msg("hud-settings-group");
            create_toggle_text(&group_text, !chat_tab.filter.message_all)
                .right_from(state.ids.btn_messages_group, 5.0)
                .set(state.ids.text_messages_group, ui);

            create_toggle_icon(self.imgs.chat_group_small, !chat_tab.filter.message_all)
                .right_from(state.ids.text_messages_group, 5.0)
                .set(state.ids.icon_messages_group, ui);

            //Messages - faction
            if chat_tab.filter.message_faction
                != create_toggle(
                    chat_tab.filter.message_faction,
                    !chat_tab.filter.message_all,
                )
                .down_from(state.ids.btn_messages_group, 10.0)
                .set(state.ids.btn_messages_faction, ui)
                && !chat_tab.filter.message_all
            {
                updated_chat_tab.filter.message_faction = !chat_tab.filter.message_faction;
            }

            let faction_text = self.localized_strings.get_msg("hud-settings-faction");
            create_toggle_text(&faction_text, !chat_tab.filter.message_all)
                .right_from(state.ids.btn_messages_faction, 5.0)
                .set(state.ids.text_messages_faction, ui);

            create_toggle_icon(self.imgs.chat_faction_small, !chat_tab.filter.message_all)
                .right_from(state.ids.text_messages_faction, 5.0)
                .set(state.ids.icon_messages_faction, ui);

            //Messages - world
            if chat_tab.filter.message_world
                != create_toggle(chat_tab.filter.message_world, !chat_tab.filter.message_all)
                    .down_from(state.ids.btn_messages_faction, 10.0)
                    .set(state.ids.btn_messages_world, ui)
                && !chat_tab.filter.message_all
            {
                updated_chat_tab.filter.message_world = !chat_tab.filter.message_world;
            }

            let world_text = self.localized_strings.get_msg("hud-settings-world");
            create_toggle_text(&world_text, !chat_tab.filter.message_all)
                .right_from(state.ids.btn_messages_world, 5.0)
                .set(state.ids.text_messages_world, ui);

            create_toggle_icon(self.imgs.chat_world_small, !chat_tab.filter.message_all)
                .right_from(state.ids.text_messages_world, 5.0)
                .set(state.ids.icon_messages_world, ui);

            //Messages - region
            if chat_tab.filter.message_region
                != create_toggle(chat_tab.filter.message_region, !chat_tab.filter.message_all)
                    .down_from(state.ids.btn_messages_world, 10.0)
                    .set(state.ids.btn_messages_region, ui)
                && !chat_tab.filter.message_all
            {
                updated_chat_tab.filter.message_region = !chat_tab.filter.message_region;
            }

            let region_text = self.localized_strings.get_msg("hud-settings-region");
            create_toggle_text(&region_text, !chat_tab.filter.message_all)
                .right_from(state.ids.btn_messages_region, 5.0)
                .set(state.ids.text_messages_region, ui);

            create_toggle_icon(self.imgs.chat_region_small, !chat_tab.filter.message_all)
                .right_from(state.ids.text_messages_region, 5.0)
                .set(state.ids.icon_messages_region, ui);

            //Messages - say
            if chat_tab.filter.message_say
                != create_toggle(chat_tab.filter.message_say, !chat_tab.filter.message_all)
                    .down_from(state.ids.btn_messages_region, 10.0)
                    .set(state.ids.btn_messages_say, ui)
                && !chat_tab.filter.message_all
            {
                updated_chat_tab.filter.message_say = !chat_tab.filter.message_say;
            }

            let say_text = self.localized_strings.get_msg("hud-settings-say");
            create_toggle_text(&say_text, !chat_tab.filter.message_all)
                .right_from(state.ids.btn_messages_say, 5.0)
                .set(state.ids.text_messages_say, ui);

            create_toggle_icon(self.imgs.chat_say_small, !chat_tab.filter.message_all)
                .right_from(state.ids.text_messages_say, 5.0)
                .set(state.ids.icon_messages_say, ui);

            //Activity
            Text::new(&self.localized_strings.get_msg("hud-settings-activity"))
                .top_left_with_margins_on(state.ids.tab_content_align_r, 0.0, 5.0)
                .align_middle_y_of(state.ids.text_messages)
                .font_size(self.fonts.cyri.scale(16))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.text_activity, ui);

            if let Some(clicked) = DropDownList::new(
                &[
                    &self.localized_strings.get_msg("hud-settings-none"),
                    &self.localized_strings.get_msg("hud-settings-all"),
                    &self.localized_strings.get_msg("hud-settings-group_only"),
                ],
                Some(if chat_tab.filter.activity_all {
                    //all
                    1
                } else if chat_tab.filter.activity_group {
                    //group only
                    2
                } else {
                    //none
                    0
                }),
            )
            .w_h(100.0, 20.0)
            .color(color::hsl(0.0, 0.0, 0.1))
            .label_color(TEXT_COLOR)
            .label_font_id(self.fonts.cyri.conrod_id)
            .label_font_size(self.fonts.cyri.scale(14))
            .label_y(Relative::Scalar(1.0))
            .down_from(state.ids.text_activity, 10.0)
            .set(state.ids.list_activity, ui)
            {
                match clicked {
                    0 => {
                        updated_chat_tab.filter.activity_all = false;
                        updated_chat_tab.filter.activity_group = false;
                    },
                    1 => {
                        updated_chat_tab.filter.activity_all = true;
                    },
                    2 => {
                        updated_chat_tab.filter.activity_all = false;
                        updated_chat_tab.filter.activity_group = true;
                    },
                    _ => unreachable!(),
                }
            }

            //Death
            Text::new(&self.localized_strings.get_msg("hud-settings-death"))
                .down_from(state.ids.list_activity, 20.0)
                .font_size(self.fonts.cyri.scale(16))
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.text_death, ui);

            if let Some(clicked) = DropDownList::new(
                &[
                    &self.localized_strings.get_msg("hud-settings-none"),
                    &self.localized_strings.get_msg("hud-settings-all"),
                    &self.localized_strings.get_msg("hud-settings-group_only"),
                ],
                Some(if chat_tab.filter.death_all {
                    //all
                    1
                } else if chat_tab.filter.death_group {
                    //group only
                    2
                } else {
                    //none
                    0
                }),
            )
            .w_h(100.0, 20.0)
            .color(color::hsl(0.0, 0.0, 0.1))
            .label_color(TEXT_COLOR)
            .label_font_id(self.fonts.cyri.conrod_id)
            .label_font_size(self.fonts.cyri.scale(14))
            .label_y(Relative::Scalar(1.0))
            .down_from(state.ids.text_death, 10.0)
            .set(state.ids.list_death, ui)
            {
                match clicked {
                    0 => {
                        updated_chat_tab.filter.death_all = false;
                        updated_chat_tab.filter.death_group = false;
                    },
                    1 => {
                        updated_chat_tab.filter.death_all = true;
                    },
                    2 => {
                        updated_chat_tab.filter.death_all = false;
                        updated_chat_tab.filter.death_group = true;
                    },
                    _ => unreachable!(),
                }
            }

            if chat_tab != &updated_chat_tab {
                //insert to front to avoid errors where the tab is moved or removed
                events.insert(0, Event::ChatChange(ChatTabUpdate(index, updated_chat_tab)));
            }
        }

        events
    }
}
