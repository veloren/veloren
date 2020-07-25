use super::{Imgs, LoginInfo, Message, FILL_FRAC_ONE, FILL_FRAC_TWO};
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
use iced::{
    button, scrollable, text_input, Align, Button, Column, Container, Length, Row, Scrollable,
    Space, Text, TextInput,
};
use vek::*;

const INPUT_WIDTH: u16 = 280;
const INPUT_TEXT_SIZE: u16 = 24;

/// Login screen for the main menu
pub struct Screen {
    quit_button: button::State,
    settings_button: button::State,
    servers_button: button::State,
    language_select_button: button::State,

    error_okay_button: button::State,

    pub banner: LoginBanner,
    language_selection: LanguageSelectBanner,
}

impl Screen {
    pub fn new() -> Self {
        Self {
            servers_button: Default::default(),
            settings_button: Default::default(),
            quit_button: Default::default(),
            language_select_button: Default::default(),

            error_okay_button: Default::default(),

            banner: LoginBanner::new(),
            language_selection: LanguageSelectBanner::new(),
        }
    }

    pub(super) fn view(
        &mut self,
        fonts: &Fonts,
        imgs: &Imgs,
        login_info: &LoginInfo,
        error: Option<&str>,
        i18n: &Localization,
        is_selecting_language: bool,
        selected_language_index: Option<usize>,
        language_metadatas: &[crate::i18n::LanguageMetadata],
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
            neat_button(
                &mut self.language_select_button,
                i18n.get("common.languages"),
                FILL_FRAC_ONE,
                button_style,
                Some(Message::OpenLanguageMenu),
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
                Graphic::gradient(Rgba::new(0, 0, 0, 240), Rgba::zero(), [500, 50], [0, 300]),
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
            .style(
                style::container::Style::color_with_double_cornerless_border(
                    (22, 18, 16, 255).into(),
                    (11, 11, 11, 255).into(),
                    (54, 46, 38, 255).into(),
                ),
            )
            .width(Length::Units(400))
            .height(Length::Units(180))
            .padding(20)
            .into()
        } else {
            if is_selecting_language {
                self.language_selection.view(
                    fonts,
                    imgs,
                    i18n,
                    language_metadatas,
                    selected_language_index,
                    button_style,
                )
            } else {
                self.banner
                    .view(fonts, imgs, login_info, i18n, button_style)
            }
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

pub struct LanguageSelectBanner {
    okay_button: button::State,
    language_buttons: Vec<button::State>,

    selection_list: scrollable::State,
}

impl LanguageSelectBanner {
    fn new() -> Self {
        Self {
            okay_button: Default::default(),
            language_buttons: Default::default(),
            selection_list: Default::default(),
        }
    }

    fn view(
        &mut self,
        fonts: &Fonts,
        imgs: &Imgs,
        i18n: &Localization,
        language_metadatas: &[crate::i18n::LanguageMetadata],
        selected_language_index: Option<usize>,
        button_style: style::button::Style,
    ) -> Element<Message> {
        // Reset button states if languages were added / removed
        if self.language_buttons.len() != language_metadatas.len() {
            self.language_buttons = vec![Default::default(); language_metadatas.len()];
        }

        let title = Text::new(i18n.get("main.login.select_language"))
            .size(fonts.cyri.scale(35))
            .horizontal_alignment(iced::HorizontalAlignment::Center);

        let mut list = Scrollable::new(&mut self.selection_list)
            .spacing(8)
            .height(Length::Fill)
            .align_items(Align::Start);

        let list_items = self
            .language_buttons
            .iter_mut()
            .zip(language_metadatas)
            .enumerate()
            .map(|(i, (state, lang))| {
                let color = if Some(i) == selected_language_index {
                    (97, 255, 18)
                } else {
                    (97, 97, 25)
                };
                let button = Button::new(
                    state,
                    Row::with_children(vec![
                        Space::new(Length::FillPortion(5), Length::Units(0)).into(),
                        Text::new(lang.language_name.clone())
                            .width(Length::FillPortion(95))
                            .size(fonts.cyri.scale(25))
                            .vertical_alignment(iced::VerticalAlignment::Center)
                            .into(),
                    ]),
                )
                .style(
                    style::button::Style::new(imgs.selection)
                        .hover_image(imgs.selection_hover)
                        .press_image(imgs.selection_press)
                        .image_color(vek::Rgba::new(color.0, color.1, color.2, 192)),
                )
                .min_height(56)
                .on_press(Message::LanguageChanged(i));
                Row::with_children(vec![
                    Space::new(Length::FillPortion(3), Length::Units(0)).into(),
                    button.width(Length::FillPortion(92)).into(),
                    Space::new(Length::FillPortion(5), Length::Units(0)).into(),
                ])
            });

        for item in list_items {
            list = list.push(item);
        }

        let okay_button = Container::new(neat_button(
            &mut self.okay_button,
            i18n.get("common.okay"),
            FILL_FRAC_TWO,
            button_style,
            Some(Message::OpenLanguageMenu),
        ))
        .center_x()
        .max_width(200);

        let content = Column::with_children(vec![title.into(), list.into(), okay_button.into()])
            .spacing(8)
            .width(Length::Fill)
            .height(Length::FillPortion(38))
            .align_items(Align::Center);

        let selection_menu = BackgroundContainer::new(
            CompoundGraphic::from_graphics(vec![
                Graphic::image(imgs.banner_top, [138, 17], [0, 0]),
                Graphic::rect(Rgba::new(0, 0, 0, 230), [130, 165], [4, 17]),
                // TODO: use non image gradient
                Graphic::gradient(Rgba::new(0, 0, 0, 230), Rgba::zero(), [130, 50], [4, 182]),
            ])
            .fix_aspect_ratio()
            .height(Length::Fill),
            content,
        )
        .padding(Padding::new().horizontal(5).top(15).bottom(50))
        .max_width(350);

        selection_menu.into()
    }
}

pub struct LoginBanner {
    pub username: text_input::State,
    pub password: text_input::State,
    pub server: text_input::State,

    multiplayer_button: button::State,
    #[cfg(feature = "singleplayer")]
    singleplayer_button: button::State,
}

impl LoginBanner {
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
            // TODO: i18n
            Column::with_children(vec![
                BackgroundContainer::new(
                    Image::new(imgs.input_bg)
                        .width(Length::Units(INPUT_WIDTH))
                        .fix_aspect_ratio(),
                    TextInput::new(
                        &mut self.username,
                        i18n.get("main.username"),
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
                        i18n.get("main.password"),
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
                        i18n.get("main.server"),
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
