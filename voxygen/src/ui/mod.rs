pub mod element;
pub mod size_request;
pub mod span;

// Reexports
pub use self::{
    span::Span,
    size_request::SizeRequest,
};

// Library
use image::DynamicImage;

// Crate
use crate::{
    Error,
    render::{
        RenderError,
        Renderer,
        Model,
        Texture,
        UiPipeline,
        create_ui_quad_mesh,
    },
};

// Local
use self::element::{
    Element,
    Bounds,
};

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
    base: Box<dyn Element>,
    cache: Cache,
}

impl Ui {
    pub fn new<E: Element>(renderer: &mut Renderer, base: Box<E>) -> Result<Self, Error> {
        Ok(Self {
            base,
            cache: Cache::new(renderer)?,
        })
    }

    pub fn maintain(&mut self, renderer: &mut Renderer) {
        self.base.maintain(
            renderer,
            &self.cache,
            Bounds::new(0.0, 0.0, 1.0, 1.0),
            renderer.get_resolution().map(|e| e as f32),
        )
    }

    pub fn render(&self, renderer: &mut Renderer) {
        self.base.render(
            renderer,
            &self.cache,
            Bounds::new(0.0, 0.0, 1.0, 1.0),
            renderer.get_resolution().map(|e| e as f32),
        );
    }
}
