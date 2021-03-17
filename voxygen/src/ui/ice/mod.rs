//    tooltip_manager: TooltipManager,
mod cache;
pub mod component;
mod renderer;
pub mod widget;

pub use cache::{load_font, Font, FontId, RawFont};
pub use graphic::{Id, Rotation};
pub use iced::Event;
pub use iced_winit::conversion::window_event;
pub use renderer::{style, IcedRenderer};

use super::{
    graphic::{self, Graphic},
    scale::{Scale, ScaleMode},
};
use crate::{
    render::{Renderer, UiDrawer},
    window::Window,
    Error,
};
use common::slowjob::SlowJobPool;
use common_base::span;
use iced::{mouse, Cache, Size, UserInterface};
use iced_winit::Clipboard;
use vek::*;

pub type Element<'a, M> = iced::Element<'a, M, IcedRenderer>;

pub struct IcedUi {
    renderer: IcedRenderer,
    cache: Option<Cache>,
    events: Vec<Event>,
    cursor_position: Vec2<f32>,
    // Scaling of the ui
    scale: Scale,
    scale_changed: bool,
}
impl IcedUi {
    pub fn new(
        window: &mut Window,
        default_font: Font,
        scale_mode: ScaleMode,
    ) -> Result<Self, Error> {
        let scale = Scale::new(window, scale_mode, 1.2);
        let renderer = window.renderer_mut();

        let scaled_resolution = scale.scaled_resolution().map(|e| e as f32);
        let physical_resolution = renderer.resolution();

        // TODO: examine how much mem fonts take up and reduce clones if significant
        Ok(Self {
            renderer: IcedRenderer::new(
                renderer,
                scaled_resolution,
                physical_resolution,
                default_font,
            )?,
            cache: Some(Cache::new()),
            events: Vec::new(),
            // TODO: handle None
            cursor_position: Vec2::zero(),
            scale,
            scale_changed: false,
        })
    }

    /// Add a new font that is referncable via the returned Id
    pub fn add_font(&mut self, font: RawFont) -> FontId { self.renderer.add_font(font) }

    /// Allows clearing out the fonts when switching languages
    pub fn clear_fonts(&mut self, default_font: Font) { self.renderer.clear_fonts(default_font); }

    /// Add a new graphic that is referencable via the returned Id
    pub fn add_graphic(&mut self, graphic: Graphic) -> graphic::Id {
        self.renderer.add_graphic(graphic)
    }

    pub fn replace_graphic(&mut self, id: graphic::Id, graphic: Graphic) {
        self.renderer.replace_graphic(id, graphic);
    }

    pub fn scale(&self) -> Scale { self.scale }

    pub fn set_scaling_mode(&mut self, mode: ScaleMode) {
        // Signal that change needs to be handled
        self.scale_changed |= self.scale.set_scaling_mode(mode);
    }

    /// Dpi factor changed
    /// Not to be confused with scaling mode
    pub fn scale_factor_changed(&mut self, scale_factor: f64) {
        self.scale_changed |= self.scale.scale_factor_changed(scale_factor);
    }

    pub fn handle_event(&mut self, event: Event) {
        use iced::window;
        match event {
            // Intercept resizing events
            // TODO: examine if we are handling dpi properly here
            // ideally these values should be the logical ones
            Event::Window(window::Event::Resized { width, height }) => {
                if width != 0 && height != 0 {
                    let new_dims = Vec2::new(width, height);
                    // TODO maybe use u32 in Scale to be consistent with iced
                    // Avoid resetting cache if window size didn't change
                    self.scale_changed |= self.scale.window_resized(new_dims.map(|e| e as f64));
                }
            },
            // Scale cursor movement events
            // Note: in some cases the scaling could be off if a resized event occured in the same
            // frame, in practice this shouldn't be an issue
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                // TODO: return f32 here
                let scale = self.scale.scale_factor_logical() as f32;
                let x = position.x / scale;
                let y = position.y / scale;
                // TODO: determine why iced moved cursor position out of the `Cache` and if we
                // may need to handle this in a different way to address
                // whatever issue iced was trying to address
                self.cursor_position = Vec2::new(x, y);
                self.events.push(Event::Mouse(mouse::Event::CursorMoved {
                    position: iced::Point::new(x, y),
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
        pool: Option<&SlowJobPool>,
        clipboard: Option<&Clipboard>,
    ) -> (Vec<M>, mouse::Interaction) {
        span!(_guard, "maintain", "IcedUi::maintain");
        // Handle window resizing, dpi factor change, and scale mode changing
        if self.scale_changed {
            self.scale_changed = false;
            let scaled_resolution = self.scale.scaled_resolution().map(|e| e as f32);
            self.events
                .push(Event::Window(iced::window::Event::Resized {
                    width: scaled_resolution.x as u32,
                    height: scaled_resolution.y as u32,
                }));
            // Avoid panic in graphic cache when minimizing.
            // Somewhat inefficient for elements that won't change size after a window
            // resize
            let physical_resolution = renderer.resolution();
            if physical_resolution.map(|e| e > 0).reduce_and() {
                self.renderer
                    .resize(scaled_resolution, physical_resolution, renderer);
            }
        }

        let cursor_position = iced::Point {
            x: self.cursor_position.x,
            y: self.cursor_position.y,
        };

        // TODO: convert to f32 at source
        let window_size = self.scale.scaled_resolution().map(|e| e as f32);

        span!(guard, "build user_interface");
        let mut user_interface = UserInterface::build(
            root,
            Size::new(window_size.x, window_size.y),
            self.cache.take().unwrap(),
            &mut self.renderer,
        );
        drop(guard);

        let messages = {
            span!(_guard, "update user_interface");
            let mut messages = Vec::new();
            #[allow(clippy::manual_map)]
            let _event_status_list = user_interface.update(
                &self.events,
                cursor_position,
                match clipboard {
                    Some(c) => Some(c),
                    None => None,
                },
                &self.renderer,
                &mut messages,
            );
            messages
        };
        // Clear events
        self.events.clear();

        span!(guard, "draw user_interface");
        let (primitive, mouse_interaction) =
            user_interface.draw(&mut self.renderer, cursor_position);
        drop(guard);

        self.cache = Some(user_interface.into_cache());

        self.renderer.draw(primitive, renderer, pool);

        (messages, mouse_interaction)
    }

    pub fn render<'a>(&'a self, drawer: &mut UiDrawer<'_, 'a>) { self.renderer.render(drawer); }
}
