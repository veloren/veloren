use crate::{
    render::Renderer,
    ui::{
        self,
        img_ids::{ImageGraphic, VoxelGraphic},
        ScaleMode, Ui,
    },
    window::Window,
};
use common::comp::{
    actor::{Belt, BodyType, Chest, Foot, Hand, Head, Pants, Race, Weapon},
    HumanoidBody,
};
use conrod_core::{
    color,
    color::TRANSPARENT,
    widget::{text_box::Event as TextBoxEvent, Button, Image, Rectangle, Scrollbar, Text, TextBox},
    widget_ids, Borderable, Color, Colorable, Labelable, Positionable, Sizeable, Widget,
};
use std::sync::Arc;

widget_ids! {
    struct Ids {
        // Background and logo
        charlist_bg,
        charlist_frame,
        charlist_alignment,
        selection_scrollbar,
        server_name_text,
        change_server,
        server_frame_bg,
        server_frame,
        v_logo,
        version,

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
        race_heading,
        race_description,
        weapon_heading,
        weapon_description,
        races_bg,
        body_type_bg,
        desc_bg,
        skin_eyes_window,
        hair_window,
        accessories_window,
        skin_eyes_button,
        hair_button,
        accessories_button,
        skin_rect,
        eyes_rect,
        human_skin_bg,
        orc_skin_bg,
        dwarf_skin_bg,
        undead_skin_bg,
        elf_skin_bg,
        danari_skin_bg,
        name_input_bg,


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
        weapon_1,
        weapon_2,
        weapon_3,
        weapon_4,
        weapon_5,
        weapon_6,
        weapon_7,

        //test_chars
        test_char_l_button,
        test_char_l_big,
        help_text_bg,
        help_text,

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
        // Weapon Icons
        weapon_bg,
        daggers,
        sword_shield,
        sword,
        axe,
        hammer,
        bow,
        staff,
        // Arrows
        arrow_left,
        arrow_right,
        // Body Features
        window_skin_eyes,
        window_skin_eyes_mid,
        window_skin_eyes_bot,
        window_hair,
        window_hair_mid,
        window_hair_bot,
        window_acessories,
        window_acessories_mid,
        window_acessories_bot,
        skin_color_picker,
        skin_color_slider,
        skin_color_text,
        skin_color_slider_text,
        eye_color_picker,
        eye_color_slider,
        eye_color_text,
        eye_color_slider_text,
        skin_color_slider_range,
        skin_color_slider_indicator,
        eye_color_slider_range,
        eye_color_slider_indicator,
        hair_color_slider_text,
        // Creation Hair Contents
        hair_style_text,
        hair_style_arrow_l,
        hair_style_arrow_r,
        hair_color_picker_bg,
        hair_color_text,
        hair_color_slider_range,
        hair_color_slider_indicator,
        eyebrow_style_text,
        eyebrow_arrow_l,
        eyebrow_arrow_r,
        beard_style_text,
        beard_arrow_l,
        beard_arrow_r,
        // Creation Accessories Contents
        warpaint_text,
        warpaint_arrow_l,
        warpaint_arrow_r,
        warpaint_color_picker_bg,
        warpaint_color_text,
        warpaint_slider_indicator,
        warpaint_slider_range,
        warpaint_slider_text,
    }
}

image_ids! {
    struct Imgs {
        <VoxelGraphic>
        v_logo: "/voxygen/element/v_logo.vox",
        button: "/voxygen/element/buttons/button.vox",
        button_hover: "/voxygen/element/buttons/button_hover.vox",
        button_press: "/voxygen/element/buttons/button_press.vox",
        button_red: "/voxygen/element/buttons/button_red.vox",
        button_red_hover: "/voxygen/element/buttons/button_red_hover.vox",
        button_red_press: "/voxygen/element/buttons/button_red_press.vox",
        name_input: "/voxygen/element/misc_bg/textbox.vox",
        charlist_frame: "/voxygen/element/frames/window_4.vox",
        selection_frame: "/voxygen/element/frames/selection_frame.vox",
        server_frame: "/voxygen/element/frames/server_frame.vox",

        <ImageGraphic>
        selection_window: "/voxygen/element/frames/selection.png",
        test_char_l_button: "/voxygen/element/misc_bg/test_char_l.png",
        test_char_l_big: "/voxygen/element/misc_bg/test_char_l_big.png",
        creation_window: "/voxygen/element/frames/char_creation.png",
        creation_window_body: "/voxygen/element/frames/body_creation.png",
        frame_closed: "/voxygen/element/buttons/frame/closed.png",
        frame_closed_mo: "/voxygen/element/buttons/frame/closed_mo.png",
        frame_closed_press: "/voxygen/element/buttons/frame/closed_press.png",
        frame_open: "/voxygen/element/buttons/frame/open.png",
        frame_open_mo: "/voxygen/element/buttons/frame/open_mo.png",
        frame_open_press: "/voxygen/element/buttons/frame/open_press.png",
        skin_eyes_window: "/voxygen/element/frames/skin_eyes.png",
        hair_window: "/voxygen/element/frames/skin_eyes.png",
        accessories_window: "/voxygen/element/frames/skin_eyes.png",
        color_picker_bg: "/voxygen/element/misc_bg/color_picker_blank.png",
        slider_range: "/voxygen/element/slider/track.png",
        slider_indicator: "/voxygen/element/slider/indicator.png",
        window_frame_2: "/voxygen/element/frames/window_2.png",


        // Weapon Icons
        daggers: "/voxygen/element/icons/daggers.png",
        sword_shield: "/voxygen/element/icons/swordshield.png",
        sword: "/voxygen/element/icons/sword.png",
        axe: "/voxygen/element/icons/axe.png",
        hammer: "/voxygen/element/icons/hammer.png",
        bow: "/voxygen/element/icons/bow.png",
        staff: "/voxygen/element/icons/staff.png",
        // Race Icons
        male: "/voxygen/element/icons/male.png",
        female: "/voxygen/element/icons/female.png",
        human_m: "/voxygen/element/icons/human_m.png",
        human_f: "/voxygen/element/icons/human_f.png",
        orc_m: "/voxygen/element/icons/orc_m.png",
        orc_f: "/voxygen/element/icons/orc_f.png",
        dwarf_m: "/voxygen/element/icons/dwarf_m.png",
        dwarf_f: "/voxygen/element/icons/dwarf_f.png",
        undead_m: "/voxygen/element/icons/ud_m.png",
        undead_f: "/voxygen/element/icons/ud_f.png",
        elf_m: "/voxygen/element/icons/elf_m.png",
        elf_f: "/voxygen/element/icons/elf_f.png",
        danari_m: "/voxygen/element/icons/danari_m.png",
        danari_f: "/voxygen/element/icons/danari_f.png",
        // Arrows
        arrow_left: "/voxygen/element/buttons/arrow/left.png",
        arrow_left_mo: "/voxygen/element/buttons/arrow/left_mo.png",
        arrow_left_press: "/voxygen/element/buttons/arrow/left_press.png",
        arrow_left_grey: "/voxygen/element/buttons/arrow/left_inactive.png",
        arrow_right: "/voxygen/element/buttons/arrow/right.png",
        arrow_right_mo: "/voxygen/element/buttons/arrow/right_mo.png",
        arrow_right_press: "/voxygen/element/buttons/arrow/right_press.png",
        arrow_right_grey: "/voxygen/element/buttons/arrow/right_inactive.png",
        // Icon Borders
        icon_border: "/voxygen/element/buttons/border.png",
        icon_border_mo: "/voxygen/element/buttons/border_mo.png",
        icon_border_press: "/voxygen/element/buttons/border_press.png",
        icon_border_pressed: "/voxygen/element/buttons/border_pressed.png",
        nothing: "/voxygen/element/nothing.png",

    }
}

font_ids! {
    pub struct Fonts {
        opensans: "/voxygen/font/OpenSans-Regular.ttf",
        metamorph: "/voxygen/font/Metamorphous-Regular.ttf",
    }
}

enum CreationState {
    Race,
    Weapon,
    Body(BodyPart),
}

#[derive(Clone, Copy)]
enum BodyPart {
    SkinEyes,
    Hair,
    Accessories,
}

pub enum Event {
    Logout,
    Play,
}

const TEXT_COLOR: Color = Color::Rgba(1.0, 1.0, 1.0, 1.0);

pub struct CharSelectionUi {
    ui: Ui,
    ids: Ids,
    imgs: Imgs,
    fonts: Fonts,
    character_creation: bool,
    selected_char_no: Option<i32>,
    pub character_name: String,
    pub character_body: HumanoidBody,
    creation_state: CreationState,
}

impl CharSelectionUi {
    pub fn new(window: &mut Window) -> Self {
        let mut ui = Ui::new(window).unwrap();
        // TODO: Adjust/remove this, right now it is used to demonstrate window scaling functionality.
        ui.scaling_mode(ScaleMode::RelativeToWindow([1920.0, 1080.0].into()));
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
            selected_char_no: None,
            character_name: "Character Name".to_string(),
            character_body: HumanoidBody::random(),
            creation_state: CreationState::Race,
        }
    }

    // TODO: Split this into multiple modules or functions.
    fn update_layout(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        let ref mut ui_widgets = self.ui.set_widgets();
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
                .down_from(self.ids.server_frame_bg, 10.0)
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
                .rgba(0.33, 0.33, 0.33, 1.0)
                .set(self.ids.selection_scrollbar, ui_widgets);
            // Server Name
            Text::new("Server Name") //TODO: Add in Server Name
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
                self.selected_char_no = None;
            }

            // Veloren Logo and Alpha Version
            Text::new(version)
                .top_right_with_margins_on(ui_widgets.window, 5.0, 5.0)
                .font_size(14)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(self.ids.version, ui_widgets);

            // 1st Character in Selection List
            if Button::image(self.imgs.selection_frame)
                .top_left_with_margins_on(self.ids.charlist_alignment, 0.0, 2.0)
                .w_h(386.0, 80.0)
                .hover_image(self.imgs.charlist_frame)
                .press_image(self.imgs.charlist_frame)
                .label_y(conrod_core::position::Relative::Scalar(20.0))
                .set(self.ids.character_box_1, ui_widgets)
                .was_clicked()
            {
                events.push(Event::Play);
            }
            Text::new("Human Default")
                .top_left_with_margins_on(self.ids.character_box_1, 5.0, 5.0)
                .font_size(20)
                .font_id(self.fonts.metamorph)
                .color(TEXT_COLOR)
                .set(self.ids.character_name_1, ui_widgets);

            Text::new("Level 1")
                .down_from(self.ids.character_name_1, 5.0)
                .font_size(18)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(self.ids.character_level_1, ui_widgets);

            Text::new("Uncanny Valley")
                .down_from(self.ids.character_level_1, 5.0)
                .font_size(18)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(self.ids.character_location_1, ui_widgets);

            // 2nd Character in List
            if Button::image(self.imgs.nothing)
                .down_from(self.ids.character_box_1, 5.0)
                .w_h(386.0, 80.0)
                .hover_image(self.imgs.charlist_frame)
                .press_image(self.imgs.charlist_frame)
                .label_y(conrod_core::position::Relative::Scalar(20.0))
                .set(self.ids.character_box_2, ui_widgets)
                .was_clicked()
            {
                events.push(Event::Play);
            }
            Text::new("Example 2nd Char")
                .top_left_with_margins_on(self.ids.character_box_2, 5.0, 5.0)
                .font_size(20)
                .font_id(self.fonts.metamorph)
                .color(TEXT_COLOR)
                .set(self.ids.character_name_2, ui_widgets);

            Text::new("Level ??")
                .down_from(self.ids.character_name_2, 5.0)
                .font_size(18)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(self.ids.character_level_2, ui_widgets);

            Text::new("Plains of Uncertainty")
                .down_from(self.ids.character_level_2, 5.0)
                .font_size(18)
                .font_id(self.fonts.opensans)
                .color(TEXT_COLOR)
                .set(self.ids.character_location_2, ui_widgets);

            if let Some(no) = self.selected_char_no {
                // Selection_Window
                Image::new(self.imgs.selection_window)
                    .w_h(522.0, 722.0)
                    .mid_right_with_margin_on(ui_widgets.window, 10.0)
                    .set(self.ids.selection_window, ui_widgets);
                // Character Name & Level
                Text::new("Character Name")
                    .mid_top_with_margin_on(self.ids.selection_window, 80.0)
                    .font_size(30)
                    .color(TEXT_COLOR)
                    .set(self.ids.char_name, ui_widgets);
                Text::new("1")
                    .mid_top_with_margin_on(self.ids.char_name, 40.0)
                    .font_size(30)
                    .color(TEXT_COLOR)
                    .set(self.ids.char_level, ui_widgets);

                // Selected Character
                if no == 1 {
                    Image::new(self.imgs.test_char_l_big)
                        .w_h(522.0, 722.0)
                        .middle_of(self.ids.selection_window)
                        .set(self.ids.test_char_l_big, ui_widgets);
                }

                // Delete Button
                if Button::image(self.imgs.button_red)
                    .bottom_right_with_margins_on(self.ids.selection_window, -25.0, 0.0)
                    .w_h(100.0, 20.0)
                    .hover_image(self.imgs.button_red_hover)
                    .press_image(self.imgs.button_red_press)
                    .label("Delete")
                    .label_color(TEXT_COLOR)
                    .label_font_size(12)
                    .label_y(conrod_core::position::Relative::Scalar(3.0))
                    .set(self.ids.delete_button, ui_widgets)
                    .was_clicked()
                {}
            }
        }
        // Character_Creation
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
            Rectangle::fill_with([320.0, 50.0], color::rgba(0.0, 0.0, 0.0, 0.99))
                .mid_bottom_with_margin_on(ui_widgets.window, 20.0)
                .set(self.ids.name_input_bg, ui_widgets);
            Button::image(self.imgs.name_input)
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

            // Window(s)
            Image::new(if let CreationState::Body(_) = self.creation_state {
                self.imgs.creation_window_body
            } else {
                self.imgs.creation_window
            })
            .w_h(628.0, 814.0)
            .top_left_with_margins_on(ui_widgets.window, 60.0, 30.0)
            .set(self.ids.creation_window, ui_widgets);

            // Arrows
            // TODO: Lower the resolution of the arrow images & use non decimal sizes below.
            const ARROW_WH: [f64; 2] = [986.0 * 0.03, 1024.0 * 0.03];
            match self.creation_state {
                CreationState::Race => {
                    Button::image(self.imgs.arrow_left_grey)
                        .wh(ARROW_WH)
                        .top_left_with_margins_on(self.ids.creation_window, 74.0, 55.0)
                        .set(self.ids.arrow_left, ui_widgets);

                    if Button::image(self.imgs.arrow_right)
                        .wh(ARROW_WH)
                        .hover_image(self.imgs.arrow_right_mo)
                        .press_image(self.imgs.arrow_right_press)
                        .top_right_with_margins_on(self.ids.creation_window, 74.0, 55.0)
                        .set(self.ids.arrow_right, ui_widgets)
                        .was_clicked()
                    {
                        self.creation_state = CreationState::Weapon;
                    }
                }
                CreationState::Weapon => {
                    if Button::image(self.imgs.arrow_left)
                        .wh(ARROW_WH)
                        .hover_image(self.imgs.arrow_left_mo)
                        .press_image(self.imgs.arrow_left_press)
                        .top_left_with_margins_on(self.ids.creation_window, 74.0, 55.0)
                        .set(self.ids.arrow_left, ui_widgets)
                        .was_clicked()
                    {
                        self.creation_state = CreationState::Race;
                    }

                    if Button::image(self.imgs.arrow_right)
                        .wh(ARROW_WH)
                        .hover_image(self.imgs.arrow_right_mo)
                        .press_image(self.imgs.arrow_right_press)
                        .top_right_with_margins_on(self.ids.creation_window, 74.0, 55.0)
                        .set(self.ids.arrow_right, ui_widgets)
                        .was_clicked()
                    {
                        self.creation_state = CreationState::Body(BodyPart::SkinEyes);
                    }
                }
                CreationState::Body(_) => {
                    if Button::image(self.imgs.arrow_left)
                        .wh(ARROW_WH)
                        .hover_image(self.imgs.arrow_left_mo)
                        .press_image(self.imgs.arrow_left_press)
                        .top_left_with_margins_on(self.ids.creation_window, 74.0, 55.0)
                        .set(self.ids.arrow_left, ui_widgets)
                        .was_clicked()
                    {
                        self.creation_state = CreationState::Weapon;
                    }
                    Button::image(self.imgs.arrow_right_grey)
                        .wh(ARROW_WH)
                        .top_right_with_margins_on(self.ids.creation_window, 74.0, 55.0)
                        .set(self.ids.arrow_right, ui_widgets);
                }
            }

            // Races

            // Weapon

            // Body

            // Race Selection
            if let CreationState::Race = self.creation_state {
                Text::new("Choose your Race")
                    .mid_top_with_margin_on(self.ids.creation_window, 74.0)
                    .font_size(28)
                    .color(TEXT_COLOR)
                    .set(self.ids.select_window_title, ui_widgets);

                // Male/Female/Race Icons
                // Alignment
                Rectangle::fill_with([151.0, 68.0], color::TRANSPARENT)
                    .mid_top_with_margin_on(self.ids.creation_window, 210.0)
                    .set(self.ids.body_type_bg, ui_widgets);

                // Male
                Image::new(self.imgs.male)
                    .w_h(68.0, 68.0)
                    .mid_left_of(self.ids.body_type_bg)
                    .set(self.ids.male, ui_widgets);
                if Button::image(if let BodyType::Male = self.character_body.body_type {
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
                    self.character_body.body_type = BodyType::Male;
                }
                // Female
                Image::new(self.imgs.female)
                    .w_h(68.0, 68.0)
                    .right_from(self.ids.male, 16.0)
                    .set(self.ids.female, ui_widgets);
                if Button::image(if let BodyType::Female = self.character_body.body_type {
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
                    self.character_body.body_type = BodyType::Female;
                }
                // Alignment
                Rectangle::fill_with([458.0, 68.0], color::TRANSPARENT)
                    .mid_top_with_margin_on(self.ids.creation_window, 120.0)
                    .set(self.ids.races_bg, ui_widgets);
                // TODO: If races were in some sort of array format, we could do this in a loop.
                // Human
                Image::new(if let BodyType::Male = self.character_body.body_type {
                    self.imgs.human_m
                } else {
                    self.imgs.human_f
                })
                .w_h(68.0, 68.0)
                .mid_left_of(self.ids.races_bg)
                .set(self.ids.human, ui_widgets);
                if Button::image(if let Race::Human = self.character_body.race {
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
                    self.character_body.race = Race::Human;
                }

                // Orc
                Image::new(if let BodyType::Male = self.character_body.body_type {
                    self.imgs.orc_m
                } else {
                    self.imgs.orc_f
                })
                .w_h(68.0, 68.0)
                .right_from(self.ids.human, 10.0)
                .set(self.ids.orc, ui_widgets);
                if Button::image(if let Race::Orc = self.character_body.race {
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
                    self.character_body.race = Race::Orc;
                }
                // Dwarf
                Image::new(if let BodyType::Male = self.character_body.body_type {
                    self.imgs.dwarf_m
                } else {
                    self.imgs.dwarf_f
                })
                .w_h(68.0, 68.0)
                .right_from(self.ids.human, 10.0 * 2.0 + 68.0)
                .set(self.ids.dwarf, ui_widgets);
                if Button::image(if let Race::Dwarf = self.character_body.race {
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
                    self.character_body.race = Race::Dwarf;
                }
                // Elf
                Image::new(if let BodyType::Male = self.character_body.body_type {
                    self.imgs.elf_m
                } else {
                    self.imgs.elf_f
                })
                .w_h(68.0, 68.0)
                .right_from(self.ids.human, 10.0 * 3.0 + 68.0 * 2.0)
                .set(self.ids.elf, ui_widgets);
                if Button::image(if let Race::Elf = self.character_body.race {
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
                    self.character_body.race = Race::Elf;
                }
                // Undead
                Image::new(if let BodyType::Male = self.character_body.body_type {
                    self.imgs.undead_m
                } else {
                    self.imgs.undead_f
                })
                .w_h(68.0, 68.0)
                .right_from(self.ids.human, 10.0 * 4.0 + 68.0 * 3.0)
                .set(self.ids.undead, ui_widgets);
                if Button::image(if let Race::Undead = self.character_body.race {
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
                    self.character_body.race = Race::Undead;
                }
                // Danari
                Image::new(if let BodyType::Male = self.character_body.body_type {
                    self.imgs.danari_m
                } else {
                    self.imgs.danari_f
                })
                .right_from(self.ids.human, 10.0 * 5.0 + 68.0 * 4.0)
                .set(self.ids.danari, ui_widgets);
                if Button::image(if let Race::Danari = self.character_body.race {
                    self.imgs.icon_border_pressed
                } else {
                    self.imgs.icon_border
                })
                .w_h(68.0, 68.0)
                .middle_of(self.ids.danari)
                .hover_image(self.imgs.icon_border_mo)
                .press_image(self.imgs.icon_border_press)
                .set(self.ids.race_6, ui_widgets)
                .was_clicked()
                {
                    self.character_body.race = Race::Danari;
                }

                // Description Headline and Text

                // TODO: Load these from files (or from the server???).
                const HUMAN_DESC: &str =
                    "The former nomads were only recently able to gain a foothold in the world of Veloren. \n\
                    \n\
                    Their greatest strengths are their adaptability and intelligence,  which makes them allrounders in many fields.\n\
                    \n\
                    Humans are extremely diverse. \n\
                    Some become wicked witches, slimy scoundrels, and members of the underworld, while others become witch-hunters, sages, and noble knights. \n\
                    This diversity however creates constant conflict and antagonism between humans themselves, rather than with the other races of Veloren.";
                const ORC_DESC: &str =
                    "They are considered brutal, rude and combative. \n\
                     But once you gained their trust they will be loyal friends \n\
                     that follow a strict code of honor in all of their actions. \n\
                     \n\
                     Their warriors are masters of melee combat, but their true power \
                     comes from the magical rituals of their powerful shamans. \n\
                     \n\
                     They are  divided into three clans. \n\
                     Two of them are led by the conflicting descendants of the recently deceased High-Warlord. \n\
                     The third clan was formed by a group of Shamans to prevent the bloodshed caused by the rivaling groups and to secure their source of magic: \n\
                     A powerful nature crystal, stolen from the Brushwood Elves...";
                const DWARF_DESC: &str =
                    "Smoking chimneys, the sound of countless hammers and hoes. \
                    Infinite tunnel systems to track down even the last chunk of metal in the ground. \n\
                    \n\
                    This race of master craftsmen and grim fighters exist almost \
                    as long as the world itself.\n\
                    And they don't plan to finish their reign over the undergrounds soon.";
                const UNDEAD_DESC: &str =
                    "No one really knows the origin of these gruesome looking creatures. \n\
                    Some may have been reawakened soldiers from a battleground others are said to be the result of experiments with dark magic.\n\
                    \n\
                    After being chased and slaughtered for years the more civilised Undead decided to form a community and negotiate a piece treaty with the other inhabitants of Veloren.\n\
                    \n\
                    They are known for being nefarious and silent assassins.";
                const ELF_DESC: &str =
                    "No matter which shade of elves you encounter, they all have something in common: Magic. \n\
                    They can be found in many Forms: \n\
                    \n\
                    Pale Elves, living in dark fortresses, executing atrocious rituals. \n\
                    \n\
                    Nature connected Brushwood Elves, that guard ancient powers inside the forests.\n\
                    \n\
                    Gold Elves that hunger for political power in their massive city states. \n\
                    \n\
                    Dark Elves, seeking war to brutalize their enemies, with ‘honor.’\n\
                    \n\
                    And many more!";
                const DANARI_DESC: &str =
                    "The white domes and towers of their underwater kingdom are often mistaken for coral reefs from above the water. \n\
                    As a punishment those demonic creatures were banished to live detached from the rest of the world in ancient times. \n\
                    \n\
                    Once in a while one of them is born unaffected by this curse. Sadly this means that after reaching a certain age they won’t be able to live underwater anymore. \n\
                    \n\
                    Outcast communities consisting of these Blessed Danari have formed all over the land.";

                let (race_str, race_desc) = match self.character_body.race {
                    Race::Human => ("Humans", HUMAN_DESC),
                    Race::Orc => ("Orcs", ORC_DESC),
                    Race::Dwarf => ("Dwarves", DWARF_DESC),
                    Race::Undead => ("Undead", UNDEAD_DESC),
                    Race::Elf => ("Elves", ELF_DESC),
                    Race::Danari => ("Danari", DANARI_DESC),
                };
                Text::new(race_str)
                    .mid_top_with_margin_on(self.ids.creation_window, 370.0)
                    .font_size(30)
                    .color(TEXT_COLOR)
                    .set(self.ids.race_heading, ui_widgets);
                Text::new(race_desc)
                    .mid_top_with_margin_on(self.ids.creation_window, 410.0)
                    .w(500.0)
                    .font_size(20)
                    .font_id(self.fonts.opensans)
                    .color(TEXT_COLOR)
                    .wrap_by_word()
                    .set(self.ids.race_description, ui_widgets);
            }

            if let CreationState::Weapon = self.creation_state {
                Text::new("Choose your Weapon")
                    .mid_top_with_margin_on(self.ids.creation_window, 74.0)
                    .font_size(28)
                    .color(TEXT_COLOR)
                    .set(self.ids.select_window_title, ui_widgets);
                // Alignment
                Rectangle::fill_with([470.0, 60.0], color::TRANSPARENT)
                    .mid_top_with_margin_on(self.ids.creation_window, 180.0)
                    .set(self.ids.weapon_bg, ui_widgets);
                // Weapons Icons
                // Sword and Shield
                Image::new(self.imgs.sword_shield)
                    .w_h(60.0, 60.0)
                    .mid_left_of(self.ids.weapon_bg)
                    .set(self.ids.sword_shield, ui_widgets);
                if Button::image(if let Weapon::SwordShield = self.character_body.weapon {
                    self.imgs.icon_border_pressed
                } else {
                    self.imgs.icon_border
                })
                .middle_of(self.ids.sword_shield)
                .hover_image(self.imgs.icon_border_mo)
                .press_image(self.imgs.icon_border_press)
                .set(self.ids.weapon_1, ui_widgets)
                .was_clicked()
                {
                    self.character_body.weapon = Weapon::SwordShield;
                }

                // Daggers
                Image::new(self.imgs.daggers)
                    .w_h(60.0, 60.0)
                    .right_from(self.ids.sword_shield, 8.0)
                    .set(self.ids.daggers, ui_widgets);
                if Button::image(if let Weapon::Daggers = self.character_body.weapon {
                    self.imgs.icon_border_pressed
                } else {
                    self.imgs.icon_border
                })
                .middle_of(self.ids.daggers)
                .hover_image(self.imgs.icon_border_mo)
                .press_image(self.imgs.icon_border_press)
                .set(self.ids.weapon_2, ui_widgets)
                .was_clicked()
                {
                    self.character_body.weapon = Weapon::Daggers;
                }

                // Sword
                Image::new(self.imgs.sword)
                    .w_h(60.0, 60.0)
                    .right_from(self.ids.sword_shield, 8.0 * 2.0 + 60.0 * 1.0)
                    .set(self.ids.sword, ui_widgets);
                if Button::image(if let Weapon::Sword = self.character_body.weapon {
                    self.imgs.icon_border_pressed
                } else {
                    self.imgs.icon_border
                })
                .middle_of(self.ids.sword)
                .hover_image(self.imgs.icon_border_mo)
                .press_image(self.imgs.icon_border_press)
                .set(self.ids.weapon_3, ui_widgets)
                .was_clicked()
                {
                    self.character_body.weapon = Weapon::Sword;
                }
                // Axe
                Image::new(self.imgs.axe)
                    .w_h(60.0, 60.0)
                    .right_from(self.ids.sword_shield, 8.0 * 3.0 + 60.0 * 2.0)
                    .set(self.ids.axe, ui_widgets);
                if Button::image(if let Weapon::Axe = self.character_body.weapon {
                    self.imgs.icon_border_pressed
                } else {
                    self.imgs.icon_border
                })
                .middle_of(self.ids.axe)
                .hover_image(self.imgs.icon_border_mo)
                .press_image(self.imgs.icon_border_press)
                .set(self.ids.weapon_4, ui_widgets)
                .was_clicked()
                {
                    self.character_body.weapon = Weapon::Axe;
                }
                // Hammer
                Image::new(self.imgs.hammer)
                    .w_h(60.0, 60.0)
                    .right_from(self.ids.sword_shield, 8.0 * 4.0 + 60.0 * 3.0)
                    .set(self.ids.hammer, ui_widgets);
                if Button::image(if let Weapon::Hammer = self.character_body.weapon {
                    self.imgs.icon_border_pressed
                } else {
                    self.imgs.icon_border
                })
                .middle_of(self.ids.hammer)
                .hover_image(self.imgs.icon_border_mo)
                .press_image(self.imgs.icon_border_press)
                .set(self.ids.weapon_5, ui_widgets)
                .was_clicked()
                {
                    self.character_body.weapon = Weapon::Hammer;
                }
                // Bow
                Image::new(self.imgs.bow)
                    .w_h(60.0, 60.0)
                    .right_from(self.ids.sword_shield, 8.0 * 5.0 + 60.0 * 4.0)
                    .set(self.ids.bow, ui_widgets);
                if Button::image(if let Weapon::Bow = self.character_body.weapon {
                    self.imgs.icon_border_pressed
                } else {
                    self.imgs.icon_border
                })
                .middle_of(self.ids.bow)
                .hover_image(self.imgs.icon_border_mo)
                .press_image(self.imgs.icon_border_press)
                .set(self.ids.weapon_6, ui_widgets)
                .was_clicked()
                {
                    self.character_body.weapon = Weapon::Bow;
                }
                // Staff
                Image::new(self.imgs.staff)
                    .w_h(60.0, 60.0)
                    .right_from(self.ids.sword_shield, 8.0 * 6.0 + 60.0 * 5.0)
                    .set(self.ids.staff, ui_widgets);
                if Button::image(if let Weapon::Staff = self.character_body.weapon {
                    self.imgs.icon_border_pressed
                } else {
                    self.imgs.icon_border
                })
                .middle_of(self.ids.staff)
                .hover_image(self.imgs.icon_border_mo)
                .press_image(self.imgs.icon_border_press)
                .set(self.ids.weapon_7, ui_widgets)
                .was_clicked()
                {
                    self.character_body.weapon = Weapon::Staff;
                }

                // TODO: Load these from files (or from the server???).
                const SWORDSHIELD_DESC: &str = " MISSING ";
                const DAGGERS_DESC: &str = " MISSING ";
                const SWORD_DESC: &str = " MISSING ";
                const AXE_DESC: &str = " MISSING ";
                const HAMMER_DESC: &str = " MISSING ";
                const BOW_DESC: &str = " MISSING ";
                const STAFF_DESC: &str = " MISSING ";

                let (weapon_str, weapon_desc) = match self.character_body.weapon {
                    Weapon::SwordShield => ("Sword and Shield", SWORDSHIELD_DESC),
                    Weapon::Daggers => ("Daggers", DAGGERS_DESC),
                    Weapon::Sword => ("Sword", SWORD_DESC),
                    Weapon::Axe => ("Axe", AXE_DESC),
                    Weapon::Hammer => ("Hammer", HAMMER_DESC),
                    Weapon::Bow => ("Bow", BOW_DESC),
                    Weapon::Staff => ("Staff", STAFF_DESC),
                };
                Text::new(weapon_str)
                    .mid_top_with_margin_on(self.ids.creation_window, 370.0)
                    .font_size(30)
                    .color(TEXT_COLOR)
                    .set(self.ids.race_heading, ui_widgets);
                Text::new(weapon_desc)
                    .mid_top_with_margin_on(self.ids.creation_window, 410.0)
                    .w(500.0)
                    .font_size(20)
                    .font_id(self.fonts.opensans)
                    .color(TEXT_COLOR)
                    .wrap_by_word()
                    .set(self.ids.race_description, ui_widgets);
            }
            // 3 states/windows: 1: Skin & Eyes 2: Hair 3: Accessories
            // If one state is activated, the other ones collapse.
            // The title bar is the button to unfold/collapse the windows.
            // The BG Frame can be stretched to the needed size.

            // Window BG
            if let CreationState::Body(state) = self.creation_state {
                Text::new("Body Customization")
                    .mid_top_with_margin_on(self.ids.creation_window, 74.0)
                    .font_size(28)
                    .color(TEXT_COLOR)
                    .set(self.ids.select_window_title, ui_widgets);

                match state {
                    // Skin Eyes Open
                    BodyPart::SkinEyes => {
                        Image::new(self.imgs.skin_eyes_window)
                            .w_h(511.0, 333.0)
                            .mid_top_with_margin_on(self.ids.select_window_title, 60.0)
                            .set(self.ids.skin_eyes_window, ui_widgets);
                        // Open Window: Skin & Eyes
                        if Button::image(self.imgs.frame_open_mo)
                            .mid_top_with_margin_on(self.ids.skin_eyes_window, 0.0)
                            .w_h(511.0, 37.0)
                            //.hover_image(self.imgs.frame_open_mo)
                            //.press_image(self.imgs.frame_open_press)
                            .label("Skin & Eyes")
                            .label_color(TEXT_COLOR)
                            .label_y(conrod_core::position::Relative::Scalar(4.0))
                            .label_font_size(16)
                            .set(self.ids.skin_eyes_button, ui_widgets)
                            .was_clicked()
                        {
                            self.creation_state = CreationState::Body(BodyPart::SkinEyes);
                        }
                        // Closed: Hair
                        if Button::image(self.imgs.frame_closed)
                            .down_from(self.ids.skin_eyes_window, 5.0)
                            .w_h(511.0, 31.0)
                            .hover_image(self.imgs.frame_closed_mo)
                            .press_image(self.imgs.frame_closed_press)
                            .label("Hair")
                            .label_color(TEXT_COLOR)
                            .label_font_size(16)
                            .set(self.ids.hair_button, ui_widgets)
                            .was_clicked()
                        {
                            self.creation_state = CreationState::Body(BodyPart::Hair);
                        }
                        // Closed: Accessories
                        if Button::image(self.imgs.frame_closed)
                            .down_from(self.ids.hair_button, 5.0)
                            .w_h(511.0, 31.0)
                            .hover_image(self.imgs.frame_closed_mo)
                            .press_image(self.imgs.frame_closed_press)
                            .label("Accessories")
                            .label_color(TEXT_COLOR)
                            .label_font_size(16)
                            .set(self.ids.accessories_button, ui_widgets)
                            .was_clicked()
                        {
                            self.creation_state = CreationState::Body(BodyPart::Accessories);
                        }
                    }

                    // Hair Open
                    BodyPart::Hair => {
                        Image::new(self.imgs.hair_window)
                            .w_h(511.0, 400.0) //333.0
                            .down_from(self.ids.skin_eyes_button, 5.0)
                            .set(self.ids.hair_window, ui_widgets);
                        // Closed Window: Skin & Eyes
                        if Button::image(self.imgs.frame_closed)
                            .mid_top_with_margin_on(self.ids.select_window_title, 60.0)
                            .w_h(511.0, 31.0)
                            .hover_image(self.imgs.frame_closed_mo)
                            .press_image(self.imgs.frame_closed_press)
                            .label("Skin & Eyes")
                            .label_color(TEXT_COLOR)
                            .label_font_size(16)
                            .set(self.ids.skin_eyes_button, ui_widgets)
                            .was_clicked()
                        {
                            self.creation_state = CreationState::Body(BodyPart::SkinEyes);
                        }
                        // Open Window: Hair
                        if Button::image(self.imgs.frame_open_mo)
                            .mid_top_with_margin_on(self.ids.hair_window, 0.0)
                            .w_h(511.0, 37.0)
                            //.hover_image(self.imgs.frame_closed_mo)
                            //.press_image(self.imgs.frame_closed_press)
                            .label("Hair")
                            .label_color(TEXT_COLOR)
                            .label_y(conrod_core::position::Relative::Scalar(4.0))
                            .label_font_size(16)
                            .set(self.ids.hair_button, ui_widgets)
                            .was_clicked()
                        {
                            self.creation_state = CreationState::Body(BodyPart::Hair);
                        }
                        // Closed: Accessories
                        if Button::image(self.imgs.frame_closed)
                            .down_from(self.ids.hair_window, 5.0)
                            .w_h(511.0, 31.0)
                            .hover_image(self.imgs.frame_closed_mo)
                            .press_image(self.imgs.frame_closed_press)
                            .label("Accessories")
                            .label_color(TEXT_COLOR)
                            .label_font_size(16)
                            .set(self.ids.accessories_button, ui_widgets)
                            .was_clicked()
                        {
                            self.creation_state = CreationState::Body(BodyPart::Accessories);
                        }
                    }

                    // Open: Accessories
                    BodyPart::Accessories => {
                        Image::new(self.imgs.hair_window)
                            .w_h(511.0, 333.0)
                            .down_from(self.ids.hair_button, 5.0)
                            .set(self.ids.accessories_window, ui_widgets);
                        // Closed Window: Skin & Eyes
                        if Button::image(self.imgs.frame_closed)
                            .mid_top_with_margin_on(self.ids.select_window_title, 60.0)
                            .w_h(511.0, 31.0)
                            .hover_image(self.imgs.frame_closed_mo)
                            .press_image(self.imgs.frame_closed_press)
                            .label("Skin & Eyes")
                            .label_color(TEXT_COLOR)
                            .label_font_size(16)
                            .set(self.ids.skin_eyes_button, ui_widgets)
                            .was_clicked()
                        {
                            self.creation_state = CreationState::Body(BodyPart::SkinEyes);
                        }
                        // Closed: Hair
                        if Button::image(self.imgs.frame_closed)
                            .down_from(self.ids.skin_eyes_button, 5.0)
                            .w_h(511.0, 31.0)
                            .hover_image(self.imgs.frame_closed_mo)
                            .press_image(self.imgs.frame_closed_press)
                            .label("Hair")
                            .label_color(TEXT_COLOR)
                            .label_font_size(16)
                            .set(self.ids.hair_button, ui_widgets)
                            .was_clicked()
                        {
                            self.creation_state = CreationState::Body(BodyPart::Hair);
                        }
                        // Open: Accessories
                        if Button::image(self.imgs.frame_open_mo)
                            .down_from(self.ids.hair_button, 5.0)
                            .w_h(511.0, 37.0)
                            //.hover_image(self.imgs.frame_closed_mo)
                            //.press_image(self.imgs.frame_closed_press)
                            .label("Accessories")
                            .label_y(conrod_core::position::Relative::Scalar(4.0))
                            .label_color(TEXT_COLOR)
                            .label_font_size(16)
                            .set(self.ids.accessories_button, ui_widgets)
                            .was_clicked()
                        {
                            self.creation_state = CreationState::Body(BodyPart::Accessories);
                        }
                    }
                }

                // Body Customization Window Contents
                match state {
                    BodyPart::SkinEyes => {
                        // Skin Color: Text, Brightness Slider, Picker
                        Text::new("Skin Color")
                            .top_left_with_margins_on(self.ids.skin_rect, 0.0, -250.0)
                            .font_size(25)
                            .color(TEXT_COLOR)
                            .set(self.ids.skin_color_text, ui_widgets);
                        // TODO: Align Buttons here.
                        // Users set a variable to a value from 0-14.
                        // Depending on the race another color will be chosen.
                        // Only the BG image (190x114 -> 2px border!) changes depending on the race.
                        Rectangle::fill_with([192.0, 116.0], color::WHITE)
                            .top_right_with_margins_on(self.ids.skin_eyes_window, 60.0, 30.0)
                            .color(TEXT_COLOR)
                            .set(self.ids.skin_rect, ui_widgets);

                        // TODO: Slider
                        // Sliders actually change the Alpha-Level of the main colour chosen above.
                        // -> They will appear "brighter", therefore the sliders are labeled "Brightness".
                        Image::new(self.imgs.slider_range)
                            .w_h(208.0, 12.0)
                            .bottom_left_with_margins_on(self.ids.skin_rect, 10.0, -255.0)
                            .set(self.ids.skin_color_slider_range, ui_widgets);

                        Image::new(self.imgs.slider_indicator)
                            .w_h(10.0, 22.0)
                            .middle_of(self.ids.skin_color_slider_range)
                            .set(self.ids.skin_color_slider_indicator, ui_widgets);

                        Text::new("Brightness")
                            .top_left_with_margins_on(self.ids.skin_color_slider_range, -27.0, 0.0)
                            .color(TEXT_COLOR)
                            .font_size(14)
                            .set(self.ids.skin_color_slider_text, ui_widgets);

                        // Eye Color: Text, Brightness Slider, Picker
                        Text::new("Eye Color")
                            .top_left_with_margins_on(self.ids.eyes_rect, 0.0, -250.0)
                            .font_size(25)
                            .color(TEXT_COLOR)
                            .set(self.ids.eye_color_text, ui_widgets);
                        // TODO: Align 16 Buttons here.
                        // Users set a variable to a value from 0-14.
                        // Depending on the race another color will be chosen.
                        // Only the BG image (190x114 -> 2px border!) changes depending on the race.
                        Rectangle::fill_with([192.0, 116.0], color::WHITE)
                            .top_right_with_margins_on(self.ids.skin_eyes_window, 186.0, 30.0)
                            .color(TEXT_COLOR)
                            .set(self.ids.eyes_rect, ui_widgets);

                        // TODO: Slider

                        Image::new(self.imgs.slider_range)
                            .w_h(208.0, 12.0)
                            .bottom_left_with_margins_on(self.ids.eyes_rect, 10.0, -255.0)
                            .set(self.ids.eye_color_slider_range, ui_widgets);

                        Image::new(self.imgs.slider_indicator)
                            .w_h(10.0, 22.0)
                            .middle_of(self.ids.eye_color_slider_range)
                            .set(self.ids.eye_color_slider_indicator, ui_widgets);

                        Text::new("Brightness")
                            .top_left_with_margins_on(self.ids.eye_color_slider_range, -27.0, 0.0)
                            .color(TEXT_COLOR)
                            .font_size(14)
                            .set(self.ids.eye_color_slider_text, ui_widgets);
                    }

                    // Hair

                    // Hair Styles -> Arrows
                    // Hair Color -> Picker
                    // Eye Brow Style -> Arrow
                    // Facial Hair -> Picker (Only active for males!)
                    BodyPart::Hair => {
                        // Hair
                        Text::new("Hair Style")
                            .mid_top_with_margin_on(self.ids.hair_window, 60.0)
                            .color(TEXT_COLOR)
                            .font_size(24)
                            .set(self.ids.hair_style_text, ui_widgets);
                        if Button::image(self.imgs.arrow_right)
                            .w_h(986.0 * 0.02, 1024.0 * 0.02)
                            .hover_image(self.imgs.arrow_right_mo)
                            .press_image(self.imgs.arrow_right_press)
                            .right_from(self.ids.hair_style_text, 15.0)
                            .set(self.ids.hair_style_arrow_r, ui_widgets)
                            .was_clicked()
                        {};
                        if Button::image(self.imgs.arrow_left)
                            .w_h(986.0 * 0.02, 1024.0 * 0.02)
                            .hover_image(self.imgs.arrow_left_mo)
                            .press_image(self.imgs.arrow_left_press)
                            .left_from(self.ids.hair_style_text, 15.0)
                            .set(self.ids.hair_style_arrow_l, ui_widgets)
                            .was_clicked()
                        {};

                        Text::new("Hair Color")
                            .top_left_with_margins_on(self.ids.hair_color_picker_bg, 0.0, -250.0)
                            .font_size(25)
                            .color(TEXT_COLOR)
                            .set(self.ids.hair_color_text, ui_widgets);

                        Rectangle::fill_with([192.0, 116.0], color::WHITE)
                            .top_right_with_margins_on(self.ids.hair_window, 114.0, 30.0)
                            .color(TEXT_COLOR)
                            .set(self.ids.hair_color_picker_bg, ui_widgets);

                        Image::new(self.imgs.slider_range)
                            .w_h(208.0, 12.0)
                            .bottom_left_with_margins_on(
                                self.ids.hair_color_picker_bg,
                                10.0,
                                -255.0,
                            )
                            .set(self.ids.hair_color_slider_range, ui_widgets);

                        Image::new(self.imgs.slider_indicator)
                            .w_h(10.0, 22.0)
                            .middle_of(self.ids.hair_color_slider_range)
                            .set(self.ids.hair_color_slider_indicator, ui_widgets);

                        Text::new("Brightness")
                            .top_left_with_margins_on(self.ids.hair_color_slider_range, -27.0, 0.0)
                            .color(TEXT_COLOR)
                            .font_size(14)
                            .set(self.ids.hair_color_slider_text, ui_widgets);
                        // Eyebrows
                        Text::new("Eyebrow Style")
                            .mid_top_with_margin_on(self.ids.hair_window, 280.0)
                            .color(TEXT_COLOR)
                            .font_size(24)
                            .set(self.ids.eyebrow_style_text, ui_widgets);
                        if Button::image(self.imgs.arrow_right)
                            .w_h(986.0 * 0.02, 1024.0 * 0.02)
                            .hover_image(self.imgs.arrow_right_mo)
                            .press_image(self.imgs.arrow_right_press)
                            .right_from(self.ids.eyebrow_style_text, 15.0)
                            .set(self.ids.eyebrow_arrow_r, ui_widgets)
                            .was_clicked()
                        {};
                        if Button::image(self.imgs.arrow_left)
                            .w_h(986.0 * 0.02, 1024.0 * 0.02)
                            .hover_image(self.imgs.arrow_left_mo)
                            .press_image(self.imgs.arrow_left_press)
                            .left_from(self.ids.eyebrow_style_text, 15.0)
                            .set(self.ids.eyebrow_arrow_l, ui_widgets)
                            .was_clicked()
                        {};
                        // Beard -> Only active if "male" was chosen.
                        if let BodyType::Male = self.character_body.body_type {
                            Text::new("Beard Style")
                                .mid_top_with_margin_on(self.ids.hair_window, 340.0)
                                .color(TEXT_COLOR)
                                .font_size(24)
                                .set(self.ids.beard_style_text, ui_widgets);
                            if Button::image(self.imgs.arrow_right)
                                .w_h(986.0 * 0.02, 1024.0 * 0.02)
                                .hover_image(self.imgs.arrow_right_mo)
                                .press_image(self.imgs.arrow_right_press)
                                .right_from(self.ids.beard_style_text, 15.0)
                                .set(self.ids.beard_arrow_r, ui_widgets)
                                .was_clicked()
                            {};
                            if Button::image(self.imgs.arrow_left)
                                .w_h(986.0 * 0.02, 1024.0 * 0.02)
                                .hover_image(self.imgs.arrow_left_mo)
                                .press_image(self.imgs.arrow_left_press)
                                .left_from(self.ids.beard_style_text, 15.0)
                                .set(self.ids.beard_arrow_l, ui_widgets)
                                .was_clicked()
                            {};
                        }
                    }

                    // Accessories

                    // Accessory Picker -> Arrows (Name changes with race!)
                    // Color -> Picker
                    // Brightness -> Slider
                    BodyPart::Accessories => {
                        match self.character_body.race {
                            Race::Human => {
                                Text::new("Head Band")
                                    .mid_top_with_margin_on(self.ids.accessories_window, 60.0)
                                    .color(TEXT_COLOR)
                                    .font_size(24)
                                    .set(self.ids.warpaint_text, ui_widgets);
                                if Button::image(self.imgs.arrow_right)
                                    .w_h(986.0 * 0.02, 1024.0 * 0.02)
                                    .hover_image(self.imgs.arrow_right_mo)
                                    .press_image(self.imgs.arrow_right_press)
                                    .right_from(self.ids.warpaint_text, 15.0)
                                    .set(self.ids.warpaint_arrow_r, ui_widgets)
                                    .was_clicked()
                                {};
                                if Button::image(self.imgs.arrow_left)
                                    .w_h(986.0 * 0.02, 1024.0 * 0.02)
                                    .hover_image(self.imgs.arrow_left_mo)
                                    .press_image(self.imgs.arrow_left_press)
                                    .left_from(self.ids.warpaint_text, 15.0)
                                    .set(self.ids.warpaint_arrow_l, ui_widgets)
                                    .was_clicked()
                                {};

                                Text::new("Color")
                                    .top_left_with_margins_on(
                                        self.ids.warpaint_color_picker_bg,
                                        0.0,
                                        -250.0,
                                    )
                                    .font_size(25)
                                    .color(TEXT_COLOR)
                                    .set(self.ids.warpaint_color_text, ui_widgets);

                                Rectangle::fill_with([192.0, 116.0], color::WHITE)
                                    .top_right_with_margins_on(
                                        self.ids.accessories_window,
                                        114.0,
                                        30.0,
                                    )
                                    .color(TEXT_COLOR)
                                    .set(self.ids.warpaint_color_picker_bg, ui_widgets);

                                Image::new(self.imgs.slider_range)
                                    .w_h(208.0, 12.0)
                                    .bottom_left_with_margins_on(
                                        self.ids.warpaint_color_picker_bg,
                                        10.0,
                                        -255.0,
                                    )
                                    .set(self.ids.warpaint_slider_range, ui_widgets);

                                Image::new(self.imgs.slider_indicator)
                                    .w_h(10.0, 22.0)
                                    .middle_of(self.ids.warpaint_slider_range)
                                    .set(self.ids.warpaint_slider_indicator, ui_widgets);

                                Text::new("Brightness")
                                    .top_left_with_margins_on(
                                        self.ids.warpaint_slider_range,
                                        -27.0,
                                        0.0,
                                    )
                                    .color(TEXT_COLOR)
                                    .font_size(14)
                                    .set(self.ids.warpaint_slider_text, ui_widgets);
                            }
                            Race::Orc => {
                                Text::new("Head Band")
                                    .mid_top_with_margin_on(self.ids.accessories_window, 60.0)
                                    .color(TEXT_COLOR)
                                    .font_size(24)
                                    .set(self.ids.warpaint_text, ui_widgets);
                                if Button::image(self.imgs.arrow_right)
                                    .w_h(986.0 * 0.02, 1024.0 * 0.02)
                                    .hover_image(self.imgs.arrow_right_mo)
                                    .press_image(self.imgs.arrow_right_press)
                                    .right_from(self.ids.warpaint_text, 15.0)
                                    .set(self.ids.warpaint_arrow_r, ui_widgets)
                                    .was_clicked()
                                {};
                                if Button::image(self.imgs.arrow_left)
                                    .w_h(986.0 * 0.02, 1024.0 * 0.02)
                                    .hover_image(self.imgs.arrow_left_mo)
                                    .press_image(self.imgs.arrow_left_press)
                                    .left_from(self.ids.warpaint_text, 15.0)
                                    .set(self.ids.warpaint_arrow_l, ui_widgets)
                                    .was_clicked()
                                {};

                                Text::new("Color")
                                    .top_left_with_margins_on(
                                        self.ids.warpaint_color_picker_bg,
                                        0.0,
                                        -250.0,
                                    )
                                    .font_size(25)
                                    .color(TEXT_COLOR)
                                    .set(self.ids.warpaint_color_text, ui_widgets);

                                Rectangle::fill_with([192.0, 116.0], color::WHITE)
                                    .top_right_with_margins_on(
                                        self.ids.accessories_window,
                                        114.0,
                                        30.0,
                                    )
                                    .color(TEXT_COLOR)
                                    .set(self.ids.warpaint_color_picker_bg, ui_widgets);

                                Image::new(self.imgs.slider_range)
                                    .w_h(208.0, 12.0)
                                    .bottom_left_with_margins_on(
                                        self.ids.warpaint_color_picker_bg,
                                        10.0,
                                        -255.0,
                                    )
                                    .set(self.ids.warpaint_slider_range, ui_widgets);

                                Image::new(self.imgs.slider_indicator)
                                    .w_h(10.0, 22.0)
                                    .middle_of(self.ids.warpaint_slider_range)
                                    .set(self.ids.warpaint_slider_indicator, ui_widgets);

                                Text::new("Brightness")
                                    .top_left_with_margins_on(
                                        self.ids.warpaint_slider_range,
                                        -27.0,
                                        0.0,
                                    )
                                    .color(TEXT_COLOR)
                                    .font_size(14)
                                    .set(self.ids.warpaint_slider_text, ui_widgets);
                            }
                            Race::Elf => {
                                Text::new("Tribe Markings")
                                    .mid_top_with_margin_on(self.ids.accessories_window, 60.0)
                                    .color(TEXT_COLOR)
                                    .font_size(24)
                                    .set(self.ids.warpaint_text, ui_widgets);
                                if Button::image(self.imgs.arrow_right)
                                    .w_h(986.0 * 0.02, 1024.0 * 0.02)
                                    .hover_image(self.imgs.arrow_right_mo)
                                    .press_image(self.imgs.arrow_right_press)
                                    .right_from(self.ids.warpaint_text, 15.0)
                                    .set(self.ids.warpaint_arrow_r, ui_widgets)
                                    .was_clicked()
                                {};
                                if Button::image(self.imgs.arrow_left)
                                    .w_h(986.0 * 0.02, 1024.0 * 0.02)
                                    .hover_image(self.imgs.arrow_left_mo)
                                    .press_image(self.imgs.arrow_left_press)
                                    .left_from(self.ids.warpaint_text, 15.0)
                                    .set(self.ids.warpaint_arrow_l, ui_widgets)
                                    .was_clicked()
                                {};

                                Text::new("Color")
                                    .top_left_with_margins_on(
                                        self.ids.warpaint_color_picker_bg,
                                        0.0,
                                        -250.0,
                                    )
                                    .font_size(25)
                                    .color(TEXT_COLOR)
                                    .set(self.ids.warpaint_color_text, ui_widgets);

                                Rectangle::fill_with([192.0, 116.0], color::WHITE)
                                    .top_right_with_margins_on(
                                        self.ids.accessories_window,
                                        114.0,
                                        30.0,
                                    )
                                    .color(TEXT_COLOR)
                                    .set(self.ids.warpaint_color_picker_bg, ui_widgets);

                                Image::new(self.imgs.slider_range)
                                    .w_h(208.0, 12.0)
                                    .bottom_left_with_margins_on(
                                        self.ids.warpaint_color_picker_bg,
                                        10.0,
                                        -255.0,
                                    )
                                    .set(self.ids.warpaint_slider_range, ui_widgets);

                                Image::new(self.imgs.slider_indicator)
                                    .w_h(10.0, 22.0)
                                    .middle_of(self.ids.warpaint_slider_range)
                                    .set(self.ids.warpaint_slider_indicator, ui_widgets);

                                Text::new("Brightness")
                                    .top_left_with_margins_on(
                                        self.ids.warpaint_slider_range,
                                        -27.0,
                                        0.0,
                                    )
                                    .color(TEXT_COLOR)
                                    .font_size(14)
                                    .set(self.ids.warpaint_slider_text, ui_widgets);
                            }
                            Race::Dwarf => {
                                Text::new("War Paint")
                                    .mid_top_with_margin_on(self.ids.accessories_window, 60.0)
                                    .color(TEXT_COLOR)
                                    .font_size(24)
                                    .set(self.ids.warpaint_text, ui_widgets);
                                if Button::image(self.imgs.arrow_right)
                                    .w_h(986.0 * 0.02, 1024.0 * 0.02)
                                    .hover_image(self.imgs.arrow_right_mo)
                                    .press_image(self.imgs.arrow_right_press)
                                    .right_from(self.ids.warpaint_text, 15.0)
                                    .set(self.ids.warpaint_arrow_r, ui_widgets)
                                    .was_clicked()
                                {};
                                if Button::image(self.imgs.arrow_left)
                                    .w_h(986.0 * 0.02, 1024.0 * 0.02)
                                    .hover_image(self.imgs.arrow_left_mo)
                                    .press_image(self.imgs.arrow_left_press)
                                    .left_from(self.ids.warpaint_text, 15.0)
                                    .set(self.ids.warpaint_arrow_l, ui_widgets)
                                    .was_clicked()
                                {};

                                Text::new("Color")
                                    .top_left_with_margins_on(
                                        self.ids.warpaint_color_picker_bg,
                                        0.0,
                                        -250.0,
                                    )
                                    .font_size(25)
                                    .color(TEXT_COLOR)
                                    .set(self.ids.warpaint_color_text, ui_widgets);

                                Rectangle::fill_with([192.0, 116.0], color::WHITE)
                                    .top_right_with_margins_on(
                                        self.ids.accessories_window,
                                        114.0,
                                        30.0,
                                    )
                                    .color(TEXT_COLOR)
                                    .set(self.ids.warpaint_color_picker_bg, ui_widgets);

                                Image::new(self.imgs.slider_range)
                                    .w_h(208.0, 12.0)
                                    .bottom_left_with_margins_on(
                                        self.ids.warpaint_color_picker_bg,
                                        10.0,
                                        -255.0,
                                    )
                                    .set(self.ids.warpaint_slider_range, ui_widgets);

                                Image::new(self.imgs.slider_indicator)
                                    .w_h(10.0, 22.0)
                                    .middle_of(self.ids.warpaint_slider_range)
                                    .set(self.ids.warpaint_slider_indicator, ui_widgets);

                                Text::new("Brightness")
                                    .top_left_with_margins_on(
                                        self.ids.warpaint_slider_range,
                                        -27.0,
                                        0.0,
                                    )
                                    .color(TEXT_COLOR)
                                    .font_size(14)
                                    .set(self.ids.warpaint_slider_text, ui_widgets);
                            }
                            Race::Undead => {
                                Text::new("Teeth")
                                    .mid_top_with_margin_on(self.ids.accessories_window, 60.0)
                                    .color(TEXT_COLOR)
                                    .font_size(24)
                                    .set(self.ids.warpaint_text, ui_widgets);
                                if Button::image(self.imgs.arrow_right)
                                    .w_h(986.0 * 0.02, 1024.0 * 0.02)
                                    .hover_image(self.imgs.arrow_right_mo)
                                    .press_image(self.imgs.arrow_right_press)
                                    .right_from(self.ids.warpaint_text, 15.0)
                                    .set(self.ids.warpaint_arrow_r, ui_widgets)
                                    .was_clicked()
                                {};
                                if Button::image(self.imgs.arrow_left)
                                    .w_h(986.0 * 0.02, 1024.0 * 0.02)
                                    .hover_image(self.imgs.arrow_left_mo)
                                    .press_image(self.imgs.arrow_left_press)
                                    .left_from(self.ids.warpaint_text, 15.0)
                                    .set(self.ids.warpaint_arrow_l, ui_widgets)
                                    .was_clicked()
                                {};

                                Text::new("Color")
                                    .top_left_with_margins_on(
                                        self.ids.warpaint_color_picker_bg,
                                        0.0,
                                        -250.0,
                                    )
                                    .font_size(25)
                                    .color(TEXT_COLOR)
                                    .set(self.ids.warpaint_color_text, ui_widgets);

                                Rectangle::fill_with([192.0, 116.0], color::WHITE)
                                    .top_right_with_margins_on(
                                        self.ids.accessories_window,
                                        114.0,
                                        30.0,
                                    )
                                    .color(TEXT_COLOR)
                                    .set(self.ids.warpaint_color_picker_bg, ui_widgets);

                                Image::new(self.imgs.slider_range)
                                    .w_h(208.0, 12.0)
                                    .bottom_left_with_margins_on(
                                        self.ids.warpaint_color_picker_bg,
                                        10.0,
                                        -255.0,
                                    )
                                    .set(self.ids.warpaint_slider_range, ui_widgets);

                                Image::new(self.imgs.slider_indicator)
                                    .w_h(10.0, 22.0)
                                    .middle_of(self.ids.warpaint_slider_range)
                                    .set(self.ids.warpaint_slider_indicator, ui_widgets);

                                Text::new("Brightness")
                                    .top_left_with_margins_on(
                                        self.ids.warpaint_slider_range,
                                        -27.0,
                                        0.0,
                                    )
                                    .color(TEXT_COLOR)
                                    .font_size(14)
                                    .set(self.ids.warpaint_slider_text, ui_widgets);
                            }
                            Race::Danari => {
                                Text::new("Horns")
                                    .mid_top_with_margin_on(self.ids.accessories_window, 60.0)
                                    .color(TEXT_COLOR)
                                    .font_size(24)
                                    .set(self.ids.warpaint_text, ui_widgets);
                                if Button::image(self.imgs.arrow_right)
                                    .w_h(986.0 * 0.02, 1024.0 * 0.02)
                                    .hover_image(self.imgs.arrow_right_mo)
                                    .press_image(self.imgs.arrow_right_press)
                                    .right_from(self.ids.warpaint_text, 15.0)
                                    .set(self.ids.warpaint_arrow_r, ui_widgets)
                                    .was_clicked()
                                {};
                                if Button::image(self.imgs.arrow_left)
                                    .w_h(986.0 * 0.02, 1024.0 * 0.02)
                                    .hover_image(self.imgs.arrow_left_mo)
                                    .press_image(self.imgs.arrow_left_press)
                                    .left_from(self.ids.warpaint_text, 15.0)
                                    .set(self.ids.warpaint_arrow_l, ui_widgets)
                                    .was_clicked()
                                {};

                                Text::new("Color")
                                    .top_left_with_margins_on(
                                        self.ids.warpaint_color_picker_bg,
                                        0.0,
                                        -250.0,
                                    )
                                    .font_size(25)
                                    .color(TEXT_COLOR)
                                    .set(self.ids.warpaint_color_text, ui_widgets);

                                Rectangle::fill_with([192.0, 116.0], color::WHITE)
                                    .top_right_with_margins_on(
                                        self.ids.accessories_window,
                                        114.0,
                                        30.0,
                                    )
                                    .color(TEXT_COLOR)
                                    .set(self.ids.warpaint_color_picker_bg, ui_widgets);

                                Image::new(self.imgs.slider_range)
                                    .w_h(208.0, 12.0)
                                    .bottom_left_with_margins_on(
                                        self.ids.warpaint_color_picker_bg,
                                        10.0,
                                        -255.0,
                                    )
                                    .set(self.ids.warpaint_slider_range, ui_widgets);

                                Image::new(self.imgs.slider_indicator)
                                    .w_h(10.0, 22.0)
                                    .middle_of(self.ids.warpaint_slider_range)
                                    .set(self.ids.warpaint_slider_indicator, ui_widgets);

                                Text::new("Brightness")
                                    .top_left_with_margins_on(
                                        self.ids.warpaint_slider_range,
                                        -27.0,
                                        0.0,
                                    )
                                    .color(TEXT_COLOR)
                                    .font_size(14)
                                    .set(self.ids.warpaint_slider_text, ui_widgets);
                            }
                        } // match Race fin
                    }
                }
            }
        }

        events
    }

    pub fn handle_event(&mut self, event: ui::Event) {
        self.ui.handle_event(event);
    }

    pub fn maintain(&mut self, renderer: &mut Renderer) -> Vec<Event> {
        let events = self.update_layout();
        self.ui.maintain(renderer);
        events
    }

    pub fn render(&self, renderer: &mut Renderer) {
        self.ui.render(renderer);
    }
}
