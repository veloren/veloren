use crate::{
    i18n::Localization,
    render::Renderer,
    ui::{
        self,
        fonts::IcedFonts as Fonts,
        ice::{
            component::{
                neat_button,
                tooltip::{self, WithTooltip},
            },
            style,
            widget::{
                mouse_detector, AspectRatioContainer, BackgroundContainer, Image, MouseDetector,
                Overlay, Padding, TooltipManager,
            },
            Element, IcedRenderer, IcedUi as Ui,
        },
        img_ids::ImageGraphic,
    },
    window, GlobalState,
};
use client::Client;
use common::{
    assets::AssetHandle,
    character::{CharacterId, CharacterItem, MAX_CHARACTERS_PER_PLAYER},
    comp::{self, humanoid, inventory::slot::EquipSlot, Inventory, Item},
    LoadoutBuilder,
};
//ImageFrame, Tooltip,
use crate::settings::Settings;
//use std::time::Duration;
//use ui::ice::widget;
use iced::{
    button, scrollable, slider, text_input, Align, Button, Column, Container, HorizontalAlignment,
    Length, Row, Scrollable, Slider, Space, Text, TextInput,
};
use vek::Rgba;

pub const TEXT_COLOR: iced::Color = iced::Color::from_rgb(1.0, 1.0, 1.0);
pub const DISABLED_TEXT_COLOR: iced::Color = iced::Color::from_rgba(1.0, 1.0, 1.0, 0.2);
pub const TOOLTIP_BACK_COLOR: Rgba<u8> = Rgba::new(20, 18, 10, 255);
const FILL_FRAC_ONE: f32 = 0.77;
const FILL_FRAC_TWO: f32 = 0.60;
const TOOLTIP_HOVER_DUR: std::time::Duration = std::time::Duration::from_millis(150);
const TOOLTIP_FADE_DUR: std::time::Duration = std::time::Duration::from_millis(350);
const BANNER_ALPHA: u8 = 210;
// Buttons in the bottom corners
const SMALL_BUTTON_HEIGHT: u16 = 31;

const STARTER_HAMMER: &str = "common.items.weapons.hammer.starter_hammer";
const STARTER_BOW: &str = "common.items.weapons.bow.starter_bow";
const STARTER_AXE: &str = "common.items.weapons.axe.starter_axe";
const STARTER_STAFF: &str = "common.items.weapons.staff.starter_staff";
const STARTER_SWORD: &str = "common.items.weapons.sword.starter_sword";
const STARTER_SCEPTRE: &str = "common.items.weapons.sceptre.starter_sceptre";
// TODO: what does this comment mean?
// // Use in future MR to make this a starter weapon

// TODO: use for info popup frame/background
const UI_MAIN: Rgba<u8> = Rgba::new(156, 179, 179, 255); // Greenish Blue

image_ids_ice! {
    struct Imgs {
        <ImageGraphic>
        frame_bottom: "voxygen.element.frames.banner_bot",

        slider_range: "voxygen.element.slider.track",
        slider_indicator: "voxygen.element.slider.indicator",

        selection: "voxygen.element.frames.selection",
        selection_hover: "voxygen.element.frames.selection_hover",
        selection_press: "voxygen.element.frames.selection_press",

        delete_button: "voxygen.element.buttons.x_red",
        delete_button_hover: "voxygen.element.buttons.x_red_hover",
        delete_button_press: "voxygen.element.buttons.x_red_press",

        name_input: "voxygen.element.misc_bg.textbox",

        // Tool Icons
        sceptre: "voxygen.element.icons.sceptre",
        sword: "voxygen.element.icons.sword",
        axe: "voxygen.element.icons.axe",
        hammer: "voxygen.element.icons.hammer",
        bow: "voxygen.element.icons.bow",
        staff: "voxygen.element.icons.staff",

        // Dice icons
        dice: "voxygen.element.icons.dice",
        dice_hover: "voxygen.element.icons.dice_hover",
        dice_press: "voxygen.element.icons.dice_press",

        // Species Icons
        human_m: "voxygen.element.icons.human_m",
        human_f: "voxygen.element.icons.human_f",
        orc_m: "voxygen.element.icons.orc_m",
        orc_f: "voxygen.element.icons.orc_f",
        dwarf_m: "voxygen.element.icons.dwarf_m",
        dwarf_f: "voxygen.element.icons.dwarf_f",
        undead_m: "voxygen.element.icons.ud_m",
        undead_f: "voxygen.element.icons.ud_f",
        elf_m: "voxygen.element.icons.elf_m",
        elf_f: "voxygen.element.icons.elf_f",
        danari_m: "voxygen.element.icons.danari_m",
        danari_f: "voxygen.element.icons.danari_f",
        // Icon Borders
        icon_border: "voxygen.element.buttons.border",
        icon_border_mo: "voxygen.element.buttons.border_mo",
        icon_border_press: "voxygen.element.buttons.border_press",
        icon_border_pressed: "voxygen.element.buttons.border_pressed",

        button: "voxygen.element.buttons.button",
        button_hover: "voxygen.element.buttons.button_hover",
        button_press: "voxygen.element.buttons.button_press",

        // Tooltips
        tt_edge: "voxygen.element.frames.tooltip.edge",
        tt_corner: "voxygen.element.frames.tooltip.corner",
    }
}

pub enum Event {
    Logout,
    Play(CharacterId),
    AddCharacter {
        alias: String,
        tool: String,
        body: comp::Body,
    },
    DeleteCharacter(CharacterId),
    ClearCharacterListError,
    SelectCharacter(Option<CharacterId>),
}

enum Mode {
    Select {
        info_content: Option<InfoContent>,

        characters_scroll: scrollable::State,
        character_buttons: Vec<button::State>,
        new_character_button: button::State,
        logout_button: button::State,
        enter_world_button: button::State,
        yes_button: button::State,
        no_button: button::State,
    },
    Create {
        name: String, // TODO: default to username
        body: humanoid::Body,
        inventory: Box<comp::inventory::Inventory>,
        tool: &'static str,

        body_type_buttons: [button::State; 2],
        species_buttons: [button::State; 6],
        tool_buttons: [button::State; 6],
        sliders: Sliders,
        scroll: scrollable::State,
        name_input: text_input::State,
        back_button: button::State,
        create_button: button::State,
        randomize_button: button::State,
    },
}

impl Mode {
    pub fn select(info_content: Option<InfoContent>) -> Self {
        Self::Select {
            info_content,
            characters_scroll: Default::default(),
            character_buttons: Vec::new(),
            new_character_button: Default::default(),
            logout_button: Default::default(),
            enter_world_button: Default::default(),
            yes_button: Default::default(),
            no_button: Default::default(),
        }
    }

    pub fn create(name: String) -> Self {
        let tool = STARTER_SWORD;

        let loadout = LoadoutBuilder::new()
            .defaults()
            .active_item(Some(Item::new_from_asset_expect(tool)))
            .build();

        let inventory = Box::new(Inventory::new_with_loadout(loadout));

        Self::Create {
            name,
            body: humanoid::Body::random(),
            inventory,
            tool,
            body_type_buttons: Default::default(),
            species_buttons: Default::default(),
            tool_buttons: Default::default(),
            sliders: Default::default(),
            scroll: Default::default(),
            name_input: Default::default(),
            back_button: Default::default(),
            create_button: Default::default(),
            randomize_button: Default::default(),
        }
    }
}

#[derive(PartialEq)]
enum InfoContent {
    Deletion(usize),
    LoadingCharacters,
    CreatingCharacter,
    DeletingCharacter,
    CharacterError(String),
}

struct Controls {
    fonts: Fonts,
    imgs: Imgs,
    // Voxygen version
    version: String,
    // Alpha disclaimer
    alpha: String,

    tooltip_manager: TooltipManager,
    // Zone for rotating the character with the mouse
    mouse_detector: mouse_detector::State,
    mode: Mode,
    // Id of the selected character
    selected: Option<CharacterId>,
}

#[derive(Clone)]
enum Message {
    Back,
    Logout,
    EnterWorld,
    Select(CharacterId),
    Delete(usize),
    NewCharacter,
    CreateCharacter,
    Name(String),
    BodyType(humanoid::BodyType),
    Species(humanoid::Species),
    Tool(&'static str),
    RandomizeCharacter,
    CancelDeletion,
    ConfirmDeletion,
    ClearCharacterListError,
    HairStyle(u8),
    HairColor(u8),
    Skin(u8),
    Eyes(u8),
    EyeColor(u8),
    Accessory(u8),
    Beard(u8),
    // Workaround for widgets that require a message but we don't want them to actually do
    // anything
    DoNothing,
}

impl Controls {
    fn new(fonts: Fonts, imgs: Imgs, selected: Option<CharacterId>) -> Self {
        let version = common::util::DISPLAY_VERSION_LONG.clone();
        let alpha = format!("Veloren {}", common::util::DISPLAY_VERSION.as_str());

        Self {
            fonts,
            imgs,
            version,
            alpha,

            tooltip_manager: TooltipManager::new(TOOLTIP_HOVER_DUR, TOOLTIP_FADE_DUR),
            mouse_detector: Default::default(),
            mode: Mode::select(Some(InfoContent::LoadingCharacters)),
            selected,
        }
    }

    fn view<'a>(
        &'a mut self,
        _settings: &Settings,
        client: &Client,
        error: &Option<String>,
        i18n: &'a Localization,
    ) -> Element<'a, Message> {
        // TODO: use font scale thing for text size (use on button size for buttons with
        // text)

        // Maintain tooltip manager
        self.tooltip_manager.maintain();

        let imgs = &self.imgs;
        let fonts = &self.fonts;
        let tooltip_manager = &self.tooltip_manager;

        let button_style = style::button::Style::new(imgs.button)
            .hover_image(imgs.button_hover)
            .press_image(imgs.button_press)
            .text_color(TEXT_COLOR)
            .disabled_text_color(DISABLED_TEXT_COLOR);

        let tooltip_style = tooltip::Style {
            container: style::container::Style::color_with_image_border(
                TOOLTIP_BACK_COLOR,
                imgs.tt_corner,
                imgs.tt_edge,
            ),
            text_color: TEXT_COLOR,
            text_size: self.fonts.cyri.scale(17),
            padding: 10,
        };

        let version = iced::Text::new(&self.version)
            .size(self.fonts.cyri.scale(15))
            .width(Length::Fill)
            .horizontal_alignment(HorizontalAlignment::Right);

        let alpha = iced::Text::new(&self.alpha)
            .size(self.fonts.cyri.scale(12))
            .width(Length::Fill)
            .horizontal_alignment(HorizontalAlignment::Center);

        let top_text = Row::with_children(vec![
            Space::new(Length::Fill, Length::Shrink).into(),
            alpha.into(),
            version.into(),
        ])
        .width(Length::Fill);

        let content = match &mut self.mode {
            Mode::Select {
                ref mut info_content,
                ref mut characters_scroll,
                ref mut character_buttons,
                ref mut new_character_button,
                ref mut logout_button,
                ref mut enter_world_button,
                ref mut yes_button,
                ref mut no_button,
            } => {
                // If no character is selected then select the first one
                // Note: we don't need to persist this because it is the default
                if self.selected.is_none() {
                    self.selected = client
                        .character_list()
                        .characters
                        .get(0)
                        .and_then(|i| i.character.id);
                }
                // Get the index of the selected character
                let selected = self.selected.and_then(|id| {
                    client
                        .character_list()
                        .characters
                        .iter()
                        .position(|i| i.character.id == Some(id))
                });

                if let Some(error) = error {
                    // TODO: use more user friendly errors with suggestions on potential solutions
                    // instead of directly showing error message here
                    *info_content = Some(InfoContent::CharacterError(format!(
                        "{}: {}",
                        i18n.get("common.error"),
                        error
                    )))
                } else if let Some(InfoContent::CharacterError(_)) = info_content {
                    *info_content = None;
                } else if matches!(
                    info_content,
                    Some(InfoContent::LoadingCharacters)
                        | Some(InfoContent::CreatingCharacter)
                        | Some(InfoContent::DeletingCharacter)
                ) && !client.character_list().loading
                {
                    *info_content = None;
                }

                let server = Container::new(
                    Column::with_children(vec![
                        Text::new(&client.server_info().name)
                            .size(fonts.cyri.scale(25))
                            .into(),
                        // TODO: show additional server info here
                        Space::new(Length::Fill, Length::Units(25)).into(),
                    ])
                    .spacing(5)
                    .align_items(Align::Center),
                )
                .style(style::container::Style::color(Rgba::new(0, 0, 0, 217)))
                .padding(12)
                .center_x()
                .width(Length::Fill);

                let characters = {
                    let characters = &client.character_list().characters;
                    let num = characters.len();
                    // Ensure we have enough button states
                    character_buttons.resize_with(num * 2, Default::default);

                    // Character Selection List
                    let mut characters = characters
                        .iter()
                        .zip(character_buttons.chunks_exact_mut(2))
                        .filter_map(|(character, buttons)| {
                            let mut buttons = buttons.iter_mut();
                            // TODO: eliminate option in character id?
                            character.character.id.map(|id| {
                                (
                                    id,
                                    character,
                                    (buttons.next().unwrap(), buttons.next().unwrap()),
                                )
                            })
                        })
                        .enumerate()
                        .map(
                            |(i, (character_id, character, (select_button, delete_button)))| {
                                Overlay::new(
                                    // Delete button
                                    Button::new(
                                        delete_button,
                                        Space::new(Length::Units(16), Length::Units(16)),
                                    )
                                    .style(
                                        style::button::Style::new(imgs.delete_button)
                                            .hover_image(imgs.delete_button_hover)
                                            .press_image(imgs.delete_button_press),
                                    )
                                    .on_press(Message::Delete(i))
                                    .with_tooltip(
                                        tooltip_manager,
                                        move || {
                                            tooltip::text(
                                                i18n.get("char_selection.delete_permanently"),
                                                tooltip_style,
                                            )
                                        },
                                    ),
                                    // Select Button
                                    AspectRatioContainer::new(
                                        Button::new(
                                            select_button,
                                            Column::with_children(vec![
                                                Text::new(&character.character.alias)
                                                    .size(fonts.cyri.scale(26))
                                                    .into(),
                                                Text::new(
                                                    i18n.get("char_selection.uncanny_valley"),
                                                )
                                                .into(),
                                            ]),
                                        )
                                        .padding(10)
                                        .style(
                                            style::button::Style::new(if Some(i) == selected {
                                                imgs.selection_hover
                                            } else {
                                                imgs.selection
                                            })
                                            .hover_image(imgs.selection_hover)
                                            .press_image(imgs.selection_press),
                                        )
                                        .width(Length::Fill)
                                        .height(Length::Fill)
                                        .on_press(Message::Select(character_id)),
                                    )
                                    .ratio_of_image(imgs.selection),
                                )
                                .padding(12)
                                .align_x(Align::End)
                                .into()
                            },
                        )
                        .collect::<Vec<_>>();

                    // Add create new character button
                    let color = if num >= MAX_CHARACTERS_PER_PLAYER {
                        (97, 97, 25)
                    } else {
                        (97, 255, 18)
                    };
                    characters.push(
                        AspectRatioContainer::new({
                            let button = Button::new(
                                new_character_button,
                                Container::new(Text::new(
                                    i18n.get("char_selection.create_new_character"),
                                ))
                                .width(Length::Fill)
                                .height(Length::Fill)
                                .center_x()
                                .center_y(),
                            )
                            .style(
                                style::button::Style::new(imgs.selection)
                                    .hover_image(imgs.selection_hover)
                                    .press_image(imgs.selection_press)
                                    .image_color(Rgba::new(color.0, color.1, color.2, 255))
                                    .text_color(iced::Color::from_rgb8(color.0, color.1, color.2))
                                    .disabled_text_color(iced::Color::from_rgb8(
                                        color.0, color.1, color.2,
                                    )),
                            )
                            .width(Length::Fill)
                            .height(Length::Fill);
                            if num < MAX_CHARACTERS_PER_PLAYER {
                                button.on_press(Message::NewCharacter)
                            } else {
                                button
                            }
                        })
                        .ratio_of_image(imgs.selection)
                        .into(),
                    );
                    characters
                };

                // TODO: could replace column with scrollable completely if it had a with
                // children method
                let characters = Column::with_children(vec![
                    Container::new(
                        Scrollable::new(characters_scroll)
                            .push(Column::with_children(characters).spacing(4))
                            .padding(6)
                            .scrollbar_width(5)
                            .scroller_width(5)
                            .width(Length::Fill)
                            .style(style::scrollable::Style {
                                track: None,
                                scroller: style::scrollable::Scroller::Color(UI_MAIN),
                            }),
                    )
                    .style(style::container::Style::color(Rgba::from_translucent(
                        0,
                        BANNER_ALPHA,
                    )))
                    .width(Length::Units(322))
                    .height(Length::Fill)
                    .center_x()
                    .into(),
                    Image::new(imgs.frame_bottom)
                        .height(Length::Units(40))
                        .width(Length::Units(322))
                        .color(Rgba::from_translucent(0, BANNER_ALPHA))
                        .into(),
                ])
                .height(Length::Fill);

                let right_column = Column::with_children(vec![server.into(), characters.into()])
                    .spacing(10)
                    .width(Length::Units(322)) // TODO: see if we can get iced to work with settings below
                    //.max_width(360)
                    //.width(Length::Fill)
                    .height(Length::Fill);

                let top = Row::with_children(vec![
                    right_column.into(),
                    MouseDetector::new(&mut self.mouse_detector, Length::Fill, Length::Fill).into(),
                ])
                .padding(15)
                .width(Length::Fill)
                .height(Length::Fill);

                let logout = neat_button(
                    logout_button,
                    i18n.get("char_selection.logout"),
                    FILL_FRAC_ONE,
                    button_style,
                    Some(Message::Logout),
                );

                let enter_world = neat_button(
                    enter_world_button,
                    i18n.get("char_selection.enter_world"),
                    FILL_FRAC_TWO,
                    button_style,
                    selected.map(|_| Message::EnterWorld),
                );

                let bottom = Row::with_children(vec![
                    Container::new(logout)
                        .width(Length::Fill)
                        .height(Length::Units(SMALL_BUTTON_HEIGHT))
                        .into(),
                    Container::new(enter_world)
                        .width(Length::Fill)
                        .height(Length::Units(52))
                        .center_x()
                        .into(),
                    Space::new(Length::Fill, Length::Shrink).into(),
                ])
                .align_items(Align::End);

                let content = Column::with_children(vec![top.into(), bottom.into()])
                    .width(Length::Fill)
                    .padding(5)
                    .height(Length::Fill);

                // Overlay delete prompt
                if let Some(info_content) = info_content {
                    let over_content: Element<_> = match &info_content {
                        InfoContent::Deletion(_) => Column::with_children(vec![
                            Text::new(i18n.get("char_selection.delete_permanently"))
                                .size(fonts.cyri.scale(24))
                                .into(),
                            Row::with_children(vec![
                                neat_button(
                                    no_button,
                                    i18n.get("common.no"),
                                    FILL_FRAC_ONE,
                                    button_style,
                                    Some(Message::CancelDeletion),
                                ),
                                neat_button(
                                    yes_button,
                                    i18n.get("common.yes"),
                                    FILL_FRAC_ONE,
                                    button_style,
                                    Some(Message::ConfirmDeletion),
                                ),
                            ])
                            .height(Length::Units(28))
                            .spacing(30)
                            .into(),
                        ])
                        .align_items(Align::Center)
                        .spacing(10)
                        .into(),
                        InfoContent::LoadingCharacters => {
                            Text::new(i18n.get("char_selection.loading_characters"))
                                .size(fonts.cyri.scale(24))
                                .into()
                        },
                        InfoContent::CreatingCharacter => {
                            Text::new(i18n.get("char_selection.creating_character"))
                                .size(fonts.cyri.scale(24))
                                .into()
                        },
                        InfoContent::DeletingCharacter => {
                            Text::new(i18n.get("char_selection.deleting_character"))
                                .size(fonts.cyri.scale(24))
                                .into()
                        },
                        InfoContent::CharacterError(error) => Column::with_children(vec![
                            Text::new(error).size(fonts.cyri.scale(24)).into(),
                            Row::with_children(vec![neat_button(
                                no_button,
                                i18n.get("common.close"),
                                FILL_FRAC_ONE,
                                button_style,
                                Some(Message::ClearCharacterListError),
                            )])
                            .height(Length::Units(28))
                            .into(),
                        ])
                        .align_items(Align::Center)
                        .spacing(10)
                        .into(),
                    };

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

                    Overlay::new(over, content)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .center_x()
                        .center_y()
                        .into()
                } else {
                    content.into()
                }
            },
            Mode::Create {
                name,
                body,
                inventory: _,
                tool,
                ref mut scroll,
                ref mut body_type_buttons,
                ref mut species_buttons,
                ref mut tool_buttons,
                ref mut sliders,
                ref mut name_input,
                ref mut back_button,
                ref mut create_button,
                ref mut randomize_button,
            } => {
                let unselected_style = style::button::Style::new(imgs.icon_border)
                    .hover_image(imgs.icon_border_mo)
                    .press_image(imgs.icon_border_press);

                let selected_style = style::button::Style::new(imgs.icon_border_pressed)
                    .hover_image(imgs.icon_border_mo)
                    .press_image(imgs.icon_border_press);

                let icon_button = |button, selected, msg, img| {
                    Container::new(
                        Button::<_, IcedRenderer>::new(
                            button,
                            Space::new(Length::Units(60), Length::Units(60)),
                        )
                        .style(if selected {
                            selected_style
                        } else {
                            unselected_style
                        })
                        .on_press(msg),
                    )
                    .style(style::container::Style::image(img))
                };
                let icon_button_tooltip = |button, selected, msg, img, tooltip_i18n_key| {
                    icon_button(button, selected, msg, img)
                        .with_tooltip(tooltip_manager, move || {
                            tooltip::text(i18n.get(tooltip_i18n_key), tooltip_style)
                        })
                };

                let (body_m_ico, body_f_ico) = match body.species {
                    humanoid::Species::Human => (imgs.human_m, imgs.human_f),
                    humanoid::Species::Orc => (imgs.orc_m, imgs.orc_f),
                    humanoid::Species::Dwarf => (imgs.dwarf_m, imgs.dwarf_f),
                    humanoid::Species::Elf => (imgs.elf_m, imgs.elf_f),
                    humanoid::Species::Undead => (imgs.undead_m, imgs.undead_f),
                    humanoid::Species::Danari => (imgs.danari_m, imgs.danari_f),
                };

                let [ref mut body_m_button, ref mut body_f_button] = body_type_buttons;
                let body_type = Row::with_children(vec![
                    icon_button(
                        body_m_button,
                        matches!(body.body_type, humanoid::BodyType::Male),
                        Message::BodyType(humanoid::BodyType::Male),
                        body_m_ico,
                    )
                    .into(),
                    icon_button(
                        body_f_button,
                        matches!(body.body_type, humanoid::BodyType::Female),
                        Message::BodyType(humanoid::BodyType::Female),
                        body_f_ico,
                    )
                    .into(),
                ])
                .spacing(1);

                let (human_icon, orc_icon, dwarf_icon, elf_icon, undead_icon, danari_icon) =
                    match body.body_type {
                        humanoid::BodyType::Male => (
                            self.imgs.human_m,
                            self.imgs.orc_m,
                            self.imgs.dwarf_m,
                            self.imgs.elf_m,
                            self.imgs.undead_m,
                            self.imgs.danari_m,
                        ),
                        humanoid::BodyType::Female => (
                            self.imgs.human_f,
                            self.imgs.orc_f,
                            self.imgs.dwarf_f,
                            self.imgs.elf_f,
                            self.imgs.undead_f,
                            self.imgs.danari_f,
                        ),
                    };

                // TODO: tooltips
                let [ref mut human_button, ref mut orc_button, ref mut dwarf_button, ref mut elf_button, ref mut undead_button, ref mut danari_button] =
                    species_buttons;
                let species = Column::with_children(vec![
                    Row::with_children(vec![
                        icon_button_tooltip(
                            human_button,
                            matches!(body.species, humanoid::Species::Human),
                            Message::Species(humanoid::Species::Human),
                            human_icon,
                            "common.species.human",
                        )
                        .into(),
                        icon_button_tooltip(
                            orc_button,
                            matches!(body.species, humanoid::Species::Orc),
                            Message::Species(humanoid::Species::Orc),
                            orc_icon,
                            "common.species.orc",
                        )
                        .into(),
                        icon_button_tooltip(
                            dwarf_button,
                            matches!(body.species, humanoid::Species::Dwarf),
                            Message::Species(humanoid::Species::Dwarf),
                            dwarf_icon,
                            "common.species.dwarf",
                        )
                        .into(),
                    ])
                    .spacing(1)
                    .into(),
                    Row::with_children(vec![
                        icon_button_tooltip(
                            elf_button,
                            matches!(body.species, humanoid::Species::Elf),
                            Message::Species(humanoid::Species::Elf),
                            elf_icon,
                            "common.species.elf",
                        )
                        .into(),
                        icon_button_tooltip(
                            undead_button,
                            matches!(body.species, humanoid::Species::Undead),
                            Message::Species(humanoid::Species::Undead),
                            undead_icon,
                            "common.species.undead",
                        )
                        .into(),
                        icon_button_tooltip(
                            danari_button,
                            matches!(body.species, humanoid::Species::Danari),
                            Message::Species(humanoid::Species::Danari),
                            danari_icon,
                            "common.species.danari",
                        )
                        .into(),
                    ])
                    .spacing(1)
                    .into(),
                ])
                .spacing(1);

                let [ref mut sword_button, ref mut sceptre_button, ref mut axe_button, ref mut hammer_button, ref mut bow_button, ref mut staff_button] =
                    tool_buttons;
                let tool = Column::with_children(vec![
                    Row::with_children(vec![
                        icon_button_tooltip(
                            sword_button,
                            *tool == STARTER_SWORD,
                            Message::Tool(STARTER_SWORD),
                            imgs.sword,
                            "common.weapons.sword",
                        )
                        .into(),
                        icon_button_tooltip(
                            hammer_button,
                            *tool == STARTER_HAMMER,
                            Message::Tool(STARTER_HAMMER),
                            imgs.hammer,
                            "common.weapons.hammer",
                        )
                        .into(),
                        icon_button_tooltip(
                            axe_button,
                            *tool == STARTER_AXE,
                            Message::Tool(STARTER_AXE),
                            imgs.axe,
                            "common.weapons.axe",
                        )
                        .into(),
                    ])
                    .spacing(1)
                    .into(),
                    Row::with_children(vec![
                        icon_button_tooltip(
                            sceptre_button,
                            *tool == STARTER_SCEPTRE,
                            Message::Tool(STARTER_SCEPTRE),
                            imgs.sceptre,
                            "common.weapons.sceptre",
                        )
                        .into(),
                        icon_button_tooltip(
                            bow_button,
                            *tool == STARTER_BOW,
                            Message::Tool(STARTER_BOW),
                            imgs.bow,
                            "common.weapons.bow",
                        )
                        .into(),
                        icon_button_tooltip(
                            staff_button,
                            *tool == STARTER_STAFF,
                            Message::Tool(STARTER_STAFF),
                            imgs.staff,
                            "common.weapons.staff",
                        )
                        .into(),
                    ])
                    .spacing(1)
                    .into(),
                ])
                .spacing(1);

                const SLIDER_TEXT_SIZE: u16 = 20;
                const SLIDER_CURSOR_SIZE: (u16, u16) = (9, 21);
                const SLIDER_BAR_HEIGHT: u16 = 9;
                const SLIDER_BAR_PAD: u16 = 5;
                // Height of interactable area
                const SLIDER_HEIGHT: u16 = 30;

                fn char_slider<'a>(
                    text: &str,
                    state: &'a mut slider::State,
                    max: u8,
                    selected_val: u8,
                    on_change: impl 'static + Fn(u8) -> Message,
                    (fonts, imgs): (&Fonts, &Imgs),
                ) -> Element<'a, Message> {
                    Column::with_children(vec![
                        Text::new(text)
                            .size(fonts.cyri.scale(SLIDER_TEXT_SIZE))
                            .into(),
                        Slider::new(state, 0..=max, selected_val, on_change)
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
                    .align_items(Align::Center)
                    .into()
                }
                fn char_slider_greyable<'a>(
                    active: bool,
                    text: &str,
                    state: &'a mut slider::State,
                    max: u8,
                    selected_val: u8,
                    on_change: impl 'static + Fn(u8) -> Message,
                    (fonts, imgs): (&Fonts, &Imgs),
                ) -> Element<'a, Message> {
                    if active {
                        char_slider(text, state, max, selected_val, on_change, (fonts, imgs))
                    } else {
                        Column::with_children(vec![
                            Text::new(text)
                                .size(fonts.cyri.scale(SLIDER_TEXT_SIZE))
                                .color(DISABLED_TEXT_COLOR)
                                .into(),
                            // "Disabled" slider
                            // TODO: add iced support for disabled sliders (like buttons)
                            Slider::new(state, 0..=max, selected_val, |_| Message::DoNothing)
                                .height(SLIDER_HEIGHT)
                                .style(style::slider::Style {
                                    cursor: style::slider::Cursor::Color(Rgba::zero()),
                                    bar: style::slider::Bar::Image(
                                        imgs.slider_range,
                                        Rgba::from_translucent(255, 51),
                                        SLIDER_BAR_PAD,
                                    ),
                                    labels: false,
                                    ..Default::default()
                                })
                                .into(),
                        ])
                        .align_items(Align::Center)
                        .into()
                    }
                }

                let slider_options = Column::with_children(vec![
                    char_slider(
                        i18n.get("char_selection.hair_style"),
                        &mut sliders.hair_style,
                        body.species.num_hair_styles(body.body_type) - 1,
                        body.hair_style,
                        Message::HairStyle,
                        (fonts, imgs),
                    ),
                    char_slider(
                        i18n.get("char_selection.hair_color"),
                        &mut sliders.hair_color,
                        body.species.num_hair_colors() - 1,
                        body.hair_color,
                        Message::HairColor,
                        (fonts, imgs),
                    ),
                    char_slider(
                        i18n.get("char_selection.skin"),
                        &mut sliders.skin,
                        body.species.num_skin_colors() - 1,
                        body.skin,
                        Message::Skin,
                        (fonts, imgs),
                    ),
                    char_slider(
                        i18n.get("char_selection.eyeshape"),
                        &mut sliders.eyes,
                        body.species.num_eyes(body.body_type) - 1,
                        body.eyes,
                        Message::Eyes,
                        (fonts, imgs),
                    ),
                    char_slider(
                        i18n.get("char_selection.eye_color"),
                        &mut sliders.eye_color,
                        body.species.num_eye_colors() - 1,
                        body.eye_color,
                        Message::EyeColor,
                        (fonts, imgs),
                    ),
                    char_slider_greyable(
                        body.species.num_accessories(body.body_type) > 1,
                        i18n.get("char_selection.accessories"),
                        &mut sliders.accessory,
                        body.species.num_accessories(body.body_type) - 1,
                        body.accessory,
                        Message::Accessory,
                        (fonts, imgs),
                    ),
                    char_slider_greyable(
                        body.species.num_beards(body.body_type) > 1,
                        i18n.get("char_selection.beard"),
                        &mut sliders.beard,
                        body.species.num_beards(body.body_type) - 1,
                        body.beard,
                        Message::Beard,
                        (fonts, imgs),
                    ),
                ])
                .max_width(200)
                .padding(5);

                let column_content = vec![
                    body_type.into(),
                    species.into(),
                    tool.into(),
                    slider_options.into(),
                ];

                let right_column = Container::new(
                    Scrollable::new(scroll)
                        .push(
                            Column::with_children(column_content)
                                .align_items(Align::Center)
                                .width(Length::Fill)
                                .spacing(5),
                        )
                        .padding(5)
                        .width(Length::Fill)
                        .align_items(Align::Center)
                        .style(style::scrollable::Style {
                            track: None,
                            scroller: style::scrollable::Scroller::Color(UI_MAIN),
                        }),
                )
                .width(Length::Units(320)) // TODO: see if we can get iced to work with settings below
                //.max_width(360)
                //.width(Length::Fill)
                .height(Length::Fill);

                let right_column = Column::with_children(vec![
                    Container::new(right_column)
                        .style(style::container::Style::color(Rgba::from_translucent(
                            0,
                            BANNER_ALPHA,
                        )))
                        .width(Length::Units(320))
                        .center_x()
                        .into(),
                    Image::new(imgs.frame_bottom)
                        .height(Length::Units(40))
                        .width(Length::Units(320))
                        .color(Rgba::from_translucent(0, BANNER_ALPHA))
                        .into(),
                ])
                .height(Length::Fill);

                let top = Row::with_children(vec![
                    right_column.into(),
                    MouseDetector::new(&mut self.mouse_detector, Length::Fill, Length::Fill).into(),
                ])
                .padding(10)
                .width(Length::Fill)
                .height(Length::Fill);

                let back = neat_button(
                    back_button,
                    i18n.get("common.back"),
                    FILL_FRAC_ONE,
                    button_style,
                    Some(Message::Back),
                );

                const DICE_SIZE: u16 = 35;
                let randomize = Button::new(
                    randomize_button,
                    Space::new(Length::Units(DICE_SIZE), Length::Units(DICE_SIZE)),
                )
                .style(
                    style::button::Style::new(imgs.dice)
                        .hover_image(imgs.dice_hover)
                        .press_image(imgs.dice_press),
                )
                .on_press(Message::RandomizeCharacter)
                .with_tooltip(tooltip_manager, move || {
                    tooltip::text(i18n.get("common.rand_appearance"), tooltip_style)
                });

                let name_input = BackgroundContainer::new(
                    Image::new(imgs.name_input)
                        .height(Length::Units(40))
                        .fix_aspect_ratio(),
                    TextInput::new(name_input, "Character Name", &name, Message::Name)
                        .size(25)
                        .on_submit(Message::CreateCharacter),
                )
                .padding(Padding::new().horizontal(7).top(5));

                let bottom_center = Container::new(
                    Row::with_children(vec![
                        randomize.into(),
                        name_input.into(),
                        Space::new(Length::Units(DICE_SIZE), Length::Units(DICE_SIZE)).into(),
                    ])
                    .align_items(Align::Center)
                    .spacing(5)
                    .padding(16),
                )
                .style(style::container::Style::color(Rgba::new(0, 0, 0, 100)));

                let create = neat_button(
                    create_button,
                    i18n.get("common.create"),
                    FILL_FRAC_ONE,
                    button_style,
                    (!name.is_empty()).then_some(Message::CreateCharacter),
                );

                let create: Element<Message> = if name.is_empty() {
                    create
                        .with_tooltip(tooltip_manager, move || {
                            tooltip::text(
                                i18n.get("char_selection.create_info_name"),
                                tooltip_style,
                            )
                        })
                        .into()
                } else {
                    create
                };

                let bottom = Row::with_children(vec![
                    Container::new(back)
                        .width(Length::Fill)
                        .height(Length::Units(SMALL_BUTTON_HEIGHT))
                        .into(),
                    Container::new(bottom_center)
                        .width(Length::Fill)
                        .center_x()
                        .into(),
                    Container::new(create)
                        .width(Length::Fill)
                        .height(Length::Units(SMALL_BUTTON_HEIGHT))
                        .align_x(Align::End)
                        .into(),
                ])
                .align_items(Align::End);

                Column::with_children(vec![top.into(), bottom.into()])
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(5)
                    .into()
            },
        };

        Container::new(
            Column::with_children(vec![top_text.into(), content])
                .spacing(3)
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .padding(3)
        .into()
    }

    fn update(&mut self, message: Message, events: &mut Vec<Event>, characters: &[CharacterItem]) {
        match message {
            Message::Back => {
                if matches!(&self.mode, Mode::Create { .. }) {
                    self.mode = Mode::select(None);
                }
            },
            Message::Logout => {
                events.push(Event::Logout);
            },
            Message::EnterWorld => {
                if let (Mode::Select { .. }, Some(selected)) = (&self.mode, self.selected) {
                    events.push(Event::Play(selected));
                }
            },
            Message::Select(id) => {
                if let Mode::Select { .. } = &mut self.mode {
                    self.selected = Some(id);
                    events.push(Event::SelectCharacter(Some(id)))
                }
            },
            Message::Delete(idx) => {
                if let Mode::Select { info_content, .. } = &mut self.mode {
                    *info_content = Some(InfoContent::Deletion(idx));
                }
            },
            Message::NewCharacter => {
                if matches!(&self.mode, Mode::Select { .. }) {
                    self.mode = Mode::create(String::new());
                }
            },
            Message::CreateCharacter => {
                if let Mode::Create {
                    name, body, tool, ..
                } = &self.mode
                {
                    events.push(Event::AddCharacter {
                        alias: name.clone(),
                        tool: String::from(*tool),
                        body: comp::Body::Humanoid(*body),
                    });
                    self.mode = Mode::select(Some(InfoContent::CreatingCharacter));
                }
            },
            Message::Name(value) => {
                if let Mode::Create { name, .. } = &mut self.mode {
                    *name = value;
                }
            },
            Message::BodyType(value) => {
                if let Mode::Create { body, .. } = &mut self.mode {
                    body.body_type = value;
                    body.validate();
                }
            },
            Message::Species(value) => {
                if let Mode::Create { body, .. } = &mut self.mode {
                    body.species = value;
                    body.validate();
                }
            },
            Message::Tool(value) => {
                if let Mode::Create {
                    tool, inventory, ..
                } = &mut self.mode
                {
                    *tool = value;
                    inventory.replace_loadout_item(
                        EquipSlot::Mainhand,
                        Some(Item::new_from_asset_expect(*tool)),
                    );
                }
            },
            Message::RandomizeCharacter => {
                if let Mode::Create { name, body, .. } = &mut self.mode {
                    use common::npc;
                    use rand::Rng;
                    let body_type = body.body_type;
                    let species = body.species;
                    let mut rng = rand::thread_rng();
                    body.hair_style = rng.gen_range(0, species.num_hair_styles(body_type));
                    body.beard = rng.gen_range(0, species.num_beards(body_type));
                    body.accessory = rng.gen_range(0, species.num_accessories(body_type));
                    body.hair_color = rng.gen_range(0, species.num_hair_colors());
                    body.skin = rng.gen_range(0, species.num_skin_colors());
                    body.eye_color = rng.gen_range(0, species.num_eye_colors());
                    body.eyes = rng.gen_range(0, species.num_eyes(body_type));
                    *name = npc::get_npc_name(
                        npc::NpcKind::Humanoid,
                        npc::BodyType::from_body(comp::Body::Humanoid(*body)),
                    );
                }
            },
            Message::ConfirmDeletion => {
                if let Mode::Select { info_content, .. } = &mut self.mode {
                    if let Some(InfoContent::Deletion(idx)) = info_content {
                        if let Some(id) = characters.get(*idx).and_then(|i| i.character.id) {
                            events.push(Event::DeleteCharacter(id));
                            // Deselect if the selected character was deleted
                            if Some(id) == self.selected {
                                self.selected = None;
                                events.push(Event::SelectCharacter(None));
                            }
                        }
                        *info_content = Some(InfoContent::DeletingCharacter);
                    }
                }
            },
            Message::CancelDeletion => {
                if let Mode::Select { info_content, .. } = &mut self.mode {
                    if let Some(InfoContent::Deletion(_)) = info_content {
                        *info_content = None;
                    }
                }
            },
            Message::ClearCharacterListError => {
                events.push(Event::ClearCharacterListError);
            },
            Message::HairStyle(value) => {
                if let Mode::Create { body, .. } = &mut self.mode {
                    body.hair_style = value;
                    body.validate();
                }
            },
            Message::HairColor(value) => {
                if let Mode::Create { body, .. } = &mut self.mode {
                    body.hair_color = value;
                    body.validate();
                }
            },
            Message::Skin(value) => {
                if let Mode::Create { body, .. } = &mut self.mode {
                    body.skin = value;
                    body.validate();
                }
            },
            Message::Eyes(value) => {
                if let Mode::Create { body, .. } = &mut self.mode {
                    body.eyes = value;
                    body.validate();
                }
            },
            Message::EyeColor(value) => {
                if let Mode::Create { body, .. } = &mut self.mode {
                    body.eye_color = value;
                    body.validate();
                }
            },
            Message::Accessory(value) => {
                if let Mode::Create { body, .. } = &mut self.mode {
                    body.accessory = value;
                    body.validate();
                }
            },
            Message::Beard(value) => {
                if let Mode::Create { body, .. } = &mut self.mode {
                    body.beard = value;
                    body.validate();
                }
            },
            Message::DoNothing => {},
        }
    }

    /// Get the character to display
    pub fn display_body_inventory<'a>(
        &'a self,
        characters: &'a [CharacterItem],
    ) -> Option<(comp::Body, &'a comp::inventory::Inventory)> {
        match &self.mode {
            Mode::Select { .. } => self
                .selected
                .and_then(|id| characters.iter().find(|i| i.character.id == Some(id)))
                .map(|i| (i.body, &i.inventory)),
            Mode::Create {
                inventory, body, ..
            } => Some((comp::Body::Humanoid(*body), inventory)),
        }
    }
}

pub struct CharSelectionUi {
    ui: Ui,
    controls: Controls,
    enter_pressed: bool,
    select_character: Option<CharacterId>,
    pub error: Option<String>,
}

impl CharSelectionUi {
    pub fn new(global_state: &mut GlobalState, client: &Client) -> Self {
        // Load up the last selected character for this server
        let server_name = &client.server_info().name;
        let selected_character = global_state.profile.get_selected_character(server_name);

        // Load language
        let i18n = global_state.i18n.read();

        // TODO: don't add default font twice
        let font = ui::ice::load_font(&i18n.fonts.get("cyri").unwrap().asset_key);

        let mut ui = Ui::new(
            &mut global_state.window,
            font,
            global_state.settings.gameplay.ui_scale,
        )
        .unwrap();

        let fonts = Fonts::load(&i18n.fonts, &mut ui).expect("Impossible to load fonts");

        let controls = Controls::new(
            fonts,
            Imgs::load(&mut ui).expect("Failed to load images"),
            selected_character,
        );

        Self {
            ui,
            controls,
            enter_pressed: false,
            select_character: None,
            error: None,
        }
    }

    pub fn display_body_inventory<'a>(
        &'a self,
        characters: &'a [CharacterItem],
    ) -> Option<(comp::Body, &'a comp::inventory::Inventory)> {
        self.controls.display_body_inventory(characters)
    }

    pub fn handle_event(&mut self, event: window::Event) -> bool {
        match event {
            window::Event::IcedUi(event) => {
                // Enter Key pressed
                use iced::keyboard;
                if let iced::Event::Keyboard(keyboard::Event::KeyPressed {
                    key_code: keyboard::KeyCode::Enter,
                    ..
                }) = event
                {
                    self.enter_pressed = true;
                }

                self.ui.handle_event(event);
                true
            },
            window::Event::MouseButton(_, window::PressState::Pressed) => {
                !self.controls.mouse_detector.mouse_over()
            },
            window::Event::ScaleFactorChanged(s) => {
                self.ui.scale_factor_changed(s);
                false
            },
            _ => false,
        }
    }

    pub fn update_language(&mut self, i18n: AssetHandle<Localization>) {
        let i18n = i18n.read();
        let font = ui::ice::load_font(&i18n.fonts.get("cyri").unwrap().asset_key);

        self.ui.clear_fonts(font);
        self.controls.fonts =
            Fonts::load(&i18n.fonts, &mut self.ui).expect("Impossible to load fonts!");
    }

    pub fn set_scale_mode(&mut self, scale_mode: ui::ScaleMode) {
        self.ui.set_scaling_mode(scale_mode);
    }

    pub fn select_character(&mut self, id: CharacterId) { self.select_character = Some(id); }

    pub fn display_error(&mut self, error: String) { self.error = Some(error); }

    // TODO: do we need whole client here or just character list?
    pub fn maintain(&mut self, global_state: &mut GlobalState, client: &Client) -> Vec<Event> {
        let mut events = Vec::new();
        let i18n = global_state.i18n.read();

        let (mut messages, _) = self.ui.maintain(
            self.controls
                .view(&global_state.settings, &client, &self.error, &i18n),
            global_state.window.renderer_mut(),
            global_state.clipboard.as_ref(),
        );

        if self.enter_pressed {
            self.enter_pressed = false;
            messages.push(Message::EnterWorld);
        }

        if let Some(id) = self.select_character.take() {
            messages.push(Message::Select(id))
        }

        messages.into_iter().for_each(|message| {
            self.controls
                .update(message, &mut events, &client.character_list().characters)
        });

        events
    }

    // TODO: do we need globals?
    pub fn render(&self, renderer: &mut Renderer) { self.ui.render(renderer); }
}

#[derive(Default)]
struct Sliders {
    hair_style: slider::State,
    hair_color: slider::State,
    skin: slider::State,
    eyes: slider::State,
    eye_color: slider::State,
    accessory: slider::State,
    beard: slider::State,
}
