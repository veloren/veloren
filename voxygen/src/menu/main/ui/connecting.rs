use super::{ConnectionState, Message};
use crate::{
    i18n::Localization,
    ui::{
        fonts::IcedFonts as Fonts,
        ice::{component::neat_button, style, Element},
    },
};
use iced::{button, Align, Color, Column, Container, Length, Row, Space, Text};

/// Connecting screen for the main menu
pub struct Screen {
    cancel_button: button::State,
    add_button: button::State,
}

impl Screen {
    pub fn new() -> Self {
        Self {
            cancel_button: Default::default(),
            add_button: Default::default(),
        }
    }

    pub(super) fn view(
        &mut self,
        fonts: &Fonts,
        connection_state: &ConnectionState,
        time: f32,
        i18n: &Localization,
        button_style: style::button::Style,
    ) -> Element<Message> {
        let fade_msg = (time * 2.0).sin() * 0.5 + 0.51;

        let children = match connection_state {
            ConnectionState::InProgress { status } => {
                let status = Text::new(status)
                    .size(fonts.alkhemi.scale(80))
                    .font(fonts.alkhemi.id)
                    .color(Color::from_rgba(1.0, 1.0, 1.0, fade_msg));

                let status = Container::new(Row::with_children(vec![
                    Space::new(Length::Units(80), Length::Shrink).into(),
                    status.into(),
                ]))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(Align::End);

                let cancel = neat_button(
                    &mut self.cancel_button,
                    i18n.get("common.cancel"),
                    0.7,
                    button_style,
                    Some(Message::CancelConnect),
                );

                let cancel = Container::new(cancel)
                    .width(Length::Fill)
                    .height(Length::Units(fonts.cyri.scale(50)))
                    .center_x()
                    .padding(3);

                vec![status.into(), cancel.into()]
            },
            ConnectionState::AuthTrustPrompt { msg, .. } => {
                let text = Text::new(msg).size(fonts.cyri.scale(25));

                let cancel = neat_button(
                    &mut self.cancel_button,
                    i18n.get("common.cancel"),
                    0.7,
                    button_style,
                    Some(Message::TrustPromptCancel),
                );
                let add = neat_button(
                    &mut self.add_button,
                    i18n.get("common.add"),
                    0.7,
                    button_style,
                    Some(Message::TrustPromptAdd),
                );

                let content = Column::with_children(vec![
                    text.into(),
                    Container::new(
                        Row::with_children(vec![cancel.into(), add.into()])
                            .spacing(20)
                            .height(Length::Units(25)),
                    )
                    .align_x(Align::End)
                    .width(Length::Fill)
                    .into(),
                ])
                .spacing(4)
                .max_width(520)
                .width(Length::Fill)
                .height(Length::Fill);

                let prompt_window = Container::new(content)
                    .style(style::container::Style::color_double_cornerless_border(
                        (22, 18, 16, 255).into(),
                        (11, 11, 11, 255).into(),
                        (54, 46, 38, 255).into(),
                    ))
                    .padding(20);

                let container = Container::new(prompt_window)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x()
                    .center_y();

                vec![
                    container.into(),
                    Space::new(Length::Fill, Length::Units(fonts.cyri.scale(15))).into(),
                ]
            },
        };

        Column::with_children(children)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
