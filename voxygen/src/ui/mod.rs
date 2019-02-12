//pub mod element;
//pub mod size_request;
//pub mod span;

// Reexports
/*pub use self::{
    span::Span,
    size_request::SizeRequest,
};*/

// TODO: what was the purpose of size request?
// TODO: cache entire UI render
// TODO: do we need to store locals for each widget?
// TODO: sizing? : renderer.get_resolution().map(|e| e as f32)

// Library
use image::DynamicImage;
use conrod_core::{
    Ui as CrUi,
    UiBuilder,
    UiCell,
    image::{Map, Id as ImgId},
    widget::Id as WidgId,
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
};

// Local
/*use self::element::{
    Element,
    Bounds,
};*/

#[derive(Debug)]
pub enum UiError {
    RenderError(RenderError),
}

pub struct Cache {
    model: Model<UiPipeline>,
    blank_texture: Texture<UiPipeline>,
}

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

pub struct Ui {
    ui: CrUi,
    image_map: Map<Texture<UiPipeline>>,
    cache: Cache,
    locals: Consts<UiLocals>,
}

impl Ui {
    pub fn new(renderer: &mut Renderer, dim: [f64; 2]) -> Result<Self, Error> {
        Ok(Self {
            ui: UiBuilder::new(dim).build(),
            image_map: Map::new(),
            cache: Cache::new(renderer)?,
            locals: renderer.create_consts(&[UiLocals::default()])?,

        })
    }

    pub fn new_image(&mut self, renderer: &mut Renderer, image: &DynamicImage) -> Result<ImgId, Error> {
        Ok(self.image_map.insert(renderer.create_texture(image)?))
    }

    pub fn new_widget(&mut self) -> WidgId {
        self.ui.widget_id_generator().next()
    }

    pub fn set_widgets<F>(&mut self, f: F) where F: FnOnce(&mut UiCell) {
        f(&mut self.ui.set_widgets());
    }

    // TODO: change render back to &self and use maintain for mutable operations
    pub fn maintain(&mut self, renderer: &mut Renderer) {

        //renderer.get_resolution().map(|e| e as f32),
    }

    pub fn render(&mut self, renderer: &mut Renderer) {

        if let Some(mut primitives) = Some(self.ui.draw()) {
            //render the primatives one at a time
            while let Some(prim) = primitives.next() {
                use conrod_core::render::{Primitive, PrimitiveKind};
                let Primitive {kind, scizzor, rect, ..} = prim;
                match kind {
                    PrimitiveKind::Image { image_id, color, source_rect } => {
                        renderer.update_consts(&mut self.locals, &[UiLocals::new(
                                [0.0, 0.0, 1.0, 1.0],
                            )]);
                        let tex = self.image_map.get(&image_id).expect("Image does not exist in image map");
                        renderer.render_ui_element(&self.cache.model(), &self.locals, &tex);
                    }
                    PrimitiveKind::Other {..} => {println!("primitive kind other with id");}
                    PrimitiveKind::Rectangle {..} => {println!("primitive kind rect");}
                    PrimitiveKind::Text {..} => {println!("primitive kind text");}
                    PrimitiveKind::TrianglesMultiColor {..} => {println!("primitive kind multicolor");}
                    PrimitiveKind::TrianglesSingleColor {..} => {println!("primitive kind singlecolor");}
                }
            }
        }
    }
}
