//    tooltip_manager: TooltipManager,
mod cache;
mod clipboard;
mod renderer;
mod widget;
mod winit_conversion;

pub use cache::Font;
pub use graphic::{Id, Rotation};
pub use iced::Event;
pub use renderer::IcedRenderer;
pub use widget::{
    background_container::{BackgroundContainer, Padding},
    compound_graphic,
    image::Image,
};
pub use winit_conversion::window_event;

use super::{
    graphic::{self, Graphic},
    scale::{Scale, ScaleMode},
};
use crate::{render::Renderer, window::Window, Error};
use clipboard::Clipboard;
use iced::{mouse, Cache, Size, UserInterface};
use vek::*;

pub type Element<'a, M> = iced::Element<'a, M, IcedRenderer>;

#[derive(Clone, Copy, Default)]
pub struct FontId(glyph_brush::FontId);

pub struct IcedUi {
    renderer: IcedRenderer,
    cache: Option<Cache>,
    events: Vec<Event>,
    clipboard: Clipboard,
    // Scaling of the ui
    scale: Scale,
    window_resized: Option<Vec2<u32>>,
}
impl IcedUi {
    pub fn new(window: &mut Window, default_font: Font) -> Result<Self, Error> {
        let scale = Scale::new(window, ScaleMode::Absolute(1.0));
        let renderer = window.renderer_mut();

        let scaled_dims = scale.scaled_window_size().map(|e| e as f32);

        // TODO: examine how much mem fonts take up and reduce clones if significant
        Ok(Self {
            renderer: IcedRenderer::new(renderer, scaled_dims, default_font)?,
            cache: Some(Cache::new()),
            events: Vec::new(),
            // TODO: handle None
            clipboard: Clipboard::new(window.window()).unwrap(),
            scale,
            window_resized: None,
        })
    }

    // Add an new graphic that is referencable via the returned Id
    pub fn add_graphic(&mut self, graphic: Graphic) -> graphic::Id {
        self.renderer.add_graphic(graphic)
    }

    pub fn handle_event(&mut self, event: Event) {
        use iced::window;
        match event {
            // Intercept resizing events
            Event::Window(window::Event::Resized { width, height }) => {
                self.window_resized = Some(Vec2::new(width, height));
            },
            // Scale cursor movement events
            // Note: in some cases the scaling could be off if a resized event occured in the same
            // frame, in practice this shouldn't be an issue
            Event::Mouse(mouse::Event::CursorMoved { x, y }) => {
                // TODO: return f32 here
                let scale = self.scale.scale_factor_logical() as f32;
                self.events.push(Event::Mouse(mouse::Event::CursorMoved {
                    x: x * scale,
                    y: y * scale,
                }));
            },
            // Scale pixel scrolling events
            Event::Mouse(mouse::Event::WheelScrolled {
                delta: mouse::ScrollDelta::Pixels { x, y },
            }) => {
                // TODO: return f32 here
                let scale = self.scale.scale_factor_logical() as f32;
                self.events.push(Event::Mouse(mouse::Event::WheelScrolled {
                    delta: mouse::ScrollDelta::Pixels {
                        x: x * scale,
                        y: y * scale,
                    },
                }));
            },
            event => self.events.push(event),
        }
    }

    // TODO: produce root internally???
    // TODO: closure/trait for sending messages back? (take a look at higher level
    // iced libs)
    pub fn maintain<'a, M, E: Into<Element<'a, M>>>(
        &mut self,
        root: E,
        renderer: &mut Renderer,
    ) -> (Vec<M>, mouse::Interaction) {
        // Handle window resizing
        if let Some(new_dims) = self.window_resized.take() {
            let old_scaled_dims = self.scale.scaled_window_size();
            // TODO maybe use u32 in Scale to be consistent with iced
            self.scale
                .window_resized(new_dims.map(|e| e as f64), renderer);
            let scaled_dims = self.scale.scaled_window_size();

            self.events
                .push(Event::Window(iced::window::Event::Resized {
                    width: scaled_dims.x as u32,
                    height: scaled_dims.y as u32,
                }));

            // Avoid panic in graphic cache when minimizing.
            // Avoid resetting cache if window size didn't change
            // Somewhat inefficient for elements that won't change size after a window
            // resize
            let res = renderer.get_resolution();
            if res.x > 0 && res.y > 0 && scaled_dims != old_scaled_dims {
                self.renderer
                    .resize(scaled_dims.map(|e| e as f32), renderer);
            }
        }

        // TODO: convert to f32 at source
        let window_size = self.scale.scaled_window_size().map(|e| e as f32);

        let mut user_interface = UserInterface::build(
            root,
            Size::new(window_size.x, window_size.y),
            self.cache.take().unwrap(),
            &mut self.renderer,
        );

        let messages = user_interface.update(
            self.events.drain(..),
            Some(&self.clipboard),
            &mut self.renderer,
        );

        let (primitive, mouse_interaction) = user_interface.draw(&mut self.renderer);

        self.cache = Some(user_interface.into_cache());

        self.renderer.draw(primitive, renderer);

        (messages, mouse_interaction)
    }

    pub fn render(&self, renderer: &mut Renderer) { self.renderer.render(renderer, None); }
}
