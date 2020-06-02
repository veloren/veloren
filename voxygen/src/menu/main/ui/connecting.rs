use super::{IcedImgs as Imgs, Message};
use crate::{
    i18n::Localization,
    ui::{
        fonts::IcedFonts as Fonts,
        ice::{
            component::neat_button,
            widget::{image, BackgroundContainer, Image},
            ButtonStyle, Element,
        },
    },
};
use iced::{button, Color, Column, Container, HorizontalAlignment, Length, Row, Space, Text};

/// Connecting screen for the main menu
pub struct Screen {
    cancel_button: button::State,
}

// TODO: move to super and unify with identical login consts
const TEXT_COLOR: iced::Color = iced::Color::from_rgb(1.0, 1.0, 1.0);
const DISABLED_TEXT_COLOR: iced::Color = iced::Color::from_rgba(1.0, 1.0, 1.0, 0.2);

impl Screen {
    pub fn new() -> Self {
        Self {
            cancel_button: Default::default(),
        }
    }

    pub(super) fn view(
        &mut self,
        fonts: &Fonts,
        imgs: &Imgs,
        bg_img: image::Handle,
        start: &std::time::Instant,
        status_text: &str,
        version: &str,
        time: f32,
        i18n: &Localization,
    ) -> Element<Message> {
        let fade_msg = (time * 2.0).sin() * 0.5 + 0.51;
        let button_style = ButtonStyle::new(imgs.button)
            .hover_image(imgs.button_hover)
            .press_image(imgs.button_press)
            .text_color(TEXT_COLOR)
            .disabled_text_color(DISABLED_TEXT_COLOR);

        let version = Text::new(version)
            .size(fonts.cyri.scale(15)) // move version text size to const
            .width(Length::Fill)
            .height(Length::Fill)
            .horizontal_alignment(HorizontalAlignment::Right);

        let status = Text::new(status_text)
            .size(fonts.alkhemi.scale(80))
            .font(fonts.alkhemi.id)
            .color(Color::from_rgba(1.0, 1.0, 1.0, fade_msg))
            .width(Length::Fill);

        let status = Row::with_children(vec![
            Space::new(Length::Units(80), Length::Shrink).into(),
            status.into(),
        ]);

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

        let content = Column::with_children(vec![version.into(), status.into(), cancel.into()])
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(3);

        // Note: could replace this with styling on iced's container since we aren't
        // using fixed aspect ratio
        BackgroundContainer::new(Image::new(bg_img), content).into()
    }
}
