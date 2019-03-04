// Standard
use std::rc::Rc;

// Library
use image::DynamicImage;
use vek::*;

// Crate
use crate::render::{
    Consts,
    UiLocals,
    Renderer,
    Texture,
    UiPipeline,
};

// Local
use super::{
    super::{
        UiError,
        Cache,
    },
    Element,
    Bounds,
    SizeRequest,
};

#[derive(Clone)]
pub struct Image {
    texture: Rc<Texture<UiPipeline>>,
    locals: Consts<UiLocals>,
}

impl Image {
    pub fn new(renderer: &mut Renderer, image: &DynamicImage) -> Result<Self, UiError> {
        Ok(Self {
            texture: Rc::new(
                renderer.create_texture(image)
                    .map_err(|err| UiError::RenderError(err))?
            ),
            locals: renderer.create_consts(&[UiLocals::default()])
                .map_err(|err| UiError::RenderError(err))?,
        })
    }
}

impl Element for Image {
    fn get_hsize_request(&self) -> SizeRequest { SizeRequest::indifferent() }
    fn get_vsize_request(&self) -> SizeRequest { SizeRequest::indifferent() }

    fn maintain(
        &mut self,
        renderer: &mut Renderer,
        cache: &Cache,
        bounds: Bounds<f32>,
        resolution: Vec2<f32>,
    ) {
        renderer.update_consts(&mut self.locals, &[UiLocals::new(
            [bounds.x, bounds.y, bounds.w, bounds.h],
        )])
            .expect("Could not update UI image consts");
    }

    fn render(
        &self,
        renderer: &mut Renderer,
        cache: &Cache,
        bounds: Bounds<f32>,
        resolution: Vec2<f32>,
    ) {
        renderer.render_ui_element(
            cache.model(),
            &self.locals,
            &self.texture,
        );
    }
}
