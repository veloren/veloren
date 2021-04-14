use crate::{
    hud::{img_ids::Imgs, TEXT_COLOR},
    i18n::list_localizations,
    session::settings_change::Language as LanguageChange,
    ui::fonts::Fonts,
    GlobalState,
};
use conrod_core::{
    color,
    widget::{self, Button, Rectangle, Scrollbar},
    widget_ids, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};

widget_ids! {
    struct Ids {
        window,
        window_r,
        window_scrollbar,
        language_list[],
    }
}

#[derive(WidgetCommon)]
pub struct Language<'a> {
    global_state: &'a GlobalState,
    imgs: &'a Imgs,
    fonts: &'a Fonts,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}
impl<'a> Language<'a> {
    pub fn new(global_state: &'a GlobalState, imgs: &'a Imgs, fonts: &'a Fonts) -> Self {
        Self {
            global_state,
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

    #[allow(clippy::unused_unit)] // TODO: Pending review in #587
    fn style(&self) -> Self::Style { () }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
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
                events.push(LanguageChange::ChangeLanguage(Box::new(
                    language.to_owned(),
                )));
            }
        }

        events
    }
}
