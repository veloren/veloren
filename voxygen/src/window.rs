use gfx_window_glutin;
use glutin;

use glutin::{EventsLoop, WindowBuilder, ContextBuilder, GlContext, GlRequest, GlWindow, WindowEvent};
use glutin::Api::OpenGl;

use renderer::{Renderer, ColorFormat, DepthFormat};

pub enum Event {
    CloseRequest,
    CursorMoved { dx: f64, dy: f64 },
}

pub struct RenderWindow {
    events_loop: EventsLoop,
    gl_window: GlWindow,
    renderer: Renderer,
    last_cursor_pos: (f64, f64),
}

impl RenderWindow {
    pub fn new() -> RenderWindow {
        let events_loop = EventsLoop::new();
        let win_builder = WindowBuilder::new()
            .with_title("Verloren (Voxygen)")
            .with_dimensions(800, 500)
            .with_maximized(false);

        let ctx_builder = ContextBuilder::new()
            .with_gl(GlRequest::Specific(OpenGl, (3, 2)))
            .with_vsync(true);

        let (gl_window, device, factory, color_view, depth_view) =
            gfx_window_glutin::init::<ColorFormat, DepthFormat>(win_builder, ctx_builder, &events_loop);

        RenderWindow {
            events_loop,
            gl_window,
            renderer: Renderer::new(device, factory, color_view, depth_view),
            last_cursor_pos: (0.0, 0.0),
        }
    }

    pub fn renderer_mut<'a>(&'a mut self) -> &'a mut Renderer {
        &mut self.renderer
    }

    pub fn handle_events<'a, F: FnMut(Event)>(&mut self, mut func: F) {
        let mut last_cursor_pos = self.last_cursor_pos; // We do this because Rust doesn't have intelligent mutable reference checking yet

        self.events_loop.poll_events(|event| {
            if let glutin::Event::WindowEvent { event, .. } = event {
                match event {
                    WindowEvent::CursorMoved { position, .. } => {
                        func(Event::CursorMoved {
                            dx: position.0 - last_cursor_pos.0,
                            dy: position.1 - last_cursor_pos.1
                        });
                        last_cursor_pos = position; // We calculate the difference in the cursor position between frames
                    },
                    WindowEvent::CloseRequested => func(Event::CloseRequest),
                    _ => {},
                }
            }
        });

        self.last_cursor_pos = last_cursor_pos;
    }

    pub fn swap_buffers(&mut self) {
        self.gl_window.swap_buffers().expect("Failed to swap window buffers");
    }
}
