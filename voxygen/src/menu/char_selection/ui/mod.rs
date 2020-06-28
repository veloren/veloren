use crate::{
    i18n::{i18n_asset_key, Localization},
    render::Renderer,
    ui::{
        self,
        fonts::IcedFonts as Fonts,
        ice::{
            component::neat_button,
            style,
            widget::{AspectRatioContainer, Overlay},
            Element, IcedUi as Ui,
        },
        img_ids::{ImageGraphic, VoxelGraphic},
    },
    window, GlobalState,
};
use client::Client;
use common::{
    assets::Asset,
    character::{CharacterId, CharacterItem, MAX_CHARACTERS_PER_PLAYER},
    comp::{self, humanoid},
};
//ImageFrame, Tooltip,
use crate::settings::Settings;
//use std::time::Duration;
//use ui::ice::widget;
use iced::{
    button, scrollable, text_input, Align, Button, Column, Container, HorizontalAlignment, Length,
    Row, Scrollable, Space, Text,
};
use vek::Rgba;

pub const TEXT_COLOR: iced::Color = iced::Color::from_rgb(1.0, 1.0, 1.0);
pub const DISABLED_TEXT_COLOR: iced::Color = iced::Color::from_rgba(1.0, 1.0, 1.0, 0.2);
const FILL_FRAC_ONE: f32 = 0.77;
const FILL_FRAC_TWO: f32 = 0.60;

const STARTER_HAMMER: &str = "common.items.weapons.hammer.starter_hammer";
const STARTER_BOW: &str = "common.items.weapons.bow.starter_bow";
const STARTER_AXE: &str = "common.items.weapons.axe.starter_axe";
const STARTER_STAFF: &str = "common.items.weapons.staff.starter_staff";
const STARTER_SWORD: &str = "common.items.weapons.sword.starter_sword";

// TODO: look into what was using this in old ui
const UI_MAIN: iced::Color = iced::Color::from_rgba(0.61, 0.70, 0.70, 1.0); // Greenish Blue

image_ids_ice! {
    struct Imgs {
        <VoxelGraphic>
        slider_range: "voxygen.element.slider.track",
        slider_indicator: "voxygen.element.slider.indicator",

        <ImageGraphic>
        gray_corner: "voxygen.element.frames.gray.corner",
        gray_edge: "voxygen.element.frames.gray.edge",

        selection: "voxygen.element.frames.selection",
        selection_hover: "voxygen.element.frames.selection_hover",
        selection_press: "voxygen.element.frames.selection_press",

        delete_button: "voxygen.element.buttons.x_red",
        delete_button_hover: "voxygen.element.buttons.x_red_hover",
        delete_button_press: "voxygen.element.buttons.x_red_press",

        name_input: "voxygen.element.misc_bg.textbox",

        // Tool Icons
        daggers: "voxygen.element.icons.daggers",
        sword: "voxygen.element.icons.sword",
        axe: "voxygen.element.icons.axe",
        hammer: "voxygen.element.icons.hammer",
        bow: "voxygen.element.icons.bow",
        staff: "voxygen.element.icons.staff",

        // Species Icons
        male: "voxygen.element.icons.male",
        female: "voxygen.element.icons.female",
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
    }
}

// TODO: do rotation in widget renderer
/*rotation_image_ids! {
    pub struct ImgsRot {
        <VoxelGraphic>

        // Tooltip Test
        tt_side: "voxygen/element/frames/tt_test_edge",
        tt_corner: "voxygen/element/frames/tt_test_corner_tr",
    }
}*/

pub enum Event {
    Logout,
    Play(CharacterId),
    AddCharacter {
        alias: String,
        tool: Option<String>,
        body: comp::Body,
    },
    DeleteCharacter(CharacterId),
}

enum Mode {
    Select {
        // Index of selected character
        selected: Option<usize>,

        characters_scroll: scrollable::State,
        character_buttons: Vec<button::State>,
        new_character_button: button::State,
        logout_button: button::State,
        enter_world_button: button::State,
        change_server_button: button::State,
    },
    Create {
        name: String, // TODO: default to username
        body: humanoid::Body,
        loadout: comp::Loadout,
        tool: Option<&'static str>,

        name_input: text_input::State,
        back_button: button::State,
        create_button: button::State,
    },
}

impl Mode {
    pub fn select() -> Self {
        Self::Select {
            selected: None,
            characters_scroll: Default::default(),
            character_buttons: Vec::new(),
            new_character_button: Default::default(),
            logout_button: Default::default(),
            enter_world_button: Default::default(),
            change_server_button: Default::default(),
        }
    }

    pub fn create(name: String) -> Self {
        Self::Create {
            name,
            body: humanoid::Body::random(),
            loadout: comp::Loadout::default(),
            tool: Some(STARTER_SWORD),

            name_input: Default::default(),
            back_button: Default::default(),
            create_button: Default::default(),
        }
    }
}

#[derive(PartialEq)]
enum InfoContent {
    Deletion(usize),
    LoadingCharacters,
    CreatingCharacter,
    DeletingCharacter,
    CharacterError,
}

/*
impl InfoContent {
    pub fn has_content(&self, character_list_loading: &bool) -> bool {
        match self {
            Self::None => false,
            Self::CreatingCharacter | Self::DeletingCharacter | Self::LoadingCharacters => {
                *character_list_loading
            },
            _ => true,
        }
    }
}
*/

struct Controls {
    fonts: Fonts,
    imgs: Imgs,
    i18n: std::sync::Arc<Localization>,
    // Voxygen version
    version: String,
    // Alpha disclaimer
    alpha: String,

    info_content: Option<InfoContent>,
    // enter: bool,
    mode: Mode,
}

#[derive(Clone)]
enum Message {
    Back,
    Logout,
    EnterWorld,
    Delete(usize),
    ChangeServer,
    NewCharacter,
    CreateCharacter,
    Name(String),
}

impl Controls {
    fn new(fonts: Fonts, imgs: Imgs, i18n: std::sync::Arc<Localization>) -> Self {
        let version = format!(
            "{}-{}",
            env!("CARGO_PKG_VERSION"),
            common::util::GIT_VERSION.to_string()
        );
        let alpha = format!("Veloren Pre-Alpha {}", env!("CARGO_PKG_VERSION"),);

        Self {
            fonts,
            imgs,
            i18n,
            version,
            alpha,

            info_content: None,
            mode: Mode::select(),
        }
    }

    fn view(&mut self, settings: &Settings, client: &Client) -> Element<Message> {
        // TODO: if enter key pressed and character is selected then enter the world
        // TODO: tooltip widget

        let imgs = &self.imgs;
        let fonts = &self.fonts;
        let i18n = &self.i18n;

        let button_style = style::button::Style::new(imgs.button)
            .hover_image(imgs.button_hover)
            .press_image(imgs.button_press)
            .text_color(TEXT_COLOR)
            .disabled_text_color(DISABLED_TEXT_COLOR);

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
                selected,
                ref mut characters_scroll,
                ref mut character_buttons,
                ref mut new_character_button,
                ref mut logout_button,
                ref mut enter_world_button,
                ref mut change_server_button,
            } => {
                // TODO: impl delete prompt as overlay
                let server = Container::new(
                    Column::with_children(vec![
                        Text::new(&client.server_info.name)
                        .size(fonts.cyri.scale(25))
                        //.horizontal_alignment(HorizontalAlignment::Center)
                        .into(),
                        Container::new(neat_button(
                            change_server_button,
                            i18n.get("char_selection.change_server"),
                            FILL_FRAC_TWO,
                            button_style,
                            Some(Message::ChangeServer),
                        ))
                        .height(Length::Units(35))
                        .into(),
                    ])
                    .spacing(5)
                    .align_items(Align::Center),
                )
                .style(style::container::Style::color_with_image_border(
                    Rgba::new(0, 0, 0, 217),
                    imgs.gray_corner,
                    imgs.gray_edge,
                ))
                .padding(12)
                .center_x()
                .width(Length::Fill);

                let characters = {
                    let characters = &client.character_list.characters;
                    let num = characters.len();
                    // Ensure we have enough button states
                    character_buttons.resize_with(num * 2, Default::default);

                    let mut characters = characters
                        .iter()
                        .zip(character_buttons.chunks_exact_mut(2))
                        .map(|(character, buttons)| {
                            let mut buttons = buttons.iter_mut();
                            (
                                character,
                                (buttons.next().unwrap(), buttons.next().unwrap()),
                            )
                        })
                        .enumerate()
                        .map(|(i, (character, (select_button, delete_button)))| {
                            Overlay::new(
                                Button::new(
                                    select_button,
                                    Space::new(Length::Units(20), Length::Units(20)),
                                )
                                .style(
                                    style::button::Style::new(imgs.delete_button)
                                        .hover_image(imgs.delete_button_hover)
                                        .press_image(imgs.delete_button_press),
                                ),
                                AspectRatioContainer::new(
                                    Button::new(
                                        delete_button,
                                        Column::with_children(vec![
                                            Text::new("Hi").width(Length::Fill).into(),
                                            Text::new("Hi").into(),
                                            Text::new("Hi").into(),
                                        ]),
                                    )
                                    .style(
                                        style::button::Style::new(imgs.selection)
                                            .hover_image(imgs.selection_hover)
                                            .press_image(imgs.selection_press),
                                    ),
                                )
                                .ratio_of_image(imgs.selection),
                            )
                            .padding(15)
                            .align_x(Align::End)
                            .into()
                        })
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
                            // TODO: try to get better interface for this in iced
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
                let characters = Container::new(
                    Scrollable::new(characters_scroll)
                        .push(Column::with_children(characters).spacing(2)),
                )
                .style(style::container::Style::color_with_image_border(
                    Rgba::new(0, 0, 0, 217),
                    imgs.gray_corner,
                    imgs.gray_edge,
                ))
                .padding(9)
                .width(Length::Fill)
                .height(Length::Fill);

                let right_column = Column::with_children(vec![server.into(), characters.into()])
                    .padding(15)
                    .spacing(10)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .max_width(360);

                let top = Container::new(right_column)
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
                    FILL_FRAC_ONE,
                    button_style,
                    selected.map(|_| Message::EnterWorld),
                );

                let bottom = Row::with_children(vec![
                    Container::new(logout)
                        .width(Length::Fill)
                        .height(Length::Units(40))
                        .into(),
                    Container::new(enter_world)
                        .width(Length::Fill)
                        .height(Length::Units(60))
                        .center_x()
                        .into(),
                    Space::new(Length::Fill, Length::Shrink).into(),
                ])
                .align_items(Align::End);

                Column::with_children(vec![top.into(), bottom.into()])
                    .width(Length::Fill)
                    .height(Length::Fill)
            },
            Mode::Create {
                name,
                body,
                loadout,
                tool,
                name_input,
                back_button,
                create_button,
            } => {
                let top_row = Row::with_children(vec![]);
                let bottom_row = Row::with_children(vec![]);

                Column::with_children(vec![top_row.into(), bottom_row.into()])
                    .width(Length::Fill)
                    .height(Length::Fill)
            },
        };

        Container::new(
            Column::with_children(vec![top_text.into(), content.into()])
                .spacing(3)
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .padding(3)
        .into()
    }

    fn update(
        &mut self,
        message: Message,
        events: &mut Vec<Event>,
        settings: &Settings,
        characters: &[CharacterItem],
    ) {
        let servers = &settings.networking.servers;

        match message {
            Message::Back => {
                if matches!(&self.mode, Mode::Create { .. }) {
                    self.mode = Mode::select();
                }
            },
            Message::Logout => {
                events.push(Event::Logout);
            },
            Message::EnterWorld => {
                if let Mode::Select {
                    selected: Some(selected),
                    ..
                } = &self.mode
                {
                    // TODO: eliminate option in character id
                    if let Some(id) = characters.get(*selected).and_then(|i| i.character.id) {
                        events.push(Event::Play(id));
                    }
                }
            },
            Message::Delete(idx) => {
                if let Some(id) = characters.get(idx).and_then(|i| i.character.id) {
                    events.push(Event::DeleteCharacter(id));
                }
            },
            Message::ChangeServer => {
                events.push(Event::Logout);
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
                        tool: tool.map(String::from),
                        body: comp::Body::Humanoid(*body),
                    });
                    self.mode = Mode::select();
                }
            },
            Message::Name(value) => {
                if let Mode::Create { name, .. } = &mut self.mode {
                    *name = value;
                }
            },
        }
    }

    /// Get the character to display
    pub fn display_character(&self, characters: &[CharacterItem]) -> Option<&CharacterItem> {
        match self.mode {
            // TODO
            Mode::Select { .. } => None,
            // TODO
            Mode::Create { .. } => None,
        }
    }
}

pub struct CharSelectionUi {
    ui: Ui,
    controls: Controls,
}

impl CharSelectionUi {
    pub fn new(global_state: &mut GlobalState) -> Self {
        // Load language
        let i18n = Localization::load_expect(&i18n_asset_key(
            &global_state.settings.language.selected_language,
        ));

        // TODO: don't add default font twice
        let font = {
            use std::io::Read;
            let mut buf = Vec::new();
            common::assets::load_file("voxygen.font.haxrcorp_4089_cyrillic_altgr_extended", &[
                "ttf",
            ])
            .unwrap()
            .read_to_end(&mut buf)
            .unwrap();
            ui::ice::Font::try_from_vec(buf).unwrap()
        };

        let mut ui = Ui::new(&mut global_state.window, font).unwrap();

        let fonts = Fonts::load(&i18n.fonts, &mut ui).expect("Impossible to load fonts");

        let controls = Controls::new(
            fonts,
            Imgs::load(&mut ui).expect("Failed to load images"),
            i18n,
        );

        Self { ui, controls }
    }

    pub fn display_character(&self, characters: &[CharacterItem]) -> Option<&CharacterItem> {
        self.controls.display_character(characters)
    }

    // TODO
    pub fn get_loadout(&mut self) -> Option<comp::Loadout> {
        // TODO: error gracefully
        // TODO: don't clone
        /*match &mut self.mode {
            Mode::Select(character_list) => {
                if let Some(data) = character_list {
                    data.get(self.selected_character).map(|c| c.loadout.clone())
                } else {
                    None
                }
            },
            Mode::Create { loadout, tool, .. } => {
                loadout.active_item = tool.map(|tool| comp::ItemConfig {
                    item: comp::Item::new_from_asset_expect(tool),
                    ability1: None,
                    ability2: None,
                    ability3: None,
                    block_ability: None,
                    dodge_ability: None,
                });
                loadout.chest = Some(comp::Item::new_from_asset_expect(
                    "common.items.armor.starter.rugged_chest",
                ));
                loadout.pants = Some(comp::Item::new_from_asset_expect(
                    "common.items.armor.starter.rugged_pants",
                ));
                loadout.foot = Some(comp::Item::new_from_asset_expect(
                    "common.items.armor.starter.sandals_0",
                ));
                Some(loadout.clone())
            },
        }*/
        None
    }

    pub fn handle_event(&mut self, event: window::Event) -> bool {
        match event {
            window::Event::IcedUi(event) => {
                self.ui.handle_event(event);
                true
            },
            window::Event::MouseButton(_, window::PressState::Pressed) => {
                // TODO: implement this with iced
                // !self.ui.no_widget_capturing_mouse()
                false
            },
            _ => false,
        }
    }

    // TODO: do we need whole client here or just character list
    pub fn maintain(&mut self, global_state: &mut GlobalState, client: &mut Client) -> Vec<Event> {
        let mut events = Vec::new();

        let (messages, _) = self.ui.maintain(
            self.controls.view(&global_state.settings, &client),
            global_state.window.renderer_mut(),
        );

        messages.into_iter().for_each(|message| {
            self.controls.update(
                message,
                &mut events,
                &global_state.settings,
                &client.character_list.characters,
            )
        });

        events
    }

    // TODO: do we need globals
    pub fn render(&self, renderer: &mut Renderer) { self.ui.render(renderer); }
}
