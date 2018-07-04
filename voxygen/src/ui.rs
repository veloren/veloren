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
    Scalar
};

pub use gfx_device_gl::Resources as ui_resources;
pub use conrod::gfx_core::handle::ShaderResourceView;

// UI image assets if I understand correctly
pub type ImageMap = Map<(ShaderResourceView<ui_resources, [f32; 4]>, (u32, u32))>;

pub struct Ui {
    ui: conrod_ui,
    image_map: ImageMap,
}

impl Ui {
    pub fn new(size: [f64; 2]) -> Self {
        let ui = UiBuilder::new(size).build();
        let image_map = Map::new();

        Self {
            ui,
            image_map,
        }
    }

    pub fn add_version_number(&mut self) {
        widget_ids!{
            struct Ids {
                master,
                left_col,
                middle_col,
                right_col,
                left_text,
                middle_text,
                right_text,
            }
        }

        let ids = Ids::new(self.ui.widget_id_generator());

        let font = self.ui.fonts.insert_from_file("assets/fonts/NotoSans-Regular.ttf").unwrap();
        self.ui.theme.font_id = Some(font);

        const DEMO_TEXT: &'static str = "Version 0.1 Alpha";

        widget::Text::new(DEMO_TEXT)
            .font_id(font)
            .color(color::LIGHT_RED)
            .bottom_left_of(self.ui.window)
            .set(ids.left_text, &mut self.ui.set_widgets());
    }

    pub fn get_image_map(&self) -> &ImageMap {
        &self.image_map
    }

    pub fn get_primitives(&self) -> Primitives {
        self.ui.draw()
    }
}