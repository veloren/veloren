use conrod_core::{
    widget::{self, Button, Image},
    widget_ids, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};

use super::{img_ids::Imgs, Fonts, TEXT_COLOR};

widget_ids! {
    struct Ids {
        esc_bg,
        fireplace,
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
    Controls,
    Characters,
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
        let widget::UpdateArgs { state, ui, .. } = args;

        Image::new(self.imgs.esc_frame)
            .w_h(200.0, 328.0)
            .middle_of(ui.window)
            .set(state.ids.esc_bg, ui);

        Image::new(self.imgs.fireplace)
            .w_h(176.0, 50.0)
            .mid_top_with_margin_on(state.ids.esc_bg, 12.0)
            .set(state.ids.fireplace, ui);

        // Settings
        if Button::image(self.imgs.button)
            .mid_bottom_with_margin_on(state.ids.fireplace, -45.0)
            .w_h(168.0, 35.0)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .label("Settings")
            .label_y(conrod_core::position::Relative::Scalar(3.0))
            .label_color(TEXT_COLOR)
            .label_font_size(17)
            .set(state.ids.menu_button_1, ui)
            .was_clicked()
        {               
            return Some(Event::OpenSettings);
        };
        // Controls
        if Button::image(self.imgs.button)
            .mid_bottom_with_margin_on(state.ids.menu_button_1, -40.0)
            .w_h(168.0, 35.0)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .label("Controls")
            .label_y(conrod_core::position::Relative::Scalar(3.0))
            .label_color(TEXT_COLOR)
            .label_font_size(17)
            .set(state.ids.menu_button_2, ui)
            .was_clicked()
        {   
            return Some(Event::Controls); // TODO: Show the Controls Tab of the Settings           
        };
        // Characters
        if Button::image(self.imgs.button)
            .mid_bottom_with_margin_on(state.ids.menu_button_2, -40.0)
            .w_h(168.0, 35.0)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .label("Characters")
            .label_y(conrod_core::position::Relative::Scalar(3.0))
            .label_color(TEXT_COLOR)
            .label_font_size(17)
            .set(state.ids.menu_button_3, ui)
            .was_clicked()
        {   

            return Some(Event::Characters); // TODO: Open Character Selection
        };
        // Logout
        if Button::image(self.imgs.button)
            .mid_bottom_with_margin_on(state.ids.menu_button_3, -40.0)
            .w_h(168.0, 35.0)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .label("Logout")
            .label_y(conrod_core::position::Relative::Scalar(3.0))
            .label_color(TEXT_COLOR)
            .label_font_size(17)
            .set(state.ids.menu_button_4, ui)
            .was_clicked()
        {
            return Some(Event::Logout);
        };
        // Quit
        if Button::image(self.imgs.button)
            .mid_bottom_with_margin_on(state.ids.menu_button_4, -40.0)
            .w_h(168.0, 35.0)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .label("Quit")
            .label_y(conrod_core::position::Relative::Scalar(3.0))
            .label_color(TEXT_COLOR)
            .label_font_size(17)
            .set(state.ids.menu_button_5, ui)
            .was_clicked()
        {
            return Some(Event::Quit);
        };
        // Close
        if Button::image(self.imgs.button)
            .mid_bottom_with_margin_on(state.ids.menu_button_5, -50.0)
            .w_h(168.0, 35.0)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .label("Resume")
            .label_y(conrod_core::position::Relative::Scalar(3.0))
            .label_color(TEXT_COLOR)
            .label_font_size(17)
            .set(state.ids.menu_button_6, ui)
            .was_clicked()
        {
            return Some(Event::Close);
        };
        None
    }
}
