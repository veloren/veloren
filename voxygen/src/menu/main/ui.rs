use conrod_core::{
    Positionable,
    Sizeable,
    Widget,
    Labelable,
    Colorable,
    Borderable,
    widget_ids,
    event::Input,
    image::Id as ImgId,
    text::font::Id as FontId,
    widget::{
        Image,
        Button,
        Canvas,
        TextBox,
        text_box::Event as TextBoxEvent,
    }
};
use crate::{
    window::Window,
    render::Renderer,
    ui::{Ui, ScaleMode}
};

widget_ids!{
    struct Ids {
        // Background and logo
        bg,
        v_logo,
        // Login
        login_button,
        login_text,
        address_text,
        address_bg,
        address_field,
        username_text,
        username_bg,
        username_field,
        // Buttons
        servers_button,
        servers_text,
        settings_button,
        settings_text,
        quit_button,
        quit_text,
    }
}

struct Imgs {
    bg: ImgId,
    v_logo: ImgId,

    address_text: ImgId,
    username_text: ImgId,
    input_bg: ImgId,

    login_text: ImgId,
    login_button: ImgId,
    login_button_hover: ImgId,
    login_button_press: ImgId,

    servers_text: ImgId,
    settings_text: ImgId,
    quit_text: ImgId,
    button: ImgId,
    button_hover: ImgId,
    button_press: ImgId,
}
impl Imgs {
    fn new(ui: &mut Ui, renderer: &mut Renderer) -> Imgs {
        let mut load = |filename| {
            let image = image::open(&[env!("CARGO_MANIFEST_DIR"), "/test_assets/ui/main/", filename].concat()).unwrap();
            ui.new_image(renderer, &image).unwrap()
        };
        Imgs {
            bg: load("bg.png"),
            v_logo: load("v_logo_a01.png"),

            // Input fields
            address_text: load("text/server_address.png"),
            username_text: load("text/username.png"),
            input_bg: load("input_bg.png"),

            // Login button
            login_text: load("text/login.png"),
            login_button: load("buttons/button_login.png"),
            login_button_hover: load("buttons/button_login_hover.png"),
            login_button_press: load("buttons/button_login_press.png"),

            // Servers, settings, and quit buttons
            servers_text: load("text/servers.png"),
            settings_text: load("text/settings.png"),
            quit_text: load("text/quit.png"),
            button: load("buttons/button.png"),
            button_hover: load("buttons/button_hover.png"),
            button_press: load("buttons/button_press.png"),
        }
    }
}

pub struct MainMenuUi {
    ui: Ui,
    ids: Ids,
    imgs: Imgs,
    font_id: FontId,
	username: String,
    server_address: String,
    attempt_login: bool,
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
        // Load font
        let font_id = ui.new_font(conrod_core::text::font::from_file(
            concat!(env!("CARGO_MANIFEST_DIR"), "/test_assets/font/Metamorphous-Regular.ttf")
        ).unwrap());
        Self {
            ui,
            imgs,
            ids,
            font_id,
			username: "Username".to_string(),
            server_address: "Server-Address".to_string(),
            attempt_login: false,
        }
    }

    // TODO: probably a better way to do this
    pub fn login_attempt(&mut self) -> Option<(String, String)> {
        if self.attempt_login {
            self.attempt_login = false;
            Some((self.username.clone(), self.server_address.clone()))
        } else {
            None
        }
    }

    fn update_layout(&mut self) {
        let ref mut ui_widgets = self.ui.set_widgets();
        // Background image & Veloren logo
    	Image::new(self.imgs.bg)
    	   .middle_of(ui_widgets.window)
    	   .set(self.ids.bg, ui_widgets);
        Image::new(self.imgs.v_logo)
            .w_h(346.0, 111.0)
            .top_left_with_margins(30.0, 40.0)
            .set(self.ids.v_logo, ui_widgets);

        // Input fields
        // Used when the login button is pressed, or enter is pressed within input field
        macro_rules! login {
            () => {
                self.attempt_login = true;
            }
        }
        use conrod_core::color::TRANSPARENT;
        // Username
        // TODO: get a lower resolution and cleaner input_bg.png
        Image::new(self.imgs.input_bg)
            .w_h(672.0/2.0, 166.0/2.0)
            .middle_of(ui_widgets.window)
            .set(self.ids.username_bg, ui_widgets);
        Image::new(self.imgs.username_text)
            .w_h(149.0, 24.0)
            .up(0.0)
            .align_left()
            .set(self.ids.username_text, ui_widgets);
        // TODO: figure out why cursor is rendered inconsistently
        for event in TextBox::new(&self.username)
            .w_h(580.0/2.0, 60.0/2.0)
            .mid_bottom_with_margin_on(self.ids.username_bg, 44.0/2.0)
            .font_size(20)
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
        Image::new(self.imgs.address_text)
            .w_h(227.0, 28.0)
            .down_from(self.ids.username_bg, 10.0)
            .align_left_of(self.ids.username_bg)
            .set(self.ids.address_text, ui_widgets);
        Image::new(self.imgs.input_bg)
            .w_h(672.0/2.0, 166.0/2.0)
            .down(0.0)
            .align_left()
            .set(self.ids.address_bg, ui_widgets);
        for event in TextBox::new(&self.server_address)
            .w_h(580.0/2.0, 60.0/2.0)
            .mid_bottom_with_margin_on(self.ids.address_bg, 44.0/2.0)
            .font_size(20)
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
            .set(self.ids.login_button, ui_widgets)
            .was_clicked()
            {
                login!();
            }
        Image::new(self.imgs.login_text)
            .w_h(83.0, 34.0)
            .graphics_for(self.ids.login_button) // capture the input for the button
            .middle_of(self.ids.login_button)
            .set(self.ids.login_text, ui_widgets);

        // Other buttons
        // Quit
        Button::image(self.imgs.button)
            .w_h(203.0, 53.0)
            .bottom_left_with_margins_on(ui_widgets.window, 60.0, 30.0)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .set(self.ids.quit_button, ui_widgets);
        Image::new(self.imgs.quit_text)
            .w_h(52.0, 26.0)
            .graphics_for(self.ids.quit_button) // capture the input for the button
            .middle_of(self.ids.quit_button)
            .set(self.ids.quit_text, ui_widgets);
        // Settings
        Button::image(self.imgs.button)
            .w_h(203.0, 53.0)
            .up_from(self.ids.quit_button, 8.0)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .set(self.ids.settings_button, ui_widgets);
        Image::new(self.imgs.settings_text)
            .w_h(98.0, 28.0)
            .graphics_for(self.ids.settings_button)
            .middle_of(self.ids.settings_button)
            .set(self.ids.settings_text, ui_widgets);
        // Servers
        Button::image(self.imgs.button)
            .w_h(203.0, 53.0)
            .up_from(self.ids.settings_button, 8.0)
            .hover_image(self.imgs.button_hover)
            .press_image(self.imgs.button_press)
            .set(self.ids.servers_button, ui_widgets);
        Image::new(self.imgs.servers_text)
            .w_h(93.0, 20.0)
            .graphics_for(self.ids.servers_button)
            .middle_of(self.ids.servers_button)
            .set(self.ids.servers_text, ui_widgets);
    }

    pub fn handle_event(&mut self, input: Input) {
        self.ui.handle_event(input);
    }

    pub fn maintain(&mut self, renderer: &mut Renderer) {
        self.update_layout();
        self.ui.maintain(renderer);
    }

    pub fn render(&self, renderer: &mut Renderer) {
        self.ui.render(renderer);
    }
}
