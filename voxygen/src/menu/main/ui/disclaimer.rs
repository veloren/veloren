use super::{IcedImgs as Imgs, Message};
use crate::{
    i18n::Localization,
    ui::{
        fonts::IcedFonts as Fonts,
        ice::{component::neat_button, style, Element},
    },
};
use iced::{button, scrollable, Column, Container, Length, Scrollable, Space};
use vek::Rgba;

/// Connecting screen for the main menu
pub struct Screen {
    accept_button: button::State,
    scroll: scrollable::State,
}

impl Screen {
    pub fn new() -> Self {
        Self {
            accept_button: Default::default(),
            scroll: Default::default(),
        }
    }

    pub(super) fn view(
        &mut self,
        fonts: &Fonts,
        imgs: &Imgs,
        i18n: &Localization,
        button_style: style::button::Style,
    ) -> Element<Message> {
        Container::new(
            Container::new(
                Column::with_children(vec![
                    iced::Text::new(i18n.get("common.disclaimer"))
                        .font(fonts.alkhemi.id)
                        .size(fonts.alkhemi.scale(35))
                        .into(),
                    Space::new(Length::Fill, Length::Units(20)).into(),
                    Scrollable::new(&mut self.scroll)
                        .push(
                            iced::Text::new(i18n.get("main.notice"))
                                .font(fonts.cyri.id)
                                .size(fonts.cyri.scale(23)),
                        )
                        .height(Length::FillPortion(1))
                        .into(),
                    Container::new(
                        Container::new(neat_button(
                            &mut self.accept_button,
                            i18n.get("common.accept"),
                            0.7,
                            button_style,
                            Some(Message::AcceptDisclaimer),
                        ))
                        .height(Length::Units(fonts.cyri.scale(50))),
                    )
                    .center_x()
                    .height(Length::Shrink)
                    .width(Length::Fill)
                    .into(),
                ])
                .spacing(5)
                .padding(20)
                .width(Length::Fill)
                .height(Length::Fill),
            )
            .style(style::container::Style::Color(Rgba::new(22, 19, 17, 255))),
        )
        .center_x()
        .center_y()
        .padding(70)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
