use super::{img_ids::Imgs, TEXT_COLOR, UI_HIGHLIGHT_0};
use crate::{
    game_input::GameInput,
    hud::{Event, PromptDialogSettings},
    settings::Settings,
    ui::fonts::Fonts,
};
use conrod_core::{
    widget::{self, Button, Image, Text},
    widget_ids, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};
use i18n::LocalizationHandle;
use keyboard_keynames::key_layout::KeyLayout;

widget_ids! {
    struct Ids {
        top,
        mid,
        bot,
        text,
        accept_txt, // optional timer
        accept_key, //button with label
        decline_txt,
        decline_key,
        prompt_txt,
    }
}
#[derive(WidgetCommon)]
pub struct PromptDialog<'a> {
    imgs: &'a Imgs,
    fonts: &'a Fonts,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    localized_strings: &'a LocalizationHandle,
    settings: &'a Settings,
    prompt_dialog_settings: &'a PromptDialogSettings,
    key_layout: &'a Option<KeyLayout>,
}

impl<'a> PromptDialog<'a> {
    pub fn new(
        imgs: &'a Imgs,
        fonts: &'a Fonts,
        localized_strings: &'a LocalizationHandle,
        settings: &'a Settings,
        prompt_dialog_settings: &'a PromptDialogSettings,
        key_layout: &'a Option<KeyLayout>,
    ) -> Self {
        Self {
            imgs,
            fonts,
            localized_strings,
            common: widget::CommonBuilder::default(),
            settings,
            prompt_dialog_settings,
            key_layout,
        }
    }
}

pub struct State {
    ids: Ids,
}

pub enum DialogOutcomeEvent {
    Affirmative(Event),
    Negative(Option<Event>),
}

impl<'a> Widget for PromptDialog<'a> {
    type Event = Option<DialogOutcomeEvent>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        common_base::prof_span!("PromptDialog::update");
        let widget::UpdateArgs { state, ui, .. } = args;
        let _localized_strings = &self.localized_strings;
        let mut event: Option<DialogOutcomeEvent> = None;

        let accept_key = self
            .settings
            .controls
            .get_binding(GameInput::AcceptGroupInvite)
            .map_or_else(|| "".into(), |key| key.display_string(self.key_layout));
        let decline_key = self
            .settings
            .controls
            .get_binding(GameInput::DeclineGroupInvite)
            .map_or_else(|| "".into(), |key| key.display_string(self.key_layout));

        // Window
        Image::new(self.imgs.prompt_top)
            .w_h(276.0, 24.0)
            .mid_top_with_margin_on(ui.window, 100.0)
            .color(Some(UI_HIGHLIGHT_0))
            .set(state.ids.top, ui);
        if !self.prompt_dialog_settings.message.is_empty() {
            Image::new(self.imgs.prompt_mid)
            .w(276.0)
            .h_of(state.ids.prompt_txt) // height relative to content, max height 150
            .down_from(state.ids.top, 0.0)
            .color(Some(UI_HIGHLIGHT_0))
            .scroll_kids_vertically()
            .set(state.ids.mid, ui);
        }
        Image::new(self.imgs.prompt_bot)
            .w_h(276.0, 35.0)
            .down_from(state.ids.mid, 0.0)
            .color(Some(UI_HIGHLIGHT_0))
            .set(state.ids.bot, ui);

        // Accept/Decline Buttons
        if Button::image(self.imgs.key_button)
            .w_h(20.0, 20.0)
            .hover_image(self.imgs.close_btn_hover)
            .press_image(self.imgs.close_btn_press)
            .label(&accept_key)
            .image_color(UI_HIGHLIGHT_0)
            .label_color(TEXT_COLOR)
            .label_font_size(self.fonts.cyri.scale(16))
            .label_font_id(self.fonts.cyri.conrod_id)
            .label_y(conrod_core::position::Relative::Scalar(2.5))
            .label_x(conrod_core::position::Relative::Scalar(0.5))
            .bottom_left_with_margins_on(state.ids.bot, 4.0, 6.0)
            .set(state.ids.accept_key, ui)
            .was_clicked()
            || self
                .prompt_dialog_settings
                .outcome_via_keypress
                .map_or(false, |outcome| outcome)
        {
            // Primary use should be through pressing the key instead of clicking this
            event = Some(DialogOutcomeEvent::Affirmative(
                self.prompt_dialog_settings.affirmative_event.clone(),
            ));
        }
        let accept_txt = if self.prompt_dialog_settings.negative_option {
            "Accept"
        } else {
            "Ok"
        };
        Text::new(accept_txt)
            .bottom_left_with_margins_on(state.ids.accept_key, 4.0, 28.0)
            .font_id(self.fonts.cyri.conrod_id)
            .font_size(self.fonts.cyri.scale(18))
            .color(TEXT_COLOR)
            .set(state.ids.accept_txt, ui);

        if self.prompt_dialog_settings.negative_option {
            if Button::image(self.imgs.key_button)
                .w_h(20.0, 20.0)
                .hover_image(self.imgs.close_btn_hover)
                .press_image(self.imgs.close_btn_press)
                .label(&decline_key)
                .image_color(UI_HIGHLIGHT_0)
                .label_color(TEXT_COLOR)
                .label_font_size(self.fonts.cyri.scale(16))
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_y(conrod_core::position::Relative::Scalar(2.5))
                .label_x(conrod_core::position::Relative::Scalar(0.5))
                .bottom_right_with_margins_on(state.ids.bot, 4.0, 6.0)
                .set(state.ids.decline_key, ui)
                .was_clicked()
                || self
                    .prompt_dialog_settings
                    .outcome_via_keypress
                    .map_or(false, |outcome| !outcome)
            {
                event = Some(DialogOutcomeEvent::Negative(
                    self.prompt_dialog_settings.negative_event.as_ref().cloned(),
                ));
            }
            Text::new("Decline")
                .bottom_left_with_margins_on(state.ids.decline_key, 4.0, -65.0)
                .font_id(self.fonts.cyri.conrod_id)
                .font_size(self.fonts.cyri.scale(18))
                .color(TEXT_COLOR)
                .set(state.ids.decline_txt, ui);
        }

        // Prompt Description
        Text::new(&self.prompt_dialog_settings.message)
        .mid_top_with_margin_on(state.ids.mid,0.0)
        .font_id(self.fonts.cyri.conrod_id)
        .font_size(self.fonts.cyri.scale(18))
        .color(TEXT_COLOR)
        .w(260.0) // Text stays within frame
        .set(state.ids.prompt_txt, ui);

        event
    }
}
