use super::{Imgs, LoginInfo, Message};
use crate::{
    i18n::Localization,
    ui::{
        fonts::IcedFonts as Fonts,
        ice::{
            component::neat_button,
            style,
            widget::{
                compound_graphic::{CompoundGraphic, Graphic},
                BackgroundContainer, Image, Padding,
            },
            Element,
        },
    },
};
use iced::{button, text_input, Align, Column, Container, Length, Row, Space, Text, TextInput};
use vek::*;

const FILL_FRAC_ONE: f32 = 0.77;
const FILL_FRAC_TWO: f32 = 0.53;
const INPUT_WIDTH: u16 = 280;
const INPUT_TEXT_SIZE: u16 = 24;

/// Login screen for the main menu
pub struct Screen {
    quit_button: button::State,
    settings_button: button::State,
    servers_button: button::State,

    error_okay_button: button::State,

    pub banner: Banner,
}

impl Screen {
    pub fn new() -> Self {
        Self {
            servers_button: Default::default(),
            settings_button: Default::default(),
            quit_button: Default::default(),

            error_okay_button: Default::default(),

            banner: Banner::new(),
        }
    }

    pub(super) fn view(
        &mut self,
        fonts: &Fonts,
        imgs: &Imgs,
        login_info: &LoginInfo,
        error: Option<&str>,
        i18n: &Localization,
        button_style: style::button::Style,
    ) -> Element<Message> {
        let buttons = Column::with_children(vec![
            neat_button(
                &mut self.servers_button,
                i18n.get("common.servers"),
                FILL_FRAC_ONE,
                button_style,
                Some(Message::ShowServers),
            ),
            neat_button(
                &mut self.settings_button,
                i18n.get("common.settings"),
                FILL_FRAC_ONE,
                button_style,
                None,
            ),
            neat_button(
                &mut self.quit_button,
                i18n.get("common.quit"),
                FILL_FRAC_ONE,
                button_style,
                Some(Message::Quit),
            ),
        ])
        .width(Length::Fill)
        .max_width(200)
        .spacing(5);

        let buttons = Container::new(buttons)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(Align::End);

        let intro_text = i18n.get("main.login_process");

        let info_window = BackgroundContainer::new(
            CompoundGraphic::from_graphics(vec![
                Graphic::rect(Rgba::new(0, 0, 0, 240), [500, 300], [0, 0]),
                // Note: a way to tell it to keep the height of this one piece constant and
                // unstreched would be nice, I suppose we could just break this out into a
                // column and use Length::Units
                Graphic::gradient(Rgba::new(0, 0, 0, 240), Rgba::zero(), [500, 30], [0, 300]),
            ])
            .height(Length::Shrink),
            Text::new(intro_text).size(fonts.cyri.scale(21)),
        )
        .max_width(450)
        .padding(Padding::new().horizontal(20).top(10).bottom(60));

        let left_column = Column::with_children(vec![info_window.into(), buttons.into()])
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(27)
            .into();

        let central_content = if let Some(error) = error {
            Container::new(
                Column::with_children(vec![
                    Container::new(Text::new(error)).height(Length::Fill).into(),
                    Container::new(neat_button(
                        &mut self.error_okay_button,
                        i18n.get("common.okay"),
                        FILL_FRAC_ONE,
                        button_style,
                        Some(Message::CloseError),
                    ))
                    .width(Length::Fill)
                    .height(Length::Units(30))
                    .center_x()
                    .into(),
                ])
                .height(Length::Fill)
                .width(Length::Fill),
            )
            .style(style::container::Style::color_double_cornerless_border(
                (22, 18, 16, 255).into(),
                (11, 11, 11, 255).into(),
                (54, 46, 38, 255).into(),
            ))
            .width(Length::Units(400))
            .height(Length::Units(180))
            .padding(20)
            .into()
        } else {
            self.banner
                .view(fonts, imgs, login_info, i18n, button_style)
        };

        let central_column = Container::new(central_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y();

        let right_column = Space::new(Length::Fill, Length::Fill);

        Row::with_children(vec![
            left_column,
            central_column.into(),
            right_column.into(),
        ])
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(10)
        .into()
    }
}

pub struct Banner {
    pub username: text_input::State,
    pub password: text_input::State,
    pub server: text_input::State,

    multiplayer_button: button::State,
    #[cfg(feature = "singleplayer")]
    singleplayer_button: button::State,
}

impl Banner {
    fn new() -> Self {
        Self {
            username: Default::default(),
            password: Default::default(),
            server: Default::default(),

            multiplayer_button: Default::default(),
            #[cfg(feature = "singleplayer")]
            singleplayer_button: Default::default(),
        }
    }

    fn view(
        &mut self,
        fonts: &Fonts,
        imgs: &Imgs,
        login_info: &LoginInfo,
        i18n: &Localization,
        button_style: style::button::Style,
    ) -> Element<Message> {
        let input_text_size = fonts.cyri.scale(INPUT_TEXT_SIZE);

        let banner_content = Column::with_children(vec![
            Container::new(Image::new(imgs.v_logo).fix_aspect_ratio())
                .padding(10)
                .height(Length::FillPortion(25))
                .into(),
            Column::with_children(vec![
                BackgroundContainer::new(
                    Image::new(imgs.input_bg)
                        .width(Length::Units(INPUT_WIDTH))
                        .fix_aspect_ratio(),
                    TextInput::new(
                        &mut self.username,
                        "Username",
                        &login_info.username,
                        Message::Username,
                    )
                    .size(input_text_size)
                    .on_submit(Message::FocusPassword),
                )
                .padding(Padding::new().horizontal(7).top(5))
                .into(),
                BackgroundContainer::new(
                    Image::new(imgs.input_bg)
                        .width(Length::Units(INPUT_WIDTH))
                        .fix_aspect_ratio(),
                    TextInput::new(
                        &mut self.password,
                        "Password",
                        &login_info.password,
                        Message::Password,
                    )
                    .size(input_text_size)
                    .password()
                    .on_submit(Message::Multiplayer),
                )
                .padding(Padding::new().horizontal(7).top(5))
                .into(),
                BackgroundContainer::new(
                    Image::new(imgs.input_bg)
                        .width(Length::Units(INPUT_WIDTH))
                        .fix_aspect_ratio(),
                    TextInput::new(
                        &mut self.server,
                        "Server",
                        &login_info.server,
                        Message::Server,
                    )
                    .size(input_text_size)
                    .on_submit(Message::Multiplayer),
                )
                .padding(Padding::new().horizontal(7).top(5))
                .into(),
            ])
            .spacing(10)
            .height(Length::FillPortion(35))
            .into(),
            Space::new(Length::Fill, Length::FillPortion(2)).into(),
            Column::with_children(vec![
                neat_button(
                    &mut self.multiplayer_button,
                    i18n.get("common.multiplayer"),
                    FILL_FRAC_TWO,
                    button_style,
                    Some(Message::Multiplayer),
                ),
                #[cfg(feature = "singleplayer")]
                neat_button(
                    &mut self.singleplayer_button,
                    i18n.get("common.singleplayer"),
                    FILL_FRAC_TWO,
                    button_style,
                    Some(Message::Singleplayer),
                ),
            ])
            .max_width(200)
            .height(Length::FillPortion(38))
            .spacing(8)
            .into(),
        ])
        .width(Length::Fill)
        .height(Length::Fill)
        .align_items(Align::Center);

        let banner = BackgroundContainer::new(
            CompoundGraphic::from_graphics(vec![
                Graphic::image(imgs.banner_top, [138, 17], [0, 0]),
                Graphic::rect(Rgba::new(0, 0, 0, 230), [130, 165], [4, 17]),
                // TODO: use non image gradient
                Graphic::gradient(Rgba::new(0, 0, 0, 230), Rgba::zero(), [130, 50], [4, 182]),
            ])
            .fix_aspect_ratio()
            .height(Length::Fill),
            banner_content,
        )
        .padding(Padding::new().horizontal(8).vertical(15))
        .max_width(350);

        banner.into()
    }
}
