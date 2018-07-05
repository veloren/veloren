use conrod::{
    Ui as conrod_ui,
    UiBuilder,
    image::Map,
    color,
    widget::{
        self,
        triangles::Triangle,
    },
    Widget,
    render::Primitives,
    Colorable,
    Sizeable,
    Positionable,
    Borderable,
    Scalar,
    UiCell,
    widget::Id as wid,
    text::font::Id as fid,
    event::Input,
};

use std::collections::HashMap;

pub use gfx_device_gl::Resources as ui_resources;
pub use conrod::gfx_core::handle::ShaderResourceView;

// UI image assets if I understand correctly
pub type ImageMap = Map<(ShaderResourceView<ui_resources, [f32; 4]>, (u32, u32))>;

pub struct Ui {
    ui: conrod_ui,
    image_map: ImageMap,
    fid: Option<fid>,
    ids: HashMap<String, widget::Id>,
}


impl Ui {
    pub fn new(size: [f64; 2]) -> Self {
        let ui = UiBuilder::new(size).build();
        let image_map = Map::new();

        Self {
            ui,
            image_map,
            fid: None,
            ids: HashMap::new(),
        }
    }

    pub fn add_version_number(&mut self) {
        let fid =self.ui.fonts.insert_from_file("assets/fonts/NotoSans-Regular.ttf").unwrap();
        self.ui.theme.font_id = Some(fid);
        self.fid = Some(fid);
    }

    pub fn generate_widget_id(&mut self) -> widget::Id {
        self.ui.widget_id_generator().next()
    }

    pub fn get_widget_id<T>(&mut self, widget_name: T) -> widget::Id where T: Into<String> {
        let key = widget_name.into();
        if self.ids.contains_key(&key) {
            *self.ids.get(&key).unwrap()
        } else {
            println!("Generated new widget_id: {}", key);
            let id = self.generate_widget_id();
            self.ids.insert(key, id);
            id
        }
    }

    pub fn set_ui(&mut self) {
        let w = self.ui.win_w;
        let h = self.ui.win_h;

        let left_text = self.get_widget_id("left_text");

        let mut ui = self.ui.set_widgets();
        let font = self.fid.unwrap();
        let ids = &self.ids;

        const PAD: Scalar = 20.0;

        widget::Text::new(&format!("Version {}", env!("CARGO_PKG_VERSION")))
            .font_id(font)
            .color(color::LIGHT_RED)
            .bottom_left_with_margin(PAD)
            .left_justify()
            .line_spacing(10.0)
            .set(left_text, &mut ui);
    }

    pub fn set_size(&mut self, w: u32, h: u32) {
        self.ui.handle_event(Input::Resize(w, h));
    }

    pub fn get_image_map(&self) -> &ImageMap {
        &self.image_map
    }

    pub fn get_primitives(&self) -> Primitives {
        self.ui.draw()
    }
}