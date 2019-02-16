// Library
use conrod_core::{
    Positionable,
    Sizeable,
    Widget,
    widget_ids,
    event::{Widget as WidgetEvent, Input, Click},
    image::Id as ImgId,
    input::state::mouse::Button as MouseButton,
    widget::{
        Image,
        Button,
        Id as WidgId,
    }
};

// Crate
use crate::{
    window::Window,
    render::Renderer,
};

// Local
use super::Ui;

widget_ids!{
    struct Ids {
        menu_buttons[],
        bag,
        bag_contents,
        bag_close,
        menu_top,
        menu_mid,
        menu_bot,
    }
}

// TODO: make macro to mimic widget_ids! for images ids or find another solution to simplify addition of new images to the code.
struct Imgs {
    menu_button: ImgId,
    bag: ImgId,
    bag_hover: ImgId,
    bag_press: ImgId,
    bag_contents: ImgId,
    menu_top: ImgId,
    menu_mid: ImgId,
    menu_bot: ImgId,
    close_button: ImgId,
    close_button_hover: ImgId,
    close_button_press: ImgId,
}
impl Imgs {
    fn new(ui: &mut Ui, renderer: &mut Renderer) -> Imgs {
        let mut load = |filename| {
            let image = image::open(&[env!("CARGO_MANIFEST_DIR"), "/test_assets/", filename].concat()).unwrap();
            ui.new_image(renderer, &image).unwrap()
        };
        Imgs {
            menu_button: load("test_menu_button_blank.png"),
            bag: load("test_bag.png"),
            bag_hover: load("test_bag_hover.png"),
            bag_press: load("test_bag_press.png"),
            bag_contents: load("test_bag_contents.png"),
            menu_top: load("test_menu_top.png"),
            menu_mid: load("test_menu_midsection.png"),
            menu_bot: load("test_menu_bottom.png"),
            close_button: load("test_close_btn.png"),
            close_button_hover: load("test_close_btn_hover.png"),
            close_button_press: load("test_close_btn_press.png"),
        }
    }
}

pub struct TestUi {
    ui: Ui,
    ids: Ids,
    imgs: Imgs,
    bag_open: bool,
}

impl TestUi {
    pub fn new(window: &mut Window) -> Self {
        let mut ui = Ui::new(window).unwrap();
        // Generate ids
        let mut ids = Ids::new(ui.id_generator());
        ids.menu_buttons.resize(5, &mut ui.id_generator());
        // Load images
        let imgs = Imgs::new(&mut ui, window.renderer_mut());
        Self {
            ui,
            imgs,
            ids,
            bag_open: false,
        }
    }

    fn ui_layout(&mut self) {
        // Update if a event has occured
        if !self.ui.global_input().events().next().is_some() {
            return;
        }
        // Process input
        for e in self.ui.widget_input(self.ids.bag).events() {
            match e {
                WidgetEvent::Click(click) => match click.button {
                    MouseButton::Left => {
                        self.bag_open = true;
                    }
                    _ => {}
                }
                _ => {}
            }
        }
        for e in self.ui.widget_input(self.ids.bag_close).events() {
            match e {
                WidgetEvent::Click(click) => match click.button {
                    MouseButton::Left => {
                        self.bag_open = false;
                    }
                    _ => {}
                }
                _ => {}
            }
        }
        let bag_open = self.bag_open;
        let mut ui_cell = self.ui.set_widgets();
        // Bag
        Button::image(self.imgs.bag)
            .bottom_right_with_margin(20.0)
            .hover_image(if bag_open { self.imgs.bag } else { self.imgs.bag_hover })
            .press_image(if bag_open { self.imgs.bag } else { self.imgs.bag_press })
            .w_h(51.0, 58.0)
            .set(self.ids.bag, &mut ui_cell);
        // Bag contents
        if self.bag_open {
            // Contents
            Image::new(self.imgs.bag_contents)
                .w_h(212.0, 246.0)
                .x_y_relative_to(self.ids.bag, -92.0-25.5+12.0, 109.0+29.0-13.0)
                .set(self.ids.bag_contents, &mut ui_cell);
            // Close button
            Button::image(self.imgs.close_button)
                .w_h(20.0, 20.0)
                .hover_image(self.imgs.close_button_hover)
                .press_image(self.imgs.close_button_press)
                .top_right_with_margins_on(self.ids.bag_contents, 0.0, 10.0)
                .set(self.ids.bag_close, &mut ui_cell);
        }
    }

    pub fn handle_event(&mut self, input: Input) {
        self.ui.handle_event(input);
    }

    pub fn maintain(&mut self, renderer: &mut Renderer) {
        self.ui_layout();
        self.ui.maintain(renderer);
    }

    pub fn render(&self, renderer: &mut Renderer) {
        self.ui.render(renderer);
    }
}
