use super::{ConnectionState, Imgs, Message};
use crate::{
    i18n::Localization,
    ui::{
        fonts::IcedFonts as Fonts,
        ice::{component::neat_button, style, widget::Image, Element},
    },
};
use iced::{button, Align, Column, Container, Length, Row, Space, Text};

const GEAR_ANIMATION_SPEED_FACTOR: f64 = 10.0;
/// Connecting screen for the main menu
pub struct Screen {
    cancel_button: button::State,
    add_button: button::State,
    tip_number: u16,
}

impl Screen {
    pub fn new() -> Self {
        Self {
            cancel_button: Default::default(),
            add_button: Default::default(),
            tip_number: rand::random(),
        }
    }

    pub(super) fn view(
        &mut self,
        fonts: &Fonts,
        imgs: &Imgs,
        connection_state: &ConnectionState,
        time: f64,
        i18n: &Localization,
        button_style: style::button::Style,
        show_tip: bool,
    ) -> Element<Message> {
        let gear_anim_time = time * GEAR_ANIMATION_SPEED_FACTOR;
        // TODO: add built in support for animated images
        let gear_anim_image = match (gear_anim_time % 5.0).trunc() as u8 {
            0 => imgs.f1,
            1 => imgs.f2,
            2 => imgs.f3,
            3 => imgs.f4,
            _ => imgs.f5,
        };

        let children = match connection_state {
            ConnectionState::InProgress => {
                let tip = if show_tip {
                    let tip = format!(
                        "{} {}",
                        &i18n.get("main.tip"),
                        &i18n.get_variation("loading.tips", self.tip_number)
                    );
                    Container::new(Text::new(tip).size(fonts.cyri.scale(25)))
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .center_x()
                        .align_y(Align::End)
                        .into()
                } else {
                    Space::new(Length::Fill, Length::Fill).into()
                };

                let cancel = Container::new(neat_button(
                    &mut self.cancel_button,
                    i18n.get("common.cancel"),
                    0.7,
                    button_style,
                    Some(Message::CancelConnect),
                ))
                .width(Length::Fill)
                .height(Length::Units(fonts.cyri.scale(40)))
                .center_x()
                .padding(3);

                let tip_cancel = Column::with_children(vec![tip, cancel.into()])
                    .width(Length::FillPortion(3))
                    .align_items(Align::Center)
                    .spacing(5)
                    .padding(5);

                let gear = Container::new(
                    Image::new(gear_anim_image)
                        .width(Length::Units(74))
                        .height(Length::Units(62)),
                )
                .width(Length::Fill)
                .padding(10)
                .align_x(Align::End);

                let bottom_content = Row::with_children(vec![
                    Space::new(Length::Fill, Length::Shrink).into(),
                    tip_cancel.into(),
                    gear.into(),
                ])
                .align_items(Align::Center)
                .width(Length::Fill);

                let left_art = Image::new(imgs.loading_art_l)
                    .width(Length::Units(12))
                    .height(Length::Units(12));
                let right_art = Image::new(imgs.loading_art_r)
                    .width(Length::Units(12))
                    .height(Length::Units(12));

                let bottom_bar = Container::new(Row::with_children(vec![
                    left_art.into(),
                    bottom_content.into(),
                    right_art.into(),
                ]))
                .height(Length::Units(100))
                .style(style::container::Style::image(imgs.loading_art));

                vec![
                    Space::new(Length::Fill, Length::Fill).into(),
                    bottom_bar.into(),
                ]
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
                    .style(
                        style::container::Style::color_with_double_cornerless_border(
                            (22, 18, 16, 255).into(),
                            (11, 11, 11, 255).into(),
                            (54, 46, 38, 255).into(),
                        ),
                    )
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
