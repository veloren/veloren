use conrod_core::{
    widget::{self, Button, Image, Text},
    widget_ids, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};

use super::{img_ids::Imgs, Windows, TEXT_COLOR};
use crate::ui::{fonts::ConrodVoxygenFonts, ToggleButton};

widget_ids! {
    struct Ids {
        bag,
        bag_text,
        bag_show_map,
        map_button,
        settings_button,
        social_button,
        social_button_bg,
        spellbook_button,
        spellbook_button_bg,
    }
}

#[derive(WidgetCommon)]
pub struct Buttons<'a> {
    open_windows: &'a Windows,
    show_map: bool,
    show_bag: bool,

    imgs: &'a Imgs,
    fonts: &'a ConrodVoxygenFonts,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> Buttons<'a> {
    pub fn new(
        open_windows: &'a Windows,
        show_map: bool,
        show_bag: bool,
        imgs: &'a Imgs,
        fonts: &'a ConrodVoxygenFonts,
    ) -> Self {
        Self {
            open_windows,
            show_map,
            show_bag,
            imgs,
            fonts,
            common: widget::CommonBuilder::default(),
        }
    }
}

pub struct State {
    ids: Ids,
}

pub enum Event {
    ToggleBag,
    ToggleSettings,
    ToggleMap,
    ToggleSocial,
    ToggleSpell,
}

impl<'a> Widget for Buttons<'a> {
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

        // Bag
        if !self.show_map {
            if self.show_bag
                != ToggleButton::new(self.show_bag, self.imgs.bag, self.imgs.bag_open)
                    .bottom_right_with_margins_on(ui.window, 5.0, 5.0)
                    .hover_images(self.imgs.bag_hover, self.imgs.bag_open_hover)
                    .press_images(self.imgs.bag_press, self.imgs.bag_open_press)
                    .w_h(420.0 / 10.0, 480.0 / 10.0)
                    .set(state.ids.bag, ui)
            {
                return Some(Event::ToggleBag);
            }

            Text::new("B")
                .bottom_right_with_margins_on(state.ids.bag, 0.0, 0.0)
                .font_size(10)
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.bag_text, ui);
        } else {
            Image::new(self.imgs.bag)
                .bottom_right_with_margins_on(ui.window, 5.0, 5.0)
                .w_h(420.0 / 10.0, 480.0 / 10.0)
                .set(state.ids.bag_show_map, ui);
            Text::new("B")
                .bottom_right_with_margins_on(state.ids.bag, 0.0, 0.0)
                .font_size(10)
                .font_id(self.fonts.cyri.conrod_id)
                .color(TEXT_COLOR)
                .set(state.ids.bag_text, ui);
        }

        // 0 Settings
        if Button::image(self.imgs.settings)
            .w_h(29.0, 25.0)
            .bottom_right_with_margins_on(ui.window, 5.0, 57.0)
            .hover_image(self.imgs.settings_hover)
            .press_image(self.imgs.settings_press)
            .label("N")
            .label_font_id(self.fonts.cyri.conrod_id)
            .label_font_size(10)
            .label_color(TEXT_COLOR)
            .label_y(conrod_core::position::Relative::Scalar(-7.0))
            .label_x(conrod_core::position::Relative::Scalar(10.0))
            .set(state.ids.settings_button, ui)
            .was_clicked()
        {
            return Some(Event::ToggleSettings);
        };

        Image::new(self.imgs.social_button)
            .w_h(25.0, 25.0)
            .left_from(state.ids.settings_button, 10.0)
            .set(state.ids.social_button_bg, ui);

        // 2 Map
        if Button::image(self.imgs.map_button)
            .w_h(22.0, 25.0)
            .left_from(state.ids.social_button_bg, 10.0)
            .hover_image(self.imgs.map_hover)
            .press_image(self.imgs.map_press)
            .label("M")
            .label_font_id(self.fonts.cyri.conrod_id)
            .label_font_size(10)
            .label_color(TEXT_COLOR)
            .label_y(conrod_core::position::Relative::Scalar(-7.0))
            .label_x(conrod_core::position::Relative::Scalar(10.0))
            .set(state.ids.map_button, ui)
            .was_clicked()
        {
            return Some(Event::ToggleMap);
        };

        // Other Windows can only be accessed when `Settings` is closed.
        // Opening `Settings` will close all other Windows, including the `Bag`.
        // Opening the `Map` won't close the previously displayed windows.
        Image::new(self.imgs.social)
            .w_h(25.0, 25.0)
            .left_from(state.ids.settings_button, 10.0)
            .set(state.ids.social_button_bg, ui);
        Image::new(self.imgs.spellbook_button)
            .w_h(28.0, 25.0)
            .left_from(state.ids.map_button, 10.0)
            .set(state.ids.spellbook_button_bg, ui);
        // Other Windows can only be accessed when `Settings` is closed.
        // Opening `Settings` will close all other Windows, including the `Bag`.
        // Opening the `Map` won't close the previously displayed windows.
        if !(*self.open_windows == Windows::Settings) && self.show_map == false {
            // 1 Social
            if Button::image(self.imgs.social)
                .w_h(25.0, 25.0)
                .left_from(state.ids.settings_button, 10.0)
                .hover_image(self.imgs.social_hover)
                .press_image(self.imgs.social_press)
                .label("O")
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_font_size(10)
                .label_color(TEXT_COLOR)
                .label_y(conrod_core::position::Relative::Scalar(-7.0))
                .label_x(conrod_core::position::Relative::Scalar(10.0))
                .set(state.ids.social_button, ui)
                .was_clicked()
            {
                return Some(Event::ToggleSocial);
            }

            // 3 Spellbook
            if Button::image(self.imgs.spellbook_button)
                .w_h(28.0, 25.0)
                .left_from(state.ids.map_button, 10.0)
                .hover_image(self.imgs.spellbook_hover)
                .press_image(self.imgs.spellbook_press)
                .label("P")
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_font_size(10)
                .label_color(TEXT_COLOR)
                .label_y(conrod_core::position::Relative::Scalar(-7.0))
                .label_x(conrod_core::position::Relative::Scalar(10.0))
                .set(state.ids.spellbook_button, ui)
                .was_clicked()
            {
                return Some(Event::ToggleSpell);
            }
        }

        None
    }
}
