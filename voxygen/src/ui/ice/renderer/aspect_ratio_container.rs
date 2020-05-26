use super::{
    super::widget::{aspect_ratio_container, image},
    IcedRenderer,
};
use iced::{Element, Layout, Point, Rectangle};

impl aspect_ratio_container::Renderer for IcedRenderer {
    //type Style
    type ImageHandle = image::Handle;

    fn dimensions(&self, handle: &Self::ImageHandle) -> (u32, u32) { self.image_dims(*handle) }

    fn draw<M>(
        &mut self,
        defaults: &Self::Defaults,
        _bounds: Rectangle,
        cursor_position: Point,
        //style: &Self::Style,
        content: &Element<'_, M, Self>,
        content_layout: Layout<'_>,
    ) -> Self::Output {
        // TODO: stlying to add a background image and such
        content.draw(self, defaults, content_layout, cursor_position)
    }
}
