//    tooltip_manager: TooltipManager,
mod clipboard;
mod renderer;
mod widget;
mod winit_conversion;

pub use graphic::{Id, Rotation};
pub use iced::Event;
pub use renderer::IcedRenderer;
pub use widget::image::Image;
pub use winit_conversion::window_event;

use super::graphic::{self, Graphic};
use crate::{render::Renderer, window::Window, Error};
use clipboard::Clipboard;
use iced::{Cache, Element, MouseCursor, Size, UserInterface};

pub struct IcedUi {
    renderer: IcedRenderer,
    cache: Option<Cache>,
    events: Vec<Event>,
    clipboard: Clipboard,
}
impl IcedUi {
    pub fn new(window: &mut Window) -> Result<Self, Error> {
        Ok(Self {
            renderer: IcedRenderer::new(window)?,
            cache: Some(Cache::new()),
            events: Vec::new(),
            // TODO: handle None
            clipboard: Clipboard::new(window.window()).unwrap(),
        })
    }

    // Add an new graphic that is referencable via the returned Id
    pub fn add_graphic(&mut self, graphic: Graphic) -> graphic::Id {
        self.renderer.add_graphic(graphic)
    }

    // TODO: handle scaling here
    pub fn handle_event(&mut self, event: Event) { self.events.push(event); }

    // TODO: produce root internally???
    pub fn maintain<'a, M, E: Into<Element<'a, M, IcedRenderer>>>(
        &mut self,
        root: E,
        renderer: &mut Renderer,
    ) -> (Vec<M>, MouseCursor) {
        // TODO: convert to f32 at source
        let window_size = self.renderer.scaled_window_size().map(|e| e as f32);

        let mut user_interface = UserInterface::build(
            root,
            Size::new(window_size.x, window_size.y),
            self.cache.take().unwrap(),
            &mut self.renderer,
        );

        let messages =
            user_interface.update(self.events.drain(..), Some(&self.clipboard), &self.renderer);

        let (primitive, mouse_cursor) = user_interface.draw(&mut self.renderer);

        self.renderer.draw(primitive, renderer);

        self.cache = Some(user_interface.into_cache());

        (messages, mouse_cursor)
    }

    pub fn render(&self, renderer: &mut Renderer) { self.renderer.render(renderer, None); }
}
