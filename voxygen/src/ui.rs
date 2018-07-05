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

pub use gfx_device_gl::Resources as ui_resources;
pub use conrod::gfx_core::handle::ShaderResourceView;

// UI image assets if I understand correctly
pub type ImageMap = Map<(ShaderResourceView<ui_resources, [f32; 4]>, (u32, u32))>;

pub struct Ui {
    ui: conrod_ui,
    image_map: ImageMap,
    fid: Option<fid>,
    ids: Ids,
}

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


impl Ui {
    pub fn new(size: [f64; 2]) -> Self {
        let mut ui = UiBuilder::new(size).build();
        let image_map = Map::new();
        let ids = Ids::new(ui.widget_id_generator());

        Self {
            ui,
            image_map,
            fid: None,
            ids,
        }
    }

    pub fn add_version_number(&mut self) {
        let fid =self.ui.fonts.insert_from_file("assets/fonts/NotoSans-Regular.ttf").unwrap();
        self.ui.theme.font_id = Some(fid);
        self.fid = Some(fid);
    }

    pub fn set_ui(&mut self) {
        let w = self.ui.win_w;
        let h = self.ui.win_h;

        let mut ui = self.ui.set_widgets();
        let font = self.fid.unwrap();
        let ids = &self.ids;


        use conrod::{color, widget, Colorable, Positionable, Scalar, Sizeable, Widget};

        // Our `Canvas` tree, upon which we will place our text widgets.
        widget::Canvas::new().flow_right(&[
            (ids.left_col, widget::Canvas::new().color(color::BLACK)),
            (ids.middle_col, widget::Canvas::new().color(color::DARK_CHARCOAL)),
            (ids.right_col, widget::Canvas::new().color(color::CHARCOAL)),
        ]).pad(100.0).color(color::DARK_RED).set(ids.master, &mut ui);

        const DEMO_TEXT: &'static str = "Version 0.1 Alpha";
        const PAD: Scalar = 20.0;

        widget::Text::new(DEMO_TEXT)
            .font_id(font)
            .color(color::LIGHT_RED)
            .padded_w_of(ids.left_col, PAD)
            .mid_top_with_margin_on(ids.left_col, PAD)
            .left_justify()
            .line_spacing(10.0)
            .set(ids.left_text, &mut ui);

        widget::Text::new(DEMO_TEXT)
            .font_id(font)
            .color(color::LIGHT_GREEN)
            .padded_w_of(ids.middle_col, PAD)
            .middle_of(ids.middle_col)
            .center_justify()
            .line_spacing(2.5)
            .set(ids.middle_text, &mut ui);

        widget::Text::new(DEMO_TEXT)
            .font_id(font)
            .color(color::LIGHT_BLUE)
            .padded_w_of(ids.right_col, PAD)
            .mid_bottom_with_margin_on(ids.right_col, PAD)
            .right_justify()
            .line_spacing(5.0)
            .set(ids.right_text, &mut ui);

    }

    pub fn set_size(&mut self, w: u32, h: u32) {
//        println!("{:?}", (w, h));
//        self.ui.handle_event(Input::Resize(w, h));
        self.ui.win_w = w as f64;
        self.ui.win_h = h as f64;
    }

    pub fn get_image_map(&self) -> &ImageMap {
        &self.image_map
    }

    pub fn get_primitives(&self) -> Primitives {
        self.ui.draw()
    }
}