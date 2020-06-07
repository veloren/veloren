use super::{IcedImgs as Imgs, Message};
use crate::{
    i18n::Localization,
    ui::{
        fonts::IcedFonts as Fonts,
        ice::{
            component::neat_button,
            style,
            widget::{
                background_container::Padding,
                compound_graphic::{CompoundGraphic, Graphic},
                BackgroundContainer,
            },
            Element,
        },
    },
};
use iced::{button, scrollable, Align, Button, Column, Container, Length, Scrollable, Text};

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
        servers: &Vec<String>,
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
            .align_items(Align::Start)
            .width(Length::Fill)
            .height(Length::Fill)
            .spacing(10);

        if self.server_buttons.len() != servers.len() {
            self.server_buttons = vec![Default::default(); servers.len()];
        }

        for (i, state) in self.server_buttons.iter_mut().enumerate() {
            let server = servers.get(i).unwrap();
            let text = format!(
                "{}{}",
                if i == selected_server_index.unwrap_or(std::usize::MAX) {
                    "-> "
                } else {
                    "  "
                },
                server
            );
            let button = Button::new(state, Text::new(text).size(fonts.cyri.scale(25)))
                .on_press(Message::ServerChanged(i));
            list = list.push(button);
        }

        Container::new(
            BackgroundContainer::new(
                CompoundGraphic::padded_image(imgs.info_frame, [500, 300], [0; 4]),
                Column::with_children(vec![list.into(), button.into()])
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .spacing(10)
                    .padding(20),
            )
            .max_width(500),
        )
        .width(Length::Fill)
        .align_x(Align::Center)
        .padding(80)
        .into()
    }
}
