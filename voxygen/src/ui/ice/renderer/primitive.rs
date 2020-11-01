use crate::ui::{graphic, ice::widget::image};

#[derive(Debug)]
pub enum Primitive {
    // Allocation :(
    Group {
        primitives: Vec<Primitive>,
    },
    Image {
        handle: (image::Handle, graphic::Rotation),
        bounds: iced::Rectangle,
        color: vek::Rgba<u8>,
        source_rect: Option<vek::Aabr<f32>>,
    },
    // A vertical gradient
    // TODO: could be combined with rectangle
    Gradient {
        bounds: iced::Rectangle,
        top_linear_color: vek::Rgba<f32>,
        bottom_linear_color: vek::Rgba<f32>,
    },
    Rectangle {
        bounds: iced::Rectangle,
        linear_color: vek::Rgba<f32>,
    },
    Text {
        glyphs: Vec<glyph_brush::SectionGlyph>,
        bounds: iced::Rectangle,
        linear_color: vek::Rgba<f32>,
    },
    Clip {
        bounds: iced::Rectangle,
        offset: vek::Vec2<u32>,
        content: Box<Primitive>,
    },
    // Make content translucent
    Opacity {
        alpha: f32,
        content: Box<Primitive>,
    },
    Nothing,
}
