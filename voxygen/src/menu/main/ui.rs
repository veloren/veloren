use crate::{
    render::Renderer,
    ui::{
        self,
        img_ids::{BlankGraphic, ImageGraphic, VoxelGraphic},
        ImageFrame, Tooltip, Ui, /*Tooltipable,*/
    },
    GlobalState,
};
use conrod_core::{
    color,
    color::TRANSPARENT,
    position::Relative,
    widget::{text_box::Event as TextBoxEvent, Button, Image, List, Rectangle, Text, TextBox},
    widget_ids, Borderable, Color, Colorable, Labelable, Positionable, Sizeable, Widget,
};

widget_ids! {
    struct Ids {
        // Background and logo
        bg,
        v_logo,
        alpha_version,
        banner,
        banner_top,
        // Disclaimer
        disc_window,
        disc_text_1,
        disc_text_2,
        disc_button,
        disc_scrollbar,
        // Login, Singleplayer
        login_button,
        login_text,
        login_error,
        login_error_bg,
        address_text,
        address_bg,
        address_field,
        username_text,
        username_bg,
        username_field,
        password_text,
        password_bg,
        password_field,
        singleplayer_button,
        singleplayer_text,
        usrnm_bg,
        srvr_bg,
        passwd_bg,
        // Server list
        servers_button,
        servers_frame,
        servers_text,
        servers_close,
        // Buttons
        settings_button,
        quit_button,
        // Error
        error_frame,
        button_ok,
        version,
        // Info Window
        info_frame,
        info_text,
        info_bottom
    }
}

image_ids! {
    struct Imgs {
        <VoxelGraphic>
        v_logo: "voxygen.element.v_logo",
        input_bg: "voxygen.element.misc_bg.textbox",
        button: "voxygen.element.buttons.button",
        button_hover: "voxygen.element.buttons.button_hover",
        button_press: "voxygen.element.buttons.button_press",
        disclaimer: "voxygen.element.frames.disclaimer",
        info_frame: "voxygen.element.frames.info_frame_2",
        banner: "voxygen.element.frames.banner",
        banner_top: "voxygen.element.frames.banner_top",
        banner_bottom: "voxygen.element.frames.banner_bottom",

        <ImageGraphic>
        bg: "voxygen.background.bg_main",

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

font_ids! {
    pub struct Fonts {
        opensans: "voxygen.font.OpenSans-Regular",
        metamorph: "voxygen.font.Metamorphous-Regular",
        alkhemi: "voxygen.font.Alkhemikal",
        cyri:"voxygen.font.haxrcorp_4089_cyrillic_altgr",
        wizard: "voxygen.font.wizard",
    }
}

pub enum Event {
    LoginAttempt {
        username: String,
        password: String,
        server_address: String,
    },
    CancelLoginAttempt,
    #[cfg(feature = "singleplayer")]
    StartSingleplayer,
    Quit,
    Settings,
    DisclaimerClosed,
}

pub enum PopupType {
    Error,
    ConnectionInfo,
}

pub struct PopupData {
    msg: String,
    button_text: String,
    popup_type: PopupType,
}

pub struct MainMenuUi {
    ui: Ui,
    ids: Ids,
    imgs: Imgs,
    rot_imgs: ImgsRot,
    fonts: Fonts,
    username: String,
    password: String,
    server_address: String,
    popup: Option<PopupData>,
    connecting: Option<std::time::Instant>,
    show_servers: bool,
    show_disclaimer: bool,
}

impl MainMenuUi {
    pub fn new(global_state: &mut GlobalState) -> Self {
        let window = &mut global_state.window;
        let networking = &global_state.settings.networking;
        let gameplay = &global_state.settings.gameplay;

        let mut ui = Ui::new(window).unwrap();
        ui.set_scaling_mode(gameplay.ui_scale);
        // Generate ids
        let ids = Ids::new(ui.id_generator());
        // Load images
        let imgs = Imgs::load(&mut ui).expect("Failed to load images");
        let rot_imgs = ImgsRot::load(&mut ui).expect("Failed to load images!");
        // Load fonts
        let fonts = Fonts::load(&mut ui).expect("Failed to load fonts");

        Self {
            ui,
            ids,
            imgs,
            rot_imgs,
            fonts,
            username: networking.username.clone(),
            password: "".to_owned(),
            server_address: networking.servers[networking.default_server].clone(),
            popup: None,
            connecting: None,
            show_servers: false,
            show_disclaimer: global_state.settings.show_disclaimer,
        }
    }

    fn update_layout(&mut self, global_state: &mut GlobalState) -> Vec<Event> {
        let mut events = Vec::new();
        let (ref mut ui_widgets, ref mut _tooltip_manager) = self.ui.set_widgets();
        let version = format!(
            "{}-{}",
            env!("CARGO_PKG_VERSION"),
            common::util::GIT_VERSION.to_string()
        );
        const TEXT_COLOR: Color = Color::Rgba(1.0, 1.0, 1.0, 1.0);
        const TEXT_COLOR_2: Color = Color::Rgba(1.0, 1.0, 1.0, 0.2);
        let intro_text: &'static str = "Information on the Login Process:\n\
                                        \n\
                                        Choose whatever Username and Password you want.\n\
                                        (The middle box is for Password input)\n\
                                        They will be saved until server restart.\n\
                                        \n\
                                        The name you put in will be your character name ingame.\n\
                                        \n\
                                        Starting Singleplayer needs some time to load.\n\
                                        During this time the game may appear unresponsive.\n\
                                        \n\
                                        As of now you can't save your characters.\n\
                                        Changing their appearance is possible though.";

        // Tooltip
        let _tooltip = Tooltip::new({
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
        .title_font_size(15)
        .desc_font_size(10)
        .title_text_color(TEXT_COLOR)
        .desc_text_color(TEXT_COLOR_2);

        // Background image, Veloren logo, Alpha-Version Label

        Image::new(self.imgs.bg)
            .middle_of(ui_widgets.window)
            .set(self.ids.bg, ui_widgets);

        Image::new(self.imgs.banner)
            .w_h(65.0 * 6.0, 100.0 * 6.0)
            .middle_of(self.ids.bg)
            .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.9)))
            .set(self.ids.banner, ui_widgets);

        Image::new(self.imgs.banner_top)
            .w_h(65.0 * 6.0, 1.0 * 6.0)
            .mid_top_with_margin_on(self.ids.banner, 0.0)
            .set(self.ids.banner_top, ui_widgets);

        // Logo
        Image::new(self.imgs.v_logo)
            .w_h(123.0 * 2.5, 35.0 * 2.5)
            .mid_top_with_margin_on(self.ids.banner_top, 40.0)
            .color(Some(Color::Rgba(1.0, 1.0, 1.0, 0.95)))
            .set(self.ids.v_logo, ui_widgets);
        // Version displayed top right corner
        Text::new(&version)
            .top_right_with_margins_on(ui_widgets.window, 5.0, 5.0)
            .font_size(14)
            .font_id(self.fonts.cyri)
            .color(TEXT_COLOR)
            .set(self.ids.version, ui_widgets);

        if self.show_disclaimer {
            Image::new(self.imgs.disclaimer)
                .w_h(1800.0, 800.0)
                .middle_of(ui_widgets.window)
                .scroll_kids()
                .scroll_kids_vertically()
                .set(self.ids.disc_window, ui_widgets);

            Text::new("Disclaimer")
                .top_left_with_margins_on(self.ids.disc_window, 30.0, 40.0)
                .font_size(35)
                .font_id(self.fonts.alkhemi)
                .color(TEXT_COLOR)
                .set(self.ids.disc_text_1, ui_widgets);
            Text::new(
            "Welcome to the alpha version of Veloren!\n\
            \n\
            \n\
            Before you dive into the fun, please keep a few things in mind:\n\
            \n\
            - This is a very early alpha. Expect bugs, extremely unfinished gameplay, unpolished mechanics, and missing features. \n\
            \n\
            -If you have constructive feedback or bug reports, you can contact us via Reddit, GitLab, or our community Discord server.\n\
            \n\
            - Veloren is licensed under the GPL 3 open-source licence. That means you're free to play, modify, and redistribute the game however you wish \n\
            (provided derived work is also under GPL 3).
            \n\
            - Veloren is a non-profit community project, and everybody working on it is a volunteer.\n\
            If you like what you see, you're welcome to join the development or art teams!
            \n\
            - 'Voxel RPG' is a genre in its own right. First-person shooters used to be called Doom clones.\n\
            Like them, we're trying to build a niche. This game is not a clone, and its development will diverge from existing games in the future.\n\
            \n\
            Thanks for taking the time to read this notice, we hope you enjoy the game!\n\
            \n\
            ~ The Veloren Devs")
            .top_left_with_margins_on(self.ids.disc_window, 110.0, 40.0)
            .font_size(26)
            .font_id(self.fonts.cyri)
            .color(TEXT_COLOR)
            .set(self.ids.disc_text_2, ui_widgets);
            if Button::image(self.imgs.button)
                .w_h(300.0, 50.0)
                .mid_bottom_with_margin_on(self.ids.disc_window, 30.0)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .label_y(Relative::Scalar(2.0))
                .label("Accept")
                .label_font_size(22)
                .label_color(TEXT_COLOR)
                .label_font_id(self.fonts.cyri)
                .set(self.ids.disc_button, ui_widgets)
                .was_clicked()
            {
                self.show_disclaimer = false;
                events.push(Event::DisclaimerClosed);
            }
        } else {
            // TODO: Don't use macros for this?
            // Input fields
            // Used when the login button is pressed, or enter is pressed within input field
            macro_rules! login {
                () => {
                    self.connecting = Some(std::time::Instant::now());
                    self.popup = Some(PopupData {
                        msg: "Connecting...".to_string(),
                        button_text: "Cancel".to_string(),
                        popup_type: PopupType::ConnectionInfo,
                    });
                    events.push(Event::LoginAttempt {
                        username: self.username.clone(),
                        password: self.password.clone(),
                        server_address: self.server_address.clone(),
                    });
                };
            }
            // Info Window
            Rectangle::fill_with([550.0, 280.0], color::BLACK)
                .top_left_with_margins_on(ui_widgets.window, 40.0, 40.0)
                .color(Color::Rgba(0.0, 0.0, 0.0, 0.95))
                .set(self.ids.info_frame, ui_widgets);
            Image::new(self.imgs.banner_bottom)
                .mid_bottom_with_margin_on(self.ids.info_frame, -50.0)
                .w_h(550.0, 50.0)
                .color(Some(Color::Rgba(0.0, 0.0, 0.0, 0.95)))
                .set(self.ids.info_bottom, ui_widgets);
            Text::new(intro_text)
                .top_left_with_margins_on(self.ids.info_frame, 15.0, 15.0)
                .font_size(20)
                .font_id(self.fonts.cyri)
                .color(TEXT_COLOR)
                .set(self.ids.info_text, ui_widgets);

            // Singleplayer
            // Used when the singleplayer button is pressed
            #[cfg(feature = "singleplayer")]
            macro_rules! singleplayer {
                () => {
                    events.push(Event::StartSingleplayer);
                    events.push(Event::LoginAttempt {
                        username: "singleplayer".to_string(),
                        password: String::default(),
                        server_address: "localhost".to_string(),
                    });
                };
            }

            // Username
            Rectangle::fill_with([320.0, 50.0], color::rgba(0.0, 0.0, 0.0, 0.97))
                .mid_top_with_margin_on(self.ids.banner_top, 160.0)
                .set(self.ids.usrnm_bg, ui_widgets);
            Image::new(self.imgs.input_bg)
                .w_h(337.0, 67.0)
                .middle_of(self.ids.usrnm_bg)
                .set(self.ids.username_bg, ui_widgets);
            for event in TextBox::new(&self.username)
                .w_h(290.0, 30.0)
                .mid_bottom_with_margin_on(self.ids.username_bg, 44.0 / 2.0)
                .font_size(22)
                .font_id(self.fonts.cyri)
                .text_color(TEXT_COLOR)
                // transparent background
                .color(TRANSPARENT)
                .border_color(TRANSPARENT)
                .set(self.ids.username_field, ui_widgets)
            {
                match event {
                    TextBoxEvent::Update(username) => {
                        // Note: TextBox limits the input string length to what fits in it
                        self.username = username.to_string();
                    }
                    TextBoxEvent::Enter => {
                        login!();
                    }
                }
            }
            // Password
            Rectangle::fill_with([320.0, 50.0], color::rgba(0.0, 0.0, 0.0, 0.97))
                .down_from(self.ids.usrnm_bg, 30.0)
                .set(self.ids.passwd_bg, ui_widgets);
            Image::new(self.imgs.input_bg)
                .w_h(337.0, 67.0)
                .middle_of(self.ids.passwd_bg)
                .set(self.ids.password_bg, ui_widgets);
            for event in TextBox::new(&self.password)
                .w_h(290.0, 30.0)
                .mid_bottom_with_margin_on(self.ids.password_bg, 44.0 / 2.0)
                .font_size(22)
                .font_id(self.fonts.cyri)
                .text_color(TEXT_COLOR)
                // transparent background
                .color(TRANSPARENT)
                .border_color(TRANSPARENT)
                .set(self.ids.password_field, ui_widgets)
            {
                match event {
                    TextBoxEvent::Update(password) => {
                        // Note: TextBox limits the input string length to what fits in it
                        self.password = password;
                    }
                    TextBoxEvent::Enter => {
                        login!();
                    }
                }
            }
            // Popup (Error/Info)
            if let Some(popup_data) = &self.popup {
                let text = Text::new(&popup_data.msg)
                    .rgba(1.0, 1.0, 1.0, 1.0)
                    .font_size(25)
                    .font_id(self.fonts.cyri);
                Rectangle::fill_with([65.0 * 6.0, 100.0], color::TRANSPARENT)
                    .rgba(0.1, 0.1, 0.1, 1.0)
                    .parent(ui_widgets.window)
                    .up_from(self.ids.banner_top, 20.0)
                    .set(self.ids.login_error_bg, ui_widgets);
                Image::new(self.imgs.info_frame)
                    .w_h(65.0 * 6.0, 100.0)
                    .middle_of(self.ids.login_error_bg)
                    .set(self.ids.error_frame, ui_widgets);
                text.mid_top_with_margin_on(self.ids.error_frame, 10.0)
                    .set(self.ids.login_error, ui_widgets);
                if Button::image(self.imgs.button)
                    .w_h(100.0, 30.0)
                    .mid_bottom_with_margin_on(self.ids.login_error_bg, 5.0)
                    .hover_image(self.imgs.button_hover)
                    .press_image(self.imgs.button_press)
                    .label_y(Relative::Scalar(2.0))
                    .label(&popup_data.button_text)
                    .label_font_id(self.fonts.cyri)
                    .label_font_size(15)
                    .label_color(TEXT_COLOR)
                    .set(self.ids.button_ok, ui_widgets)
                    .was_clicked()
                {
                    match popup_data.popup_type {
                        PopupType::ConnectionInfo => {
                            events.push(Event::CancelLoginAttempt);
                        }
                        _ => (),
                    };
                    self.popup = None;
                };
            }
            if self.show_servers {
                Image::new(self.imgs.info_frame)
                    .mid_top_with_margin_on(self.ids.username_bg, -320.0)
                    .w_h(400.0, 300.0)
                    .set(self.ids.servers_frame, ui_widgets);

                let ref mut net_settings = global_state.settings.networking;

                // TODO: Draw scroll bar or remove it.
                let (mut items, _scrollbar) = List::flow_down(net_settings.servers.len())
                    .top_left_with_margins_on(self.ids.servers_frame, 0.0, 5.0)
                    .w_h(400.0, 300.0)
                    .scrollbar_next_to()
                    .scrollbar_thickness(18.0)
                    .scrollbar_color(TEXT_COLOR)
                    .set(self.ids.servers_text, ui_widgets);

                while let Some(item) = items.next(ui_widgets) {
                    let mut text = "".to_string();
                    if &net_settings.servers[item.i] == &self.server_address {
                        text.push_str("-> ")
                    } else {
                        text.push_str("  ")
                    }
                    text.push_str(&net_settings.servers[item.i]);

                    if item
                        .set(
                            Button::image(self.imgs.nothing)
                                .w_h(100.0, 50.0)
                                .mid_top_with_margin_on(self.ids.servers_frame, 10.0)
                                //.hover_image(self.imgs.button_hover)
                                //.press_image(self.imgs.button_press)
                                .label_y(Relative::Scalar(2.0))
                                .label(&text)
                                .label_font_size(20)
                                .label_font_id(self.fonts.cyri)
                                .label_color(TEXT_COLOR),
                            ui_widgets,
                        )
                        .was_clicked()
                    {
                        self.server_address = net_settings.servers[item.i].clone();
                        net_settings.default_server = item.i;
                    }
                }

                if Button::image(self.imgs.button)
                    .w_h(200.0, 53.0)
                    .mid_bottom_with_margin_on(self.ids.servers_frame, 5.0)
                    .hover_image(self.imgs.button_hover)
                    .press_image(self.imgs.button_press)
                    .label_y(Relative::Scalar(2.0))
                    .label("Close")
                    .label_font_size(20)
                    .label_font_id(self.fonts.cyri)
                    .label_color(TEXT_COLOR)
                    .set(self.ids.servers_close, ui_widgets)
                    .was_clicked()
                {
                    self.show_servers = false
                };
            }
            // Server address
            Rectangle::fill_with([320.0, 50.0], color::rgba(0.0, 0.0, 0.0, 0.97))
                .down_from(self.ids.passwd_bg, 30.0)
                .set(self.ids.srvr_bg, ui_widgets);
            Image::new(self.imgs.input_bg)
                .w_h(337.0, 67.0)
                .middle_of(self.ids.srvr_bg)
                .set(self.ids.address_bg, ui_widgets);
            for event in TextBox::new(&self.server_address)
                .w_h(290.0, 30.0)
                .mid_bottom_with_margin_on(self.ids.address_bg, 44.0 / 2.0)
                .font_size(22)
                .font_id(self.fonts.cyri)
                .text_color(TEXT_COLOR)
                // transparent background
                .color(TRANSPARENT)
                .border_color(TRANSPARENT)
                .set(self.ids.address_field, ui_widgets)
            {
                match event {
                    TextBoxEvent::Update(server_address) => {
                        self.server_address = server_address.to_string();
                    }
                    TextBoxEvent::Enter => {
                        login!();
                    }
                }
            }
            // Login button
            if Button::image(self.imgs.button)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .w_h(258.0, 55.0)
                .down_from(self.ids.address_bg, 20.0)
                .align_middle_x_of(self.ids.address_bg)
                .label("Login")
                .label_font_id(self.fonts.cyri)
                .label_color(TEXT_COLOR)
                .label_font_size(26)
                .label_y(Relative::Scalar(5.0))
                /*.with_tooltip(
                    tooltip_manager,
                    "Login",
                    "Click to login with the entered details",
                    &tooltip,
                )
                .tooltip_image(self.imgs.v_logo)*/
                .set(self.ids.login_button, ui_widgets)
                .was_clicked()
            {
                login!();
            }

            // Singleplayer button
            #[cfg(feature = "singleplayer")]
            {
                if Button::image(self.imgs.button)
                    .hover_image(self.imgs.button_hover)
                    .press_image(self.imgs.button_press)
                    .w_h(258.0, 55.0)
                    .down_from(self.ids.login_button, 20.0)
                    .align_middle_x_of(self.ids.address_bg)
                    .label("Singleplayer")
                    .label_font_id(self.fonts.cyri)
                    .label_color(TEXT_COLOR)
                    .label_font_size(22)
                    .label_y(Relative::Scalar(5.0))
                    .label_x(Relative::Scalar(2.0))
                    .set(self.ids.singleplayer_button, ui_widgets)
                    .was_clicked()
                {
                    singleplayer!();
                }
            }
            // Quit
            if Button::image(self.imgs.button)
                .w_h(190.0, 40.0)
                .bottom_left_with_margins_on(ui_widgets.window, 60.0, 30.0)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .label("Quit")
                .label_font_id(self.fonts.cyri)
                .label_color(TEXT_COLOR)
                .label_font_size(20)
                .label_y(Relative::Scalar(3.0))
                .set(self.ids.quit_button, ui_widgets)
                .was_clicked()
            {
                events.push(Event::Quit);
            }

            // Settings
            if Button::image(self.imgs.button)
                .w_h(190.0, 40.0)
                .up_from(self.ids.quit_button, 8.0)
                //.hover_image(self.imgs.button_hover)
                //.press_image(self.imgs.button_press)
                .label("Settings")
                .label_font_id(self.fonts.cyri)
                .label_color(TEXT_COLOR_2)
                .label_font_size(20)
                .label_y(Relative::Scalar(3.0))
                .set(self.ids.settings_button, ui_widgets)
                .was_clicked()
            {
                events.push(Event::Settings);
            }

            // Servers
            if Button::image(self.imgs.button)
                .w_h(190.0, 40.0)
                .up_from(self.ids.settings_button, 8.0)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .label("Servers")
                .label_font_id(self.fonts.cyri)
                .label_color(TEXT_COLOR)
                .label_font_size(20)
                .label_y(Relative::Scalar(3.0))
                .set(self.ids.servers_button, ui_widgets)
                .was_clicked()
            {
                self.show_servers = !self.show_servers;
            };
        }

        events
    }

    pub fn login_error(&mut self, msg: String) {
        self.popup = Some(PopupData {
            msg,
            button_text: "Okay".to_string(),
            popup_type: PopupType::Error,
        });
        self.connecting = None;
    }

    pub fn connected(&mut self) {
        self.popup = None;
        self.connecting = None;
    }

    pub fn cancel_connection(&mut self) {
        self.popup = None;
        self.connecting = None;
    }

    pub fn handle_event(&mut self, event: ui::Event) {
        self.ui.handle_event(event);
    }

    pub fn maintain(&mut self, global_state: &mut GlobalState) -> Vec<Event> {
        let events = self.update_layout(global_state);
        self.ui.maintain(global_state.window.renderer_mut(), None);
        events
    }

    pub fn render(&self, renderer: &mut Renderer) {
        self.ui.render(renderer, None);
    }
}
