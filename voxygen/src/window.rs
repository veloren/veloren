use gfx;
use gfx_window_glutin;
use glutin;

use gfx::{handle::RenderTargetView, handle::DepthStencilView};
use glutin::{EventsLoop, WindowBuilder, ContextBuilder, GlContext, GlRequest, GlWindow};
use gfx_device_gl;
use glutin::Api::OpenGl;

pub type ColorFormat = gfx::format::Srgba8;
pub type DepthFormat = gfx::format::DepthStencil;

pub type ColorView = RenderTargetView<gfx_device_gl::Resources, ColorFormat>;
pub type DepthView = DepthStencilView<gfx_device_gl::Resources, DepthFormat>;

pub struct RenderWindow {
    events_loop: EventsLoop,
    gl_window: GlWindow,
}

impl RenderWindow {
    pub fn new() -> (RenderWindow, gfx_device_gl::Device, gfx_device_gl::Factory, ColorView, DepthView) {
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

        (RenderWindow { events_loop, gl_window }, device, factory, color_view, depth_view)
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

    pub fn swap_buffers(&mut self) {
        self.gl_window.swap_buffers().expect("Failed to swap window buffers");
    }
}
