use conrod_core::{
    builder_methods, color,
    text::font,
    widget::{self, Button, Image, Rectangle, Text},
    widget_ids, Color, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon,
};

use super::{
    imgs::Imgs,
    Style, XP_COLOR,
};

widget_ids! {
    struct Ids {
    }
}

#[derive(WidgetCommon)]
pub struct EscMenu<'a> {
    xp_percentage: f64,
    imgs: &'a Imgs,

    #[conrod(common_builder)]
    common: widget::CommonBuilder,
    style: Style,
}

impl<'a> CharacterWindow<'a> {
    pub fn new(imgs: &'a Imgs) -> Self {
        Self {
            xp_percentage: 0.4,
            imgs,
            common: widget::CommonBuilder::default(),
            style: Style::default(),
        }
    }
    builder_methods! {
        pub text_color { style.text_color = Some(Color) }
    }
}

pub struct State {
    ids: Ids,
}

pub enum Event {
    Close,
}

impl<'a> Widget for CharacterWindow<'a> {
    type State = State;
    type Style = Style;
    type Event = Option<Event>;

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {
        self.style.clone()
    }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        let widget::UpdateArgs {
            id,
            state,
            ui,
            style,
            ..
        } = args;

        let font_id = style.font_id(&ui.theme).or(ui.fonts.ids().next());
        let text_color = style.text_color(&ui.theme);

        if self.menu_open {
            Image::new(self.imgs.esc_bg)
                .w_h(228.0, 450.0)
                .middle_of(ui_widgets.window)
                .set(self.ids.esc_bg, ui_widgets);

            Image::new(self.imgs.fireplace)
                .w_h(180.0, 60.0)
                .mid_top_with_margin_on(self.ids.esc_bg, 50.0)
                .set(self.ids.fireplace, ui_widgets);

            // Settings
            if Button::image(self.imgs.button_dark)
                .mid_top_with_margin_on(self.ids.esc_bg, 115.0)
                .w_h(170.0, 50.0)
                .hover_image(self.imgs.button_dark_hover)
                .press_image(self.imgs.button_dark_press)
                .label("Settings")
                .label_y(conrod_core::position::Relative::Scalar(2.0))
                .label_color(TEXT_COLOR)
                .label_font_size(17)
                .set(self.ids.menu_button_1, ui_widgets)
                .was_clicked()
            {
                self.menu_open = false;
                self.open_windows = Windows::Settings;
            };
            // Controls
            if Button::image(self.imgs.button_dark)
                .mid_top_with_margin_on(self.ids.esc_bg, 175.0)
                .w_h(170.0, 50.0)
                .hover_image(self.imgs.button_dark_hover)
                .press_image(self.imgs.button_dark_press)
                .label("Controls")
                .label_y(conrod_core::position::Relative::Scalar(2.0))
                .label_color(TEXT_COLOR)
                .label_font_size(17)
                .set(self.ids.menu_button_2, ui_widgets)
                .was_clicked()
            {
                //self.menu_open = false;
            };
            // Servers
            if Button::image(self.imgs.button_dark)
                .mid_top_with_margin_on(self.ids.esc_bg, 235.0)
                .w_h(170.0, 50.0)
                .hover_image(self.imgs.button_dark_hover)
                .press_image(self.imgs.button_dark_press)
                .label("Servers")
                .label_y(conrod_core::position::Relative::Scalar(2.0))
                .label_color(TEXT_COLOR)
                .label_font_size(17)
                .set(self.ids.menu_button_3, ui_widgets)
                .was_clicked()
            {
                //self.menu_open = false;
            };
            // Logout
            if Button::image(self.imgs.button_dark)
                .mid_top_with_margin_on(self.ids.esc_bg, 295.0)
                .w_h(170.0, 50.0)
                .hover_image(self.imgs.button_dark_hover)
                .press_image(self.imgs.button_dark_press)
                .label("Logout")
                .label_y(conrod_core::position::Relative::Scalar(2.0))
                .label_color(TEXT_COLOR)
                .label_font_size(17)
                .set(self.ids.menu_button_4, ui_widgets)
                .was_clicked()
            {
                events.push(Event::Logout);
            };
            // Quit
            if Button::image(self.imgs.button_dark)
                .mid_top_with_margin_on(self.ids.esc_bg, 355.0)
                .w_h(170.0, 50.0)
                .hover_image(self.imgs.button_dark_hover)
                .press_image(self.imgs.button_dark_press)
                .label("Quit")
                .label_y(conrod_core::position::Relative::Scalar(2.0))
                .label_color(TEXT_COLOR)
                .label_font_size(17)
                .set(self.ids.menu_button_5, ui_widgets)
                .was_clicked()
            {
                events.push(Event::Quit);
            };
        }
