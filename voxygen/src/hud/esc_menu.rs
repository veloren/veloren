use super::{img_ids::Imgs, settings_window::SettingsTab, TEXT_COLOR};
use crate::{i18n::VoxygenLocalization, ui::fonts::ConrodVoxygenFonts};
use conrod_core::{
    widget::{self, Button, Image},
    widget_ids, Color, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};

widget_ids! {
    struct Ids {
        esc_bg,
        fireplace,
        banner_top,
        menu_button_1,
        menu_button_2,
        menu_button_3,
        menu_button_4,
        menu_button_5,
        menu_button_6,
    }
}

#[derive(WidgetCommon)]
pub struct EscMenu<'a> {
    imgs: &'a Imgs,
    fonts: &'a ConrodVoxygenFonts,
    localized_strings: &'a std::sync::Arc<VoxygenLocalization>,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> EscMenu<'a> {
    pub fn new(
        imgs: &'a Imgs,
        fonts: &'a ConrodVoxygenFonts,
        localized_strings: &'a std::sync::Arc<VoxygenLocalization>,
    ) -> Self {
        Self {
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
    OpenSettings(SettingsTab),
    CharacterSelection,
    Logout,
    Quit,
    Close,
}

impl<'a> Widget for EscMenu<'a> {
    type Event = Option<Event>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style { () }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs { state, ui, .. } = args;

        Image::new(self.imgs.esc_frame)
            .w_h(240.0, 380.0)
            .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.9)))
            .middle_of(ui.window)
            .set(state.ids.esc_bg, ui);

        Image::new(self.imgs.banner_top)
            .w_h(250.0, 34.0)
            .mid_top_with_margin_on(state.ids.esc_bg, -34.0)
            .set(state.ids.banner_top, ui);

        /*Image::new(self.imgs.fireplace)
        .w_h(210.0, 60.0)
        .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.8)))
        .mid_top_with_margin_on(state.ids.esc_bg, 5.0)
        .set(state.ids.fireplace, ui);*/

        // Resume
        if Button::image(self.imgs.button)
            .mid_bottom_with_margin_on(state.ids.banner_top, -60.0)
            .w_h(210.0, 50.0)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .label(&self.localized_strings.get("common.resume"))
            .label_y(conrod_core::position::Relative::Scalar(3.0))
            .label_color(TEXT_COLOR)
            .label_font_size(self.fonts.cyri.scale(20))
            .label_font_id(self.fonts.cyri.conrod_id)
            .set(state.ids.menu_button_1, ui)
            .was_clicked()
        {
            return Some(Event::Close);
        };

        // Settings
        if Button::image(self.imgs.button)
            .mid_bottom_with_margin_on(state.ids.menu_button_1, -65.0)
            .w_h(210.0, 50.0)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .label(&self.localized_strings.get("common.settings"))
            .label_y(conrod_core::position::Relative::Scalar(3.0))
            .label_color(TEXT_COLOR)
            .label_font_size(self.fonts.cyri.scale(20))
            .label_font_id(self.fonts.cyri.conrod_id)
            .set(state.ids.menu_button_2, ui)
            .was_clicked()
        {
            return Some(Event::OpenSettings(SettingsTab::Interface));
        };
        // Controls
        if Button::image(self.imgs.button)
            .mid_bottom_with_margin_on(state.ids.menu_button_2, -55.0)
            .w_h(210.0, 50.0)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .label(&self.localized_strings.get("common.controls"))
            .label_y(conrod_core::position::Relative::Scalar(3.0))
            .label_color(TEXT_COLOR)
            .label_font_size(self.fonts.cyri.scale(20))
            .label_font_id(self.fonts.cyri.conrod_id)
            .set(state.ids.menu_button_3, ui)
            .was_clicked()
        {
            return Some(Event::OpenSettings(SettingsTab::Controls));
        };
        // Characters
        if Button::image(self.imgs.button)
            .mid_bottom_with_margin_on(state.ids.menu_button_3, -55.0)
            .w_h(210.0, 50.0)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .label(&self.localized_strings.get("common.characters"))
            .label_y(conrod_core::position::Relative::Scalar(3.0))
            .label_color(TEXT_COLOR)
            .label_font_size(self.fonts.cyri.scale(20))
            .label_font_id(self.fonts.cyri.conrod_id)
            .set(state.ids.menu_button_4, ui)
            .was_clicked()
        {
            return Some(Event::CharacterSelection);
        };
        // Logout
        if Button::image(self.imgs.button)
            .mid_bottom_with_margin_on(state.ids.menu_button_4, -65.0)
            .w_h(210.0, 50.0)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .label(&self.localized_strings.get("esc_menu.logout"))
            .label_y(conrod_core::position::Relative::Scalar(3.0))
            .label_color(TEXT_COLOR)
            .label_font_size(self.fonts.cyri.scale(20))
            .label_font_id(self.fonts.cyri.conrod_id)
            .set(state.ids.menu_button_5, ui)
            .was_clicked()
        {
            return Some(Event::Logout);
        };
        // Quit
        if Button::image(self.imgs.button)
            .mid_bottom_with_margin_on(state.ids.menu_button_5, -55.0)
            .w_h(210.0, 50.0)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .label(&self.localized_strings.get("esc_menu.quit_game"))
            .label_y(conrod_core::position::Relative::Scalar(3.0))
            .label_color(TEXT_COLOR)
            .label_font_size(self.fonts.cyri.scale(20))
            .label_font_id(self.fonts.cyri.conrod_id)
            .set(state.ids.menu_button_6, ui)
            .was_clicked()
        {
            return Some(Event::Quit);
        };
        None
    }
}
