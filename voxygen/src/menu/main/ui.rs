use crate::{
    render::Renderer,
    ui::{self, ScaleMode, Ui},
    window::Window,
    GlobalState, DEFAULT_PUBLIC_SERVER,
};
use common::{assets, figure::Segment};
use conrod_core::{
    color,
    color::TRANSPARENT,
    image::Id as ImgId,
    position::{Dimension, Relative},
    text::font::Id as FontId,
    widget::{text_box::Event as TextBoxEvent, Button, Image, List, Rectangle, Text, TextBox},
    widget_ids, Borderable, Color, Colorable, Labelable, Positionable, Sizeable, Widget,
};

widget_ids! {
    struct Ids {
        // Background and logo
        bg,
        v_logo,
        alpha_version,
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
        singleplayer_button,
        singleplayer_text,
        // Serverlist
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
    }
}

struct Imgs {
    bg: ImgId,
    v_logo: ImgId,

    input_bg: ImgId,

    error_frame: ImgId,
    button_dark: ImgId,
    button_dark_hover: ImgId,
    button_dark_press: ImgId,
    button: ImgId,
    button_hover: ImgId,
    button_press: ImgId,
}
impl Imgs {
    fn new(ui: &mut Ui, renderer: &mut Renderer) -> Imgs {
        let load_img = |filename, ui: &mut Ui| {
            let fullpath: String = ["/voxygen/", filename].concat();
            let image = image::load_from_memory(
                assets::load(fullpath.as_str())
                    .expect("Error loading file")
                    .as_slice(),
            )
            .unwrap();
            ui.new_graphic(ui::Graphic::Image(image))
        };
        let load_vox = |filename, ui: &mut Ui| {
            let fullpath: String = ["/voxygen/", filename].concat();
            let dot_vox = dot_vox::load_bytes(
                assets::load(fullpath.as_str())
                    .expect("Error loading file")
                    .as_slice(),
            )
            .unwrap();
            ui.new_graphic(ui::Graphic::Voxel(Segment::from(dot_vox)))
        };
        Imgs {
            bg: load_img("background/bg_main.png", ui),
            v_logo: load_vox("element/v_logo.vox", ui),

            // Input fields
            input_bg: load_vox("element/misc_bg/textbox.vox", ui),

            error_frame: load_img("element/frames/window_2.png", ui),
            button_dark: load_vox("element/buttons/button_dark.vox", ui),
            button_dark_hover: load_vox("element/buttons/button_dark_hover.vox", ui),
            button_dark_press: load_vox("element/buttons/button_dark_press.vox", ui),
            button: load_vox("element/buttons/button.vox", ui),
            button_hover: load_vox("element/buttons/button_hover.vox", ui),
            button_press: load_vox("element/buttons/button_press.vox", ui),
        }
    }
}

pub enum Event {
    LoginAttempt {
        username: String,
        server_address: String,
    },
    StartSingleplayer,
    Quit,
}

pub struct MainMenuUi {
    ui: Ui,
    ids: Ids,
    imgs: Imgs,
    font_metamorph: FontId,
    font_opensans: FontId,
    username: String,
    server_address: String,
    login_error: Option<String>,
    connecting: Option<std::time::Instant>,
    show_servers: bool,
}

impl MainMenuUi {
    pub fn new(global_state: &mut GlobalState) -> Self {
        let mut window = &mut global_state.window;
        let networking = &global_state.settings.networking;
        let mut ui = Ui::new(window).unwrap();
        // TODO: adjust/remove this, right now it is used to demonstrate window scaling functionality
        ui.scaling_mode(ScaleMode::RelativeToWindow([1920.0, 1080.0].into()));
        // Generate ids
        let ids = Ids::new(ui.id_generator());
        // Load images
        let imgs = Imgs::new(&mut ui, window.renderer_mut());
        // Load fonts
        let load_font = |filename, ui: &mut Ui| {
            let fullpath: String = ["/voxygen/font", filename].concat();
            ui.new_font(
                conrod_core::text::Font::from_bytes(
                    assets::load(fullpath.as_str()).expect("Error loading file"),
                )
                .unwrap(),
            )
        };
        let font_opensans = load_font("/OpenSans-Regular.ttf", &mut ui);
        let font_metamorph = load_font("/Metamorphous-Regular.ttf", &mut ui);

        Self {
            ui,
            imgs,
            ids,
            font_metamorph,
            font_opensans,
            username: networking.username.clone(),
            server_address: networking.servers[networking.default_server].clone(),
            login_error: None,
            connecting: None,
            show_servers: false,
        }
    }

    fn update_layout(&mut self, global_state: &GlobalState) -> Vec<Event> {
        let mut events = Vec::new();
        let ref mut ui_widgets = self.ui.set_widgets();
        let version = env!("CARGO_PKG_VERSION");
        // Background image, Veloren logo, Alpha-Version Label
        Image::new(self.imgs.bg)
            .middle_of(ui_widgets.window)
            .set(self.ids.bg, ui_widgets);
        Image::new(self.imgs.v_logo)
            .w_h(123.0 * 3.0, 35.0 * 3.0)
            .top_left_with_margins(30.0, 30.0)
            .set(self.ids.v_logo, ui_widgets);
        Text::new(version)
            .top_left_with_margins_on(ui_widgets.window, 5.0, 5.0)
            .font_size(14)
            .color(TEXT_COLOR)
            .set(self.ids.version, ui_widgets);

        // Input fields
        // Used when the login button is pressed, or enter is pressed within input field
        macro_rules! login {
            () => {
                self.login_error = None;
                self.connecting = Some(std::time::Instant::now());
                events.push(Event::LoginAttempt {
                    username: self.username.clone(),
                    server_address: self.server_address.clone(),
                });
            };
        }

        //Singleplayer
        //Used when the singleplayer button is pressed
        macro_rules! singleplayer {
            () => {
                self.login_error = None;
                events.push(Event::StartSingleplayer);
                events.push(Event::LoginAttempt {
                    username: "singleplayer".to_string(),
                    server_address: "localhost".to_string(),
                });
            };
        }

        const TEXT_COLOR: Color = Color::Rgba(1.0, 1.0, 1.0, 1.0);
        // Username
        // TODO: get a lower resolution and cleaner input_bg.png
        Image::new(self.imgs.input_bg)
            .w_h(337.0, 67.0)
            .middle_of(ui_widgets.window)
            .set(self.ids.username_bg, ui_widgets);
        for event in TextBox::new(&self.username)
            .w_h(580.0 / 2.0, 60.0 / 2.0)
            .mid_bottom_with_margin_on(self.ids.username_bg, 44.0 / 2.0)
            .font_size(20)
            .font_id(self.font_opensans)
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
        // Login error
        if let Some(msg) = &self.login_error {
            let text = Text::new(&msg)
                .rgba(1.0, 1.0, 1.0, 1.0)
                .font_size(30)
                .font_id(self.font_opensans);
            Rectangle::fill_with([400.0, 100.0], color::TRANSPARENT)
                .rgba(0.1, 0.1, 0.1, 1.0)
                .parent(ui_widgets.window)
                .mid_top_with_margin_on(self.ids.username_bg, -35.0)
                .set(self.ids.login_error_bg, ui_widgets);
            Image::new(self.imgs.error_frame)
                .w_h(400.0, 100.0)
                .middle_of(self.ids.login_error_bg)
                .set(self.ids.error_frame, ui_widgets);
            text.mid_top_with_margin_on(self.ids.error_frame, 10.0)
                .set(self.ids.login_error, ui_widgets);
            if Button::image(self.imgs.button_dark)
                .w_h(100.0, 30.0)
                .mid_bottom_with_margin_on(self.ids.login_error_bg, 5.0)
                .hover_image(self.imgs.button_dark_hover)
                .press_image(self.imgs.button_dark_press)
                .label_y(Relative::Scalar(2.0))
                .label("Okay")
                .label_font_size(10)
                .label_color(TEXT_COLOR)
                .set(self.ids.button_ok, ui_widgets)
                .was_clicked()
            {
                self.login_error = None
            };
        }
        if self.show_servers {
            Image::new(self.imgs.error_frame)
                .top_left_with_margins_on(ui_widgets.window, 3.0, 3.0)
                .w_h(400.0, 400.0)
                .set(self.ids.servers_frame, ui_widgets);

            let netsettings = &global_state.settings.networking;

            let (mut items, scrollbar) = List::flow_down(netsettings.servers.len())
                .top_left_with_margins_on(self.ids.servers_frame, 0.0, 5.0)
                .w_h(400.0, 300.0)
                .scrollbar_next_to()
                .scrollbar_thickness(18.0)
                .scrollbar_color(TEXT_COLOR)
                .set(self.ids.servers_text, ui_widgets);

            while let Some(item) = items.next(ui_widgets) {
                let mut text = "".to_string();
                if &netsettings.servers[item.i] == &self.server_address {
                    text.push_str("* ")
                } else {
                    text.push_str("  ")
                }
                text.push_str(&netsettings.servers[item.i]);

                if item
                    .set(
                        Button::image(self.imgs.button_dark)
                            .w_h(100.0, 53.0)
                            .mid_bottom_with_margin_on(self.ids.servers_frame, 5.0)
                            .hover_image(self.imgs.button_dark_hover)
                            .press_image(self.imgs.button_dark_press)
                            .label_y(Relative::Scalar(2.0))
                            .label(&text)
                            .label_font_size(20)
                            .label_color(TEXT_COLOR),
                        ui_widgets,
                    )
                    .was_clicked()
                {
                    // TODO: Set as current server address
                    self.server_address = netsettings.servers[item.i].clone();
                }
            }

            if Button::image(self.imgs.button_dark)
                .w_h(200.0, 53.0)
                .mid_bottom_with_margin_on(self.ids.servers_frame, 5.0)
                .hover_image(self.imgs.button_dark_hover)
                .press_image(self.imgs.button_dark_press)
                .label_y(Relative::Scalar(2.0))
                .label("Close")
                .label_font_size(20)
                .label_color(TEXT_COLOR)
                .set(self.ids.servers_close, ui_widgets)
                .was_clicked()
            {
                self.show_servers = false
            };
        }
        // Server address
        Image::new(self.imgs.input_bg)
            .w_h(337.0, 67.0)
            .down_from(self.ids.username_bg, 10.0)
            .set(self.ids.address_bg, ui_widgets);
        for event in TextBox::new(&self.server_address)
            .w_h(580.0 / 2.0, 60.0 / 2.0)
            .mid_bottom_with_margin_on(self.ids.address_bg, 44.0 / 2.0)
            .font_size(20)
            .font_id(self.font_opensans)
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
        // Change button text and remove hover/press images if a connection is in progress
        if let Some(start) = self.connecting {
            Button::image(self.imgs.button)
                .w_h(258.0, 68.0)
                .down_from(self.ids.address_bg, 20.0)
                .align_middle_x_of(self.ids.address_bg)
                .label("Connecting...")
                .label_color({
                    let pulse = ((start.elapsed().as_millis() as f32 * 0.008).sin() + 1.0) / 2.0;
                    Color::Rgba(
                        TEXT_COLOR.red() * (pulse / 2.0 + 0.5),
                        TEXT_COLOR.green() * (pulse / 2.0 + 0.5),
                        TEXT_COLOR.blue() * (pulse / 2.0 + 0.5),
                        pulse / 4.0 + 0.75,
                    )
                })
                .label_font_size(24)
                .label_y(Relative::Scalar(5.0))
                .set(self.ids.login_button, ui_widgets);
        } else {
            if Button::image(self.imgs.button)
                .hover_image(self.imgs.button_hover)
                .press_image(self.imgs.button_press)
                .w_h(258.0, 68.0)
                .down_from(self.ids.address_bg, 20.0)
                .align_middle_x_of(self.ids.address_bg)
                .label("Login")
                .label_color(TEXT_COLOR)
                .label_font_size(24)
                .label_y(Relative::Scalar(5.0))
                .set(self.ids.login_button, ui_widgets)
                .was_clicked()
            {
                login!();
            }
        };

        // Singleplayer button
        if Button::image(self.imgs.button)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .w_h(258.0, 68.0)
            .down_from(self.ids.login_button, 20.0)
            .align_middle_x_of(self.ids.address_bg)
            .label("Singleplayer")
            .label_color(TEXT_COLOR)
            .label_font_size(24)
            .label_y(Relative::Scalar(5.0))
            .label_x(Relative::Scalar(2.0))
            .set(self.ids.singleplayer_button, ui_widgets)
            .was_clicked()
        {
            singleplayer!();
        }
        // Quit
        if Button::image(self.imgs.button)
            .w_h(203.0, 53.0)
            .bottom_left_with_margins_on(ui_widgets.window, 60.0, 30.0)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .label("Quit")
            .label_color(TEXT_COLOR)
            .label_font_size(20)
            .label_y(Relative::Scalar(3.0))
            .set(self.ids.quit_button, ui_widgets)
            .was_clicked()
        {
            events.push(Event::Quit);
        };
        // Settings
        if Button::image(self.imgs.button)
            .w_h(203.0, 53.0)
            .up_from(self.ids.quit_button, 8.0)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .label("Settings")
            .label_color(TEXT_COLOR)
            .label_font_size(20)
            .label_y(Relative::Scalar(3.0))
            .set(self.ids.settings_button, ui_widgets)
            .was_clicked()
        {};
        // Servers
        if Button::image(self.imgs.button)
            .w_h(203.0, 53.0)
            .up_from(self.ids.settings_button, 8.0)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .label("Servers")
            .label_color(TEXT_COLOR)
            .label_font_size(20)
            .label_y(Relative::Scalar(3.0))
            .set(self.ids.servers_button, ui_widgets)
            .was_clicked()
        {
            self.show_servers = true;
        };

        events
    }

    pub fn login_error(&mut self, msg: String) {
        self.login_error = Some(msg);
        self.connecting = None;
    }

    pub fn connected(&mut self) {
        self.connecting = None;
    }

    pub fn handle_event(&mut self, event: ui::Event) {
        self.ui.handle_event(event);
    }

    pub fn maintain(&mut self, global_state: &mut GlobalState) -> Vec<Event> {
        let events = self.update_layout(global_state);
        self.ui.maintain(global_state.window.renderer_mut());
        events
    }

    pub fn render(&self, renderer: &mut Renderer) {
        self.ui.render(renderer);
    }
}
