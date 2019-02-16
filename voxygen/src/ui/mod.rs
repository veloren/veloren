// TODO: cache entire UI render (would be somewhat pointless if we are planning on constantly animated ui)
// TODO: figure out proper way to propagate events down to the ui

// Library
use image::DynamicImage;
use conrod_core::{
    Ui as CrUi,
    UiBuilder,
    UiCell,
    image::{Map, Id as ImgId},
    widget::{Id as WidgId, id::Generator},
    render::Primitive,
    event::Input,
    input::{Global, Widget},
};

// Crate
use crate::{
    Error,
    render::{
        RenderError,
        Renderer,
        Model,
        Texture,
        UiPipeline,
        UiLocals,
        Consts,
        create_ui_quad_mesh,
    },
    window::Window,
};

#[derive(Debug)]
pub enum UiError {
    RenderError(RenderError),
}

pub struct Cache {
    model: Model<UiPipeline>,
    blank_texture: Texture<UiPipeline>,
}

// TODO: Should functions be returning UiError instead of Error?
impl Cache {
    pub fn new(renderer: &mut Renderer) -> Result<Self, Error> {
        Ok(Self {
            model: renderer.create_model(&create_ui_quad_mesh())?,
            blank_texture: renderer.create_texture(&DynamicImage::new_rgba8(1, 1))?,
        })
    }

    pub fn model(&self) -> &Model<UiPipeline> { &self.model }
    pub fn blank_texture(&self) -> &Texture<UiPipeline> { &self.blank_texture }
}

pub enum UiPrimitive {
    Image(Consts<UiLocals>, ImgId)
}

pub struct Ui {
    ui: CrUi,
    image_map: Map<Texture<UiPipeline>>,
    cache: Cache,
    // Primatives to draw on the next render
    ui_primitives: Vec<UiPrimitive>,
}

impl Ui {
    pub fn new(window: &mut Window) -> Result<Self, Error> {
        // Retrieve the logical size of the window content
        let (w, h) = window.logical_size();
        Ok(Self {
            ui: UiBuilder::new([w, h]).build(),
            image_map: Map::new(),
            cache: Cache::new(window.renderer_mut())?,
            ui_primitives: vec![],
        })
    }

    pub fn new_image(&mut self, renderer: &mut Renderer, image: &DynamicImage) -> Result<ImgId, Error> {
        Ok(self.image_map.insert(renderer.create_texture(image)?))
    }

    pub fn id_generator(&mut self) -> Generator {
        self.ui.widget_id_generator()
    }

    pub fn set_widgets(&mut self) -> UiCell {
        self.ui.set_widgets()
    }

    pub fn handle_event(&mut self, event: Input) {
        self.ui.handle_event(event);
    }

    pub fn widget_input(&self, id: WidgId) -> Widget {
        self.ui.widget_input(id)
    }

    pub fn global_input(&self) -> &Global {
        self.ui.global_input()
    }

    pub fn maintain(&mut self, renderer: &mut Renderer) {
        let ref mut ui = self.ui;
        // removed this because ui will resize itself now that it recieves events
        // update window size
        //let res = renderer.get_resolution().map(|e| e as f64);
        //if res[0] != ui.win_w || res[1] != ui.win_h {
        //    ui.win_w = res[0];
        //    ui.win_h = res[1];
        //    ui.needs_redraw();
        //}
        // Gather primatives and recreate locals only if ui_changed
        if let Some(mut primitives) = ui.draw_if_changed() {
            self.ui_primitives.clear();
            while let Some(prim) = primitives.next() {
                // Transform from conrod to our render coords
                // Conrod uses the center of the screen as the origin
                // Up & Right are positive directions
                let x = prim.rect.left();
                let y = prim.rect.top();
                let (w, h) = prim.rect.w_h();
                let bounds = [
                    (x / ui.win_w + 0.5) as f32,
                    (-1.0 * (y / ui.win_h) + 0.5) as f32,
                    (w / ui.win_w) as f32,
                    (h / ui.win_h) as f32
                ];
                // TODO: Remove this
                let new_ui_locals = renderer.create_consts(&[UiLocals::new(bounds)])
                                            .expect("Could not create new const for ui locals");
                use conrod_core::render::{PrimitiveKind};
                // TODO: Use scizzor
                let Primitive {kind, scizzor, id, ..} = prim;
                match kind {
                    PrimitiveKind::Image { image_id, color, source_rect } => {
                        //renderer.update_consts(&mut self.locals, &[UiLocals::new(
                        //        [0.0, 0.0, 1.0, 1.0],
                        //    )]);
                        self.ui_primitives.push(UiPrimitive::Image(new_ui_locals, image_id));
                    }
                    _ => {}
                    // TODO: Add these
                    //PrimitiveKind::Other {..} => {println!("primitive kind other with id {:?}", id);}
                    //PrimitiveKind::Rectangle { color } => {println!("primitive kind rect[x:{},y:{},w:{},h:{}] with color {:?} and id {:?}", x, y, w, h, color, id);}
                    //PrimitiveKind::Text {..} => {println!("primitive kind text with id {:?}", id);}
                    //PrimitiveKind::TrianglesMultiColor {..} => {println!("primitive kind multicolor with id {:?}", id);}
                    //PrimitiveKind::TrianglesSingleColor {..} => {println!("primitive kind singlecolor with id {:?}", id);}
                }
            }
        }
    }

    pub fn render(&self, renderer: &mut Renderer) {
        self.ui_primitives.iter().for_each(|ui_primitive| match ui_primitive {
            UiPrimitive::Image(ui_locals, image_id) => {
                let tex = self.image_map.get(&image_id).expect("Image does not exist in image map");
                renderer.render_ui_element(&self.cache.model(), &ui_locals, &tex);
            }
        });
    }
}
