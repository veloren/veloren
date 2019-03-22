use conrod_core::color::TRANSPARENT;
use crate::{
    render::Renderer,
    ui::{self, ScaleMode, Ui},
    window::Window,
};
use conrod_core::{
    color,
    event::Input,
    image::Id as ImgId,
    text::font::Id as FontId,
    widget::{text_box::Event as TextBoxEvent, Button, Rectangle, Image, Text, TextBox, TitleBar},
    widget_ids, Borderable, Color, Colorable, Labelable, Positionable, Sizeable, Widget,
};

widget_ids! {
    struct Ids {
        // Background and logo
        bg_selection,
        bg_creation,
        v_logo,
        alpha_version,

        // Windows
        selection_window,
        creation_window,
        select_race_title,
        select_weapon_title,
        race_heading,
        race_description,
        weapon_heading,
        weapon_description,
        races_bg,
        gender_bg,
        desc_bg,

        // Buttons
        enter_world_button,
        back_button,
        logout_button,
        create_character_button,
        delete_button,
        create_button,
        character_name_input,
        race_1,
        race_2,
        race_3,
        race_4,
        race_5,
        race_6,
        sex_1,
        sex_2,

        //test_chars
        test_char_l_button,
        test_char_l_big,
        //test_char_m_button,
        //test_char_r_button,

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
        dw_daggers,
        sword_shield,
        sword,
        axe,
        hammer,
        bow,
        // Arrows
        arrow_left,
        arrow_right,

    }
}

struct Imgs {
    v_logo: ImgId,
    bg_selection: ImgId,
    bg_creation: ImgId,
    button_dark: ImgId,
    button_dark_hover: ImgId,
    button_dark_press: ImgId,
    button_dark_red: ImgId,
    button_dark_red_hover: ImgId,
    button_dark_red_press: ImgId,
    selection_window: ImgId,
    test_char_l_button: ImgId,
    test_char_l_big: ImgId,
    name_input: ImgId,
    creation_window: ImgId,
    desc_bg: ImgId,
    //test_char_m_button: ImgId,
    //test_char_r_button: ImgId,
    // Race Icons
    male: ImgId,
    female: ImgId,
    human_m: ImgId,
    human_f: ImgId,
    orc_m: ImgId,
    orc_f: ImgId,
    dwarf_m: ImgId,
    dwarf_f: ImgId,
    undead_m: ImgId,
    undead_f: ImgId,
    elf_m: ImgId,
    elf_f: ImgId,
    danari_m: ImgId,
    danari_f: ImgId,
    // Weapon Icons
    dw_daggers: ImgId,
    sword_shield: ImgId,
    sword: ImgId,
    axe: ImgId,
    hammer: ImgId,
    bow: ImgId,
    // Arrows
    arrow_left: ImgId,
    arrow_left_mo: ImgId,
    arrow_left_press: ImgId,
    arrow_left_grey: ImgId,
    arrow_right: ImgId,
    arrow_right_mo: ImgId,
    arrow_right_press: ImgId,
    arrow_right_grey: ImgId,
    // Icon Borders
    icon_border: ImgId,
    icon_border_mo: ImgId,
    icon_border_press: ImgId,
    icon_border_pressed: ImgId,
}
impl Imgs {
    fn new(ui: &mut Ui, renderer: &mut Renderer) -> Imgs {
        let mut load = |filename| {
            let image = image::open(
                &[
                    env!("CARGO_MANIFEST_DIR"),
                    "/test_assets/ui/char_selection/",
                    filename,
                ]
                .concat(),
            )
            .unwrap();
            ui.new_image(renderer, &image).unwrap()
        };
        Imgs {
            v_logo: load("v_logo.png"),
            bg_selection: load("bg_selection.png"),
            bg_creation: load("bg_creation.png"),
            selection_window: load("selection_frame.png"),
            button_dark: load("buttons/button_dark.png"),
            button_dark_hover: load("buttons/button_dark_hover.png"),
            button_dark_press: load("buttons/button_dark_press.png"),
            button_dark_red: load("buttons/button_dark_red.png"),
            button_dark_red_hover: load("buttons/button_dark_red_hover.png"),
            button_dark_red_press: load("buttons/button_dark_red_press.png"),
            test_char_l_button: load("test_char_l.png"),
            test_char_l_big: load("test_char_l_big.png"),
            name_input: load("input_bg.png"),
            creation_window: load("creation_window.png"),
            desc_bg: load("desc_bg.png"),
            //test_char_m_button: load("test_char_m_button"),
            //test_char_r_button: load("test_char_r_button"),
            // Race Icons
            male: load("icons/male.png"),
            female: load("icons/female.png"),
            human_m: load("icons/human_m.png"),
            human_f: load("icons/human_f.png"),
            orc_m: load("icons/orc_m.png"),
            orc_f: load("icons/orc_f.png"),
            dwarf_m: load("icons/dwarf_m.png"),
            dwarf_f: load("icons/dwarf_f.png"),
            undead_m: load("icons/ud_m.png"),
            undead_f: load("icons/ud_f.png"),
            elf_m: load("icons/elf_m.png"),
            elf_f: load("icons/elf_f.png"),
            danari_m: load("icons/danari_m.png"),
            danari_f: load("icons/danari_f.png"),
            // Weapon Icons
            dw_daggers: load("missing_icon.png"),
            sword_shield: load("missing_icon.png"),
            sword: load("missing_icon.png"),
            axe: load("missing_icon.png"),
            hammer: load("missing_icon.png"),
            bow: load("missing_icon.png"),
            // Arrows
            arrow_left: load("icons/arrow_left.png"),
            arrow_left_mo: load("icons/arrow_left_mo.png"),
            arrow_left_press: load("icons/arrow_left_press.png"),
            arrow_left_grey: load("icons/arrow_left_grey.png"),
            arrow_right: load("icons/arrow_right.png"),
            arrow_right_mo: load("icons/arrow_right_mo.png"),
            arrow_right_press: load("icons/arrow_right_press.png"),
            arrow_right_grey: load("icons/arrow_right_grey.png"),
            // Icon Borders
            icon_border: load("buttons/border.png"),
            icon_border_mo: load("buttons/border_mo.png"),
            icon_border_press: load("buttons/border_press.png"),
            icon_border_pressed: load("buttons/border_pressed.png"),
        }
    }
}

enum CreationState {
    Race,
    Weapon,
    Body,
}
enum Races {
    Human,
    Orc,
    Elf,
    Dwarf,
    Undead,
    Danari,
}
enum Sex {
    Male,
    Female,
    Undefined,
}
enum Weapons {
    Daggers,
    SwordShield,
    Sword,
    Axe,
    Hammer,
    Bow,
    Staff,
}

pub enum Event {
    Logout,
    Play,
}

pub struct CharSelectionUi {
    ui: Ui,
    ids: Ids,
    imgs: Imgs,
    font_metamorph: FontId,
    font_whitney: FontId,
    character_creation: bool,
    selected_char_no: Option<i32>,
    race: Races,
    sex: Sex,
    weapon: Weapons,
    creation_state: CreationState,
}

impl CharSelectionUi {
    pub fn new(window: &mut Window) -> Self {
        let mut ui = Ui::new(window).unwrap();
        // TODO: adjust/remove this, right now it is used to demonstrate window scaling functionality
        ui.scaling_mode(ScaleMode::RelativeToWindow([1920.0, 1080.0].into()));
        // Generate ids
        let ids = Ids::new(ui.id_generator());
        // Load images
        let imgs = Imgs::new(&mut ui, window.renderer_mut());
        // Load fonts
        let font_whitney = ui.new_font(
            conrod_core::text::font::from_file(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/test_assets/font/Whitney-Book.ttf"
            ))
            .unwrap(),
        );
        let font_metamorph = ui.new_font(
            conrod_core::text::font::from_file(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/test_assets/font/Metamorphous-Regular.ttf"
            ))
            .unwrap(),
        );
        Self {
            ui,
            imgs,
            ids,
            font_metamorph,
            font_whitney,
            character_creation: false,
            selected_char_no: None,
            race: Races::Human,
            sex: Sex::Male,
            weapon: Weapons::Sword,
            creation_state: CreationState::Race,
        }
    }

    fn update_layout(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        let ref mut ui_widgets = self.ui.set_widgets();

        // Character Selection /////////////////
        // Supposed functionality:
        // 3d rendered characters have to be clicked for selection
        // Selected characters will appear in the selection window
        // the selection window is only active when there are >0 characters on the server
        // after logging into the server the character that was played last will be selected automatically
        // if >1 characters are on the server but none of them was logged in last the one that was created last will be selected
        // if the no. of characters = character_limit the "Create Character" button won't be clickable anymore

        // Background Image
        if !self.character_creation {

            Image::new(self.imgs.bg_selection)
                .middle_of(ui_widgets.window)
                .set(self.ids.bg_selection, ui_widgets);

            // Logout_Button
            if Button::image(self.imgs.button_dark)
                .bottom_left_with_margins_on(self.ids.bg_selection, 10.0, 10.0)
                .w_h(150.0, 40.0)
                .hover_image(self.imgs.button_dark_hover)
                .press_image(self.imgs.button_dark_press)
                .label("Logout")
                .label_rgba(220.0, 220.0, 220.0, 0.8)
                .label_font_size(18)
                .label_y(conrod_core::position::Relative::Scalar(3.0))
                .set(self.ids.logout_button, ui_widgets)
                .was_clicked()
            {
                events.push(Event::Logout)
            };

            // Create_Character_Button
            if Button::image(self.imgs.button_dark)
                .mid_bottom_with_margin_on(self.ids.bg_selection, 10.0)
                .w_h(270.0, 50.0)
                .hover_image(self.imgs.button_dark_hover)
                .press_image(self.imgs.button_dark_press)
                .label("Create Character")
                .label_rgba(220.0, 220.0, 220.0, 0.8)
                .label_font_size(20)
                .label_y(conrod_core::position::Relative::Scalar(3.0))
                .set(self.ids.create_character_button, ui_widgets)
                .was_clicked()
            {
                self.character_creation = true;
                self.selected_char_no = None;

            };
            //Test_Characters
            if Button::image(self.imgs.test_char_l_button)
                .bottom_left_with_margins_on(self.ids.bg_selection, 555.0, 716.0)
                .w_h(95.0, 130.0)
                .hover_image(self.imgs.test_char_l_button)
                .press_image(self.imgs.test_char_l_button)
                .set(self.ids.test_char_l_button, ui_widgets)
                .was_clicked()
            {
                self.selected_char_no = Some(1);
                self.creation_state = CreationState::Race;
            };

            // Veloren Logo and Alpha Version
            Button::image(self.imgs.v_logo)
                .w_h(346.0, 111.0)
                .top_left_with_margins_on(self.ids.bg_selection, 30.0, 40.0)
                .label("Alpha 0.1")
                .label_rgba(255.0, 255.0, 255.0, 1.0)
                .label_font_size(10)
                .label_y(conrod_core::position::Relative::Scalar(-40.0))
                .label_x(conrod_core::position::Relative::Scalar(-100.0))
                .set(self.ids.v_logo, ui_widgets);

            if let Some(no) = self.selected_char_no {
                // Selection_Window
                Image::new(self.imgs.selection_window)
                    .w_h(522.0, 722.0)
                    .mid_right_with_margin_on(ui_widgets.window, 10.0)
                    .set(self.ids.selection_window, ui_widgets);

                // Selected Character
                if no == 1 {
                    Image::new(self.imgs.test_char_l_big)
                        .middle_of(self.ids.selection_window)
                        .set(self.ids.test_char_l_big, ui_widgets);
                }

                // Enter World Button
                if Button::image(self.imgs.button_dark)
                    .mid_bottom_with_margin_on(self.ids.selection_window, 65.0)
                    .w_h(210.0, 55.0)
                    .hover_image(self.imgs.button_dark_hover)
                    .press_image(self.imgs.button_dark_press)
                    .label("Enter World")
                    .label_rgba(220.0, 220.0, 220.0, 0.8)
                    .label_font_size(22)
                    .label_y(conrod_core::position::Relative::Scalar(3.0))
                    .set(self.ids.enter_world_button, ui_widgets)
                    .was_clicked()
                {
                    // Enter World
                    events.push(Event::Play);
                }

                // Delete Button
                if Button::image(self.imgs.button_dark_red)
                    .bottom_right_with_margins_on(self.ids.selection_window, -25.0, 0.0)
                    .w_h(100.0, 20.0)
                    .hover_image(self.imgs.button_dark_red_hover)
                    .press_image(self.imgs.button_dark_red_press)
                    .label("Delete")
                    .label_rgba(220.0, 220.0, 220.0, 0.8)
                    .label_font_size(12)
                    .label_y(conrod_core::position::Relative::Scalar(3.0))
                    .set(self.ids.delete_button, ui_widgets)
                    .was_clicked()
                {}
            }
        }
        // Character_Creation //////////////
        else {
            // Background
            Image::new(self.imgs.bg_creation)
                .middle_of(ui_widgets.window)
                .set(self.ids.bg_creation, ui_widgets);
            // Back Button
            if Button::image(self.imgs.button_dark)
                .bottom_left_with_margins_on(self.ids.bg_creation, 10.0, 10.0)
                .w_h(150.0, 40.0)
                .hover_image(self.imgs.button_dark_hover)
                .press_image(self.imgs.button_dark_press)
                .label("Back")
                .label_rgba(220.0, 220.0, 220.0, 0.8)
                .label_font_size(18)
                .label_y(conrod_core::position::Relative::Scalar(3.0))
                .set(self.ids.back_button, ui_widgets)
                .was_clicked()
            {
                self.character_creation = false;
            }
            // Create Button
            if Button::image(self.imgs.button_dark)
                .bottom_right_with_margins_on(self.ids.bg_creation, 10.0, 10.0)
                .w_h(150.0, 40.0)
                .hover_image(self.imgs.button_dark_hover)
                .press_image(self.imgs.button_dark_press)
                .label("Create")
                .label_rgba(220.0, 220.0, 220.0, 0.8)
                .label_font_size(18)
                .label_y(conrod_core::position::Relative::Scalar(3.0))
                .set(self.ids.create_button, ui_widgets)
                .was_clicked()
            {
                self.character_creation = false;
            }
            // Character Name Input

            // Window(s)
            Image::new(self.imgs.creation_window)
                .w_h(628.0, 814.0)
                .top_left_with_margins_on(self.ids.bg_creation, 60.0, 30.0)
                .set(self.ids.creation_window, ui_widgets);

            // Arrows
            // TODO: lower the resolution of the arrow files so that we don't multiply by .03 here
            const ARROW_WH: [f64; 2] = [986.0*0.03, 1024.0*0.03];
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
                        .was_clicked() {self.creation_state = CreationState::Weapon};
                }
                CreationState::Weapon => {
                    if Button::image(self.imgs.arrow_left)
                        .wh(ARROW_WH)
                        .hover_image(self.imgs.arrow_left_mo)
                        .press_image(self.imgs.arrow_left_press)
                        .top_left_with_margins_on(self.ids.creation_window, 74.0, 55.0)
                        .set(self.ids.arrow_left, ui_widgets)
                        .was_clicked(){self.creation_state = CreationState::Race};

                    if Button::image(self.imgs.arrow_right)
                        .wh(ARROW_WH)
                        .hover_image(self.imgs.arrow_right_mo)
                        .press_image(self.imgs.arrow_right_press)
                        .top_right_with_margins_on(self.ids.creation_window, 74.0, 55.0)
                        .set(self.ids.arrow_right, ui_widgets)
                        .was_clicked() {self.creation_state = CreationState::Body};
                }
                CreationState::Body => {
                    if Button::image(self.imgs.arrow_left)
                        .wh(ARROW_WH)
                        .hover_image(self.imgs.arrow_left_mo)
                        .press_image(self.imgs.arrow_left_press)
                        .top_left_with_margins_on(self.ids.creation_window, 74.0, 55.0)
                        .set(self.ids.arrow_left, ui_widgets)
                        .was_clicked(){self.creation_state = CreationState::Weapon};
                    Button::image(self.imgs.arrow_right_grey)
                        .wh(ARROW_WH)
                        .top_right_with_margins_on(self.ids.creation_window, 74.0, 55.0)
                        .set(self.ids.arrow_right, ui_widgets);
                }
            }


            // Races

            // Weapon

            // Body

            //Race Selection
            if let CreationState::Race = self.creation_state {
                Text::new("Choose your Race")
                    .mid_top_with_margin_on(self.ids.creation_window, 74.0)
                    .font_size(28)
                    .rgba(220.0, 220.0, 220.0, 0.8)
                    .set(self.ids.select_race_title, ui_widgets);

                // Male/Female/Race Icons
                // for alignment
                Rectangle::fill_with([151.0, 68.0], color::TRANSPARENT)
                    .mid_top_with_margin_on(self.ids.creation_window, 210.0)
                    .set(self.ids.gender_bg, ui_widgets);

                // Male
                Image::new(self.imgs.male)
                    .w_h(68.0, 68.0)
                    .mid_left_of(self.ids.gender_bg)
                    .set(self.ids.male, ui_widgets);
                if Button::image(if let Sex::Male = self.sex {self.imgs.icon_border_pressed} else {self.imgs.icon_border})
                    .middle_of(self.ids.male)
                    .hover_image(self.imgs.icon_border_mo)
                    .press_image(self.imgs.icon_border_press)
                    .set(self.ids.sex_1, ui_widgets)
                    .was_clicked() {self.sex = Sex::Male;};
                // Female
                Image::new(self.imgs.female)
                    .w_h(68.0, 68.0)
                    .right_from(self.ids.male, 15.0)
                    .set(self.ids.female, ui_widgets);
                 if Button::image(if let Sex::Female = self.sex {self.imgs.icon_border_pressed} else {self.imgs.icon_border})
                    .middle_of(self.ids.female)
                    .hover_image(self.imgs.icon_border_mo)
                    .press_image(self.imgs.icon_border_press)
                    .set(self.ids.sex_2, ui_widgets)
                    .was_clicked() {self.sex = Sex::Female;};
                // for alignment
                Rectangle::fill_with([458.0, 68.0], color::TRANSPARENT)
                    .mid_top_with_margin_on(self.ids.creation_window, 120.0)
                    .set(self.ids.races_bg, ui_widgets);
                // TODO: If races where in some sort of array format we could do this in a loop
                // Human
                Image::new(if let Sex::Male = self.sex {self.imgs.human_m} else {self.imgs.human_f})
                    .w_h(68.0, 68.0)
                    .mid_left_of(self.ids.races_bg)
                    .set(self.ids.human, ui_widgets);
                if Button::image(if let Races::Human = self.race {self.imgs.icon_border_pressed} else {self.imgs.icon_border})
                    .middle_of(self.ids.human)
                    .hover_image(self.imgs.icon_border_mo)
                    .press_image(self.imgs.icon_border_press)
                    .set(self.ids.race_1, ui_widgets)
                    .was_clicked() {self.race = Races::Human}

                // Orc
                Image::new(if let Sex::Male = self.sex {self.imgs.orc_m} else {self.imgs.orc_f})
                    .w_h(68.0, 68.0)
                    .right_from(self.ids.human, 10.0)
                    .set(self.ids.orc, ui_widgets);
                if Button::image(if let Races::Orc = self.race {self.imgs.icon_border_pressed} else {self.imgs.icon_border})
                    .middle_of(self.ids.orc)
                    .hover_image(self.imgs.icon_border_mo)
                    .press_image(self.imgs.icon_border_press)
                    .set(self.ids.race_2, ui_widgets)
                    .was_clicked() {self.race = Races::Orc}

                // Dwarf
                Image::new(if let Sex::Male = self.sex {self.imgs.dwarf_m} else {self.imgs.dwarf_f})
                    .w_h(68.0, 68.0)
                    .right_from(self.ids.human, 10.0 + 68.0)
                    .set(self.ids.dwarf, ui_widgets);
                 if Button::image(if let Races::Dwarf = self.race {self.imgs.icon_border_pressed} else {self.imgs.icon_border})
                    .middle_of(self.ids.dwarf)
                    .hover_image(self.imgs.icon_border_mo)
                    .press_image(self.imgs.icon_border_press)
                    .set(self.ids.race_3, ui_widgets)
                    .was_clicked() {self.race = Races::Dwarf}

                // Elf
                Image::new(if let Sex::Male = self.sex {self.imgs.elf_m} else {self.imgs.elf_f})
                    .w_h(68.0, 68.0)
                    .right_from(self.ids.human, 10.0 + 68.0*2.0)
                    .set(self.ids.elf, ui_widgets);
                if Button::image(if let Races::Elf = self.race {self.imgs.icon_border_pressed} else {self.imgs.icon_border})
                    .middle_of(self.ids.elf)
                    .hover_image(self.imgs.icon_border_mo)
                    .press_image(self.imgs.icon_border_press)
                    .set(self.ids.race_4, ui_widgets)
                    .was_clicked() {self.race = Races::Elf}

                // Undead
                Image::new(if let Sex::Male = self.sex {self.imgs.undead_m} else {self.imgs.undead_f})
                    .w_h(68.0, 68.0)
                    .right_from(self.ids.human, 10.0 + 68.0*3.0)
                    .set(self.ids.undead, ui_widgets);
                if Button::image(if let Races::Undead = self.race {self.imgs.icon_border_pressed} else {self.imgs.icon_border})
                    .middle_of(self.ids.undead)
                    .hover_image(self.imgs.icon_border_mo)
                    .press_image(self.imgs.icon_border_press)
                    .set(self.ids.race_5, ui_widgets)
                    .was_clicked() {self.race = Races::Undead}

                // Danari
                Image::new(if let Sex::Male = self.sex {self.imgs.danari_m} else {self.imgs.danari_f})
                    .right_from(self.ids.human, 10.0 + 68.0*4.0)
                    .set(self.ids.danari, ui_widgets);
                if Button::image(if let Races::Danari = self.race {self.imgs.icon_border_pressed} else {self.imgs.icon_border})
                    .w_h(68.0, 68.0)
                    .middle_of(self.ids.danari)
                    .hover_image(self.imgs.icon_border_mo)
                    .press_image(self.imgs.icon_border_press)
                    .set(self.ids.race_6, ui_widgets)
                    .was_clicked() {self.race = Races::Danari}

                // Description Headline and Text

                //Image::new(self.imgs.desc_bg)
                    //.w_h(528.0, 353.0)
                    //.mid_top_with_margin_on(self.ids.creation_window, 400.0)
                    //.scroll_kids_horizontally()
                    //.set(self.ids.desc_bg, ui_widgets);

                // TODO: Load these from files (or from the server???)
                const HUMAN_DESC: &str = "Lorem ipsum dolor sit amet, consectetuer \
                                          adipiscing elit. Aenean commodo ligula eget \
                                          dolor. Aenean massa. Cum sociis natoque \
                                          penatibus et magnis dis parturient montes, \
                                          nascetur ridiculus mus. Donec quam felis, \
                                          ultricies nec, pellentesque eu, pretium quis, \
                                          sem. Nulla consequat massa quis enim. Donec \
                                          pede justo, fringilla vel, aliquet nec, \
                                          vulputate eget, arcu.";
                const ORC_DESC: &str = HUMAN_DESC;
                const DWARF_DESC: &str = HUMAN_DESC;
                const UNDEAD_DESC: &str = HUMAN_DESC;
                const ELF_DESC: &str = HUMAN_DESC;
                const DANARI_DESC: &str = HUMAN_DESC;

    			let (race_str, race_desc) = match self.race {
        			Races::Human => ("Human", HUMAN_DESC),
        			Races::Orc => ("Orcs", ORC_DESC),
                    Races::Dwarf => ("Dwarf", DWARF_DESC),
        			Races::Undead => ("Undead", UNDEAD_DESC),
        			Races::Elf => ("Elves", ELF_DESC),
        			Races::Danari => ("Danari", DANARI_DESC),
    			};
                Text::new(race_str)
                    .mid_top_with_margin_on(self.ids.creation_window, 370.0)
                    .font_size(30)
                    .rgba(220.0, 220.0, 220.0, 0.8)
                    .set(self.ids.race_heading, ui_widgets);
                Text::new(race_desc)
                    .mid_top_with_margin_on(self.ids.creation_window, 410.0)
                    .w(500.0)
                    .font_size(20)
                    .font_id(self.font_whitney)
                    .rgba(220.0, 220.0, 220.0, 0.8)
                    .wrap_by_word()
                    .set(self.ids.race_description, ui_widgets);
                // Races Descriptions
            }

            if let CreationState::Weapon =  self.creation_state {}

            // Weapons Icons
            // Weapons Descriptions
            if let CreationState::Body = self.creation_state {}
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
