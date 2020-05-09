use crate::{
    i18n::{i18n_asset_key, VoxygenLocalization},
    render::{Consts, Globals, Renderer},
    ui::{
        fonts::ConrodVoxygenFonts,
        img_ids::{BlankGraphic, ImageGraphic, VoxelGraphic, VoxelSs9Graphic},
        ImageFrame, ImageSlider, Tooltip, Tooltipable, Ui,
    },
    window::{Event as WinEvent, PressState},
    GlobalState,
};
use client::Client;
use common::{
    assets,
    assets::{load, load_expect},
    character::{Character, CharacterItem, MAX_CHARACTERS_PER_PLAYER},
    comp::{self, humanoid},
};
use conrod_core::{
    color,
    color::TRANSPARENT,
    event::{Event as WorldEvent, Input},
    input::{Button as ButtonType, Key},
    position::Relative,
    widget::{text_box::Event as TextBoxEvent, Button, Image, Rectangle, Scrollbar, Text, TextBox},
    widget_ids, Borderable, Color, Colorable, Labelable, Positionable, Sizeable, UiCell, Widget,
};
use log::error;
use std::sync::Arc;

const STARTER_HAMMER: &str = "common.items.weapons.hammer.starter_hammer";
const STARTER_BOW: &str = "common.items.weapons.bow.starter_bow";
const STARTER_AXE: &str = "common.items.weapons.axe.starter_axe";
const STARTER_STAFF: &str = "common.items.weapons.staff.starter_staff";
const STARTER_SWORD: &str = "common.items.weapons.sword.starter_sword";
const STARTER_DAGGER: &str = "common.items.weapons.dagger.starter_dagger";

// UI Color-Theme
const UI_MAIN: Color = Color::Rgba(0.61, 0.70, 0.70, 1.0); // Greenish Blue
//const UI_HIGHLIGHT_0: Color = Color::Rgba(0.79, 1.09, 1.09, 1.0);

widget_ids! {
    struct Ids {
        // Background and logo
        charlist_bg,
        charlist_frame,
        charlist_alignment,
        selection_scrollbar,
        creation_bg,
        creation_frame,
        creation_alignment,
        server_name_text,
        change_server,
        server_frame_bg,
        server_frame,
        v_logo,
        version,
        divider,
        bodyrace_text,
        facialfeatures_text,
        info_bg,
        info_frame,
        info_button_align,
        info_ok,
        info_no,
        delete_text,
        space,
        loading_characters_text,
        creating_character_text,
        deleting_character_text,
        character_error_message,

        // REMOVE THIS AFTER IMPLEMENTATION
        daggers_grey,
        axe_grey,
        hammer_grey,
        bow_grey,
        staff_grey,


        // Characters
        character_boxes[],
        character_deletes[],
        character_names[],
        character_locations[],
        character_levels[],

        character_box_2,
        character_name_2,
        character_location_2,
        character_level_2,


        // Windows
        selection_window,
        char_name,
        char_level,
        creation_window,
        select_window_title,
        creation_buttons_alignment_1,
        creation_buttons_alignment_2,
        weapon_heading,
        weapon_description,
        human_skin_bg,
        orc_skin_bg,
        dwarf_skin_bg,
        undead_skin_bg,
        elf_skin_bg,
        danari_skin_bg,
        name_input_bg,
        info,

        // Sliders
        hairstyle_slider,
        hairstyle_text,
        haircolor_slider,
        haircolor_text,
        skin_slider,
        skin_text,
        eyecolor_slider,
        eyecolor_text,
        eyebrows_slider,
        eyebrows_text,
        beard_slider,
        beard_text,
        accessories_slider,
        accessories_text,
        chest_slider,
        chest_text,
        pants_slider,
        pants_text,

        // Buttons
        enter_world_button,
        back_button,
        logout_button,
        create_character_button,
        delete_button,
        create_button,
        name_input,
        name_field,
        race_1,
        race_2,
        race_3,
        race_4,
        race_5,
        race_6,
        body_type_1,
        body_type_2,

        // Tools
        sword,
        sword_button,
        daggers,
        daggers_button,
        axe,
        axe_button,
        hammer,
        hammer_button,
        bow,
        bow_button,
        staff,
        staff_button,
        // Char Creation
        // Race Icons
        male,
        female,
        human,
        orc,
        dwarf,
        undead,
        elf,
        danari,
    }
}

image_ids! {
    struct Imgs {
        <VoxelGraphic>
        charlist_frame: "voxygen.element.frames.window_4",
        server_frame: "voxygen.element.frames.server_frame",
        selection: "voxygen.element.frames.selection",
        selection_hover: "voxygen.element.frames.selection_hover",
        selection_press: "voxygen.element.frames.selection_press",
        slider_range: "voxygen.element.slider.track",
        slider_indicator: "voxygen.element.slider.indicator",

        // Info Window
        info_frame: "voxygen.element.frames.info_frame",

        <VoxelSs9Graphic>
        delete_button: "voxygen.element.buttons.x_red",
        delete_button_hover: "voxygen.element.buttons.x_red_hover",
        delete_button_press: "voxygen.element.buttons.x_red_press",

        <ImageGraphic>

        name_input: "voxygen.element.misc_bg.textbox_mid",

        // Tool Icons
        daggers: "voxygen.element.icons.daggers",
        sword: "voxygen.element.icons.sword",
        axe: "voxygen.element.icons.axe",
        hammer: "voxygen.element.icons.hammer",
        bow: "voxygen.element.icons.bow",
        staff: "voxygen.element.icons.staff",

        // Race Icons
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

        <BlankGraphic>
        nothing: (),
    }
}
rotation_image_ids! {
    pub struct ImgsRot {
        <VoxelGraphic>

        // Tooltip Test
        tt_side: "voxygen/element/frames/tt_test_edge",
        tt_corner: "voxygen/element/frames/tt_test_corner_tr",
    }
}

pub enum Event {
    Logout,
    Play,
    AddCharacter {
        alias: String,
        tool: Option<String>,
        body: comp::Body,
    },
    DeleteCharacter(i32),
}

const TEXT_COLOR: Color = Color::Rgba(1.0, 1.0, 1.0, 1.0);
const TEXT_COLOR_2: Color = Color::Rgba(1.0, 1.0, 1.0, 0.2);

#[derive(PartialEq)]
enum InfoContent {
    None,
    Deletion(usize),
    LoadingCharacters,
    CreatingCharacter,
    DeletingCharacter,
    CharacterError,
}

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

pub enum Mode {
    Select(Option<Vec<CharacterItem>>),
    Create {
        name: String,
        body: humanoid::Body,
        loadout: comp::Loadout,
        tool: Option<&'static str>,
    },
}

pub struct CharSelectionUi {
    ui: Ui,
    ids: Ids,
    imgs: Imgs,
    rot_imgs: ImgsRot,
    fonts: ConrodVoxygenFonts,
    info_content: InfoContent,
    voxygen_i18n: Arc<VoxygenLocalization>,
    pub mode: Mode,
    pub selected_character: usize,
}

impl CharSelectionUi {
    pub fn new(global_state: &mut GlobalState) -> Self {
        let window = &mut global_state.window;
        let settings = &global_state.settings;

        let mut ui = Ui::new(window).unwrap();
        ui.set_scaling_mode(settings.gameplay.ui_scale);
        // Generate ids
        let ids = Ids::new(ui.id_generator());
        // Load images
        let imgs = Imgs::load(&mut ui).expect("Failed to load images!");
        let rot_imgs = ImgsRot::load(&mut ui).expect("Failed to load images!");
        // Load language
        let voxygen_i18n = load_expect::<VoxygenLocalization>(&i18n_asset_key(
            &global_state.settings.language.selected_language,
        ));
        // Load fonts.
        let fonts = ConrodVoxygenFonts::load(&voxygen_i18n.fonts, &mut ui)
            .expect("Impossible to load fonts!");

        Self {
            ui,
            ids,
            imgs,
            rot_imgs,
            fonts,
            info_content: InfoContent::LoadingCharacters,
            selected_character: 0,
            voxygen_i18n,
            mode: Mode::Select(None),
        }
    }

    pub fn get_character_list(&self) -> Option<Vec<CharacterItem>> {
        match &self.mode {
            Mode::Select(data) => data.clone(),
            Mode::Create {
                name, body, tool, ..
            } => Some(vec![CharacterItem {
                character: Character {
                    id: None,
                    alias: name.clone(),
                    tool: tool.map(|specifier| specifier.to_string()),
                },
                body: comp::Body::Humanoid(body.clone()),
            }]),
        }
    }

    pub fn get_loadout(&mut self) -> Option<comp::Loadout> {
        match &mut self.mode {
            Mode::Select(character_list) => {
                if let Some(data) = character_list {
                    if let Some(character_item) = data.get(self.selected_character) {
                        let loadout = comp::Loadout {
                            active_item: character_item.character.tool.as_ref().map(|tool| {
                                comp::ItemConfig {
                                    item: (*load::<comp::Item>(&tool).unwrap_or_else(|err| {
                                        error!(
                                            "Could not load item {} maybe it no longer exists: \
                                             {:?}",
                                            &tool, err
                                        );
                                        load_expect("common.items.weapons.sword.starter_sword")
                                    }))
                                    .clone(),
                                    ability1: None,
                                    ability2: None,
                                    ability3: None,
                                    block_ability: None,
                                    dodge_ability: None,
                                }
                            }),
                            second_item: None,
                            shoulder: None,
                            chest: Some(assets::load_expect_cloned(
                                "common.items.armor.starter.rugged_chest",
                            )),
                            belt: None,
                            hand: None,
                            pants: Some(assets::load_expect_cloned(
                                "common.items.armor.starter.rugged_pants",
                            )),
                            foot: Some(assets::load_expect_cloned(
                                "common.items.armor.starter.sandals_0",
                            )),
                            back: None,
                            ring: None,
                            neck: None,
                            lantern: None,
                            head: None,
                            tabard: None,
                        };
                        Some(loadout)
                    } else {
                        None
                    }
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
        }
    }

    // TODO: Split this into multiple modules or functions.
    fn update_layout(&mut self, client: &mut Client) -> Vec<Event> {
        let mut events = Vec::new();

        let can_enter_world = match &self.mode {
            Mode::Select(opt) => opt.is_some(),
            Mode::Create { .. } => false,
        };

        // Handle enter keypress to enter world
        if can_enter_world {
            for event in self.ui.ui.global_input().events() {
                match event {
                    // TODO allow this to be rebound
                    WorldEvent::Raw(Input::Press(ButtonType::Keyboard(Key::Return)))
                    | WorldEvent::Raw(Input::Press(ButtonType::Keyboard(Key::Return2)))
                    | WorldEvent::Raw(Input::Press(ButtonType::Keyboard(Key::NumPadEnter))) => {
                        events.push(Event::Play)
                    },
                    _ => {},
                }
            }
        }
        let (ref mut ui_widgets, ref mut tooltip_manager) = self.ui.set_widgets();
        let version = format!(
            "{}-{}",
            env!("CARGO_PKG_VERSION"),
            common::util::GIT_VERSION.to_string()
        );

        // Tooltip
        let tooltip_human = Tooltip::new({
            // Edge images [t, b, r, l]
            // Corner images [tr, tl, br, bl]
            let edge = &self.rot_imgs.tt_side;
            let corner = &self.rot_imgs.tt_corner;
            ImageFrame::new(
                [edge.cw180, edge.none, edge.cw270, edge.cw90],
                [corner.none, corner.cw270, corner.cw90, corner.cw180],
                Color::Rgba(0.08, 0.07, 0.04, 1.0),
                5.0,
            )
        })
        .title_font_size(self.fonts.cyri.scale(15))
        .desc_font_size(self.fonts.cyri.scale(10))
        .font_id(self.fonts.cyri.conrod_id)
        .title_text_color(TEXT_COLOR)
        .desc_text_color(TEXT_COLOR_2);

        // Set the info content if we encountered an error related to characters
        if client.character_list.error.is_some() {
            self.info_content = InfoContent::CharacterError;
        }

        // Information Window
        if self
            .info_content
            .has_content(&client.character_list.loading)
        {
            Rectangle::fill_with([520.0, 150.0], color::rgba(0.0, 0.0, 0.0, 0.9))
                .mid_top_with_margin_on(ui_widgets.window, 300.0)
                .set(self.ids.info_bg, ui_widgets);
            Image::new(self.imgs.info_frame)
                .w_h(550.0, 150.0)
                .middle_of(self.ids.info_bg)
                .color(Some(UI_MAIN))
                .set(self.ids.info_frame, ui_widgets);
            Rectangle::fill_with([275.0, 150.0], color::TRANSPARENT)
                .bottom_left_with_margins_on(self.ids.info_frame, 0.0, 0.0)
                .set(self.ids.info_button_align, ui_widgets);

            match self.info_content {
                InfoContent::None => unreachable!(),
                InfoContent::Deletion(character_index) => {
                    Text::new(&self.voxygen_i18n.get("char_selection.delete_permanently"))
                        .mid_top_with_margin_on(self.ids.info_frame, 40.0)
                        .font_size(self.fonts.cyri.scale(24))
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(TEXT_COLOR)
                        .set(self.ids.delete_text, ui_widgets);
                    if Button::image(self.imgs.button)
                        .w_h(150.0, 40.0)
                        .bottom_right_with_margins_on(self.ids.info_button_align, 20.0, 50.0)
                        .hover_image(self.imgs.button_hover)
                        .press_image(self.imgs.button_press)
                        .label_y(Relative::Scalar(2.0))
                        .label(&self.voxygen_i18n.get("common.no"))
                        .label_font_id(self.fonts.cyri.conrod_id)
                        .label_font_size(self.fonts.cyri.scale(18))
                        .label_color(TEXT_COLOR)
                        .set(self.ids.info_no, ui_widgets)
                        .was_clicked()
                    {
                        self.info_content = InfoContent::None;
                    };
                    if Button::image(self.imgs.button)
                        .w_h(150.0, 40.0)
                        .right_from(self.ids.info_no, 100.0)
                        .hover_image(self.imgs.button_hover)
                        .press_image(self.imgs.button_press)
                        .label_y(Relative::Scalar(2.0))
                        .label(&self.voxygen_i18n.get("common.yes"))
                        .label_font_id(self.fonts.cyri.conrod_id)
                        .label_font_size(self.fonts.cyri.scale(18))
                        .label_color(TEXT_COLOR)
                        .set(self.ids.info_ok, ui_widgets)
                        .was_clicked()
                    {
                        self.info_content = InfoContent::None;

                        if let Some(character_item) =
                            client.character_list.characters.get(character_index)
                        {
                            // Unsaved characters have no id, this should never be the case here
                            if let Some(character_id) = character_item.character.id {
                                self.info_content = InfoContent::DeletingCharacter;

                                events.push(Event::DeleteCharacter(character_id));
                            }
                        }
                    };
                },
                InfoContent::LoadingCharacters => {
                    Text::new(&self.voxygen_i18n.get("char_selection.loading_characters"))
                        .mid_top_with_margin_on(self.ids.info_frame, 40.0)
                        .font_size(self.fonts.cyri.scale(24))
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(TEXT_COLOR)
                        .set(self.ids.loading_characters_text, ui_widgets);
                },
                InfoContent::CreatingCharacter => {
                    Text::new(&self.voxygen_i18n.get("char_selection.creating_character"))
                        .mid_top_with_margin_on(self.ids.info_frame, 40.0)
                        .font_size(self.fonts.cyri.scale(24))
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(TEXT_COLOR)
                        .set(self.ids.creating_character_text, ui_widgets);
                },
                InfoContent::DeletingCharacter => {
                    Text::new(&self.voxygen_i18n.get("char_selection.deleting_character"))
                        .mid_top_with_margin_on(self.ids.info_frame, 40.0)
                        .font_size(self.fonts.cyri.scale(24))
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(TEXT_COLOR)
                        .set(self.ids.deleting_character_text, ui_widgets);
                },
                InfoContent::CharacterError => {
                    if let Some(error_message) = &client.character_list.error {
                        Text::new(&format!(
                            "{}: {}",
                            &self.voxygen_i18n.get("common.error"),
                            error_message
                        ))
                        .mid_top_with_margin_on(self.ids.info_frame, 40.0)
                        .font_size(self.fonts.cyri.scale(24))
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(TEXT_COLOR)
                        .set(self.ids.character_error_message, ui_widgets);

                        if Button::image(self.imgs.button)
                            .w_h(150.0, 40.0)
                            .bottom_right_with_margins_on(self.ids.info_button_align, 20.0, 20.0)
                            .hover_image(self.imgs.button_hover)
                            .press_image(self.imgs.button_press)
                            .label_y(Relative::Scalar(2.0))
                            .label(&self.voxygen_i18n.get("common.close"))
                            .label_font_id(self.fonts.cyri.conrod_id)
                            .label_font_size(self.fonts.cyri.scale(18))
                            .label_color(TEXT_COLOR)
                            .set(self.ids.info_ok, ui_widgets)
                            .was_clicked()
                        {
                            self.info_content = InfoContent::None;
                            client.character_list.error = None;
                        }
                    } else {
                        self.info_content = InfoContent::None;
                    }
                },
            }
        }

        // Character Selection /////////////////
        match &mut self.mode {
            Mode::Select(data) => {
                // Set active body
                *data = if client
                    .character_list
                    .characters
                    .get(self.selected_character)
                    .is_some()
                {
                    Some(client.character_list.characters.clone())
                } else {
                    None
                };

                // Background for Server Frame
                Rectangle::fill_with([386.0, 95.0], color::rgba(0.0, 0.0, 0.0, 0.9))
                    .top_left_with_margins_on(ui_widgets.window, 30.0, 30.0)
                    .set(self.ids.server_frame_bg, ui_widgets);
                Image::new(self.imgs.server_frame)
                    .w_h(400.0, 100.0)
                    .color(Some(UI_MAIN))
                    .middle_of(self.ids.server_frame_bg)
                    .set(self.ids.server_frame, ui_widgets);

                // Background for Char List
                Rectangle::fill_with([386.0, 788.0], color::rgba(0.0, 0.0, 0.0, 0.8))
                    .down_from(self.ids.server_frame_bg, 20.0)
                    .set(self.ids.charlist_bg, ui_widgets);
                Image::new(self.imgs.charlist_frame)
                    .w_h(400.0, 800.0)
                    .middle_of(self.ids.charlist_bg)
                    .color(Some(UI_MAIN))
                    .set(self.ids.charlist_frame, ui_widgets);
                Rectangle::fill_with([386.0, 783.0], color::TRANSPARENT)
                    .middle_of(self.ids.charlist_bg)
                    .scroll_kids()
                    .scroll_kids_vertically()
                    .set(self.ids.charlist_alignment, ui_widgets);
                Scrollbar::y_axis(self.ids.charlist_alignment)
                    .thickness(5.0)
                    .auto_hide(true)
                    .color(UI_MAIN)
                    .set(self.ids.selection_scrollbar, ui_widgets);
                // Server Name
                Text::new(&client.server_info.name)
                    .mid_top_with_margin_on(self.ids.server_frame_bg, 5.0)
                    .font_size(self.fonts.cyri.scale(26))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(TEXT_COLOR)
                    .set(self.ids.server_name_text, ui_widgets);
                //Change Server
                if Button::image(self.imgs.button)
                    .mid_top_with_margin_on(self.ids.server_frame_bg, 45.0)
                    .w_h(200.0, 40.0)
                    .parent(self.ids.charlist_bg)
                    .hover_image(self.imgs.button_hover)
                    .press_image(self.imgs.button_press)
                    .label(&self.voxygen_i18n.get("char_selection.change_server"))
                    .label_color(TEXT_COLOR)
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .label_font_size(self.fonts.cyri.scale(18))
                    .label_y(conrod_core::position::Relative::Scalar(3.0))
                    .set(self.ids.change_server, ui_widgets)
                    .was_clicked()
                {
                    events.push(Event::Logout);
                }

                // Enter World Button
                let character_count = client.character_list.characters.len();
                let enter_world_str = &self.voxygen_i18n.get("char_selection.enter_world");
                let enter_button = Button::image(self.imgs.button)
                    .mid_bottom_with_margin_on(ui_widgets.window, 10.0)
                    .w_h(250.0, 60.0)
                    .label(enter_world_str)
                    .label_font_size(self.fonts.cyri.scale(26))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .label_y(conrod_core::position::Relative::Scalar(3.0));

                if can_enter_world {
                    if enter_button
                        .hover_image(self.imgs.button_hover)
                        .press_image(self.imgs.button_press)
                        .label_color(TEXT_COLOR)
                        .set(self.ids.enter_world_button, ui_widgets)
                        .was_clicked()
                    {
                        events.push(Event::Play);
                    }
                } else {
                    &enter_button
                        .label_color(TEXT_COLOR_2)
                        .set(self.ids.enter_world_button, ui_widgets);
                }

                // Logout_Button
                if Button::image(self.imgs.button)
                    .bottom_left_with_margins_on(ui_widgets.window, 10.0, 10.0)
                    .w_h(150.0, 40.0)
                    .hover_image(self.imgs.button_hover)
                    .press_image(self.imgs.button_press)
                    .label(&self.voxygen_i18n.get("char_selection.logout"))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(20))
                    .label_y(conrod_core::position::Relative::Scalar(3.0))
                    .set(self.ids.logout_button, ui_widgets)
                    .was_clicked()
                {
                    events.push(Event::Logout);
                }

                // Alpha Version
                Text::new(&version)
                    .top_right_with_margins_on(ui_widgets.window, 5.0, 5.0)
                    .font_size(self.fonts.cyri.scale(14))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(TEXT_COLOR)
                    .set(self.ids.version, ui_widgets);

                // Resize character selection widgets
                self.ids
                    .character_boxes
                    .resize(character_count, &mut ui_widgets.widget_id_generator());
                self.ids
                    .character_deletes
                    .resize(character_count, &mut ui_widgets.widget_id_generator());
                self.ids
                    .character_names
                    .resize(character_count, &mut ui_widgets.widget_id_generator());
                self.ids
                    .character_levels
                    .resize(character_count, &mut ui_widgets.widget_id_generator());
                self.ids
                    .character_locations
                    .resize(character_count, &mut ui_widgets.widget_id_generator());

                // Character selection
                for (i, character_item) in client.character_list.characters.iter().enumerate() {
                    let character_box = Button::image(if self.selected_character == i {
                        self.imgs.selection_hover
                    } else {
                        self.imgs.selection
                    });
                    let character_box = if i == 0 {
                        character_box.top_left_with_margins_on(
                            self.ids.charlist_alignment,
                            0.0,
                            2.0,
                        )
                    } else {
                        character_box.down_from(self.ids.character_boxes[i - 1], 5.0)
                    };
                    if character_box
                        .w_h(386.0, 80.0)
                        .image_color(Color::Rgba(1.0, 1.0, 1.0, 0.8))
                        .hover_image(self.imgs.selection_hover)
                        .press_image(self.imgs.selection_press)
                        .label_font_id(self.fonts.cyri.conrod_id)
                        .label_y(conrod_core::position::Relative::Scalar(20.0))
                        .set(self.ids.character_boxes[i], ui_widgets)
                        .was_clicked()
                    {
                        self.selected_character = i;
                    }
                    if Button::image(self.imgs.delete_button)
                        .w_h(30.0 * 0.5, 30.0 * 0.5)
                        .top_right_with_margins_on(self.ids.character_boxes[i], 15.0, 15.0)
                        .hover_image(self.imgs.delete_button_hover)
                        .press_image(self.imgs.delete_button_press)
                        .with_tooltip(
                            tooltip_manager,
                            &self.voxygen_i18n.get("char_selection.delete_permanently"),
                            "",
                            &tooltip_human,
                        )
                        .set(self.ids.character_deletes[i], ui_widgets)
                        .was_clicked()
                    {
                        self.info_content = InfoContent::Deletion(i);
                    }
                    Text::new(&character_item.character.alias)
                        .top_left_with_margins_on(self.ids.character_boxes[i], 6.0, 9.0)
                        .font_size(self.fonts.cyri.scale(19))
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(TEXT_COLOR)
                        .set(self.ids.character_names[i], ui_widgets);

                    Text::new(
                        &self
                            .voxygen_i18n
                            .get("char_selection.level_fmt")
                            .replace("{level_nb}", "1"),
                    ) //TODO Insert real level here as soon as they get saved
                    .down_from(self.ids.character_names[i], 4.0)
                    .font_size(self.fonts.cyri.scale(17))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(TEXT_COLOR)
                    .set(self.ids.character_levels[i], ui_widgets);

                    Text::new(&self.voxygen_i18n.get("char_selection.uncanny_valley"))
                        .down_from(self.ids.character_levels[i], 4.0)
                        .font_size(self.fonts.cyri.scale(17))
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(TEXT_COLOR)
                        .set(self.ids.character_locations[i], ui_widgets);
                }

                // Create Character Button
                let create_char_button = Button::image(self.imgs.selection);

                let create_char_button = if character_count > 0 {
                    create_char_button.down_from(self.ids.character_boxes[character_count - 1], 5.0)
                } else {
                    create_char_button.top_left_with_margins_on(
                        self.ids.charlist_alignment,
                        0.0,
                        2.0,
                    )
                };

                let character_limit_reached = character_count >= MAX_CHARACTERS_PER_PLAYER;

                let color = if character_limit_reached {
                    Color::Rgba(0.38, 0.38, 0.10, 1.0)
                } else {
                    Color::Rgba(0.38, 1.0, 0.07, 1.0)
                };

                if create_char_button
                    .w_h(386.0, 80.0)
                    .hover_image(self.imgs.selection_hover)
                    .press_image(self.imgs.selection_press)
                    .label(&self.voxygen_i18n.get("char_selection.create_new_charater"))
                    .label_color(color)
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .image_color(color)
                    .set(self.ids.character_box_2, ui_widgets)
                    .was_clicked()
                {
                    if !character_limit_reached {
                        self.mode = Mode::Create {
                            name: "Character Name".to_string(),
                            body: humanoid::Body::random(),
                            loadout: comp::Loadout::default(),
                            tool: Some(STARTER_SWORD),
                        };
                    }
                }
            },
            // Character_Creation
            // //////////////////////////////////////////////////////////////////////
            Mode::Create {
                name,
                body,
                loadout: _,
                tool,
            } => {
                let mut to_select = false;
                // Back Button
                if Button::image(self.imgs.button)
                    .bottom_left_with_margins_on(ui_widgets.window, 10.0, 10.0)
                    .w_h(150.0, 40.0)
                    .hover_image(self.imgs.button_hover)
                    .press_image(self.imgs.button_press)
                    .label(&self.voxygen_i18n.get("common.back"))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .label_color(TEXT_COLOR)
                    .label_font_size(self.fonts.cyri.scale(20))
                    .label_y(conrod_core::position::Relative::Scalar(3.0))
                    .set(self.ids.back_button, ui_widgets)
                    .was_clicked()
                {
                    to_select = true;
                }
                // Create Button
                if Button::image(self.imgs.button)
                    .bottom_right_with_margins_on(ui_widgets.window, 10.0, 10.0)
                    .w_h(150.0, 40.0)
                    .hover_image(self.imgs.button_hover)
                    .press_image(self.imgs.button_press)
                    .label(&self.voxygen_i18n.get("common.create"))
                    .label_font_id(self.fonts.cyri.conrod_id)
                    .label_color(
                        /* if self.mode { TEXT_COLOR } else { */ TEXT_COLOR, /* ,  } */
                    )
                    .label_font_size(self.fonts.cyri.scale(20))
                    .label_y(conrod_core::position::Relative::Scalar(3.0))
                    .set(self.ids.create_button, ui_widgets)
                    .was_clicked()
                {
                    self.info_content = InfoContent::CreatingCharacter;

                    events.push(Event::AddCharacter {
                        alias: name.clone(),
                        tool: tool.map(|tool| tool.to_string()),
                        body: comp::Body::Humanoid(body.clone()),
                    });

                    to_select = true;
                }
                // Character Name Input
                Rectangle::fill_with([320.0, 50.0], color::rgba(0.0, 0.0, 0.0, 0.97))
                    .mid_bottom_with_margin_on(ui_widgets.window, 20.0)
                    .set(self.ids.name_input_bg, ui_widgets);
                Button::image(self.imgs.name_input)
                    .image_color(Color::Rgba(1.0, 1.0, 1.0, 0.9))
                    .w_h(337.0, 67.0)
                    .middle_of(self.ids.name_input_bg)
                    .set(self.ids.name_input, ui_widgets);
                for event in TextBox::new(name)
                    .w_h(300.0, 60.0)
                    .mid_top_with_margin_on(self.ids.name_input, 2.0)
                    .font_size(self.fonts.cyri.scale(26))
                    .font_id(self.fonts.cyri.conrod_id)
                    .center_justify()
                    .text_color(TEXT_COLOR)
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(TRANSPARENT)
                    .border_color(TRANSPARENT)
                    .set(self.ids.name_field, ui_widgets)
                {
                    match event {
                        TextBoxEvent::Update(new_name) => *name = new_name,
                        TextBoxEvent::Enter => {},
                    }
                }

                // Window
                Rectangle::fill_with(
                    [386.0, ui_widgets.win_h - ui_widgets.win_h * 0.2],
                    color::rgba(0.0, 0.0, 0.0, 0.8),
                )
                .top_left_with_margins_on(ui_widgets.window, 30.0, 30.0)
                .set(self.ids.creation_bg, ui_widgets);
                Image::new(self.imgs.charlist_frame)
                    .w_h(400.0, ui_widgets.win_h - ui_widgets.win_h * 0.19)
                    .middle_of(self.ids.creation_bg)
                    .color(Some(UI_MAIN))
                    .set(self.ids.charlist_frame, ui_widgets);
                Rectangle::fill_with(
                    [386.0, ui_widgets.win_h - ui_widgets.win_h * 0.19],
                    color::TRANSPARENT,
                )
                .middle_of(self.ids.creation_bg)
                .scroll_kids_vertically()
                .set(self.ids.creation_alignment, ui_widgets);
                Scrollbar::y_axis(self.ids.creation_alignment)
                    .thickness(5.0)
                    .auto_hide(true)
                    .rgba(0.33, 0.33, 0.33, 1.0)
                    .set(self.ids.selection_scrollbar, ui_widgets);

                // Male/Female/Race Icons
                Text::new(&self.voxygen_i18n.get("char_selection.character_creation"))
                    .mid_top_with_margin_on(self.ids.creation_alignment, 10.0)
                    .font_size(self.fonts.cyri.scale(24))
                    .font_id(self.fonts.cyri.conrod_id)
                    .color(TEXT_COLOR)
                    .set(self.ids.bodyrace_text, ui_widgets);
                // Alignment
                Rectangle::fill_with([140.0, 72.0], color::TRANSPARENT)
                    .mid_top_with_margin_on(self.ids.creation_alignment, 60.0)
                    .set(self.ids.creation_buttons_alignment_1, ui_widgets);
                // Male
                Image::new(self.imgs.male)
                    .w_h(70.0, 70.0)
                    .top_left_with_margins_on(self.ids.creation_buttons_alignment_1, 0.0, 0.0)
                    .set(self.ids.male, ui_widgets);
                if Button::image(if let humanoid::BodyType::Male = body.body_type {
                    self.imgs.icon_border_pressed
                } else {
                    self.imgs.icon_border
                })
                .middle_of(self.ids.male)
                .hover_image(self.imgs.icon_border_mo)
                .press_image(self.imgs.icon_border_press)
                .set(self.ids.body_type_1, ui_widgets)
                .was_clicked()
                {
                    body.body_type = humanoid::BodyType::Male;
                    body.validate();
                }
                // Female
                Image::new(self.imgs.female)
                    .w_h(70.0, 70.0)
                    .top_right_with_margins_on(self.ids.creation_buttons_alignment_1, 0.0, 0.0)
                    .set(self.ids.female, ui_widgets);
                if Button::image(if let humanoid::BodyType::Female = body.body_type {
                    self.imgs.icon_border_pressed
                } else {
                    self.imgs.icon_border
                })
                .middle_of(self.ids.female)
                .hover_image(self.imgs.icon_border_mo)
                .press_image(self.imgs.icon_border_press)
                .set(self.ids.body_type_2, ui_widgets)
                .was_clicked()
                {
                    body.body_type = humanoid::BodyType::Female;
                    body.validate();
                }

                // Alignment for Races and Tools
                Rectangle::fill_with([214.0, 304.0], color::TRANSPARENT)
                    .mid_bottom_with_margin_on(self.ids.creation_buttons_alignment_1, -324.0)
                    .set(self.ids.creation_buttons_alignment_2, ui_widgets);

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
                // Human
                Image::new(human_icon)
                    .w_h(70.0, 70.0)
                    .top_left_with_margins_on(self.ids.creation_buttons_alignment_2, 0.0, 0.0)
                    .set(self.ids.human, ui_widgets);
                if Button::image(if let humanoid::Race::Human = body.race {
                    self.imgs.icon_border_pressed
                } else {
                    self.imgs.icon_border
                })
                .middle_of(self.ids.human)
                .hover_image(self.imgs.icon_border_mo)
                .press_image(self.imgs.icon_border_press)
                .with_tooltip(
                    tooltip_manager,
                    &self.voxygen_i18n.get("common.races.human"),
                    "",
                    &tooltip_human,
                )
                .set(self.ids.race_1, ui_widgets)
                .was_clicked()
                {
                    body.race = humanoid::Race::Human;
                    body.validate();
                }

                // Orc
                Image::new(orc_icon)
                    .w_h(70.0, 70.0)
                    .right_from(self.ids.human, 2.0)
                    .set(self.ids.orc, ui_widgets);
                if Button::image(if let humanoid::Race::Orc = body.race {
                    self.imgs.icon_border_pressed
                } else {
                    self.imgs.icon_border
                })
                .middle_of(self.ids.orc)
                .hover_image(self.imgs.icon_border_mo)
                .press_image(self.imgs.icon_border_press)
                .with_tooltip(
                    tooltip_manager,
                    &self.voxygen_i18n.get("common.races.orc"),
                    "",
                    &tooltip_human,
                )
                .set(self.ids.race_2, ui_widgets)
                .was_clicked()
                {
                    body.race = humanoid::Race::Orc;
                    body.validate();
                }
                // Dwarf
                Image::new(dwarf_icon)
                    .w_h(70.0, 70.0)
                    .right_from(self.ids.orc, 2.0)
                    .set(self.ids.dwarf, ui_widgets);
                if Button::image(if let humanoid::Race::Dwarf = body.race {
                    self.imgs.icon_border_pressed
                } else {
                    self.imgs.icon_border
                })
                .middle_of(self.ids.dwarf)
                .hover_image(self.imgs.icon_border_mo)
                .press_image(self.imgs.icon_border_press)
                .with_tooltip(
                    tooltip_manager,
                    &self.voxygen_i18n.get("common.races.dwarf"),
                    "",
                    &tooltip_human,
                )
                .set(self.ids.race_3, ui_widgets)
                .was_clicked()
                {
                    body.race = humanoid::Race::Dwarf;
                    body.validate();
                }
                // Elf
                Image::new(elf_icon)
                    .w_h(70.0, 70.0)
                    .down_from(self.ids.human, 2.0)
                    .set(self.ids.elf, ui_widgets);
                if Button::image(if let humanoid::Race::Elf = body.race {
                    self.imgs.icon_border_pressed
                } else {
                    self.imgs.icon_border
                })
                .middle_of(self.ids.elf)
                .hover_image(self.imgs.icon_border_mo)
                .press_image(self.imgs.icon_border_press)
                .with_tooltip(
                    tooltip_manager,
                    &self.voxygen_i18n.get("common.races.elf"),
                    "",
                    &tooltip_human,
                )
                .set(self.ids.race_4, ui_widgets)
                .was_clicked()
                {
                    body.race = humanoid::Race::Elf;
                    body.validate();
                }

                // Undead
                Image::new(undead_icon)
                    .w_h(70.0, 70.0)
                    .right_from(self.ids.elf, 2.0)
                    .set(self.ids.undead, ui_widgets);
                if Button::image(if let humanoid::Race::Undead = body.race {
                    self.imgs.icon_border_pressed
                } else {
                    self.imgs.icon_border
                })
                .middle_of(self.ids.undead)
                .hover_image(self.imgs.icon_border_mo)
                .press_image(self.imgs.icon_border_press)
                .with_tooltip(
                    tooltip_manager,
                    &self.voxygen_i18n.get("common.races.undead"),
                    "",
                    &tooltip_human,
                )
                .set(self.ids.race_5, ui_widgets)
                .was_clicked()
                {
                    body.race = humanoid::Race::Undead;
                    body.validate();
                }
                // Danari
                Image::new(danari_icon)
                    .w_h(70.0, 70.0)
                    .right_from(self.ids.undead, 2.0)
                    .set(self.ids.danari, ui_widgets);
                if Button::image(if let humanoid::Race::Danari = body.race {
                    self.imgs.icon_border_pressed
                } else {
                    self.imgs.icon_border
                })
                .middle_of(self.ids.danari)
                .hover_image(self.imgs.icon_border_mo)
                .press_image(self.imgs.icon_border_press)
                .with_tooltip(
                    tooltip_manager,
                    &self.voxygen_i18n.get("common.races.danari"),
                    "",
                    &tooltip_human,
                )
                .set(self.ids.race_6, ui_widgets)
                .was_clicked()
                {
                    body.race = humanoid::Race::Danari;
                    body.validate();
                }

                // Hammer
                Image::new(self.imgs.hammer)
                    .w_h(70.0, 70.0)
                    .bottom_left_with_margins_on(self.ids.creation_buttons_alignment_2, 0.0, 0.0)
                    .set(self.ids.hammer, ui_widgets);
                if Button::image(if let Some(STARTER_HAMMER) = tool {
                    self.imgs.icon_border_pressed
                } else {
                    self.imgs.icon_border
                })
                .middle_of(self.ids.hammer)
                .hover_image(self.imgs.icon_border_mo)
                .press_image(self.imgs.icon_border_press)
                .with_tooltip(
                    tooltip_manager,
                    &self.voxygen_i18n.get("common.weapons.hammer"),
                    "",
                    &tooltip_human,
                )
                .set(self.ids.hammer_button, ui_widgets)
                .was_clicked()
                {
                    *tool = Some(STARTER_HAMMER);
                }

                // Bow
                Image::new(self.imgs.bow)
                    .w_h(70.0, 70.0)
                    .right_from(self.ids.hammer, 2.0)
                    .set(self.ids.bow, ui_widgets);
                if Button::image(if let Some(STARTER_BOW) = tool {
                    self.imgs.icon_border_pressed
                } else {
                    self.imgs.icon_border
                })
                .middle_of(self.ids.bow)
                .hover_image(self.imgs.icon_border_mo)
                .press_image(self.imgs.icon_border_press)
                .with_tooltip(
                    tooltip_manager,
                    &self.voxygen_i18n.get("common.weapons.bow"),
                    "",
                    &tooltip_human,
                )
                .set(self.ids.bow_button, ui_widgets)
                .was_clicked()
                {
                    *tool = Some(STARTER_BOW);
                }
                // Staff
                Image::new(self.imgs.staff)
                    .w_h(70.0, 70.0)
                    .right_from(self.ids.bow, 2.0)
                    .set(self.ids.staff, ui_widgets);
                if Button::image(if let Some(STARTER_STAFF) = tool {
                    self.imgs.icon_border_pressed
                } else {
                    self.imgs.icon_border
                })
                .middle_of(self.ids.staff)
                .hover_image(self.imgs.icon_border_mo)
                .press_image(self.imgs.icon_border_press)
                .with_tooltip(
                    tooltip_manager,
                    &self.voxygen_i18n.get("common.weapons.staff"),
                    "",
                    &tooltip_human,
                )
                .set(self.ids.staff_button, ui_widgets)
                .was_clicked()
                {
                    *tool = Some(STARTER_STAFF);
                }
                // Sword
                Image::new(self.imgs.sword)
                    .w_h(70.0, 70.0)
                    .up_from(self.ids.hammer, 2.0)
                    .set(self.ids.sword, ui_widgets);
                if Button::image(if let Some(STARTER_SWORD) = tool {
                    self.imgs.icon_border_pressed
                } else {
                    self.imgs.icon_border
                })
                .middle_of(self.ids.sword)
                .hover_image(self.imgs.icon_border_mo)
                .press_image(self.imgs.icon_border_press)
                .with_tooltip(
                    tooltip_manager,
                    &self.voxygen_i18n.get("common.weapons.sword"),
                    "",
                    &tooltip_human,
                )
                .set(self.ids.sword_button, ui_widgets)
                .was_clicked()
                {
                    *tool = Some(STARTER_SWORD);
                }

                // Daggers
                Image::new(self.imgs.daggers)
                    .w_h(70.0, 70.0)
                    .right_from(self.ids.sword, 2.0)
                    .set(self.ids.daggers, ui_widgets);
                if Button::image(if let Some(STARTER_DAGGER) = tool {
                    self.imgs.icon_border_pressed
                } else {
                    self.imgs.icon_border
                })
                .middle_of(self.ids.daggers)
                //.hover_image(self.imgs.icon_border_mo)
                //.press_image(self.imgs.icon_border_press)
                //.with_tooltip(tooltip_manager, "Daggers", "", &tooltip_human)
                .set(self.ids.daggers_button, ui_widgets)
                .was_clicked()
                {
                    // self.character_tool = Some(STARTER_DAGGER);
                } // REMOVE THIS AFTER IMPLEMENTATION
                Rectangle::fill_with([67.0, 67.0], color::rgba(0.0, 0.0, 0.0, 0.8))
                    .middle_of(self.ids.daggers)
                    .set(self.ids.daggers_grey, ui_widgets);

                // Axe
                Image::new(self.imgs.axe)
                    .w_h(70.0, 70.0)
                    .right_from(self.ids.daggers, 2.0)
                    .set(self.ids.axe, ui_widgets);
                if Button::image(if let Some(STARTER_AXE) = tool {
                    self.imgs.icon_border_pressed
                } else {
                    self.imgs.icon_border
                })
                .middle_of(self.ids.axe)
                .hover_image(self.imgs.icon_border_mo)
                .press_image(self.imgs.icon_border_press)
                .with_tooltip(
                    tooltip_manager,
                    &self.voxygen_i18n.get("common.weapons.axe"),
                    "",
                    &tooltip_human,
                )
                .set(self.ids.axe_button, ui_widgets)
                .was_clicked()
                {
                    *tool = Some(STARTER_AXE);
                }
                // Sliders
                let (cyri, cyri_size, slider_indicator, slider_range) = (
                    self.fonts.cyri.conrod_id,
                    self.fonts.cyri.scale(18),
                    self.imgs.slider_indicator,
                    self.imgs.slider_range,
                );
                let char_slider = move |prev_id,
                                        text: &str,
                                        text_id,
                                        max,
                                        selected_val,
                                        slider_id,
                                        ui_widgets: &mut UiCell| {
                    Text::new(text)
                        .down_from(prev_id, 22.0)
                        .align_middle_x_of(prev_id)
                        .font_size(cyri_size)
                        .font_id(cyri)
                        .color(TEXT_COLOR)
                        .set(text_id, ui_widgets);
                    ImageSlider::discrete(selected_val, 0, max, slider_indicator, slider_range)
                        .w_h(208.0, 22.0)
                        .down_from(text_id, 8.0)
                        .align_middle_x()
                        .track_breadth(12.0)
                        .slider_length(10.0)
                        .pad_track((5.0, 5.0))
                        .set(slider_id, ui_widgets)
                };
                // Hair Style
                if let Some(new_val) = char_slider(
                    self.ids.creation_buttons_alignment_2,
                    self.voxygen_i18n.get("char_selection.hair_style"),
                    self.ids.hairstyle_text,
                    body.race.num_hair_styles(body.body_type) as usize - 1,
                    body.hair_style as usize,
                    self.ids.hairstyle_slider,
                    ui_widgets,
                ) {
                    body.hair_style = new_val as u8;
                }
                // Hair Color
                if let Some(new_val) = char_slider(
                    self.ids.hairstyle_slider,
                    self.voxygen_i18n.get("char_selection.hair_color"),
                    self.ids.haircolor_text,
                    body.race.num_hair_colors() as usize - 1,
                    body.hair_color as usize,
                    self.ids.haircolor_slider,
                    ui_widgets,
                ) {
                    body.hair_color = new_val as u8;
                }
                // Skin
                if let Some(new_val) = char_slider(
                    self.ids.haircolor_slider,
                    self.voxygen_i18n.get("char_selection.skin"),
                    self.ids.skin_text,
                    body.race.num_skin_colors() as usize - 1,
                    body.skin as usize,
                    self.ids.skin_slider,
                    ui_widgets,
                ) {
                    body.skin = new_val as u8;
                }
                // Eyebrows
                if let Some(new_val) = char_slider(
                    self.ids.skin_slider,
                    self.voxygen_i18n.get("char_selection.eyebrows"),
                    self.ids.eyebrows_text,
                    body.race.num_eyebrows(body.body_type) as usize - 1,
                    body.eyebrows as usize,
                    self.ids.eyebrows_slider,
                    ui_widgets,
                ) {
                    body.eyebrows = new_val as u8;
                }
                // EyeColor
                if let Some(new_val) = char_slider(
                    self.ids.eyebrows_slider,
                    self.voxygen_i18n.get("char_selection.eye_color"),
                    self.ids.eyecolor_text,
                    body.race.num_eye_colors() as usize - 1,
                    body.eye_color as usize,
                    self.ids.eyecolor_slider,
                    ui_widgets,
                ) {
                    body.eye_color = new_val as u8;
                }
                // Accessories
                let _current_accessory = body.accessory;
                if let Some(new_val) = char_slider(
                    self.ids.eyecolor_slider,
                    self.voxygen_i18n.get("char_selection.accessories"),
                    self.ids.accessories_text,
                    body.race.num_accessories(body.body_type) as usize - 1,
                    body.accessory as usize,
                    self.ids.accessories_slider,
                    ui_widgets,
                ) {
                    body.accessory = new_val as u8;
                }
                // Beard
                if body.race.num_beards(body.body_type) > 1 {
                    if let Some(new_val) = char_slider(
                        self.ids.accessories_slider,
                        self.voxygen_i18n.get("char_selection.beard"),
                        self.ids.beard_text,
                        body.race.num_beards(body.body_type) as usize - 1,
                        body.beard as usize,
                        self.ids.beard_slider,
                        ui_widgets,
                    ) {
                        body.beard = new_val as u8;
                    }
                } else {
                    Text::new(&self.voxygen_i18n.get("char_selection.beard"))
                        .mid_bottom_with_margin_on(self.ids.accessories_slider, -40.0)
                        .font_size(self.fonts.cyri.scale(18))
                        .font_id(self.fonts.cyri.conrod_id)
                        .color(TEXT_COLOR_2)
                        .set(self.ids.beard_text, ui_widgets);
                    ImageSlider::discrete(5, 0, 10, self.imgs.nothing, self.imgs.slider_range)
                        .w_h(208.0, 22.0)
                        .mid_bottom_with_margin_on(self.ids.beard_text, -30.0)
                        .track_breadth(12.0)
                        .slider_length(10.0)
                        .track_color(Color::Rgba(1.0, 1.0, 1.0, 0.2))
                        .slider_color(Color::Rgba(1.0, 1.0, 1.0, 0.2))
                        .pad_track((5.0, 5.0))
                        .set(self.ids.beard_slider, ui_widgets);
                }
                // Chest
                /*let armor = load_glob::<comp::Item>("common.items.armor.chest.*")
                    .expect("Unable to load armor!");
                if let Some(new_val) = char_slider(
                    self.ids.beard_slider,
                    self.voxygen_i18n.get("char_selection.chest_color"),
                    self.ids.chest_text,
                    armor.len() - 1,
                    armor
                        .iter()
                        .position(|c| {
                            loadout
                                .chest
                                .as_ref()
                                .map(|lc| lc == c.borrow())
                                .unwrap_or_default()
                        })
                        .unwrap_or(0),
                    self.ids.chest_slider,
                    ui_widgets,
                ) {
                    loadout.chest = Some((*armor[new_val]).clone());
                }*/
                // Pants
                /*let current_pants = body.pants;
                if let Some(new_val) = char_slider(
                    self.ids.chest_slider,
                    "Pants",
                    self.ids.pants_text,
                    humanoid::ALL_PANTS.len() - 1,
                    humanoid::ALL_PANTS
                        .iter()
                        .position(|&c| c == current_pants)
                        .unwrap_or(0),
                    self.ids.pants_slider,
                    ui_widgets,
                ) {
                    body.pants = humanoid::ALL_PANTS[new_val];
                }*/
                Rectangle::fill_with([20.0, 20.0], color::TRANSPARENT)
                    .down_from(self.ids.beard_slider, 15.0)
                    .set(self.ids.space, ui_widgets);

                if to_select {
                    self.mode = Mode::Select(None);
                }
            }, // Char Creation fin
        }

        events
    }

    pub fn handle_event(&mut self, event: WinEvent) -> bool {
        match event {
            WinEvent::Ui(event) => {
                self.ui.handle_event(event);
                true
            },
            WinEvent::MouseButton(_, PressState::Pressed) => !self.ui.no_widget_capturing_mouse(),
            _ => false,
        }
    }

    pub fn maintain(&mut self, global_state: &mut GlobalState, client: &mut Client) -> Vec<Event> {
        let events = self.update_layout(client);
        self.ui.maintain(global_state.window.renderer_mut(), None);
        events
    }

    pub fn render(&self, renderer: &mut Renderer, globals: &Consts<Globals>) {
        self.ui.render(renderer, Some(globals));
    }
}
