use crate::{
    i18n::{i18n_asset_key, Localization},
    render::Renderer,
    ui::{
        self,
        fonts::IcedFonts as Fonts,
        ice::{component::neat_button, style, widget::Overlay, Element, IcedUi as Ui},
        img_ids::{ImageGraphic, VoxelGraphic},
    },
    window, GlobalState,
};
use client::Client;
use common::{
    character::{CharacterItem, MAX_CHARACTERS_PER_PLAYER},
    comp,
    comp::humanoid,
};
//ImageFrame, Tooltip,
use crate::settings::Settings;
use common::assets::load_expect;
//use std::time::Duration;
//use ui::ice::widget;
use iced::{
    button, text_input, Align, Button, Column, Container, HorizontalAlignment, Length, Row, Space,
    Text,
};

pub const TEXT_COLOR: iced::Color = iced::Color::from_rgb(1.0, 1.0, 1.0);
pub const DISABLED_TEXT_COLOR: iced::Color = iced::Color::from_rgba(1.0, 1.0, 1.0, 0.2);
const FILL_FRAC_ONE: f32 = 0.77;
const FILL_FRAC_TWO: f32 = 0.53;

image_ids_ice! {
    struct Imgs {
        <VoxelGraphic>
        // TODO: convert large frames into borders
        charlist_frame: "voxygen.element.frames.window_4",
        server_frame: "voxygen.element.frames.server_frame",
        slider_range: "voxygen.element.slider.track",
        slider_indicator: "voxygen.element.slider.indicator",

        <ImageGraphic>
        selection: "voxygen.element.frames.selection",
        selection_hover: "voxygen.element.frames.selection_hover",
        selection_press: "voxygen.element.frames.selection_press",

        delete_button: "voxygen.element.buttons.x_red",
        delete_button_hover: "voxygen.element.buttons.x_red_hover",
        delete_button_press: "voxygen.element.buttons.x_red_press",


        name_input: "voxygen.element.misc_bg.textbox_mid",

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

        <ImageGraphic>
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
    Play(CharacterItem),
    AddCharacter {
        alias: String,
        tool: Option<String>,
        body: comp::Body,
    },
    DeleteCharacter(i32),
}

struct CharacterList {
    characters: Vec<CharacterItem>,
    selected_character: usize,
}

enum Mode {
    Select {
        list: Option<CharacterList>,
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

        Self {
            fonts,
            imgs,
            i18n,
            version,

            info_content: None,
            mode: Mode::Select {
                list: None,
                character_buttons: Vec::new(),
                new_character_button: Default::default(),
                logout_button: Default::default(),
                enter_world_button: Default::default(),
                change_server_button: Default::default(),
            },
        }
    }

    fn view(&mut self, settings: &Settings) -> Element<Message> {
        // TODO: if enter key pressed and character is selected then enter the world
        // TODO: tooltip widget

        let imgs = &self.imgs;
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

        let content = match &mut self.mode {
            Mode::Select {
                list,
                ref mut character_buttons,
                ref mut new_character_button,
                ref mut logout_button,
                ref mut enter_world_button,
                ref mut change_server_button,
            } => {
                // TODO: impl delete prompt as overlay
                let change_server = Space::new(Length::Units(100), Length::Units(40));
                let characters = if let Some(list) = list {
                    let num = list.characters.len();
                    // Ensure we have enough button states
                    character_buttons.resize_with(num * 2, Default::default);

                    let mut characters = list
                        .characters
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
                                Button::new(select_button, Space::new(Length::Fill, Length::Fill))
                                    .width(Length::Units(20))
                                    .height(Length::Units(20))
                                    .style(
                                        style::button::Style::new(imgs.delete_button)
                                            .hover_image(imgs.delete_button_hover)
                                            .press_image(imgs.delete_button_press),
                                    ),
                                Button::new(
                                    delete_button,
                                    Column::with_children(vec![
                                        Text::new("Hi").into(),
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
                            .into()
                        })
                        .collect::<Vec<_>>();

                    // Add create new character button
                    let color = if num >= MAX_CHARACTERS_PER_PLAYER {
                        iced::Color::from_rgb8(97, 97, 25)
                    } else {
                        iced::Color::from_rgb8(97, 255, 18)
                    };
                    characters.push(
                        Button::new(
                            new_character_button,
                            Text::new(i18n.get("char_selection.create_new_character")),
                        )
                        .style(
                            style::button::Style::new(imgs.selection)
                                .hover_image(imgs.selection_hover)
                                .press_image(imgs.selection_press)
                                .image_color(color)
                                .text_color(color),
                        )
                        .into(),
                    );
                    characters
                } else {
                    Vec::new()
                };

                let characters = Column::with_children(characters);

                let right_column =
                    Column::with_children(vec![change_server.into(), characters.into()])
                        .spacing(10)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .max_width(300);

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
                    Some(Message::EnterWorld),
                );

                let bottom = Row::with_children(vec![
                    Container::new(logout)
                        .width(Length::Fill)
                        .height(Length::Units(40))
                        .align_y(Align::End)
                        .into(),
                    Container::new(enter_world)
                        .width(Length::Fill)
                        .height(Length::Units(60))
                        .center_x()
                        .align_y(Align::End)
                        .into(),
                    Space::new(Length::Fill, Length::Shrink).into(),
                ]);

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
            Column::with_children(vec![version.into(), content.into()])
                .spacing(3)
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .padding(3)
        .into()
    }

    fn update(&mut self, message: Message, events: &mut Vec<Event>, settings: &Settings) {
        let servers = &settings.networking.servers;

        //match message { }
    }

    pub fn selected_character(&self) -> Option<&CharacterItem> {
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
        let i18n = load_expect::<Localization>(&i18n_asset_key(
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

    pub fn selected_character(&self) -> Option<&CharacterItem> {
        self.controls.selected_character()
    }

    // TODO
    pub fn get_loadout(&mut self) -> Option<comp::Loadout> {
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
                    item: (*load_expect::<comp::Item>(tool)).clone(),
                    ability1: None,
                    ability2: None,
                    ability3: None,
                    block_ability: None,
                    dodge_ability: None,
                });
                loadout.chest = Some(assets::load_expect_cloned(
                    "common.items.armor.starter.rugged_chest",
                ));
                loadout.pants = Some(assets::load_expect_cloned(
                    "common.items.armor.starter.rugged_pants",
                ));
                loadout.foot = Some(assets::load_expect_cloned(
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

    pub fn maintain(&mut self, global_state: &mut GlobalState, client: &mut Client) -> Vec<Event> {
        let mut events = Vec::new();

        let (messages, _) = self.ui.maintain(
            self.controls.view(&global_state.settings),
            global_state.window.renderer_mut(),
        );

        messages.into_iter().for_each(|message| {
            self.controls
                .update(message, &mut events, &global_state.settings)
        });

        events
    }

    // TODO: do we need globals
    pub fn render(&self, renderer: &mut Renderer) { self.ui.render(renderer); }
}
