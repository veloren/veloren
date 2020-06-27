use crate::ui::{graphic, ice::widget::image};

pub enum Primitive {
    // Allocation :(
    Group {
        primitives: Vec<Primitive>,
    },
    Image {
        handle: (image::Handle, graphic::Rotation),
        bounds: iced::Rectangle,
        color: vek::Rgba<u8>,
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
        //size: f32,
        bounds: iced::Rectangle,
        linear_color: vek::Rgba<f32>,
        /*font: iced::Font,
         *horizontal_alignment: iced::HorizontalAlignment,
         *vertical_alignment: iced::VerticalAlignment, */
    },
    Clip {
        bounds: iced::Rectangle,
        offset: vek::Vec2<u32>,
        content: Box<Primitive>,
    },
    Nothing,
}
