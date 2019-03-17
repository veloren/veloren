use crate::{render::Renderer, ui::Ui, window::Window};
use conrod_core::{
    event::Input,
    image::Id as ImgId,
    widget::{Id as WidgId, Image as ImageWidget},
    Positionable, Widget,
};

pub struct CharSelectionUi {
    ui: Ui,
    widget_id: WidgId,
    splash_img_id: ImgId,
}

impl CharSelectionUi {
    pub fn new(window: &mut Window) -> Self {
        let mut ui = Ui::new(window).unwrap();
        let widget_id = ui.id_generator().next();
        let image = image::open(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_assets/ui/char_selection/splash.png"
        ))
        .unwrap();
        let splash_img_id = ui.new_image(window.renderer_mut(), &image).unwrap();
        Self {
            ui,
            widget_id,
            splash_img_id,
        }
    }

    fn ui_layout(&mut self) {
        let mut ui_cell = self.ui.set_widgets();
        ImageWidget::new(self.splash_img_id)
            .top_left()
            .set(self.widget_id, &mut ui_cell);
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
