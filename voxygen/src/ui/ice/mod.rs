//    tooltip_manager: TooltipManager,
mod cache;
pub mod component;
mod renderer;
pub mod widget;

pub use cache::{Font, FontId, RawFont};
pub use graphic::{Id, Rotation};
pub use iced::Event;
pub use iced_winit::conversion::window_event;
pub use renderer::{style, IcedRenderer};

use super::{
    graphic::{self, Graphic},
    scale::{Scale, ScaleMode},
};
use crate::{render::Renderer, window::Window, Error};
use iced::{mouse, Cache, Size, UserInterface};
use iced_winit::Clipboard;
use vek::*;

pub type Element<'a, M> = iced::Element<'a, M, IcedRenderer>;

pub struct IcedUi {
    renderer: IcedRenderer,
    cache: Option<Cache>,
    events: Vec<Event>,
    clipboard: Clipboard,
    cursor_position: Vec2<f32>,
    // Scaling of the ui
    scale: Scale,
    window_resized: Option<Vec2<u32>>,
    scale_mode_changed: bool,
}
impl IcedUi {
    pub fn new(
        window: &mut Window,
        default_font: Font,
        scale_mode: ScaleMode,
    ) -> Result<Self, Error> {
        let scale = Scale::new(window, scale_mode, 1.2);
        let renderer = window.renderer_mut();

        let scaled_dims = scale.scaled_window_size().map(|e| e as f32);

        // TODO: examine how much mem fonts take up and reduce clones if significant
        Ok(Self {
            renderer: IcedRenderer::new(renderer, scaled_dims, default_font)?,
            cache: Some(Cache::new()),
            events: Vec::new(),
            // TODO: handle None
            clipboard: Clipboard::new(window.window()).unwrap(),
            cursor_position: Vec2::zero(),
            scale,
            window_resized: None,
            scale_mode_changed: false,
        })
    }

    /// Add a new font that is referncable via the returned Id
    pub fn add_font(&mut self, font: RawFont) -> FontId { self.renderer.add_font(font) }

    /// Add a new graphic that is referencable via the returned Id
    pub fn add_graphic(&mut self, graphic: Graphic) -> graphic::Id {
        self.renderer.add_graphic(graphic)
    }

    pub fn scale(&self) -> Scale { self.scale }

    pub fn set_scaling_mode(&mut self, mode: ScaleMode) {
        self.scale.set_scaling_mode(mode);
        // Signal that change needs to be handled
        self.scale_mode_changed = true;
    }

    pub fn handle_event(&mut self, event: Event) {
        use iced::window;
        match event {
            // Intercept resizing events
            // TODO: examine if we are handling dpi properly here
            // ideally these values should be the logical ones
            Event::Window(window::Event::Resized { width, height }) => {
                if width != 0 && height != 0 {
                    self.window_resized = Some(Vec2::new(width, height));
                }
            },
            // Scale cursor movement events
            // Note: in some cases the scaling could be off if a resized event occured in the same
            // frame, in practice this shouldn't be an issue
            Event::Mouse(mouse::Event::CursorMoved { x, y }) => {
                // TODO: return f32 here
                let scale = self.scale.scale_factor_logical() as f32;
                // TODO: determine why iced moved cursor position out of the `Cache` and if we
                // may need to handle this in a different way to address
                // whatever issue iced was trying to address
                self.cursor_position = Vec2 {
                    x: x / scale,
                    y: y / scale,
                };
                self.events.push(Event::Mouse(mouse::Event::CursorMoved {
                    x: x / scale,
                    y: y / scale,
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
                        x: x / scale,
                        y: y / scale,
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
        // Handle window resizing and scale mode changing
        let scaled_dims = if let Some(new_dims) = self.window_resized.take() {
            let old_scaled_dims = self.scale.scaled_window_size();
            // TODO maybe use u32 in Scale to be consistent with iced
            self.scale
                .window_resized(new_dims.map(|e| e as f64), renderer);
            let scaled_dims = self.scale.scaled_window_size();

            // Avoid resetting cache if window size didn't change
            (scaled_dims != old_scaled_dims).then_some(scaled_dims)
        } else if self.scale_mode_changed {
            Some(self.scale.scaled_window_size())
        } else {
            None
        };
        if let Some(scaled_dims) = scaled_dims {
            self.scale_mode_changed = false;
            self.events
                .push(Event::Window(iced::window::Event::Resized {
                    width: scaled_dims.x as u32,
                    height: scaled_dims.y as u32,
                }));
            // Avoid panic in graphic cache when minimizing.
            // Somewhat inefficient for elements that won't change size after a window
            // resize
            let res = renderer.get_resolution();
            if res.x > 0 && res.y > 0 {
                self.renderer
                    .resize(scaled_dims.map(|e| e as f32), renderer);
            }
        }

        let cursor_position = iced::Point {
            x: self.cursor_position.x,
            y: self.cursor_position.y,
        };

        // TODO: convert to f32 at source
        let window_size = self.scale.scaled_window_size().map(|e| e as f32);

        let mut user_interface = UserInterface::build(
            root,
            Size::new(window_size.x, window_size.y),
            self.cache.take().unwrap(),
            &mut self.renderer,
        );

        let messages = user_interface.update(
            &self.events,
            cursor_position,
            Some(&self.clipboard),
            &mut self.renderer,
        );
        // Clear events
        self.events.clear();

        let (primitive, mouse_interaction) =
            user_interface.draw(&mut self.renderer, cursor_position);

        self.cache = Some(user_interface.into_cache());

        self.renderer.draw(primitive, renderer);

        (messages, mouse_interaction)
    }

    pub fn render(&self, renderer: &mut Renderer) { self.renderer.render(renderer, None); }
}
