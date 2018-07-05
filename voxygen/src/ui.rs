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
    backend::gfx::Renderer as ConrodRenderer,
};

use renderer::Renderer;

use std::collections::HashMap;

pub use gfx_device_gl::Resources as ui_resources;
pub use conrod::gfx_core::handle::ShaderResourceView;

// UI image assets if I understand correctly
pub type ImageMap = Map<(ShaderResourceView<ui_resources, [f32; 4]>, (u32, u32))>;

pub struct Ui {
    conrodRenderer: ConrodRenderer<'static, ui_resources>,
    ui: conrod_ui,
    image_map: ImageMap,
    fid: Option<fid>,
    ids: HashMap<String, widget::Id>,
}


impl Ui {
    pub fn new(renderer: &mut Renderer, size: [f64; 2]) -> Self {
        let ui = UiBuilder::new(size).build();

        let image_map = Map::new();

        let color_view = renderer.color_view().clone();
        let mut factory = renderer.factory_mut().clone();

        let conrodRenderer = ConrodRenderer::new(&mut factory, &color_view , 1.0).unwrap();

        Self {
            conrodRenderer,
            ui,
            image_map,
            fid: None,
            ids: HashMap::new(),
        }
    }

    pub fn render(&mut self, renderer: &mut Renderer, window_size: &[f64; 2]) {
        self.ui.handle_event(Input::Resize(window_size[0] as u32, window_size[1] as u32));
        self.set_ui();
        self.conrodRenderer.on_resize(renderer.color_view().clone());
        self.conrodRenderer.fill(&mut renderer.encoder_mut(), (window_size[0] as f32 , window_size[1] as f32), 1.0, self.ui.draw(), &self.image_map);
        self.conrodRenderer.draw(&mut renderer.factory_mut().clone(), &mut renderer.encoder_mut(), &self.image_map);
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
        let left_text = self.get_widget_id("left_text");

        let mut ui = self.ui.set_widgets();
        let font = self.fid.unwrap();

        widget::Text::new(&format!("Version {}", env!("CARGO_PKG_VERSION")))
            .font_id(font)
            .color(color::LIGHT_RED)
            .bottom_left_with_margin(20.0)
            .left_justify()
            .line_spacing(10.0)
            .set(left_text, &mut ui);
    }
}