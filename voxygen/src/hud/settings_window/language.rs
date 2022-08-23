use crate::{
    hud::{img_ids::Imgs, TEXT_COLOR},
    session::settings_change::{Language as LanguageChange, Language::*},
    ui::{fonts::Fonts, ToggleButton},
    GlobalState,
};
use conrod_core::{
    color,
    widget::{self, Button, Rectangle, Scrollbar, Text},
    widget_ids, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};
use i18n::{list_localizations, Localization};

widget_ids! {
    struct Ids {
        window,
        window_r,
        english_fallback_button,
        english_fallback_button_label,
        window_scrollbar,
        language_list[],
    }
}

#[derive(WidgetCommon)]
pub struct Language<'a> {
    global_state: &'a GlobalState,
    localized_strings: &'a Localization,
    imgs: &'a Imgs,
    fonts: &'a Fonts,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}
impl<'a> Language<'a> {
    pub fn new(
        global_state: &'a GlobalState,
        imgs: &'a Imgs,
        fonts: &'a Fonts,
        localized_strings: &'a Localization,
    ) -> Self {
        Self {
            global_state,
            localized_strings,
            imgs,
            fonts,
            common: widget::CommonBuilder::default(),
        }
    }
}

pub struct State {
    ids: Ids,
}

impl<'a> Widget for Language<'a> {
    type Event = Vec<LanguageChange>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        common_base::prof_span!("Language::update");
        let widget::UpdateArgs { state, ui, .. } = args;

        let mut events = Vec::new();

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

        // List available languages
        let selected_language = &self.global_state.settings.language.selected_language;
        let english_fallback = self.global_state.settings.language.use_english_fallback;
        let language_list = list_localizations();
        if state.ids.language_list.len() < language_list.len() {
            state.update(|state| {
                state
                    .ids
                    .language_list
                    .resize(language_list.len(), &mut ui.widget_id_generator())
            });
        };
        for (i, language) in language_list.iter().enumerate() {
            let button_w = 400.0;
            let button_h = 50.0;
            let button = Button::image(if selected_language == &language.language_identifier {
                self.imgs.selection
            } else {
                self.imgs.nothing
            });
            let button = if i == 0 {
                button.mid_top_with_margin_on(state.ids.window, 20.0)
            } else {
                button.mid_bottom_with_margin_on(state.ids.language_list[i - 1], -button_h)
            };
            if button
                .label(&language.language_name)
                .w_h(button_w, button_h)
                .hover_image(self.imgs.selection_hover)
                .press_image(self.imgs.selection_press)
                .label_color(TEXT_COLOR)
                .label_font_size(self.fonts.cyri.scale(22))
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_y(conrod_core::position::Relative::Scalar(2.0))
                .set(state.ids.language_list[i], ui)
                .was_clicked()
            {
                events.push(ChangeLanguage(Box::new(language.to_owned())));
            }
        }

        // English as fallback language
        let show_english_fallback = ToggleButton::new(
            english_fallback,
            self.imgs.checkbox,
            self.imgs.checkbox_checked,
        )
        .w_h(18.0, 18.0);
        let show_english_fallback = if let Some(id) = state.ids.language_list.last() {
            show_english_fallback.down_from(*id, 8.0)
            //mid_bottom_with_margin_on(id, -button_h)
        } else {
            show_english_fallback.mid_top_with_margin_on(state.ids.window, 20.0)
        };
        let show_english_fallback = show_english_fallback
            .hover_images(self.imgs.checkbox_mo, self.imgs.checkbox_checked_mo)
            .press_images(self.imgs.checkbox_press, self.imgs.checkbox_checked)
            .set(state.ids.english_fallback_button, ui);

        if english_fallback != show_english_fallback {
            events.push(ToggleEnglishFallback(show_english_fallback));
        }

        Text::new(
            &self
                .localized_strings
                .get_msg("hud-settings-english_fallback"),
        )
        .right_from(state.ids.english_fallback_button, 10.0)
        .font_size(self.fonts.cyri.scale(14))
        .font_id(self.fonts.cyri.conrod_id)
        .graphics_for(state.ids.english_fallback_button)
        .color(TEXT_COLOR)
        .set(state.ids.english_fallback_button_label, ui);

        events
    }
}
