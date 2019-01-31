pub mod image;

// Standard
use std::rc::Rc;

// Library
use vek::*;

// Crate
use crate::render::{
    Renderer,
    Texture,
    Consts,
    UiLocals,
    UiPipeline,
};

// Local
use super::{
    UiError,
    Cache,
    Span,
    SizeRequest,
};

// Bounds

pub type Bounds<T> = Rect<T, T>;

pub trait BoundsExt {
    fn relative_to(self, other: Self) -> Self;
}

impl BoundsExt for Bounds<f32> {
    fn relative_to(self, other: Self) -> Self {
        Self::new(
            other.x + self.x * other.w,
            other.y + self.y * other.h,
            self.w * other.w,
            self.h * other.h,
        )
    }
}

pub trait BoundsSpan {
    fn in_resolution(self, resolution: Vec2<f32>) -> Bounds<f32>;
}

impl BoundsSpan for Bounds<Span> {
    fn in_resolution(self, resolution: Vec2<f32>) -> Bounds<f32> {
        Bounds::new(
            self.x.to_rel(resolution.x).rel,
            self.y.to_rel(resolution.y).rel,
            self.w.to_rel(resolution.x).rel,
            self.h.to_rel(resolution.y).rel,
        )
    }
}

// Element

pub trait Element: 'static {
    //fn deep_clone(&self) -> Rc<dyn Element>;

    fn get_hsize_request(&self) -> SizeRequest;
    fn get_vsize_request(&self) -> SizeRequest;

    fn maintain(
        &mut self,
        renderer: &mut Renderer,
        cache: &Cache,
        bounds: Bounds<f32>,
        resolution: Vec2<f32>,
    );

    fn render(
        &self,
        renderer: &mut Renderer,
        cache: &Cache,
        bounds: Bounds<f32>,
        resolution: Vec2<f32>,
    );
}

// Surface

#[derive(Clone)]
pub enum Surface {
    Transparent,
    Color(Rgba<f32>),
    Texture(Rc<Texture<UiPipeline>>),
    Bevel,
}

// Widget

#[derive(Clone)]
pub struct Widget<E: Element> {
    inner: Box<E>,
    background: Surface,
    margin_top: Span,
    margin_bottom: Span,
    margin_left: Span,
    margin_right: Span,
    locals: Consts<UiLocals>,
}

impl<E: Element> Widget<E> {
    pub fn new(renderer: &mut Renderer, inner: E) -> Result<Box<Self>, UiError> {
        Ok(Box::new(Self {
            inner: Box::new(inner),
            background: Surface::Transparent,
            margin_top: Span::rel(0.2),
            margin_bottom: Span::rel(0.2),
            margin_left: Span::rel(0.2),
            margin_right: Span::rel(0.2),
            locals: renderer.create_consts(&[UiLocals::default()])
                .map_err(|err| UiError::RenderError(err))?,
        }))
    }

    fn get_inner_bounds(&self) -> Bounds<Span> {
        Bounds::new(
            self.margin_left,
            self.margin_top,
            Span::full() - self.margin_left - self.margin_right,
            Span::full() - self.margin_top - self.margin_bottom,
        )
    }
}

impl<E: Element> Element for Widget<E> {
    fn get_hsize_request(&self) -> SizeRequest {
        self.inner.get_hsize_request() + self.margin_left + self.margin_right
    }

    fn get_vsize_request(&self) -> SizeRequest {
        self.inner.get_vsize_request() + self.margin_top + self.margin_bottom
    }

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

        let inner_bounds = self
            .get_inner_bounds()
            .in_resolution(resolution)
            .relative_to(bounds);

        self.inner.maintain(renderer, cache, inner_bounds, resolution);
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
            &cache.blank_texture(),
        );

        let inner_bounds = self
            .get_inner_bounds()
            .in_resolution(resolution)
            .relative_to(bounds);

        self.inner.render(renderer, cache, inner_bounds, resolution);
    }
}
