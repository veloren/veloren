use crate::{
    render::Renderer,
    ui::{self, ScaleMode, Ui},
    window::Window,
};
use conrod_core::{
    color::TRANSPARENT,
    image::Id as ImgId,
    text::font::Id as FontId,
    widget::{text_box::Event as TextBoxEvent, Button, Image, TextBox},
    widget_ids, Borderable, Color,
    Colorable, Labelable, Positionable, Sizeable, Widget,
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
        address_text,
        address_bg,
        address_field,
        username_text,
        username_bg,
        username_field,
        singleplayer_button,
        singleplayer_text,
        // Buttons
        servers_button,
        settings_button,
        quit_button,
    }
}

struct Imgs {
    bg: ImgId,
    v_logo: ImgId,

    input_bg: ImgId,

    login_button: ImgId,
    login_button_hover: ImgId,
    login_button_press: ImgId,

    button: ImgId,
    button_hover: ImgId,
    button_press: ImgId,
}
impl Imgs {
    fn new(ui: &mut Ui, renderer: &mut Renderer) -> Imgs {
        let mut load = |filename| {
            let image = image::open(
                &[
                    env!("CARGO_MANIFEST_DIR"),
                    "/test_assets/ui/main/",
                    filename,
                ]
                .concat(),
            )
            .unwrap();
            ui.new_image(renderer, &image).unwrap()
        };
        Imgs {
            bg: load("bg.png"),
            v_logo: load("v_logo.png"),

            // Input fields
            input_bg: load("input_bg.png"),

            // Login button
            login_button: load("buttons/button_login.png"),
            login_button_hover: load("buttons/button_login_hover.png"),
            login_button_press: load("buttons/button_login_press.png"),

            // Servers, settings, and quit buttons
            button: load("buttons/button.png"),
            button_hover: load("buttons/button_hover.png"),
            button_press: load("buttons/button_press.png"),
        }
    }
}

pub enum Event {
    LoginAttempt {
        username: String,
        server_address: String,
    },
    Quit,
}

pub struct MainMenuUi {
    ui: Ui,
    ids: Ids,
    imgs: Imgs,
    font_metamorph: FontId,
    font_whitney: FontId,
    username: String,
    server_address: String,
}

impl MainMenuUi {
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
            username: "Username".to_string(),
            server_address: "Server Address".to_string(),
        }
    }

    fn update_layout(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        let ref mut ui_widgets = self.ui.set_widgets();
        // Background image, Veloren logo, Alpha-Version Label
        Image::new(self.imgs.bg)
            .middle_of(ui_widgets.window)
            .set(self.ids.bg, ui_widgets);
        Button::image(self.imgs.v_logo)
            .w_h(346.0, 111.0)
            .top_left_with_margins(30.0, 40.0)
            .label("Alpha 0.1")
            .label_rgba(255.0, 255.0, 255.0, 1.0)
            .label_font_size(10)
            .label_y(conrod_core::position::Relative::Scalar(-40.0))
            .label_x(conrod_core::position::Relative::Scalar(-100.0))
            .set(self.ids.v_logo, ui_widgets);

        // Input fields
        // Used when the login button is pressed, or enter is pressed within input field
        macro_rules! login {
            () => {
                events.push(Event::LoginAttempt {
                    username: self.username.clone(),
                    server_address: self.server_address.clone(),
                });
            };
        }
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
            .font_id(self.font_whitney)
            .text_color(Color::Rgba(220.0, 220.0, 220.0, 0.8))
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
                TextBoxEvent::Enter => login!(),
            }
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
            .font_id(self.font_whitney)
            .text_color(Color::Rgba(220.0, 220.0, 220.0, 0.8))
            // transparent background
            .color(TRANSPARENT)
            .border_color(TRANSPARENT)
            .set(self.ids.address_field, ui_widgets)
        {
            match event {
                TextBoxEvent::Update(server_address) => {
                    self.server_address = server_address.to_string();
                }
                TextBoxEvent::Enter => login!(),
            }
        }
        // Login button
        if Button::image(self.imgs.login_button)
            .hover_image(self.imgs.login_button_hover)
            .press_image(self.imgs.login_button_press)
            .w_h(258.0, 68.0)
            .down_from(self.ids.address_bg, 20.0)
            .align_middle_x_of(self.ids.address_bg)
            .label("Login")
            .label_rgba(220.0, 220.0, 220.0, 0.8)
            .label_font_size(28)
            .label_y(conrod_core::position::Relative::Scalar(5.0))
            .set(self.ids.login_button, ui_widgets)
            .was_clicked()
        {
            login!();
        }
        //Singleplayer button
        if Button::image(self.imgs.login_button)
            .hover_image(self.imgs.login_button_hover)
            .press_image(self.imgs.login_button_press)
            .w_h(258.0, 68.0)
            .down_from(self.ids.login_button, 20.0)
            .align_middle_x_of(self.ids.address_bg)
            .label("Singleplayer")
            .label_rgba(220.0, 220.0, 220.0, 0.8)
            .label_font_size(26)
            .label_y(conrod_core::position::Relative::Scalar(5.0))
            .label_x(conrod_core::position::Relative::Scalar(2.0))
            .set(self.ids.singleplayer_button, ui_widgets)
            .was_clicked()
        {
            login!();
        }
        // Quit
        if Button::image(self.imgs.button)
            .w_h(203.0, 53.0)
            .bottom_left_with_margins_on(ui_widgets.window, 60.0, 30.0)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .label("Quit")
            .label_rgba(220.0, 220.0, 220.0, 0.8)
            .label_font_size(20)
            .label_y(conrod_core::position::Relative::Scalar(3.0))
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
            .label_rgba(220.0, 220.0, 220.0, 0.8)
            .label_font_size(20)
            .label_y(conrod_core::position::Relative::Scalar(3.0))
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
            .label_rgba(220.0, 220.0, 220.0, 0.8)
            .label_font_size(20)
            .label_y(conrod_core::position::Relative::Scalar(3.0))
            .set(self.ids.servers_button, ui_widgets)
            .was_clicked()
        {};

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
