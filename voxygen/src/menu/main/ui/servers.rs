use super::{Imgs, Message};
use crate::{
    i18n::Localization,
    ui::{
        fonts::IcedFonts as Fonts,
        ice::{
            component::neat_button,
            style,
            widget::background_container::{BackgroundContainer, Padding},
            Element,
        },
    },
};
use iced::{
    button, scrollable, Align, Button, Column, Container, Length, Row, Scrollable, Space, Text,
};

pub struct Screen {
    back_button: button::State,
    server_buttons: Vec<button::State>,
    servers_list: scrollable::State,
}

impl Screen {
    pub fn new() -> Self {
        Self {
            back_button: Default::default(),
            server_buttons: vec![],
            servers_list: Default::default(),
        }
    }

    pub(super) fn view(
        &mut self,
        fonts: &Fonts,
        imgs: &Imgs,
        servers: &[impl AsRef<str>],
        selected_server_index: Option<usize>,
        i18n: &Localization,
        button_style: style::button::Style,
    ) -> Element<Message> {
        let button = neat_button(
            &mut self.back_button,
            i18n.get("common.back"),
            0.77_f32,
            button_style,
            Some(Message::Back),
        );

        let button = Container::new(Container::new(button).max_width(200))
            .width(Length::Fill)
            .align_x(Align::Center);

        let mut list = Scrollable::new(&mut self.servers_list)
            .spacing(8)
            .align_items(Align::Start)
            .width(Length::Fill)
            .height(Length::Fill);

        if self.server_buttons.len() != servers.len() {
            self.server_buttons = vec![Default::default(); servers.len()];
        }

        for (i, (state, server)) in self.server_buttons.iter_mut().zip(servers).enumerate() {
            let color = if Some(i) == selected_server_index {
                (97, 255, 18)
            } else {
                (97, 97, 25)
            };
            let button = Button::new(
                state,
                Container::new(Text::new(server.as_ref()).size(fonts.cyri.scale(30)))
                    .padding(24)
                    .center_y()
                    .height(Length::Fill),
            )
            .style(
                style::button::Style::new(imgs.selection)
                    .hover_image(imgs.selection_hover)
                    .press_image(imgs.selection_press)
                    .image_color(vek::Rgba::new(color.0, color.1, color.2, 255)),
            )
            .width(Length::Fill)
            .min_height(100)
            .on_press(Message::ServerChanged(i));
            list = list.push(button);
        }

        Container::new(
            Container::new(
                Column::with_children(vec![list.into(), button.into()])
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .spacing(10)
                    .padding(20),
            )
            .style(
                style::container::Style::color_with_double_cornerless_border(
                    (22, 18, 16, 255).into(),
                    (11, 11, 11, 255).into(),
                    (54, 46, 38, 255).into(),
                ),
            )
            .max_width(500),
        )
        .width(Length::Fill)
        .align_x(Align::Center)
        .padding(80)
        .into()
    }
}
