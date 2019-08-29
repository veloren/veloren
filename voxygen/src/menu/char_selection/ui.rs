use crate::{
    render::{Consts, Globals, Renderer},
    ui::{
        self,
        img_ids::{BlankGraphic, ImageGraphic, VoxelGraphic},
        ImageSlider, Ui,
    },
    GlobalState,
};
use client::Client;
use common::comp::{humanoid, item::Tool};
use conrod_core::{
    color,
    color::TRANSPARENT,
    widget::{text_box::Event as TextBoxEvent, Button, Image, Rectangle, Scrollbar, Text, TextBox},
    widget_ids, Borderable, Color, Colorable, Labelable, Positionable, Sizeable, UiCell, Widget,
};

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

        // REMOVE THIS AFTER IMPLEMENTATION
        daggers_grey,
        axe_grey,
        hammer_grey,
        bow_grey,
        staff_grey,


        // Characters
        character_box_1,
        character_name_1,
        character_location_1,
        character_level_1,

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
        beard_slider_2,
        beard_text,
        accessories_slider,
        accessories_text,

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
        // Body Features
        chest_slider,
    }
}

image_ids! {
    struct Imgs {
        <VoxelGraphic>
        button: "voxygen.element.buttons.button",
        button_hover: "voxygen.element.buttons.button_hover",
        button_press: "voxygen.element.buttons.button_press",
        name_input: "voxygen.element.misc_bg.textbox",
        charlist_frame: "voxygen.element.frames.window_4",
        server_frame: "voxygen.element.frames.server_frame",
        selection: "voxygen.element.frames.selection",
        slider_range: "voxygen.element.slider.track",
        slider_indicator: "voxygen.element.slider.indicator",

        <ImageGraphic>

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

        <BlankGraphic>
        nothing: (),
    }
}

font_ids! {
    pub struct Fonts {
        opensans: "voxygen.font.OpenSans-Regular",
        metamorph: "voxygen.font.Metamorphous-Regular",
    }
}

pub enum Event {
    Logout,
    Play,
}

const TEXT_COLOR: Color = Color::Rgba(1.0, 1.0, 1.0, 1.0);
const TEXT_COLOR_2: Color = Color::Rgba(1.0, 1.0, 1.0, 0.2);

pub struct CharSelectionUi {
    ui: Ui,
    ids: Ids,
    imgs: Imgs,
    fonts: Fonts,
    character_creation: bool,
    pub character_name: String,
    pub character_body: humanoid::Body,
    pub character_tool: Option<Tool>,
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
        // Load fonts
        let fonts = Fonts::load(&mut ui).expect("Failed to load fonts!");

        // TODO: Randomize initial values.
        Self {
            ui,
            ids,
            imgs,
            fonts,
            character_creation: false,
            character_name: "Character Name".to_string(),
            character_body: humanoid::Body::random(),
            character_tool: Some(Tool::Sword),
        }
    }

    // TODO: Split this into multiple modules or functions.
    fn update_layout(&mut self, client: &Client) -> Vec<Event> {
        let mut events = Vec::new();
        let ref mut ui_widgets = self.ui.set_widgets().0;
        let version = env!("CARGO_PKG_VERSION");

        // Character Selection /////////////////
        if !self.character_creation {
            // Background for Server Frame
            Rectangle::fill_with([386.0, 95.0], color::rgba(0.0, 0.0, 0.0, 0.8))
                .top_left_with_margins_on(ui_widgets.window, 30.0, 30.0)
                .set(self.ids.server_frame_bg, ui_widgets);
            Image::new(self.imgs.server_frame)
                .w_h(400.0, 100.0)
                .middle_of(self.ids.server_frame_bg)
                .set(self.ids.server_frame, ui_widgets);

            // Background for Char List
            Rectangle::fill_with([386.0, 788.0], color::rgba(0.0, 0.0, 0.0, 0.8))
                .down_from(self.ids.server_frame_bg, 20.0)
                .set(self.ids.charlist_bg, ui_widgets);
            Image::new(self.imgs.charlist_frame)
                .w_h(400.0, 800.0)
                .middle_of(self.ids.charlist_bg)
                .set(self.ids.charlist_frame, ui_widgets);
            Rectangle::fill_with([386.0, 783.0], color::TRANSPARENT)
                .middle_of(self.ids.charlist_bg)
                .scroll_kids()
                .scroll_kids_vertically()
                .set(self.ids.charlist_alignment, ui_widgets);
            Scrollbar::y_axis(self.ids.charlist_alignment)
                .thickness(5.0)
                .auto_hide(true)
                .rgba(0.0, 0.0, 0., 0.0)
                .set(self.ids.selection_scrollbar, ui_widgets);
            // Server Name
            Text::new(&client.server_info.name)
                .mid_top_with_margin_on(self.ids.server_frame_bg, 5.0)
                .font_size(24)
                .font_id(self.fonts.metamorph)
                .color(TEXT_COLOR)
                .set(self.ids.server_name_text, ui_widgets);
            //Change Server
            if Button::image(self.imgs.button)
                .mid_top_with_margin_on(self.ids.server_frame_bg, 45.0)
                .w_h(200.0, 40.0)
                .parent(self.ids.charlist_bg)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .label("Change Server")
                .label_color(TEXT_COLOR)
                .label_font_size(18)
                .label_y(conrod_core::position::Relative::Scalar(3.0))
                .set(self.ids.change_server, ui_widgets)
                .was_clicked()
            {
                events.push(Event::Logout);
            }

            // Enter World Button
            if Button::image(self.imgs.button)
                .mid_bottom_with_margin_on(ui_widgets.window, 10.0)
                .w_h(250.0, 60.0)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .label("Enter World")
                .label_color(TEXT_COLOR)
                .label_font_size(22)
                .label_y(conrod_core::position::Relative::Scalar(3.0))
                .set(self.ids.enter_world_button, ui_widgets)
                .was_clicked()
            {
                events.push(Event::Play);
            }

            // Logout_Button
            if Button::image(self.imgs.button)
                .bottom_left_with_margins_on(ui_widgets.window, 10.0, 10.0)
                .w_h(150.0, 40.0)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .label("Logout")
                .label_color(TEXT_COLOR)
                .label_font_size(18)
                .label_y(conrod_core::position::Relative::Scalar(3.0))
                .set(self.ids.logout_button, ui_widgets)
                .was_clicked()
            {
                events.push(Event::Logout);
            }

            // Create Character Button.
            if Button::image(self.imgs.button)
                .mid_bottom_with_margin_on(self.ids.charlist_bg, -60.0)
                .w_h(270.0, 50.0)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .label("Create Character")
                .label_color(TEXT_COLOR)
                .label_font_size(20)
                .label_y(conrod_core::position::Relative::Scalar(3.0))
                .set(self.ids.create_character_button, ui_widgets)
                .was_clicked()
            {
                self.character_creation = true;
                self.character_tool = Some(Tool::Sword);
            }

            // Alpha Version
            Text::new(version)
                .top_right_with_margins_on(ui_widgets.window, 5.0, 5.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(self.ids.version, ui_widgets);

            // 1st Character in Selection List
            if Button::image(self.imgs.selection)
                .top_left_with_margins_on(self.ids.charlist_alignment, 0.0, 2.0)
                .w_h(386.0, 80.0)
                .image_color(Color::Rgba(1.0, 1.0, 1.0, 0.8))
                .hover_image(self.imgs.selection)
                .press_image(self.imgs.selection)
                .label_y(conrod_core::position::Relative::Scalar(20.0))
                .set(self.ids.character_box_1, ui_widgets)
                .was_clicked()
            {}
            Text::new("Human Default")
                .top_left_with_margins_on(self.ids.character_box_1, 6.0, 9.0)
                .font_size(19)
                .font_id(self.fonts.metamorph)
                .color(TEXT_COLOR)
                .set(self.ids.character_name_1, ui_widgets);

            Text::new("Level 1")
                .down_from(self.ids.character_name_1, 4.0)
                .font_size(17)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(self.ids.character_level_1, ui_widgets);

            Text::new("Uncanny Valley")
                .down_from(self.ids.character_level_1, 4.0)
                .font_size(17)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(self.ids.character_location_1, ui_widgets);

            // 2nd Character in List
            if Button::image(self.imgs.nothing)
                .down_from(self.ids.character_box_1, 5.0)
                .w_h(386.0, 80.0)
                .hover_image(self.imgs.selection)
                .press_image(self.imgs.selection)
                .image_color(Color::Rgba(1.0, 1.0, 1.0, 0.8))
                .label_y(conrod_core::position::Relative::Scalar(20.0))
                .set(self.ids.character_box_2, ui_widgets)
                .was_clicked()
            {}
            Text::new("Example 2nd Char")
                .top_left_with_margins_on(self.ids.character_box_2, 6.0, 9.0)
                .font_size(19)
                .font_id(self.fonts.metamorph)
                .color(TEXT_COLOR)
                .set(self.ids.character_name_2, ui_widgets);

            Text::new("Level ??")
                .down_from(self.ids.character_name_2, 4.0)
                .font_size(17)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(self.ids.character_level_2, ui_widgets);

            Text::new("Plains of Uncertainty")
                .down_from(self.ids.character_level_2, 4.0)
                .font_size(17)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(self.ids.character_location_2, ui_widgets);
        }
        // Character_Creation //////////////////////////////////////////////////////////////////////
        else {
            // Back Button
            if Button::image(self.imgs.button)
                .bottom_left_with_margins_on(ui_widgets.window, 10.0, 10.0)
                .w_h(150.0, 40.0)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .label("Back")
                .label_color(TEXT_COLOR)
                .label_font_size(18)
                .label_y(conrod_core::position::Relative::Scalar(3.0))
                .set(self.ids.back_button, ui_widgets)
                .was_clicked()
            {
                self.character_creation = false;
            }
            // Create Button
            if Button::image(self.imgs.button)
                .bottom_right_with_margins_on(ui_widgets.window, 10.0, 10.0)
                .w_h(150.0, 40.0)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .label("Create")
                .label_color(TEXT_COLOR)
                .label_font_size(18)
                .label_y(conrod_core::position::Relative::Scalar(3.0))
                .set(self.ids.create_button, ui_widgets)
                .was_clicked()
            {
                // TODO: Save character.
                self.character_creation = false;
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
            for event in TextBox::new(&self.character_name)
                .w_h(300.0, 60.0)
                .mid_top_with_margin_on(self.ids.name_input, 2.0)
                .font_size(26)
                .font_id(self.fonts.metamorph)
                .center_justify()
                .text_color(TEXT_COLOR)
                .color(TRANSPARENT)
                .border_color(TRANSPARENT)
                .set(self.ids.name_field, ui_widgets)
            {
                match event {
                    TextBoxEvent::Update(name) => {
                        self.character_name = name;
                    }
                    TextBoxEvent::Enter => {}
                }
            }

            // Window

            Rectangle::fill_with([386.0, 988.0], color::rgba(0.0, 0.0, 0.0, 0.8))
                .top_left_with_margins_on(ui_widgets.window, 30.0, 30.0)
                .set(self.ids.creation_bg, ui_widgets);
            Image::new(self.imgs.charlist_frame)
                .w_h(400.0, 1000.0)
                .middle_of(self.ids.creation_bg)
                .set(self.ids.charlist_frame, ui_widgets);
            Rectangle::fill_with([386.0, 983.0], color::TRANSPARENT)
                .middle_of(self.ids.creation_bg)
                .scroll_kids()
                .scroll_kids_vertically()
                .set(self.ids.creation_alignment, ui_widgets);
            Scrollbar::y_axis(self.ids.creation_alignment)
                .thickness(5.0)
                .auto_hide(true)
                .rgba(0.33, 0.33, 0.33, 1.0)
                .set(self.ids.selection_scrollbar, ui_widgets);

            // Male/Female/Race Icons

            Text::new("Character Creation")
                .mid_top_with_margin_on(self.ids.creation_alignment, 10.0)
                .font_size(24)
                .font_id(self.fonts.metamorph)
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
            if Button::image(
                if let humanoid::BodyType::Male = self.character_body.body_type {
                    self.imgs.icon_border_pressed
                } else {
                    self.imgs.icon_border
                },
            )
            .middle_of(self.ids.male)
            .hover_image(self.imgs.icon_border_mo)
            .press_image(self.imgs.icon_border_press)
            .set(self.ids.body_type_1, ui_widgets)
            .was_clicked()
            {
                self.character_body.body_type = humanoid::BodyType::Male;
            }
            // Female
            Image::new(self.imgs.female)
                .w_h(70.0, 70.0)
                .top_right_with_margins_on(self.ids.creation_buttons_alignment_1, 0.0, 0.0)
                .set(self.ids.female, ui_widgets);
            if Button::image(
                if let humanoid::BodyType::Female = self.character_body.body_type {
                    self.imgs.icon_border_pressed
                } else {
                    self.imgs.icon_border
                },
            )
            .middle_of(self.ids.female)
            .hover_image(self.imgs.icon_border_mo)
            .press_image(self.imgs.icon_border_press)
            .set(self.ids.body_type_2, ui_widgets)
            .was_clicked()
            {
                self.character_body.body_type = humanoid::BodyType::Female;
            }

            // Alignment for Races and Tools
            Rectangle::fill_with([214.0, 304.0], color::TRANSPARENT)
                .mid_bottom_with_margin_on(self.ids.creation_buttons_alignment_1, -324.0)
                .set(self.ids.creation_buttons_alignment_2, ui_widgets);

            // Human
            Image::new(
                if let humanoid::BodyType::Male = self.character_body.body_type {
                    self.imgs.human_m
                } else {
                    self.imgs.human_f
                },
            )
            .w_h(70.0, 70.0)
            .top_left_with_margins_on(self.ids.creation_buttons_alignment_2, 0.0, 0.0)
            .set(self.ids.human, ui_widgets);
            if Button::image(if let humanoid::Race::Human = self.character_body.race {
                self.imgs.icon_border_pressed
            } else {
                self.imgs.icon_border
            })
            .middle_of(self.ids.human)
            .hover_image(self.imgs.icon_border_mo)
            .press_image(self.imgs.icon_border_press)
            .set(self.ids.race_1, ui_widgets)
            .was_clicked()
            {
                self.character_body.race = humanoid::Race::Human;
            }

            // Orc
            Image::new(
                if let humanoid::BodyType::Male = self.character_body.body_type {
                    self.imgs.orc_m
                } else {
                    self.imgs.orc_f
                },
            )
            .w_h(70.0, 70.0)
            .right_from(self.ids.human, 2.0)
            .set(self.ids.orc, ui_widgets);
            if Button::image(if let humanoid::Race::Orc = self.character_body.race {
                self.imgs.icon_border_pressed
            } else {
                self.imgs.icon_border
            })
            .middle_of(self.ids.orc)
            .hover_image(self.imgs.icon_border_mo)
            .press_image(self.imgs.icon_border_press)
            .set(self.ids.race_2, ui_widgets)
            .was_clicked()
            {
                self.character_body.race = humanoid::Race::Orc;
            }
            // Dwarf
            Image::new(
                if let humanoid::BodyType::Male = self.character_body.body_type {
                    self.imgs.dwarf_m
                } else {
                    self.imgs.dwarf_f
                },
            )
            .w_h(70.0, 70.0)
            .right_from(self.ids.orc, 2.0)
            .set(self.ids.dwarf, ui_widgets);
            if Button::image(if let humanoid::Race::Dwarf = self.character_body.race {
                self.imgs.icon_border_pressed
            } else {
                self.imgs.icon_border
            })
            .middle_of(self.ids.dwarf)
            .hover_image(self.imgs.icon_border_mo)
            .press_image(self.imgs.icon_border_press)
            .set(self.ids.race_3, ui_widgets)
            .was_clicked()
            {
                self.character_body.race = humanoid::Race::Dwarf;
            }
            // Elf
            Image::new(
                if let humanoid::BodyType::Male = self.character_body.body_type {
                    self.imgs.elf_m
                } else {
                    self.imgs.elf_f
                },
            )
            .w_h(70.0, 70.0)
            .down_from(self.ids.human, 2.0)
            .set(self.ids.elf, ui_widgets);
            if Button::image(if let humanoid::Race::Elf = self.character_body.race {
                self.imgs.icon_border_pressed
            } else {
                self.imgs.icon_border
            })
            .middle_of(self.ids.elf)
            .hover_image(self.imgs.icon_border_mo)
            .press_image(self.imgs.icon_border_press)
            .set(self.ids.race_4, ui_widgets)
            .was_clicked()
            {
                self.character_body.race = humanoid::Race::Elf;
            }
            // Undead
            Image::new(
                if let humanoid::BodyType::Male = self.character_body.body_type {
                    self.imgs.undead_m
                } else {
                    self.imgs.undead_f
                },
            )
            .w_h(70.0, 70.0)
            .right_from(self.ids.elf, 2.0)
            .set(self.ids.undead, ui_widgets);
            if Button::image(if let humanoid::Race::Undead = self.character_body.race {
                self.imgs.icon_border_pressed
            } else {
                self.imgs.icon_border
            })
            .middle_of(self.ids.undead)
            .hover_image(self.imgs.icon_border_mo)
            .press_image(self.imgs.icon_border_press)
            .set(self.ids.race_5, ui_widgets)
            .was_clicked()
            {
                self.character_body.race = humanoid::Race::Undead;
            }
            // Danari
            Image::new(
                if let humanoid::BodyType::Male = self.character_body.body_type {
                    self.imgs.danari_m
                } else {
                    self.imgs.danari_f
                },
            )
            .right_from(self.ids.undead, 2.0)
            .set(self.ids.danari, ui_widgets);
            if Button::image(if let humanoid::Race::Danari = self.character_body.race {
                self.imgs.icon_border_pressed
            } else {
                self.imgs.icon_border
            })
            .w_h(70.0, 70.0)
            .middle_of(self.ids.danari)
            .hover_image(self.imgs.icon_border_mo)
            .press_image(self.imgs.icon_border_press)
            .set(self.ids.race_6, ui_widgets)
            .was_clicked()
            {
                self.character_body.race = humanoid::Race::Danari;
            }

            // Hammer

            Image::new(self.imgs.hammer)
                .w_h(70.0, 70.0)
                .bottom_left_with_margins_on(self.ids.creation_buttons_alignment_2, 0.0, 0.0)
                .set(self.ids.hammer, ui_widgets);
            if Button::image(if let Some(Tool::Hammer) = self.character_tool {
                self.imgs.icon_border_pressed
            } else {
                self.imgs.icon_border
            })
            .middle_of(self.ids.hammer)
            .hover_image(self.imgs.icon_border_mo)
            .press_image(self.imgs.icon_border_press)
            .set(self.ids.hammer_button, ui_widgets)
            .was_clicked()
            {
                self.character_tool = Some(Tool::Hammer);
            }
            // REMOVE THIS AFTER IMPLEMENTATION
            /*Rectangle::fill_with([67.0, 67.0], color::rgba(0.0, 0.0, 0.0, 0.8))
            .middle_of(self.ids.hammer)
            .set(self.ids.hammer_grey, ui_widgets);*/

            // Bow

            Image::new(self.imgs.bow)
                .w_h(70.0, 70.0)
                .right_from(self.ids.hammer, 2.0)
                .set(self.ids.bow, ui_widgets);
            if Button::image(if let Some(Tool::Bow) = self.character_tool {
                self.imgs.icon_border_pressed
            } else {
                self.imgs.icon_border
            })
            .middle_of(self.ids.bow)
            //.hover_image(self.imgs.icon_border_mo)
            //.press_image(self.imgs.icon_border_press)
            .set(self.ids.bow_button, ui_widgets)
            .was_clicked()
            {
                //self.character_tool = Some(Tool::Bow);
            }
            // REMOVE THIS AFTER IMPLEMENTATION
            Rectangle::fill_with([67.0, 67.0], color::rgba(0.0, 0.0, 0.0, 0.8))
                .middle_of(self.ids.bow)
                .set(self.ids.bow_grey, ui_widgets);
            // Staff
            Image::new(self.imgs.staff)
                .w_h(70.0, 70.0)
                .right_from(self.ids.bow, 2.0)
                .set(self.ids.staff, ui_widgets);
            if Button::image(if let Some(Tool::Staff) = self.character_tool {
                self.imgs.icon_border_pressed
            } else {
                self.imgs.icon_border
            })
            .middle_of(self.ids.staff)
            //.hover_image(self.imgs.icon_border_mo)
            //.press_image(self.imgs.icon_border_press)
            .set(self.ids.staff_button, ui_widgets)
            .was_clicked()
            {
                //self.character_tool = Some(Tool::Staff);
            }
            // REMOVE THIS AFTER IMPLEMENTATION
            Rectangle::fill_with([67.0, 67.0], color::rgba(0.0, 0.0, 0.0, 0.8))
                .middle_of(self.ids.staff)
                .set(self.ids.staff_grey, ui_widgets);
            // Sword
            Image::new(self.imgs.sword)
                .w_h(70.0, 70.0)
                .up_from(self.ids.hammer, 2.0)
                .set(self.ids.sword, ui_widgets);
            if Button::image(if let Some(Tool::Sword) = self.character_tool {
                self.imgs.icon_border_pressed
            } else {
                self.imgs.icon_border
            })
            .middle_of(self.ids.sword)
            .hover_image(self.imgs.icon_border_mo)
            .press_image(self.imgs.icon_border_press)
            .set(self.ids.sword_button, ui_widgets)
            .was_clicked()
            {
                self.character_tool = Some(Tool::Sword);
            }

            // Daggers
            Image::new(self.imgs.daggers)
                .w_h(70.0, 70.0)
                .right_from(self.ids.sword, 2.0)
                .set(self.ids.daggers, ui_widgets);
            if Button::image(if let Some(Tool::Daggers) = self.character_tool {
                self.imgs.icon_border_pressed
            } else {
                self.imgs.icon_border
            })
            .middle_of(self.ids.daggers)
            //.hover_image(self.imgs.icon_border_mo)
            //.press_image(self.imgs.icon_border_press)
            .set(self.ids.daggers_button, ui_widgets)
            .was_clicked()
            {
                // self.character_tool = Some(Tool::Daggers);
            } // REMOVE THIS AFTER IMPLEMENTATION
            Rectangle::fill_with([67.0, 67.0], color::rgba(0.0, 0.0, 0.0, 0.8))
                .middle_of(self.ids.daggers)
                .set(self.ids.daggers_grey, ui_widgets);

            // Axe
            Image::new(self.imgs.axe)
                .w_h(70.0, 70.0)
                .right_from(self.ids.daggers, 2.0)
                .set(self.ids.axe, ui_widgets);
            if Button::image(if let Some(Tool::Axe) = self.character_tool {
                self.imgs.icon_border_pressed
            } else {
                self.imgs.icon_border
            })
            .middle_of(self.ids.axe)
            .hover_image(self.imgs.icon_border_mo)
            .press_image(self.imgs.icon_border_press)
            .set(self.ids.axe_button, ui_widgets)
            .was_clicked()
            {
                self.character_tool = Some(Tool::Axe);
            }
            // REMOVE THIS AFTER IMPLEMENTATION
            /*Rectangle::fill_with([67.0, 67.0], color::rgba(0.0, 0.0, 0.0, 0.8))
            .middle_of(self.ids.axe)
            .set(self.ids.axe_grey, ui_widgets);*/

            // Sliders
            let (metamorph, slider_indicator, slider_range) = (
                self.fonts.metamorph,
                self.imgs.slider_indicator,
                self.imgs.slider_range,
            );
            let char_slider = move |prev_id,
                                    text,
                                    text_id,
                                    max,
                                    selected_val,
                                    slider_id,
                                    ui_widgets: &mut UiCell| {
                Text::new(text)
                    .down_from(prev_id, 22.0)
                    .align_middle_x_of(prev_id)
                    .font_size(18)
                    .font_id(metamorph)
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
            let current_hair_style = self.character_body.hair_style;
            if let Some(new_val) = char_slider(
                self.ids.creation_buttons_alignment_2,
                "Hair Style",
                self.ids.hairstyle_text,
                humanoid::ALL_HAIR_STYLES.len() - 1,
                humanoid::ALL_HAIR_STYLES
                    .iter()
                    .position(|&c| c == current_hair_style)
                    .unwrap_or(0),
                self.ids.hairstyle_slider,
                ui_widgets,
            ) {
                self.character_body.hair_style = humanoid::ALL_HAIR_STYLES[new_val];
            }
            // Hair Color
            if let Some(new_val) = char_slider(
                self.ids.hairstyle_slider,
                "Hair Color",
                self.ids.haircolor_text,
                self.character_body.race.num_hair_colors() - 1,
                self.character_body.hair_color as usize,
                self.ids.haircolor_slider,
                ui_widgets,
            ) {
                self.character_body.hair_color = new_val as u8;
            }
            // Skin
            if let Some(new_val) = char_slider(
                self.ids.haircolor_slider,
                "Skin",
                self.ids.skin_text,
                self.character_body.race.num_skin_colors() - 1,
                self.character_body.skin as usize,
                self.ids.skin_slider,
                ui_widgets,
            ) {
                self.character_body.skin = new_val as u8;
            }
            // Eyebrows
            let current_eyebrows = self.character_body.eyebrows;
            if let Some(new_val) = char_slider(
                self.ids.skin_slider,
                "Eyebrows",
                self.ids.eyebrows_text,
                humanoid::ALL_EYEBROWS.len() - 1,
                humanoid::ALL_EYEBROWS
                    .iter()
                    .position(|&c| c == current_eyebrows)
                    .unwrap_or(0),
                self.ids.eyebrows_slider,
                ui_widgets,
            ) {
                self.character_body.eyebrows = humanoid::ALL_EYEBROWS[new_val];
            }
            // EyeColor
            if let Some(new_val) = char_slider(
                self.ids.eyebrows_slider,
                "Eye Color",
                self.ids.eyecolor_text,
                self.character_body.race.num_eye_colors() - 1,
                self.character_body.eye_color as usize,
                self.ids.eyecolor_slider,
                ui_widgets,
            ) {
                self.character_body.eye_color = new_val as u8;
            }
            // Accessories
            let current_accessory = self.character_body.accessory;
            if let Some(new_val) = char_slider(
                self.ids.eyecolor_slider,
                "Accessories",
                self.ids.accessories_text,
                humanoid::ALL_ACCESSORIES.len() - 1,
                humanoid::ALL_ACCESSORIES
                    .iter()
                    .position(|&c| c == current_accessory)
                    .unwrap_or(0),
                self.ids.accessories_slider,
                ui_widgets,
            ) {
                self.character_body.accessory = humanoid::ALL_ACCESSORIES[new_val];
            }
            // Beard
            if let humanoid::BodyType::Male = self.character_body.body_type {
                let current_beard = self.character_body.beard;
                if let Some(new_val) = char_slider(
                    self.ids.accessories_slider,
                    "Beard",
                    self.ids.beard_text,
                    humanoid::ALL_BEARDS.len() - 1,
                    humanoid::ALL_BEARDS
                        .iter()
                        .position(|&c| c == current_beard)
                        .unwrap_or(0),
                    self.ids.beard_slider,
                    ui_widgets,
                ) {
                    self.character_body.beard = humanoid::ALL_BEARDS[new_val];
                }
            } else {
                Text::new("Beard")
                    .mid_bottom_with_margin_on(self.ids.accessories_slider, -40.0)
                    .font_size(18)
                    .font_id(self.fonts.metamorph)
                    .color(TEXT_COLOR_2)
                    .set(self.ids.beard_text, ui_widgets);
                ImageSlider::continuous(5.0, 0.0, 10.0, self.imgs.nothing, self.imgs.slider_range)
                    .w_h(208.0, 22.0)
                    .mid_bottom_with_margin_on(self.ids.beard_text, -30.0)
                    .track_breadth(12.0)
                    .slider_length(10.0)
                    .track_color(Color::Rgba(1.0, 1.0, 1.0, 0.2))
                    .slider_color(Color::Rgba(1.0, 1.0, 1.0, 0.2))
                    .pad_track((5.0, 5.0))
                    .set(self.ids.beard_slider_2, ui_widgets);
            }
        } // Char Creation fin

        events
    }

    pub fn handle_event(&mut self, event: ui::Event) {
        self.ui.handle_event(event);
    }

    pub fn maintain(&mut self, renderer: &mut Renderer, client: &Client) -> Vec<Event> {
        let events = self.update_layout(client);
        self.ui.maintain(renderer, None);
        events
    }

    pub fn render(&self, renderer: &mut Renderer, globals: &Consts<Globals>) {
        self.ui.render(renderer, Some(globals));
    }
}
