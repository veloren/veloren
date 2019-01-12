// Library
use glutin;
use gfx_window_glutin;
use vek::*;

// Crate
use crate::{
    Error,
    render::{
        Renderer,
        TgtColorFmt,
        TgtDepthFmt,
    },
};

pub struct Window {
    events_loop: glutin::EventsLoop,
    window: glutin::GlWindow,
    renderer: Renderer,
}


impl Window {
    pub fn new() -> Result<Window, Error> {
        let events_loop = glutin::EventsLoop::new();

        let win_builder = glutin::WindowBuilder::new()
            .with_title("Veloren (Voxygen)")
            .with_dimensions(glutin::dpi::LogicalSize::new(800.0, 500.0))
            .with_maximized(false);

        let ctx_builder = glutin::ContextBuilder::new()
            .with_gl(glutin::GlRequest::Specific(glutin::Api::OpenGl, (3, 2)))
            .with_vsync(true);

        let (
            window,
            device,
            factory,
            tgt_color_view,
            tgt_depth_view,
        ) = gfx_window_glutin::init::<TgtColorFmt, TgtDepthFmt>(
            win_builder,
            ctx_builder,
            &events_loop,
        ).map_err(|err| Error::BackendError(Box::new(err)))?;

        let tmp = Ok(Self {
            events_loop,
            window,
            renderer: Renderer::new(
                device,
                factory,
                tgt_color_view,
                tgt_depth_view,
            )?,
        });
        tmp
    }

    pub fn renderer(&self) -> &Renderer { &self.renderer }
    pub fn renderer_mut(&mut self) -> &mut Renderer { &mut self.renderer }

    pub fn fetch_events(&mut self) -> Vec<Event> {
        let mut events = vec![];
        self.events_loop.poll_events(|event| match event {
            glutin::Event::WindowEvent { event, .. } => match event {
                glutin::WindowEvent::CloseRequested => events.push(Event::Close),
                glutin::WindowEvent::ReceivedCharacter(c) => events.push(Event::Char(c)),
                _ => {},
            },
            glutin::Event::DeviceEvent { event, .. } => match event {
                glutin::DeviceEvent::MouseMotion { delta: (dx, dy), .. } =>
                    events.push(Event::CursorPan(Vec2::new(dx as f32, dy as f32))),
                _ => {},
            },
            _ => {},
        });
        events
    }

    pub fn swap_buffers(&self) -> Result<(), Error> {
        self.window.swap_buffers()
            .map_err(|err| Error::BackendError(Box::new(err)))
    }

    pub fn trap_cursor(&mut self) {
        self.window.hide_cursor(true);
        self.window.grab_cursor(true)
            .expect("Failed to grab cursor");
    }

    pub fn untrap_cursor(&mut self) {
        self.window.hide_cursor(false);
        self.window.grab_cursor(false)
            .expect("Failed to ungrab cursor");
    }
}

/// Represents an incoming event from the window
pub enum Event {
    /// The window has been requested to close.
    Close,
    /// A key has been typed that corresponds to a specific character.
    Char(char),
    /// The cursor has been panned across the screen while trapped.
    CursorPan(Vec2<f32>),
}
