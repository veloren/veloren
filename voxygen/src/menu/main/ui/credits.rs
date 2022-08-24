use super::Message;
use crate::{
    credits::Credits,
    ui::{
        fonts::IcedFonts as Fonts,
        ice::{component::neat_button, style, Element},
    },
};
use i18n::Localization;
use iced::{button, scrollable, Column, Container, HorizontalAlignment, Length, Scrollable, Space};

/// Connecting screen for the main menu
pub struct Screen {
    back_button: button::State,
    scroll: scrollable::State,
}

impl Screen {
    pub fn new() -> Self {
        Self {
            back_button: Default::default(),
            scroll: Default::default(),
        }
    }

    pub(super) fn view(
        &mut self,
        fonts: &Fonts,
        i18n: &Localization,
        credits: &Credits,
        button_style: style::button::Style,
    ) -> Element<Message> {
        use core::fmt::Write;
        let format_art_credit = |credit: &crate::credits::Art| -> Result<String, core::fmt::Error> {
            let mut text = String::new();
            write!(&mut text, "\"{}\"", &credit.name)?;

            let mut authors = credit.authors.iter();
            if let Some(author) = authors.next() {
                write!(
                    &mut text,
                    " {} {}",
                    i18n.get_msg("main-credits-created_by"),
                    author
                )?;
            }
            authors.try_for_each(|author| write!(&mut text, ", {}", author))?;

            if !credit.license.is_empty() {
                write!(&mut text, " ({})", &credit.license)?;
            }

            Ok::<_, core::fmt::Error>(text)
        };
        let format_contributor_credit =
            |credit: &crate::credits::Contributor| -> Result<String, core::fmt::Error> {
                let mut text = String::new();
                text.push_str(&credit.name);

                if !credit.contributions.is_empty() {
                    write!(&mut text, "- {}", &credit.contributions)?;
                }

                Ok(text)
            };

        let music_header_color = iced::Color::from_rgb8(0xfc, 0x71, 0x76);
        let fonts_header_color = iced::Color::from_rgb8(0xf7, 0xd1, 0x81);
        let other_art_header_color = iced::Color::from_rgb8(0xc5, 0xe9, 0x80);
        let contributors_header_color = iced::Color::from_rgb8(0x4a, 0xa6, 0x7b);

        fn credit_section<'a, T>(
            header_i18n_key: &str,
            header_color: iced::Color,
            credit_iter: impl Iterator<Item = T>,
            format_credit: impl Fn(T) -> Result<String, core::fmt::Error>,
            fonts: &Fonts,
            i18n: &Localization,
        ) -> Element<'a, Message> {
            Column::with_children(
                core::iter::once(
                    iced::Text::new(i18n.get_msg(header_i18n_key))
                        .font(fonts.cyri.id)
                        .size(fonts.cyri.scale(30))
                        .color(header_color)
                        .width(Length::Fill)
                        .horizontal_alignment(HorizontalAlignment::Center)
                        .into(),
                )
                .chain(credit_iter.map(|credit| {
                    let text = format_credit(credit).expect("Formatting failed!!!");
                    iced::Text::new(text)
                        .font(fonts.cyri.id)
                        .size(fonts.cyri.scale(23))
                        .width(Length::Fill)
                        .horizontal_alignment(HorizontalAlignment::Center)
                        .into()
                }))
                .chain(core::iter::once(
                    Space::new(Length::Fill, Length::Units(15)).into(),
                ))
                .collect(),
            )
            .width(Length::Fill)
            .into()
        }

        let art_section = |header_i18n_key, header_color, art: &[_]| {
            credit_section(
                header_i18n_key,
                header_color,
                art.iter(),
                format_art_credit,
                fonts,
                i18n,
            )
        };

        Container::new(
            Container::new(
                Column::with_children(vec![
                    iced::Text::new(i18n.get_msg("main-credits"))
                        .font(fonts.alkhemi.id)
                        .size(fonts.alkhemi.scale(35))
                        .width(Length::Fill)
                        .horizontal_alignment(HorizontalAlignment::Center)
                        .into(),
                    Space::new(Length::Fill, Length::Units(25)).into(),
                    Scrollable::new(&mut self.scroll)
                        .push(art_section(
                            "main-credits-music",
                            music_header_color,
                            &credits.music,
                        ))
                        .push(art_section(
                            "main-credits-fonts",
                            fonts_header_color,
                            &credits.fonts,
                        ))
                        .push(art_section(
                            "main-credits-other_art",
                            other_art_header_color,
                            &credits.other_art,
                        ))
                        .push(credit_section(
                            "main-credits-contributors",
                            contributors_header_color,
                            credits.contributors.iter(),
                            format_contributor_credit,
                            fonts,
                            i18n,
                        ))
                        .height(Length::FillPortion(1))
                        .width(Length::Fill)
                        .into(),
                    Container::new(
                        Container::new(neat_button(
                            &mut self.back_button,
                            i18n.get_msg("common-back"),
                            0.7,
                            button_style,
                            Some(Message::Back),
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
            .style(
                style::container::Style::color_with_double_cornerless_border(
                    (22, 19, 17, 255).into(),
                    (11, 11, 11, 255).into(),
                    (54, 46, 38, 255).into(),
                ),
            ),
        )
        .center_x()
        .center_y()
        .padding(70)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
