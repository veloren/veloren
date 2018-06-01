use gfx;
use gfx_window_glutin;
use glutin;

use glutin::{EventsLoop, GlWindow, WindowBuilder, ContextBuilder, GlContext, GlRequest};
use glutin::Api::OpenGl;

pub type ColorFormat = gfx::format::Srgba8;
pub type DepthFormat = gfx::format::DepthStencil;

pub struct RenderWindow {
    running: bool,
    gl_window: GlWindow,
    events_loop: EventsLoop,
}

impl RenderWindow {
    pub fn new() -> RenderWindow {
        let mut events_loop = EventsLoop::new();
        let win_builder = WindowBuilder::new()
            .with_title("Verloren (Voxygen)")
            .with_dimensions(800, 500)
            .with_maximized(false);

        let ctx_builder = ContextBuilder::new()
            .with_gl(GlRequest::Specific(OpenGl, (3, 2)))
            .with_vsync(true);

        let (gl_window, mut device, mut factory, color_view, mut depth_view) =
            gfx_window_glutin::init::<ColorFormat, DepthFormat>(win_builder, ctx_builder, &events_loop);

        RenderWindow {
            running: true,
            gl_window,
            events_loop,
        }
    }

    pub fn handle_events(&mut self) -> bool {
        let mut keep_open = true;
        self.events_loop.poll_events(|event| {
            if let glutin::Event::WindowEvent { event, .. } = event {
                match event {
                    glutin::WindowEvent::CloseRequested => keep_open = false,
                    _ => {},
                }
            }
        });

        keep_open
    }
}
