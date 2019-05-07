use conrod_core::{
    widget::{self, Button, Image},
    widget_ids, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};

use super::{
    img_ids::Imgs,
    font_ids::Fonts,
    TEXT_COLOR,
};

widget_ids! {
    struct Ids {
        esc_bg,
        fireplace,
        menu_button_1,
        menu_button_2,
        menu_button_3,
        menu_button_4,
        menu_button_5,
    }
}

#[derive(WidgetCommon)]
pub struct EscMenu<'a> {

    imgs: &'a Imgs,
    fonts: &'a Fonts,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> EscMenu<'a> {
    pub fn new(imgs: &'a Imgs, fonts: &'a Fonts) -> Self {
        Self {
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
    OpenSettings,
    Logout,
    Quit,
    Close,
}

impl<'a> Widget for EscMenu<'a> {
    type State = State;
    type Style = ();
    type Event = Option<Event>;

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {
        ()
    }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs {
            state,
            ui,
            ..
        } = args;

        Image::new(self.imgs.esc_bg)
            .w_h(228.0, 450.0)
            .middle_of(ui.window)
            .set(state.ids.esc_bg, ui);

        Image::new(self.imgs.fireplace)
            .w_h(180.0, 60.0)
            .mid_top_with_margin_on(state.ids.esc_bg, 50.0)
            .set(state.ids.fireplace, ui);

        // Settings
        if Button::image(self.imgs.button)
            .mid_top_with_margin_on(state.ids.esc_bg, 115.0)
            .w_h(170.0, 50.0)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .label("Settings")
            .label_y(conrod_core::position::Relative::Scalar(2.0))
            .label_color(TEXT_COLOR)
            .label_font_size(17)
            .set(state.ids.menu_button_1, ui)
            .was_clicked()
        {
            return Some(Event::OpenSettings);
        };
        // Controls
        if Button::image(self.imgs.button)
            .mid_top_with_margin_on(state.ids.esc_bg, 175.0)
            .w_h(170.0, 50.0)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .label("Controls")
            .label_y(conrod_core::position::Relative::Scalar(2.0))
            .label_color(TEXT_COLOR)
            .label_font_size(17)
            .set(state.ids.menu_button_2, ui)
            .was_clicked()
        {
            //self.menu_open = false;
        };
        // Servers
        if Button::image(self.imgs.button)
            .mid_top_with_margin_on(state.ids.esc_bg, 235.0)
            .w_h(170.0, 50.0)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .label("Servers")
            .label_y(conrod_core::position::Relative::Scalar(2.0))
            .label_color(TEXT_COLOR)
            .label_font_size(17)
            .set(state.ids.menu_button_3, ui)
            .was_clicked()
        {
            //self.menu_open = false;
        };
        // Logout
        if Button::image(self.imgs.button)
            .mid_top_with_margin_on(state.ids.esc_bg, 295.0)
            .w_h(170.0, 50.0)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .label("Logout")
            .label_y(conrod_core::position::Relative::Scalar(2.0))
            .label_color(TEXT_COLOR)
            .label_font_size(17)
            .set(state.ids.menu_button_4, ui)
            .was_clicked()
        {
            return Some(Event::Logout);
        };
        // Quit
        if Button::image(self.imgs.button)
            .mid_top_with_margin_on(state.ids.esc_bg, 355.0)
            .w_h(170.0, 50.0)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .label("Quit")
            .label_y(conrod_core::position::Relative::Scalar(2.0))
            .label_color(TEXT_COLOR)
            .label_font_size(17)
            .set(state.ids.menu_button_5, ui)
            .was_clicked()
        {
            return Some(Event::Quit);
        };

        None
    }
}
