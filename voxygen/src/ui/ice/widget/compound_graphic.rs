use super::image::Handle;
use iced::{layout, Element, Hasher, Layout, Length, Point, Rectangle, Widget};
use std::hash::Hash;
use vek::{Aabr, Rgba, Vec2};

// TODO: this widget combines multiple images in precise ways, they may or may
// nor overlap and it would be helpful for optimising the renderer by telling it
// if there is no overlap (i.e. draw calls can be reordered freely), we don't
// need to do this yet since the renderer isn't that advanced

// TODO: design trait to interface with background container
#[derive(Copy, Clone)]
pub enum GraphicKind {
    Image(Handle, Rgba<u8>),
    Color(Rgba<u8>),
    /// Vertical gradient
    Gradient(Rgba<u8>, Rgba<u8>),
}

// TODO: consider faculties for composing compound graphics (if a use case pops
// up)
pub struct Graphic {
    aabr: Aabr<u16>,
    kind: GraphicKind,
}

impl Graphic {
    fn new(kind: GraphicKind, size: [u16; 2], offset: [u16; 2]) -> Self {
        let size = Vec2::from(size);
        let offset = Vec2::from(offset);
        Self {
            aabr: Aabr {
                min: offset,
                max: offset + size,
            },
            kind,
        }
    }

    pub fn image(handle: Handle, size: [u16; 2], offset: [u16; 2]) -> Self {
        Self::new(
            GraphicKind::Image(handle, Rgba::broadcast(255)),
            size,
            offset,
        )
    }

    pub fn color(mut self, color: Rgba<u8>) -> Self {
        match &mut self.kind {
            GraphicKind::Image(_, c) => *c = color,
            GraphicKind::Color(c) => *c = color,
            // Not relevant here
            GraphicKind::Gradient(_, _) => (),
        }
        self
    }

    pub fn gradient(
        top_color: Rgba<u8>,
        bottom_color: Rgba<u8>,
        size: [u16; 2],
        offset: [u16; 2],
    ) -> Self {
        Self::new(GraphicKind::Gradient(top_color, bottom_color), size, offset)
    }

    pub fn rect(color: Rgba<u8>, size: [u16; 2], offset: [u16; 2]) -> Self {
        Self::new(GraphicKind::Color(color), size, offset)
    }
}

pub struct CompoundGraphic {
    graphics: Vec<Graphic>,
    // move into option inside fix_aspect_ratio?
    graphics_size: [u16; 2],
    width: Length,
    height: Length,
    fix_aspect_ratio: bool,
    /* TODO: allow coloring the widget as a whole (if there is a use case)
     *color: Rgba<u8>, */
}

impl CompoundGraphic {
    pub fn from_graphics(graphics: Vec<Graphic>) -> Self {
        let width = Length::Fill;
        let height = Length::Fill;
        let graphics_size = graphics
            .iter()
            .fold(Vec2::zero(), |size, graphic| {
                Vec2::max(size, graphic.aabr.max)
            })
            .into_array();
        Self {
            graphics,
            graphics_size,
            width,
            height,
            fix_aspect_ratio: false,
            //color: Rgba::broadcast(255),
        }
    }

    pub fn padded_image(image: Handle, size: [u16; 2], pad: [u16; 4]) -> Self {
        let image = Graphic::image(image, size, [pad[0], pad[1]]);
        let mut this = Self::from_graphics(vec![image]);
        this.graphics_size[0] += pad[2];
        this.graphics_size[1] += pad[3];
        this
    }

    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    pub fn fix_aspect_ratio(mut self) -> Self {
        self.fix_aspect_ratio = true;
        self
    }

    //pub fn color(mut self, color: Rgba<u8>) -> Self {
    //    self.color = color;
    //    self
    //}

    fn draw<R: self::Renderer>(
        &self,
        renderer: &mut R,
        _defaults: &R::Defaults,
        layout: Layout<'_>,
        _cursor_position: Point,
    ) -> R::Output {
        let [pixel_w, pixel_h] = self.graphics_size;
        let bounds = layout.bounds();
        let scale = Vec2::new(
            bounds.width / pixel_w as f32,
            bounds.height / pixel_h as f32,
        );
        let graphics = self.graphics.iter().map(|graphic| {
            let bounds = {
                let Aabr { min, max } = graphic.aabr.map(|e| e as f32);
                let min = min * scale;
                let size = max * scale - min;
                Rectangle {
                    x: min.x + bounds.x,
                    y: min.y + bounds.y,
                    width: size.x,
                    height: size.y,
                }
            };
            (bounds, graphic.kind)
        });

        renderer.draw(graphics, /* self.color, */ layout)
    }
}

impl<M, R> Widget<M, R> for CompoundGraphic
where
    R: self::Renderer,
{
    fn width(&self) -> Length { self.width }

    fn height(&self) -> Length { self.height }

    fn layout(&self, _renderer: &R, limits: &layout::Limits) -> layout::Node {
        let mut size = limits.width(self.width).height(self.height).max();

        if self.fix_aspect_ratio {
            let aspect_ratio = {
                let [w, h] = self.graphics_size;
                w as f32 / h as f32
            };

            let max_aspect_ratio = size.width / size.height;

            if max_aspect_ratio > aspect_ratio {
                size.width = size.height * aspect_ratio;
            } else {
                size.height = size.width / aspect_ratio;
            }
        }

        layout::Node::new(size)
    }

    fn draw(
        &self,
        renderer: &mut R,
        defaults: &R::Defaults,
        layout: Layout<'_>,
        cursor_position: Point,
    ) -> R::Output {
        Self::draw(self, renderer, defaults, layout, cursor_position)
    }

    fn hash_layout(&self, state: &mut Hasher) {
        struct Marker;
        std::any::TypeId::of::<Marker>().hash(state);

        self.width.hash(state);
        self.height.hash(state);
        if self.fix_aspect_ratio {
            self.graphics_size.hash(state);
        }
    }
}

pub trait Renderer: iced::Renderer {
    fn draw<I>(
        &mut self,
        graphics: I,
        //color: Rgba<u8>,
        layout: Layout<'_>,
    ) -> Self::Output
    where
        I: Iterator<Item = (Rectangle, GraphicKind)>;
}

impl<'a, M, R> From<CompoundGraphic> for Element<'a, M, R>
where
    R: self::Renderer,
{
    fn from(compound_graphic: CompoundGraphic) -> Element<'a, M, R> {
        Element::new(compound_graphic)
    }
}

impl<R> super::background_container::Background<R> for CompoundGraphic
where
    R: self::Renderer,
{
    fn width(&self) -> Length { self.width }

    fn height(&self) -> Length { self.height }

    fn aspect_ratio_fixed(&self) -> bool { self.fix_aspect_ratio }

    fn pixel_dims(&self, _renderer: &R) -> (u16, u16) {
        (self.graphics_size[0], self.graphics_size[1])
    }

    fn draw(
        &self,
        renderer: &mut R,
        defaults: &R::Defaults,
        layout: Layout<'_>,
        cursor_position: Point,
    ) -> R::Output {
        Self::draw(self, renderer, defaults, layout, cursor_position)
    }
}
