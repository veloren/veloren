use common::resources::MapKind;
use i18n::Localization;
use iced::{
    button, scrollable, slider, text_input, Align, Button, Column, Container, Length, Row,
    Scrollable, Slider, Space, Text, TextInput,
};
use rand::Rng;
use vek::Rgba;

use crate::{
    menu::main::ui::{WorldsChange, FILL_FRAC_TWO},
    ui::{
        fonts::IcedFonts,
        ice::{
            component::neat_button,
            style,
            widget::{
                compound_graphic::{CompoundGraphic, Graphic},
                BackgroundContainer, Image, Overlay, Padding,
            },
            Element,
        },
    },
};

use super::{Imgs, Message};

const INPUT_TEXT_SIZE: u16 = 20;

#[derive(Clone)]
pub enum Confirmation {
    Regenerate(usize),
    Delete(usize),
}

#[derive(Default)]
pub struct Screen {
    back_button: button::State,
    play_button: button::State,
    new_button: button::State,
    yes_button: button::State,
    no_button: button::State,

    worlds_buttons: Vec<button::State>,

    selection_list: scrollable::State,

    world_name: text_input::State,
    map_seed: text_input::State,
    day_length: slider::State,
    random_seed_button: button::State,
    world_size_x: slider::State,
    world_size_y: slider::State,

    map_vertical_scale: slider::State,
    shape_buttons: enum_map::EnumMap<MapKind, button::State>,
    map_erosion_quality: slider::State,

    delete_world: button::State,
    regenerate_map: button::State,
    generate_map: button::State,

    pub confirmation: Option<Confirmation>,
}

impl Screen {
    pub(super) fn view(
        &mut self,
        fonts: &IcedFonts,
        imgs: &Imgs,
        worlds: &crate::singleplayer::SingleplayerWorlds,
        i18n: &Localization,
        button_style: style::button::Style,
    ) -> Element<Message> {
        let input_text_size = fonts.cyri.scale(INPUT_TEXT_SIZE);

        let worlds_count = worlds.worlds.len();
        if self.worlds_buttons.len() != worlds_count {
            self.worlds_buttons = vec![Default::default(); worlds_count];
        }

        let title = Text::new(i18n.get_msg("gameinput-map"))
            .size(fonts.cyri.scale(35))
            .horizontal_alignment(iced::HorizontalAlignment::Center);

        let mut list = Scrollable::new(&mut self.selection_list)
            .spacing(8)
            .height(Length::Fill)
            .align_items(Align::Start);

        let list_items = self
            .worlds_buttons
            .iter_mut()
            .zip(
                worlds
                    .worlds
                    .iter()
                    .enumerate()
                    .map(|(i, w)| (Some(i), &w.name)),
            )
            .map(|(state, (i, map))| {
                let color = if i == worlds.current {
                    (97, 255, 18)
                } else {
                    (97, 97, 25)
                };
                let button = Button::new(
                    state,
                    Row::with_children(vec![
                        Space::new(Length::FillPortion(5), Length::Units(0)).into(),
                        Text::new(map)
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
                        .image_color(Rgba::new(color.0, color.1, color.2, 192)),
                )
                .min_height(56)
                .on_press(Message::WorldChanged(super::WorldsChange::SetActive(i)));
                Row::with_children(vec![
                    Space::new(Length::FillPortion(3), Length::Units(0)).into(),
                    button.width(Length::FillPortion(92)).into(),
                    Space::new(Length::FillPortion(5), Length::Units(0)).into(),
                ])
            });

        for item in list_items {
            list = list.push(item);
        }

        let new_button = Container::new(neat_button(
            &mut self.new_button,
            i18n.get_msg("main-singleplayer-new"),
            FILL_FRAC_TWO,
            button_style,
            Some(Message::WorldChanged(super::WorldsChange::AddNew)),
        ))
        .center_x()
        .max_width(200);

        let back_button = Container::new(neat_button(
            &mut self.back_button,
            i18n.get_msg("common-back"),
            FILL_FRAC_TWO,
            button_style,
            Some(Message::Back),
        ))
        .center_x()
        .max_width(200);

        let content = Column::with_children(vec![
            title.into(),
            list.into(),
            new_button.into(),
            back_button.into(),
        ])
        .spacing(8)
        .width(Length::Fill)
        .height(Length::FillPortion(38))
        .align_items(Align::Center)
        .padding(iced::Padding {
            bottom: 25,
            ..iced::Padding::new(0)
        });

        let selection_menu = BackgroundContainer::new(
            CompoundGraphic::from_graphics(vec![
                Graphic::image(imgs.banner_top, [138, 17], [0, 0]),
                Graphic::rect(Rgba::new(0, 0, 0, 230), [130, 300], [4, 17]),
                // TODO: use non image gradient
                Graphic::gradient(Rgba::new(0, 0, 0, 230), Rgba::zero(), [130, 50], [4, 182]),
            ])
            .fix_aspect_ratio()
            .height(Length::Fill)
            .width(Length::Fill),
            content,
        )
        .padding(Padding::new().horizontal(5).top(15));
        let mut items = vec![selection_menu.into()];

        if let Some(i) = worlds.current {
            let world = &worlds.worlds[i];
            let can_edit = !world.is_generated;
            let message = |m| Message::WorldChanged(super::WorldsChange::CurrentWorldChange(m));

            use super::WorldChange;

            const SLIDER_TEXT_SIZE: u16 = 20;
            const SLIDER_CURSOR_SIZE: (u16, u16) = (9, 21);
            const SLIDER_BAR_HEIGHT: u16 = 9;
            const SLIDER_BAR_PAD: u16 = 0;
            // Height of interactable area
            const SLIDER_HEIGHT: u16 = 30;
            // Day length slider values
            pub const DAY_LENGTH_MIN: f64 = 10.0;
            pub const DAY_LENGTH_MAX: f64 = 60.0;

            let mut gen_content = vec![
                BackgroundContainer::new(
                    Image::new(imgs.input_bg)
                        .width(Length::Units(230))
                        .fix_aspect_ratio(),
                    if can_edit {
                        Element::from(
                            TextInput::new(
                                &mut self.world_name,
                                &i18n.get_msg("main-singleplayer-world_name"),
                                &world.name,
                                move |s| message(WorldChange::Name(s)),
                            )
                            .size(input_text_size),
                        )
                    } else {
                        Text::new(&world.name)
                            .size(input_text_size)
                            .width(Length::Fill)
                            .height(Length::Shrink)
                            .into()
                    },
                )
                .padding(Padding::new().horizontal(7).top(5))
                .into(),
            ];

            let seed = world.seed;
            let seed_str = i18n.get_msg("main-singleplayer-seed");
            let mut seed_content = vec![
                Column::with_children(vec![
                    Text::new(seed_str.to_string())
                        .size(SLIDER_TEXT_SIZE)
                        .horizontal_alignment(iced::HorizontalAlignment::Center)
                        .into(),
                ])
                .padding(iced::Padding::new(5))
                .into(),
                BackgroundContainer::new(
                    Image::new(imgs.input_bg)
                        .width(Length::Units(190))
                        .fix_aspect_ratio(),
                    if can_edit {
                        Element::from(
                            TextInput::new(
                                &mut self.map_seed,
                                &seed_str,
                                &seed.to_string(),
                                move |s| {
                                    if let Ok(seed) = if s.is_empty() {
                                        Ok(0)
                                    } else {
                                        s.parse::<u32>()
                                    } {
                                        message(WorldChange::Seed(seed))
                                    } else {
                                        message(WorldChange::Seed(seed))
                                    }
                                },
                            )
                            .size(input_text_size),
                        )
                    } else {
                        Text::new(world.seed.to_string())
                            .size(input_text_size)
                            .width(Length::Fill)
                            .height(Length::Shrink)
                            .into()
                    },
                )
                .padding(Padding::new().horizontal(7).top(5))
                .into(),
            ];

            if can_edit {
                seed_content.push(
                    Container::new(neat_button(
                        &mut self.random_seed_button,
                        i18n.get_msg("main-singleplayer-random_seed"),
                        FILL_FRAC_TWO,
                        button_style,
                        Some(message(WorldChange::Seed(rand::thread_rng().gen()))),
                    ))
                    .max_width(200)
                    .into(),
                )
            }

            gen_content.push(Row::with_children(seed_content).into());

            if let Some(gen_opts) = world.gen_opts.as_ref() {
                // Day length setting label
                gen_content.push(
                    Text::new(format!(
                        "{}: {}",
                        i18n.get_msg("main-singleplayer-day_length"),
                        world.day_length
                    ))
                    .size(SLIDER_TEXT_SIZE)
                    .horizontal_alignment(iced::HorizontalAlignment::Center)
                    .into(),
                );

                // Day length setting slider
                if can_edit {
                    gen_content.push(
                        Row::with_children(vec![
                            Slider::new(
                                &mut self.day_length,
                                DAY_LENGTH_MIN..=DAY_LENGTH_MAX,
                                world.day_length,
                                move |d| message(WorldChange::DayLength(d)),
                            )
                            .height(SLIDER_HEIGHT)
                            .style(style::slider::Style::images(
                                imgs.slider_indicator,
                                imgs.slider_range,
                                SLIDER_BAR_PAD,
                                SLIDER_CURSOR_SIZE,
                                SLIDER_BAR_HEIGHT,
                            ))
                            .into(),
                        ])
                        .into(),
                    )
                }

                gen_content.push(
                    Text::new(format!(
                        "{}: x: {}, y: {}",
                        i18n.get_msg("main-singleplayer-size_lg"),
                        gen_opts.x_lg,
                        gen_opts.y_lg
                    ))
                    .size(SLIDER_TEXT_SIZE)
                    .horizontal_alignment(iced::HorizontalAlignment::Center)
                    .into(),
                );

                if can_edit {
                    gen_content.push(
                        Row::with_children(vec![
                            Slider::new(&mut self.world_size_x, 4..=13, gen_opts.x_lg, move |s| {
                                message(WorldChange::SizeX(s))
                            })
                            .height(SLIDER_HEIGHT)
                            .style(style::slider::Style::images(
                                imgs.slider_indicator,
                                imgs.slider_range,
                                SLIDER_BAR_PAD,
                                SLIDER_CURSOR_SIZE,
                                SLIDER_BAR_HEIGHT,
                            ))
                            .into(),
                            Slider::new(&mut self.world_size_y, 4..=13, gen_opts.y_lg, move |s| {
                                message(WorldChange::SizeY(s))
                            })
                            .height(SLIDER_HEIGHT)
                            .style(style::slider::Style::images(
                                imgs.slider_indicator,
                                imgs.slider_range,
                                SLIDER_BAR_PAD,
                                SLIDER_CURSOR_SIZE,
                                SLIDER_BAR_HEIGHT,
                            ))
                            .into(),
                        ])
                        .into(),
                    );
                    let height = Length::Units(56);
                    if gen_opts.x_lg + gen_opts.y_lg >= 19 {
                        gen_content.push(
                            Text::new(i18n.get_msg("main-singleplayer-map_large_warning"))
                                .size(SLIDER_TEXT_SIZE)
                                .height(height)
                                .color([0.914, 0.835, 0.008])
                                .horizontal_alignment(iced::HorizontalAlignment::Center)
                                .into(),
                        );
                    } else {
                        gen_content.push(Space::new(Length::Units(0), height).into());
                    }
                }

                gen_content.push(
                    Text::new(format!(
                        "{}: {}",
                        i18n.get_msg("main-singleplayer-map_scale"),
                        gen_opts.scale
                    ))
                    .size(SLIDER_TEXT_SIZE)
                    .horizontal_alignment(iced::HorizontalAlignment::Center)
                    .into(),
                );

                if can_edit {
                    gen_content.push(
                        Slider::new(
                            &mut self.map_vertical_scale,
                            0.0..=160.0,
                            gen_opts.scale * 10.0,
                            move |s| message(WorldChange::Scale(s / 10.0)),
                        )
                        .height(SLIDER_HEIGHT)
                        .style(style::slider::Style::images(
                            imgs.slider_indicator,
                            imgs.slider_range,
                            SLIDER_BAR_PAD,
                            SLIDER_CURSOR_SIZE,
                            SLIDER_BAR_HEIGHT,
                        ))
                        .into(),
                    );
                }

                if can_edit {
                    gen_content.extend([
                        Text::new(i18n.get_msg("main-singleplayer-map_shape"))
                            .size(SLIDER_TEXT_SIZE)
                            .horizontal_alignment(iced::HorizontalAlignment::Center)
                            .into(),
                        Row::with_children(
                            self.shape_buttons
                                .iter_mut()
                                .map(|(shape, state)| {
                                    let color = if gen_opts.map_kind == shape {
                                        (97, 255, 18)
                                    } else {
                                        (97, 97, 25)
                                    };
                                    Button::new(
                                        state,
                                        Row::with_children(vec![
                                            Space::new(Length::FillPortion(5), Length::Units(0))
                                                .into(),
                                            Text::new(shape.to_string())
                                                .width(Length::FillPortion(95))
                                                .size(fonts.cyri.scale(14))
                                                .vertical_alignment(iced::VerticalAlignment::Center)
                                                .into(),
                                        ])
                                        .align_items(Align::Center),
                                    )
                                    .style(
                                        style::button::Style::new(imgs.selection)
                                            .hover_image(imgs.selection_hover)
                                            .press_image(imgs.selection_press)
                                            .image_color(Rgba::new(color.0, color.1, color.2, 192)),
                                    )
                                    .width(Length::FillPortion(1))
                                    .min_height(18)
                                    .on_press(Message::WorldChanged(
                                        super::WorldsChange::CurrentWorldChange(
                                            WorldChange::MapKind(shape),
                                        ),
                                    ))
                                    .into()
                                })
                                .collect(),
                        )
                        .into(),
                    ]);
                } else {
                    gen_content.push(
                        Text::new(format!(
                            "{}: {}",
                            i18n.get_msg("main-singleplayer-map_shape"),
                            gen_opts.map_kind,
                        ))
                        .size(SLIDER_TEXT_SIZE)
                        .horizontal_alignment(iced::HorizontalAlignment::Center)
                        .into(),
                    );
                }

                gen_content.push(
                    Text::new(format!(
                        "{}: {}",
                        i18n.get_msg("main-singleplayer-map_erosion_quality"),
                        gen_opts.erosion_quality
                    ))
                    .size(SLIDER_TEXT_SIZE)
                    .horizontal_alignment(iced::HorizontalAlignment::Center)
                    .into(),
                );

                if can_edit {
                    gen_content.push(
                        Slider::new(
                            &mut self.map_erosion_quality,
                            0.0..=20.0,
                            gen_opts.erosion_quality * 10.0,
                            move |s| message(WorldChange::ErosionQuality(s / 10.0)),
                        )
                        .height(SLIDER_HEIGHT)
                        .style(style::slider::Style::images(
                            imgs.slider_indicator,
                            imgs.slider_range,
                            SLIDER_BAR_PAD,
                            SLIDER_CURSOR_SIZE,
                            SLIDER_BAR_HEIGHT,
                        ))
                        .into(),
                    );
                }
            }

            let mut world_buttons = vec![];

            if world.gen_opts.is_none() && can_edit {
                let create_custom = Container::new(neat_button(
                    &mut self.regenerate_map,
                    i18n.get_msg("main-singleplayer-create_custom"),
                    FILL_FRAC_TWO,
                    button_style,
                    Some(Message::WorldChanged(
                        super::WorldsChange::CurrentWorldChange(WorldChange::DefaultGenOps),
                    )),
                ))
                .center_x()
                .width(Length::FillPortion(1))
                .max_width(200);
                world_buttons.push(create_custom.into());
            }

            if world.is_generated {
                let regenerate = Container::new(neat_button(
                    &mut self.generate_map,
                    i18n.get_msg("main-singleplayer-regenerate"),
                    FILL_FRAC_TWO,
                    button_style,
                    Some(Message::WorldConfirmation(Confirmation::Regenerate(i))),
                ))
                .center_x()
                .width(Length::FillPortion(1))
                .max_width(200);
                world_buttons.push(regenerate.into())
            }
            let delete = Container::new(neat_button(
                &mut self.delete_world,
                i18n.get_msg("main-singleplayer-delete"),
                FILL_FRAC_TWO,
                button_style,
                Some(Message::WorldConfirmation(Confirmation::Delete(i))),
            ))
            .center_x()
            .width(Length::FillPortion(1))
            .max_width(200);

            world_buttons.push(delete.into());

            gen_content.push(Row::with_children(world_buttons).into());

            let play_button = Container::new(neat_button(
                &mut self.play_button,
                i18n.get_msg(if world.is_generated || world.gen_opts.is_none() {
                    "main-singleplayer-play"
                } else {
                    "main-singleplayer-generate_and_play"
                }),
                FILL_FRAC_TWO,
                button_style,
                Some(Message::SingleplayerPlay),
            ))
            .center_x()
            .max_width(200);

            gen_content.push(play_button.into());

            let gen_opts = Column::with_children(gen_content).align_items(Align::Center);

            let opts_menu = BackgroundContainer::new(
                CompoundGraphic::from_graphics(vec![
                    Graphic::image(imgs.banner_top, [138, 17], [0, 0]),
                    Graphic::rect(Rgba::new(0, 0, 0, 230), [130, 300], [4, 17]),
                    // TODO: use non image gradient
                    Graphic::gradient(Rgba::new(0, 0, 0, 230), Rgba::zero(), [130, 50], [4, 182]),
                ])
                .fix_aspect_ratio()
                .height(Length::Fill)
                .width(Length::Fill),
                gen_opts,
            )
            .padding(Padding::new().horizontal(5).top(15));

            items.push(opts_menu.into());
        }

        let all = Row::with_children(items)
            .height(Length::Fill)
            .width(Length::Fill);

        if let Some(confirmation) = self.confirmation.as_ref() {
            const FILL_FRAC_ONE: f32 = 0.77;

            let (text, yes_msg, index) = match confirmation {
                Confirmation::Regenerate(i) => (
                    "menu-singleplayer-confirm_regenerate",
                    Message::WorldChanged(WorldsChange::Regenerate(*i)),
                    i,
                ),
                Confirmation::Delete(i) => (
                    "menu-singleplayer-confirm_delete",
                    Message::WorldChanged(WorldsChange::Delete(*i)),
                    i,
                ),
            };

            if let Some(name) = worlds.worlds.get(*index).map(|world| &world.name) {
                let over_content = Column::with_children(vec![
                    Text::new(i18n.get_msg_ctx(text, &i18n::fluent_args! { "world_name" => name }))
                        .size(fonts.cyri.scale(24))
                        .into(),
                    Row::with_children(vec![
                        neat_button(
                            &mut self.no_button,
                            i18n.get_msg("common-no").into_owned(),
                            FILL_FRAC_ONE,
                            button_style,
                            Some(Message::WorldCancelConfirmation),
                        ),
                        neat_button(
                            &mut self.yes_button,
                            i18n.get_msg("common-yes").into_owned(),
                            FILL_FRAC_ONE,
                            button_style,
                            Some(yes_msg),
                        ),
                    ])
                    .height(Length::Units(28))
                    .spacing(30)
                    .into(),
                ])
                .align_items(Align::Center)
                .spacing(10);

                let over = Container::new(over_content)
                    .style(
                        style::container::Style::color_with_double_cornerless_border(
                            (0, 0, 0, 200).into(),
                            (3, 4, 4, 255).into(),
                            (28, 28, 22, 255).into(),
                        ),
                    )
                    .width(Length::Shrink)
                    .height(Length::Shrink)
                    .max_width(400)
                    .max_height(500)
                    .padding(24)
                    .center_x()
                    .center_y();

                Overlay::new(over, all)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x()
                    .center_y()
                    .into()
            } else {
                self.confirmation = None;
                all.into()
            }
        } else {
            all.into()
        }
    }
}
