use crate::{
    render::UiDrawer,
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
use client::{Client, ServerInfo};
use common::{
    character::{CharacterId, CharacterItem, MAX_CHARACTERS_PER_PLAYER, MAX_NAME_LENGTH},
    comp::{self, humanoid, inventory::slot::EquipSlot, Inventory, Item},
    LoadoutBuilder,
};
use i18n::{Localization, LocalizationHandle};
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
const FILL_FRAC_TWO: f32 = 0.53;
const TOOLTIP_HOVER_DUR: std::time::Duration = std::time::Duration::from_millis(150);
const TOOLTIP_FADE_DUR: std::time::Duration = std::time::Duration::from_millis(350);
const BANNER_ALPHA: u8 = 210;
// Buttons in the bottom corners
const SMALL_BUTTON_HEIGHT: u16 = 31;

const STARTER_HAMMER: &str = "common.items.weapons.hammer.starter_hammer";
const STARTER_BOW: &str = "common.items.weapons.bow.starter";
const STARTER_AXE: &str = "common.items.weapons.axe.starter_axe";
const STARTER_STAFF: &str = "common.items.weapons.staff.starter_staff";
const STARTER_SWORD: &str = "common.items.weapons.sword.starter";
const STARTER_SWORDS: &str = "common.items.weapons.sword_1h.starter";

// TODO: what does this comment mean?
// // Use in future MR to make this a starter weapon

// TODO: use for info popup frame/background
const UI_MAIN: Rgba<u8> = Rgba::new(156, 179, 179, 255); // Greenish Blue

image_ids_ice! {
    struct Imgs {
        <ImageGraphic>
        frame_bottom: "voxygen.element.ui.generic.frames.banner_bot",

        slider_range: "voxygen.element.ui.generic.slider.track",
        slider_indicator: "voxygen.element.ui.generic.slider.indicator",

        char_selection: "voxygen.element.ui.generic.frames.selection",
        char_selection_hover: "voxygen.element.ui.generic.frames.selection_hover",
        char_selection_press: "voxygen.element.ui.generic.frames.selection_press",

        delete_button: "voxygen.element.ui.char_select.icons.bin",
        delete_button_hover: "voxygen.element.ui.char_select.icons.bin_hover",
        delete_button_press: "voxygen.element.ui.char_select.icons.bin_press",

        edit_button: "voxygen.element.ui.char_select.icons.pen",
        edit_button_hover: "voxygen.element.ui.char_select.icons.pen_hover",
        edit_button_press: "voxygen.element.ui.char_select.icons.pen_press",

        name_input: "voxygen.element.ui.generic.textbox",

        // Tool Icons
        swords: "voxygen.element.weapons.swords",
        sword: "voxygen.element.weapons.sword",
        axe: "voxygen.element.weapons.axe",
        hammer: "voxygen.element.weapons.hammer",
        bow: "voxygen.element.weapons.bow",
        staff: "voxygen.element.weapons.staff",

        // Dice icons
        dice: "voxygen.element.ui.char_select.icons.dice",
        dice_hover: "voxygen.element.ui.char_select.icons.dice_hover",
        dice_press: "voxygen.element.ui.char_select.icons.dice_press",

        // Species Icons
        human_m: "voxygen.element.ui.char_select.portraits.human_m",
        human_f: "voxygen.element.ui.char_select.portraits.human_f",
        orc_m: "voxygen.element.ui.char_select.portraits.orc_m",
        orc_f: "voxygen.element.ui.char_select.portraits.orc_f",
        dwarf_m: "voxygen.element.ui.char_select.portraits.dwarf_m",
        dwarf_f: "voxygen.element.ui.char_select.portraits.dwarf_f",
        draugr_m: "voxygen.element.ui.char_select.portraits.ud_m",
        draugr_f: "voxygen.element.ui.char_select.portraits.ud_f",
        elf_m: "voxygen.element.ui.char_select.portraits.elf_m",
        elf_f: "voxygen.element.ui.char_select.portraits.elf_f",
        danari_m: "voxygen.element.ui.char_select.portraits.danari_m",
        danari_f: "voxygen.element.ui.char_select.portraits.danari_f",
        // Icon Borders
        icon_border: "voxygen.element.ui.generic.buttons.border",
        icon_border_mo: "voxygen.element.ui.generic.buttons.border_mo",
        icon_border_press: "voxygen.element.ui.generic.buttons.border_press",
        icon_border_pressed: "voxygen.element.ui.generic.buttons.border_pressed",

        button: "voxygen.element.ui.generic.buttons.button",
        button_hover: "voxygen.element.ui.generic.buttons.button_hover",
        button_press: "voxygen.element.ui.generic.buttons.button_press",

        // Tooltips
        tt_edge: "voxygen.element.ui.generic.frames.tooltip.edge",
        tt_corner: "voxygen.element.ui.generic.frames.tooltip.corner",
    }
}

pub enum Event {
    Logout,
    Play(CharacterId),
    Spectate,
    AddCharacter {
        alias: String,
        mainhand: Option<String>,
        offhand: Option<String>,
        body: comp::Body,
    },
    EditCharacter {
        alias: String,
        character_id: CharacterId,
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
        spectate_button: button::State,
        yes_button: button::State,
        no_button: button::State,
    },
    CreateOrEdit {
        name: String,
        body: humanoid::Body,
        inventory: Box<Inventory>,
        mainhand: Option<&'static str>,
        offhand: Option<&'static str>,

        body_type_buttons: [button::State; 2],
        species_buttons: [button::State; 6],
        tool_buttons: [button::State; 6],
        sliders: Sliders,
        scroll: scrollable::State,
        name_input: text_input::State,
        back_button: button::State,
        create_button: button::State,
        rand_character_button: button::State,
        rand_name_button: button::State,
        character_id: Option<CharacterId>,
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
            spectate_button: Default::default(),
            yes_button: Default::default(),
            no_button: Default::default(),
        }
    }

    pub fn create(name: String) -> Self {
        // TODO: Load these from the server (presumably from a .ron) to allow for easier
        // modification of custom starting weapons
        let mainhand = Some(STARTER_SWORD);
        let offhand = None;

        let loadout = LoadoutBuilder::empty()
            .defaults()
            .active_mainhand(mainhand.map(Item::new_from_asset_expect))
            .active_offhand(offhand.map(Item::new_from_asset_expect))
            .build();

        let inventory = Box::new(Inventory::with_loadout_humanoid(loadout));

        Self::CreateOrEdit {
            name,
            body: humanoid::Body::random(),
            inventory,
            mainhand,
            offhand,
            body_type_buttons: Default::default(),
            species_buttons: Default::default(),
            tool_buttons: Default::default(),
            sliders: Default::default(),
            scroll: Default::default(),
            name_input: Default::default(),
            back_button: Default::default(),
            create_button: Default::default(),
            rand_character_button: Default::default(),
            rand_name_button: Default::default(),
            character_id: None,
        }
    }

    pub fn edit(
        name: String,
        character_id: CharacterId,
        body: humanoid::Body,
        inventory: &Inventory,
    ) -> Self {
        Self::CreateOrEdit {
            name,
            body,
            inventory: Box::new(inventory.clone()),
            mainhand: None,
            offhand: None,
            body_type_buttons: Default::default(),
            species_buttons: Default::default(),
            tool_buttons: Default::default(),
            sliders: Default::default(),
            scroll: Default::default(),
            name_input: Default::default(),
            back_button: Default::default(),
            create_button: Default::default(),
            rand_character_button: Default::default(),
            rand_name_button: Default::default(),
            character_id: Some(character_id),
        }
    }
}

#[derive(PartialEq)]
enum InfoContent {
    Deletion(usize),
    LoadingCharacters,
    CreatingCharacter,
    EditingCharacter,
    JoiningCharacter,
    CharacterError(String),
}

struct Controls {
    fonts: Fonts,
    imgs: Imgs,
    // Voxygen version
    version: String,
    // Alpha disclaimer
    alpha: String,
    server_mismatched_version: Option<String>,
    tooltip_manager: TooltipManager,
    // Zone for rotating the character with the mouse
    mouse_detector: mouse_detector::State,
    mode: Mode,
    // Id of the selected character
    selected: Option<CharacterId>,
    default_name: String,
}

#[derive(Clone)]
enum Message {
    Back,
    Logout,
    EnterWorld,
    Spectate,
    Select(CharacterId),
    Delete(usize),
    Edit(usize),
    ConfirmEdit(CharacterId),
    NewCharacter,
    CreateCharacter,
    Name(String),
    BodyType(humanoid::BodyType),
    Species(humanoid::Species),
    Tool((Option<&'static str>, Option<&'static str>)),
    RandomizeCharacter,
    RandomizeName,
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
    fn new(
        fonts: Fonts,
        imgs: Imgs,
        selected: Option<CharacterId>,
        default_name: String,
        server_info: &ServerInfo,
    ) -> Self {
        let version = common::util::DISPLAY_VERSION_LONG.clone();
        let alpha = format!("Veloren {}", common::util::DISPLAY_VERSION.as_str());
        let server_mismatched_version = (common::util::GIT_HASH.to_string()
            != server_info.git_hash)
            .then(|| server_info.git_hash.clone());

        Self {
            fonts,
            imgs,
            version,
            alpha,
            server_mismatched_version,
            tooltip_manager: TooltipManager::new(TOOLTIP_HOVER_DUR, TOOLTIP_FADE_DUR),
            mouse_detector: Default::default(),
            mode: Mode::select(Some(InfoContent::LoadingCharacters)),
            selected,
            default_name,
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

        let version = Text::new(&self.version)
            .size(self.fonts.cyri.scale(15))
            .width(Length::Fill)
            .horizontal_alignment(HorizontalAlignment::Right);

        let alpha = Text::new(&self.alpha)
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
                ref mut spectate_button,
                ref mut yes_button,
                ref mut no_button,
            } => {
                match self.selected {
                    Some(character_id) => {
                        // If the selected character no longer exists, deselect it.
                        if !client
                            .character_list()
                            .characters
                            .iter()
                            .any(|char| char.character.id.map_or(false, |id| id == character_id))
                        {
                            self.selected = None;
                        }
                    },
                    None => {
                        // If no character is selected then select the first one
                        // Note: we don't need to persist this because it is the default
                        self.selected = client
                            .character_list()
                            .characters
                            .get(0)
                            .and_then(|i| i.character.id);
                    },
                }

                // Get the index of the selected character
                let selected = self.selected.and_then(|id| {
                    client
                        .character_list()
                        .characters
                        .iter()
                        .position(|i| i.character.id == Some(id))
                });

                // TODO: this appears to be instance of https://github.com/rust-lang/rust-clippy/issues/7579
                #[allow(clippy::if_same_then_else)]
                if let Some(error) = error {
                    // TODO: use more user friendly errors with suggestions on potential solutions
                    // instead of directly showing error message here
                    *info_content = Some(InfoContent::CharacterError(format!(
                        "{}: {}",
                        i18n.get_msg("common-error"),
                        error
                    )))
                } else if let Some(InfoContent::CharacterError(_)) = info_content {
                    *info_content = None;
                } else if matches!(
                    info_content,
                    Some(InfoContent::LoadingCharacters)
                        | Some(InfoContent::CreatingCharacter)
                        | Some(InfoContent::EditingCharacter)
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
                .center_y()
                .width(Length::Fill);

                let characters = {
                    let characters = &client.character_list().characters;
                    let num = characters.len();
                    // Ensure we have enough button states
                    const CHAR_BUTTONS: usize = 3;
                    character_buttons.resize_with(num * CHAR_BUTTONS, Default::default);

                    // Character Selection List
                    let mut characters = characters
                        .iter()
                        .zip(character_buttons.chunks_exact_mut(CHAR_BUTTONS))
                        .filter_map(|(character, buttons)| {
                            let mut buttons = buttons.iter_mut();
                            // TODO: eliminate option in character id?
                            character.character.id.map(|id| {
                                (
                                    id,
                                    character,
                                    (
                                        buttons.next().unwrap(),
                                        buttons.next().unwrap(),
                                        buttons.next().unwrap(),
                                    ),
                                )
                            })
                        })
                        .enumerate()
                        .map(
                            |(
                                i,
                                (
                                    character_id,
                                    character,
                                    (select_button, edit_button, delete_button),
                                ),
                            )| {
                                let select_col = if Some(i) == selected {
                                    (255, 208, 69)
                                } else {
                                    (255, 255, 255)
                                };
                                Overlay::new(
                                    Container::new(
                                        Row::with_children(vec![
                                            // Edit button
                                            Button::new(
                                                edit_button,
                                                Space::new(Length::Units(16), Length::Units(16)),
                                            )
                                            .style(
                                                style::button::Style::new(imgs.edit_button)
                                                    .hover_image(imgs.edit_button_hover)
                                                    .press_image(imgs.edit_button_press),
                                            )
                                            .on_press(Message::Edit(i))
                                            .into(),
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
                                            .into(),
                                        ])
                                        .spacing(5),
                                    )
                                    .padding(4),
                                    // Select Button
                                    AspectRatioContainer::new(
                                        Button::new(
                                            select_button,
                                            Column::with_children(vec![
                                                Text::new(&character.character.alias)
                                                    .size(fonts.cyri.scale(26))
                                                    .into(),
                                                Text::new(
                                                    // TODO: Add actual location here
                                                    i18n.get_msg("char_selection-uncanny_valley"),
                                                )
                                                .into(),
                                            ]),
                                        )
                                        .padding(10)
                                        .style(
                                            style::button::Style::new(if Some(i) == selected {
                                                imgs.char_selection_hover
                                            } else {
                                                imgs.char_selection
                                            })
                                            .hover_image(imgs.char_selection_hover)
                                            .press_image(imgs.char_selection_press)
                                            .image_color(Rgba::new(
                                                select_col.0,
                                                select_col.1,
                                                select_col.2,
                                                255,
                                            )),
                                        )
                                        .width(Length::Fill)
                                        .height(Length::Fill)
                                        .on_press(Message::Select(character_id)),
                                    )
                                    .ratio_of_image(imgs.char_selection),
                                )
                                .padding(0)
                                .align_x(Align::End)
                                .align_y(Align::End)
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
                                    i18n.get_msg("char_selection-create_new_character"),
                                ))
                                .width(Length::Fill)
                                .height(Length::Fill)
                                .center_x()
                                .center_y(),
                            )
                            .style(
                                style::button::Style::new(imgs.char_selection)
                                    .hover_image(imgs.char_selection_hover)
                                    .press_image(imgs.char_selection_press)
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
                        .ratio_of_image(imgs.char_selection)
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

                let left_column = Column::with_children(vec![server.into(), characters.into()])
                    .spacing(10)
                    .width(Length::Units(322)) // TODO: see if we can get iced to work with settings below
                    //.max_width(360)
                    //.width(Length::Fill)
                    .height(Length::Fill);

                let top = Row::with_children(vec![
                    left_column.into(),
                    MouseDetector::new(&mut self.mouse_detector, Length::Fill, Length::Fill).into(),
                ])
                .padding(15)
                .width(Length::Fill)
                .height(Length::Fill);
                let mut bottom_content = vec![
                    Container::new(neat_button(
                        logout_button,
                        i18n.get_msg("char_selection-logout").into_owned(),
                        FILL_FRAC_ONE,
                        button_style,
                        Some(Message::Logout),
                    ))
                    .width(Length::Fill)
                    .height(Length::Units(SMALL_BUTTON_HEIGHT))
                    .into(),
                ];

                if client.is_moderator() {
                    bottom_content.push(
                        Container::new(neat_button(
                            spectate_button,
                            i18n.get_msg("char_selection-spectate").into_owned(),
                            FILL_FRAC_TWO,
                            button_style,
                            Some(Message::Spectate),
                        ))
                        .width(Length::Fill)
                        .height(Length::Units(52))
                        .center_x()
                        .into(),
                    );
                }

                bottom_content.push(
                    Container::new(neat_button(
                        enter_world_button,
                        i18n.get_msg("char_selection-enter_world").into_owned(),
                        FILL_FRAC_TWO,
                        button_style,
                        selected.map(|_| Message::EnterWorld),
                    ))
                    .width(Length::Fill)
                    .height(Length::Units(52))
                    .center_x()
                    .into(),
                );

                bottom_content.push(Space::new(Length::Fill, Length::Shrink).into());

                let bottom = Row::with_children(bottom_content).align_items(Align::End);

                let content = Column::with_children(vec![top.into(), bottom.into()])
                    .width(Length::Fill)
                    .padding(5)
                    .height(Length::Fill);

                // Overlay delete prompt
                if let Some(info_content) = info_content {
                    let over_content: Element<_> = match &info_content {
                        InfoContent::Deletion(_) => Column::with_children(vec![
                            Text::new(i18n.get_msg("char_selection-delete_permanently"))
                                .size(fonts.cyri.scale(24))
                                .into(),
                            Row::with_children(vec![
                                neat_button(
                                    no_button,
                                    i18n.get_msg("common-no").into_owned(),
                                    FILL_FRAC_ONE,
                                    button_style,
                                    Some(Message::CancelDeletion),
                                ),
                                neat_button(
                                    yes_button,
                                    i18n.get_msg("common-yes").into_owned(),
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
                            Text::new(i18n.get_msg("char_selection-loading_characters"))
                                .size(fonts.cyri.scale(24))
                                .into()
                        },
                        InfoContent::CreatingCharacter => {
                            Text::new(i18n.get_msg("char_selection-creating_character"))
                                .size(fonts.cyri.scale(24))
                                .into()
                        },
                        InfoContent::EditingCharacter => {
                            Text::new(i18n.get_msg("char_selection-editing_character"))
                                .size(fonts.cyri.scale(24))
                                .into()
                        },
                        InfoContent::JoiningCharacter => {
                            Text::new(i18n.get_msg("char_selection-joining_character"))
                                .size(fonts.cyri.scale(24))
                                .into()
                        },
                        InfoContent::CharacterError(error) => Column::with_children(vec![
                            Text::new(error).size(fonts.cyri.scale(24)).into(),
                            Row::with_children(vec![neat_button(
                                no_button,
                                i18n.get_msg("common-close").into_owned(),
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
            Mode::CreateOrEdit {
                name,
                body,
                inventory: _,
                mainhand,
                offhand: _,
                ref mut scroll,
                ref mut body_type_buttons,
                ref mut species_buttons,
                ref mut tool_buttons,
                ref mut sliders,
                ref mut name_input,
                ref mut back_button,
                ref mut create_button,
                ref mut rand_character_button,
                ref mut rand_name_button,
                character_id,
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
                    icon_button(button, selected, msg, img).with_tooltip(
                        tooltip_manager,
                        move || {
                            let tooltip_text = i18n.get_msg(tooltip_i18n_key);
                            tooltip::text(&tooltip_text, tooltip_style)
                        },
                    )
                };

                // TODO: tooltips
                let (tool, species, body_type) = if character_id.is_some() {
                    (Column::new(), Column::new(), Row::new())
                } else {
                    let (body_m_ico, body_f_ico) = match body.species {
                        humanoid::Species::Human => (imgs.human_m, imgs.human_f),
                        humanoid::Species::Orc => (imgs.orc_m, imgs.orc_f),
                        humanoid::Species::Dwarf => (imgs.dwarf_m, imgs.dwarf_f),
                        humanoid::Species::Elf => (imgs.elf_m, imgs.elf_f),
                        humanoid::Species::Draugr => (imgs.draugr_m, imgs.draugr_f),
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
                    let (human_icon, orc_icon, dwarf_icon, elf_icon, draugr_icon, danari_icon) =
                        match body.body_type {
                            humanoid::BodyType::Male => (
                                self.imgs.human_m,
                                self.imgs.orc_m,
                                self.imgs.dwarf_m,
                                self.imgs.elf_m,
                                self.imgs.draugr_m,
                                self.imgs.danari_m,
                            ),
                            humanoid::BodyType::Female => (
                                self.imgs.human_f,
                                self.imgs.orc_f,
                                self.imgs.dwarf_f,
                                self.imgs.elf_f,
                                self.imgs.draugr_f,
                                self.imgs.danari_f,
                            ),
                        };
                    let [
                        ref mut human_button,
                        ref mut orc_button,
                        ref mut dwarf_button,
                        ref mut elf_button,
                        ref mut draugr_button,
                        ref mut danari_button,
                    ] = species_buttons;
                    let species = Column::with_children(vec![
                        Row::with_children(vec![
                            icon_button_tooltip(
                                human_button,
                                matches!(body.species, humanoid::Species::Human),
                                Message::Species(humanoid::Species::Human),
                                human_icon,
                                "common-species-human",
                            )
                            .into(),
                            icon_button_tooltip(
                                orc_button,
                                matches!(body.species, humanoid::Species::Orc),
                                Message::Species(humanoid::Species::Orc),
                                orc_icon,
                                "common-species-orc",
                            )
                            .into(),
                            icon_button_tooltip(
                                dwarf_button,
                                matches!(body.species, humanoid::Species::Dwarf),
                                Message::Species(humanoid::Species::Dwarf),
                                dwarf_icon,
                                "common-species-dwarf",
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
                                "common-species-elf",
                            )
                            .into(),
                            icon_button_tooltip(
                                draugr_button,
                                matches!(body.species, humanoid::Species::Draugr),
                                Message::Species(humanoid::Species::Draugr),
                                draugr_icon,
                                "common-species-draugr",
                            )
                            .into(),
                            icon_button_tooltip(
                                danari_button,
                                matches!(body.species, humanoid::Species::Danari),
                                Message::Species(humanoid::Species::Danari),
                                danari_icon,
                                "common-species-danari",
                            )
                            .into(),
                        ])
                        .spacing(1)
                        .into(),
                    ])
                    .spacing(1);
                    let [
                        ref mut sword_button,
                        ref mut swords_button,
                        ref mut axe_button,
                        ref mut hammer_button,
                        ref mut bow_button,
                        ref mut staff_button,
                    ] = tool_buttons;
                    let tool = Column::with_children(vec![
                        Row::with_children(vec![
                            icon_button_tooltip(
                                sword_button,
                                *mainhand == Some(STARTER_SWORD),
                                Message::Tool((Some(STARTER_SWORD), None)),
                                imgs.sword,
                                "common-weapons-greatsword",
                            )
                            .into(),
                            icon_button_tooltip(
                                hammer_button,
                                *mainhand == Some(STARTER_HAMMER),
                                Message::Tool((Some(STARTER_HAMMER), None)),
                                imgs.hammer,
                                "common-weapons-hammer",
                            )
                            .into(),
                            icon_button_tooltip(
                                axe_button,
                                *mainhand == Some(STARTER_AXE),
                                Message::Tool((Some(STARTER_AXE), None)),
                                imgs.axe,
                                "common-weapons-axe",
                            )
                            .into(),
                        ])
                        .spacing(1)
                        .into(),
                        Row::with_children(vec![
                            icon_button_tooltip(
                                swords_button,
                                *mainhand == Some(STARTER_SWORDS),
                                Message::Tool((Some(STARTER_SWORDS), Some(STARTER_SWORDS))),
                                imgs.swords,
                                "common-weapons-shortswords",
                            )
                            .into(),
                            icon_button_tooltip(
                                bow_button,
                                *mainhand == Some(STARTER_BOW),
                                Message::Tool((Some(STARTER_BOW), None)),
                                imgs.bow,
                                "common-weapons-bow",
                            )
                            .into(),
                            icon_button_tooltip(
                                staff_button,
                                *mainhand == Some(STARTER_STAFF),
                                Message::Tool((Some(STARTER_STAFF), None)),
                                imgs.staff,
                                "common-weapons-staff",
                            )
                            .into(),
                        ])
                        .spacing(1)
                        .into(),
                    ])
                    .spacing(1);

                    (tool, species, body_type)
                };

                const SLIDER_TEXT_SIZE: u16 = 20;
                const SLIDER_CURSOR_SIZE: (u16, u16) = (9, 21);
                const SLIDER_BAR_HEIGHT: u16 = 9;
                const SLIDER_BAR_PAD: u16 = 5;
                // Height of interactable area
                const SLIDER_HEIGHT: u16 = 30;

                fn char_slider<'a>(
                    text: String,
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
                    text: String,
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
                        i18n.get_msg("char_selection-hair_style").into_owned(),
                        &mut sliders.hair_style,
                        body.species.num_hair_styles(body.body_type) - 1,
                        body.hair_style,
                        Message::HairStyle,
                        (fonts, imgs),
                    ),
                    char_slider(
                        i18n.get_msg("char_selection-hair_color").into_owned(),
                        &mut sliders.hair_color,
                        body.species.num_hair_colors() - 1,
                        body.hair_color,
                        Message::HairColor,
                        (fonts, imgs),
                    ),
                    char_slider(
                        i18n.get_msg("char_selection-skin").into_owned(),
                        &mut sliders.skin,
                        body.species.num_skin_colors() - 1,
                        body.skin,
                        Message::Skin,
                        (fonts, imgs),
                    ),
                    char_slider(
                        i18n.get_msg("char_selection-eyeshape").into_owned(),
                        &mut sliders.eyes,
                        body.species.num_eyes(body.body_type) - 1,
                        body.eyes,
                        Message::Eyes,
                        (fonts, imgs),
                    ),
                    char_slider(
                        i18n.get_msg("char_selection-eye_color").into_owned(),
                        &mut sliders.eye_color,
                        body.species.num_eye_colors() - 1,
                        body.eye_color,
                        Message::EyeColor,
                        (fonts, imgs),
                    ),
                    char_slider_greyable(
                        body.species.num_accessories(body.body_type) > 1,
                        i18n.get_msg("char_selection-accessories").into_owned(),
                        &mut sliders.accessory,
                        body.species.num_accessories(body.body_type) - 1,
                        body.accessory,
                        Message::Accessory,
                        (fonts, imgs),
                    ),
                    char_slider_greyable(
                        body.species.num_beards(body.body_type) > 1,
                        i18n.get_msg("char_selection-beard").into_owned(),
                        &mut sliders.beard,
                        body.species.num_beards(body.body_type) - 1,
                        body.beard,
                        Message::Beard,
                        (fonts, imgs),
                    ),
                ])
                .max_width(200)
                .padding(5);

                const CHAR_DICE_SIZE: u16 = 50;
                let rand_character = Button::new(
                    rand_character_button,
                    Space::new(Length::Units(CHAR_DICE_SIZE), Length::Units(CHAR_DICE_SIZE)),
                )
                .style(
                    style::button::Style::new(imgs.dice)
                        .hover_image(imgs.dice_hover)
                        .press_image(imgs.dice_press),
                )
                .on_press(Message::RandomizeCharacter)
                .with_tooltip(tooltip_manager, move || {
                    let tooltip_text = i18n.get_msg("common-rand_appearance");
                    tooltip::text(&tooltip_text, tooltip_style)
                });

                let column_content = vec![
                    body_type.into(),
                    tool.into(),
                    species.into(),
                    slider_options.into(),
                    rand_character.into(),
                ];

                let left_column = Container::new(
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

                let left_column = Column::with_children(vec![
                    Container::new(left_column)
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
                    left_column.into(),
                    MouseDetector::new(&mut self.mouse_detector, Length::Fill, Length::Fill).into(),
                ])
                .padding(10)
                .width(Length::Fill)
                .height(Length::Fill);

                let back = neat_button(
                    back_button,
                    i18n.get_msg("common-back").into_owned(),
                    FILL_FRAC_ONE,
                    button_style,
                    Some(Message::Back),
                );

                const NAME_DICE_SIZE: u16 = 35;
                let rand_name = Button::new(
                    rand_name_button,
                    Space::new(Length::Units(NAME_DICE_SIZE), Length::Units(NAME_DICE_SIZE)),
                )
                .style(
                    style::button::Style::new(imgs.dice)
                        .hover_image(imgs.dice_hover)
                        .press_image(imgs.dice_press),
                )
                .on_press(Message::RandomizeName)
                .with_tooltip(tooltip_manager, move || {
                    let tooltip_text = i18n.get_msg("common-rand_name");
                    tooltip::text(&tooltip_text, tooltip_style)
                });

                let confirm_msg = if let Some(character_id) = character_id {
                    Message::ConfirmEdit(*character_id)
                } else {
                    Message::CreateCharacter
                };

                let name_input = BackgroundContainer::new(
                    Image::new(imgs.name_input)
                        .height(Length::Units(40))
                        .fix_aspect_ratio(),
                    TextInput::new(
                        name_input,
                        &i18n.get_msg("character_window-character_name"),
                        name,
                        Message::Name,
                    )
                    .size(25)
                    .on_submit(confirm_msg.clone()),
                )
                .padding(Padding::new().horizontal(7).top(5));

                let bottom_center = Container::new(
                    Row::with_children(vec![
                        rand_name.into(),
                        name_input.into(),
                        Space::new(Length::Units(NAME_DICE_SIZE), Length::Units(NAME_DICE_SIZE))
                            .into(),
                    ])
                    .align_items(Align::Center)
                    .spacing(5)
                    .padding(16),
                )
                .style(style::container::Style::color(Rgba::new(0, 0, 0, 100)));

                let create = neat_button(
                    create_button,
                    i18n.get_msg(if character_id.is_some() {
                        "common-confirm"
                    } else {
                        "common-create"
                    }),
                    FILL_FRAC_ONE,
                    button_style,
                    (!name.is_empty()).then_some(confirm_msg),
                );

                let create: Element<Message> = if name.is_empty() {
                    create
                        .with_tooltip(tooltip_manager, move || {
                            let tooltip_text = i18n.get_msg("char_selection-create_info_name");
                            tooltip::text(&tooltip_text, tooltip_style)
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

        // TODO: There is probably a better way to conditionally add in the warning box
        // here
        if let Some(mismatched_version) = &self.server_mismatched_version {
            let warning = Text::<IcedRenderer>::new(format!(
                "{}\n{}: {} {}: {}",
                i18n.get_msg("char_selection-version_mismatch"),
                i18n.get_msg("main-login-server_version"),
                mismatched_version,
                i18n.get_msg("main-login-client_version"),
                *common::util::GIT_HASH
            ))
            .size(self.fonts.cyri.scale(18))
            .color(iced::Color::from_rgb(1.0, 0.0, 0.0))
            .width(Length::Fill)
            .horizontal_alignment(HorizontalAlignment::Center);
            let warning_container =
                Container::new(Row::with_children(vec![warning.into()]).width(Length::Fill))
                    .style(style::container::Style::color(Rgba::new(0, 0, 0, 217)))
                    .padding(12)
                    .center_x()
                    .width(Length::Fill);

            Container::new(
                Column::with_children(vec![top_text.into(), warning_container.into(), content])
                    .spacing(3)
                    .width(Length::Fill)
                    .height(Length::Fill),
            )
            .padding(3)
            .into()
        } else {
            Container::new(
                Column::with_children(vec![top_text.into(), content])
                    .spacing(3)
                    .width(Length::Fill)
                    .height(Length::Fill),
            )
            .padding(3)
            .into()
        }
    }

    fn update(&mut self, message: Message, events: &mut Vec<Event>, characters: &[CharacterItem]) {
        match message {
            Message::Back => {
                if matches!(&self.mode, Mode::CreateOrEdit { .. }) {
                    self.mode = Mode::select(None);
                }
            },
            Message::Logout => {
                events.push(Event::Logout);
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
                        *info_content = None;
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
            Message::DoNothing => {},
            _ if matches!(self.mode, Mode::Select {
                info_content: Some(_),
                ..
            }) =>
            {
                // Don't allow use of the UI on the select screen to deal with
                // things other than the event currently being
                // procesed; all the select screen events after this
                // modify the info content or selection, except for Spectate
                // which currently causes us to exit the
                // character select state.
            },
            Message::EnterWorld => {
                if let (Mode::Select { info_content, .. }, Some(selected)) =
                    (&mut self.mode, self.selected)
                {
                    events.push(Event::Play(selected));
                    *info_content = Some(InfoContent::JoiningCharacter);
                }
            },
            Message::Spectate => {
                if matches!(self.mode, Mode::Select { .. }) {
                    events.push(Event::Spectate);
                    // FIXME: Enter JoiningCharacter when we have a proper error
                    // event for spectating.
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
            Message::Edit(idx) => {
                if matches!(&self.mode, Mode::Select { .. }) {
                    if let Some(character) = characters.get(idx) {
                        if let comp::Body::Humanoid(body) = character.body {
                            if let Some(id) = character.character.id {
                                self.mode = Mode::edit(
                                    character.character.alias.clone(),
                                    id,
                                    body,
                                    &character.inventory,
                                );
                            }
                        }
                    }
                }
            },
            Message::NewCharacter => {
                if matches!(&self.mode, Mode::Select { .. }) {
                    self.mode = Mode::create(self.default_name.clone());
                }
            },
            Message::CreateCharacter => {
                if let Mode::CreateOrEdit {
                    name,
                    body,
                    mainhand,
                    offhand,
                    ..
                } = &self.mode
                {
                    events.push(Event::AddCharacter {
                        alias: name.clone(),
                        mainhand: mainhand.map(String::from),
                        offhand: offhand.map(String::from),
                        body: comp::Body::Humanoid(*body),
                    });
                    self.mode = Mode::select(Some(InfoContent::CreatingCharacter));
                }
            },
            Message::ConfirmEdit(character_id) => {
                if let Mode::CreateOrEdit { name, body, .. } = &self.mode {
                    events.push(Event::EditCharacter {
                        alias: name.clone(),
                        character_id,
                        body: comp::Body::Humanoid(*body),
                    });
                    self.mode = Mode::select(Some(InfoContent::EditingCharacter));
                }
            },
            Message::Name(value) => {
                if let Mode::CreateOrEdit { name, .. } = &mut self.mode {
                    *name = value.chars().take(MAX_NAME_LENGTH).collect();
                }
            },
            Message::BodyType(value) => {
                if let Mode::CreateOrEdit { body, .. } = &mut self.mode {
                    body.body_type = value;
                    body.validate();
                }
            },
            Message::Species(value) => {
                if let Mode::CreateOrEdit { body, .. } = &mut self.mode {
                    body.species = value;
                    body.validate();
                }
            },
            Message::Tool(value) => {
                if let Mode::CreateOrEdit {
                    mainhand,
                    offhand,
                    inventory,
                    ..
                } = &mut self.mode
                {
                    *mainhand = value.0;
                    *offhand = value.1;
                    inventory.replace_loadout_item(
                        EquipSlot::ActiveMainhand,
                        mainhand.map(Item::new_from_asset_expect),
                    );
                    inventory.replace_loadout_item(
                        EquipSlot::ActiveOffhand,
                        offhand.map(Item::new_from_asset_expect),
                    );
                }
            },
            //Todo: Add species and body type to randomization.
            Message::RandomizeCharacter => {
                if let Mode::CreateOrEdit { body, .. } = &mut self.mode {
                    use rand::Rng;
                    let body_type = body.body_type;
                    let species = body.species;
                    let mut rng = rand::thread_rng();
                    body.hair_style = rng.gen_range(0..species.num_hair_styles(body_type));
                    body.beard = rng.gen_range(0..species.num_beards(body_type));
                    body.accessory = rng.gen_range(0..species.num_accessories(body_type));
                    body.hair_color = rng.gen_range(0..species.num_hair_colors());
                    body.skin = rng.gen_range(0..species.num_skin_colors());
                    body.eye_color = rng.gen_range(0..species.num_eye_colors());
                    body.eyes = rng.gen_range(0..species.num_eyes(body_type));
                }
            },

            Message::RandomizeName => {
                if let Mode::CreateOrEdit { name, body, .. } = &mut self.mode {
                    use common::npc;
                    *name = npc::get_npc_name(
                        npc::NpcKind::Humanoid,
                        npc::BodyType::from_body(comp::Body::Humanoid(*body)),
                    );
                }
            },
            Message::HairStyle(value) => {
                if let Mode::CreateOrEdit { body, .. } = &mut self.mode {
                    body.hair_style = value;
                    body.validate();
                }
            },
            Message::HairColor(value) => {
                if let Mode::CreateOrEdit { body, .. } = &mut self.mode {
                    body.hair_color = value;
                    body.validate();
                }
            },
            Message::Skin(value) => {
                if let Mode::CreateOrEdit { body, .. } = &mut self.mode {
                    body.skin = value;
                    body.validate();
                }
            },
            Message::Eyes(value) => {
                if let Mode::CreateOrEdit { body, .. } = &mut self.mode {
                    body.eyes = value;
                    body.validate();
                }
            },
            Message::EyeColor(value) => {
                if let Mode::CreateOrEdit { body, .. } = &mut self.mode {
                    body.eye_color = value;
                    body.validate();
                }
            },
            Message::Accessory(value) => {
                if let Mode::CreateOrEdit { body, .. } = &mut self.mode {
                    body.accessory = value;
                    body.validate();
                }
            },
            Message::Beard(value) => {
                if let Mode::CreateOrEdit { body, .. } = &mut self.mode {
                    body.beard = value;
                    body.validate();
                }
            },
        }
    }

    /// Get the character to display
    pub fn display_body_inventory<'a>(
        &'a self,
        characters: &'a [CharacterItem],
    ) -> Option<(comp::Body, &'a Inventory)> {
        match &self.mode {
            Mode::Select { .. } => self
                .selected
                .and_then(|id| characters.iter().find(|i| i.character.id == Some(id)))
                .map(|i| (i.body, &i.inventory)),
            Mode::CreateOrEdit {
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
        let font = ui::ice::load_font(&i18n.fonts().get("cyri").unwrap().asset_key);

        let mut ui = Ui::new(
            &mut global_state.window,
            font,
            global_state.settings.interface.ui_scale,
        )
        .unwrap();

        let fonts = Fonts::load(i18n.fonts(), &mut ui).expect("Impossible to load fonts");

        #[cfg(feature = "singleplayer")]
        let default_name = match global_state.singleplayer {
            Some(_) => String::new(),
            None => global_state.settings.networking.username.clone(),
        };

        #[cfg(not(feature = "singleplayer"))]
        let default_name = global_state.settings.networking.username.clone();

        let controls = Controls::new(
            fonts,
            Imgs::load(&mut ui).expect("Failed to load images"),
            selected_character,
            default_name,
            client.server_info(),
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
    ) -> Option<(comp::Body, &'a Inventory)> {
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

    pub fn update_language(&mut self, i18n: LocalizationHandle) {
        let i18n = i18n.read();
        let font = ui::ice::load_font(&i18n.fonts().get("cyri").unwrap().asset_key);

        self.ui.clear_fonts(font);
        self.controls.fonts =
            Fonts::load(i18n.fonts(), &mut self.ui).expect("Impossible to load fonts!");
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
                .view(&global_state.settings, client, &self.error, &i18n),
            global_state.window.renderer_mut(),
            None,
            &mut global_state.clipboard,
        );

        if self.enter_pressed {
            self.enter_pressed = false;
            messages.push(match self.controls.mode {
                Mode::Select { .. } => Message::EnterWorld,
                Mode::CreateOrEdit { .. } => Message::CreateCharacter,
            });
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

    pub fn render<'a>(&'a self, drawer: &mut UiDrawer<'_, 'a>) { self.ui.render(drawer); }
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
