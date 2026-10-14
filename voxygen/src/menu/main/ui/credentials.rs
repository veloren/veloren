use super::{Imgs, LoginInfo, Message, TEXT_COLOR};
use crate::ui::{
    fonts::IcedFonts as Fonts,
    ice::{
        Element, style,
        widget::{BackgroundContainer, Image, Padding},
    },
};

use credentialstore::CredentialStatus;
use i18n::Localization;
use iced::{
    Align, Button, Column, Container, Length, Row, Space, Text, TextInput, button, text_input,
};

const INPUT_WIDTH: u16 = 230;
const INPUT_TEXT_SIZE: u16 = 20;

/// Credential prompt for the connecting screen
#[derive(Default)]
pub struct Prompt {
    pub(super) error: Option<String>,
    secure_service_button: button::State,
    submit_button: button::State,
    username: text_input::State,
    password: text_input::State,
}

impl Prompt {
    pub(super) fn view(
        &mut self,
        fonts: &Fonts,
        imgs: &Imgs,
        login_info: &LoginInfo,
        auth_server: &Option<String>,
        i18n: &Localization,
        button_style: style::button::Style,
    ) -> Element<'_, Message> {
        let title_text = Text::new("Login").size(fonts.cyri.scale(25));
        let password_required = auth_server.is_some();
        let msg = if let Some(auth_server) = auth_server {
            i18n.get_msg_ctx("main-login-enter-credentials", &i18n::fluent_args! {
                "server" => auth_server
            })
        } else {
            i18n.get_msg("main-login-enter-username")
        };
        let msg_text = Text::new(msg).size(fonts.cyri.scale(15));
        let input_text_size = fonts.cyri.scale(INPUT_TEXT_SIZE);

        let username = BackgroundContainer::new(
            Image::new(imgs.input_bg)
                .width(Length::Units(INPUT_WIDTH))
                .fix_aspect_ratio(),
            TextInput::new(
                &mut self.username,
                &i18n.get_msg("main-username"),
                &login_info.username,
                Message::Username,
            )
            .size(input_text_size)
            .on_submit(if password_required {
                Message::FocusPassword
            } else {
                Message::CredentialsSubmit
            }),
        )
        .padding(Padding::new().horizontal(7).top(5));

        let mut inputs = vec![username.into()];
        if password_required {
            inputs.push(
                BackgroundContainer::new(
                    Image::new(imgs.input_bg)
                        .width(Length::Units(INPUT_WIDTH))
                        .fix_aspect_ratio(),
                    TextInput::new(
                        &mut self.password,
                        &i18n.get_msg("main-password"),
                        &login_info.password,
                        Message::Password,
                    )
                    .size(input_text_size)
                    .password()
                    .on_submit(Message::CredentialsSubmit),
                )
                .padding(Padding::new().horizontal(7).top(5))
                .into(),
            );

            if login_info.secure_service_status > CredentialStatus::Disabled {
                let text_color = if login_info.secure_service_status == CredentialStatus::Ready {
                    TEXT_COLOR
                } else {
                    // XAG 102 (Inactive Elements)
                    iced::Color::from_rgba(1.0, 1.0, 1.0, 0.4)
                };
                let checkmark = if login_info.use_secure_service {
                    "[X]"
                } else {
                    "[ ]"
                };
                let checkbox = Button::new(
                    &mut self.secure_service_button,
                    Row::with_children(vec![
                        Text::new(checkmark)
                            .size(input_text_size)
                            .color(text_color)
                            .into(),
                        Text::new(i18n.get_msg("main-remember-password"))
                            .size(input_text_size)
                            .color(text_color)
                            .into(),
                    ])
                    .spacing(6),
                );
                let checkbox = if login_info.secure_service_status == CredentialStatus::Ready {
                    checkbox.on_press(Message::UseSecureService(!login_info.use_secure_service))
                } else {
                    checkbox
                };

                inputs.push(checkbox.into());
            };
        }

        let inputs = Column::with_children(inputs)
            .spacing(5)
            .align_items(Align::Center)
            .width(Length::Fill);

        let submit = Button::new(
            &mut self.submit_button,
            Text::new(i18n.get_msg("common-okay"))
                .size(input_text_size)
                .width(Length::Fill)
                .height(Length::Fill)
                .horizontal_alignment(iced::HorizontalAlignment::Center)
                .vertical_alignment(iced::VerticalAlignment::Center),
        )
        .width(Length::Units(INPUT_WIDTH))
        .height(Length::Units(35))
        .style(button_style)
        .on_press(Message::CredentialsSubmit);

        let submit = Container::new(submit).width(Length::Fill).center_x();

        let mut children = vec![title_text.into(), msg_text.into()];
        if let Some(error) = self.error.as_deref() {
            children.push(
                Text::new(error)
                    .color(iced::Color::from_rgb(1.0, 0.7, 0.3))
                    .size(fonts.cyri.scale(15))
                    .into(),
            );
        }
        children.extend([
            Space::new(Length::Shrink, Length::Units(16)).into(),
            inputs.into(),
            submit.into(),
        ]);

        let content = Column::with_children(children)
            .spacing(4)
            .max_width(320)
            .width(Length::Fill)
            .height(Length::Shrink);

        let prompt_window = Container::new(content)
            .style(
                style::container::Style::color_with_double_cornerless_border(
                    (22, 18, 16, 255).into(),
                    (11, 11, 11, 255).into(),
                    (54, 46, 38, 255).into(),
                ),
            )
            .padding(20);

        Container::new(prompt_window)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }

    pub(super) fn focus_username(&mut self) {
        self.password = text_input::State::new();
        self.username = text_input::State::focused();
        self.username.move_cursor_to_end();
    }

    pub(super) fn focus_password(&mut self) {
        self.username = text_input::State::new();
        self.password = text_input::State::focused();
        self.password.move_cursor_to_end();
    }

    pub(super) fn tab(&mut self, password_required: bool) {
        if self.username.is_focused() && password_required {
            self.focus_password();
        } else {
            self.focus_username();
        }
    }
}
