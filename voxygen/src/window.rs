// External
use gfx_window_glutin;
use glutin::{
    Api::OpenGl,
    dpi::LogicalSize,
    ContextBuilder,
    EventsLoop,
    GlContext,
    GlRequest,
    GlWindow,
    WindowBuilder,
};

// Crate
use crate::render_ctx::RenderCtx;

pub struct Window {
    events_loop: EventsLoop,
    gl_window: GlWindow,
    render_ctx: RenderCtx,
}


impl Window {
    pub fn new() -> Window {
        let events_loop = EventsLoop::new();

        let (
            gl_window,
            device,
            factory,
            tgt_color_view,
            tgt_depth_view,
        ) = gfx_window_glutin::init(
            WindowBuilder::new()
                .with_title("Veloren (Voxygen)")
                .with_dimensions(LogicalSize::new(800.0, 500.0))
                .with_maximized(false),
            ContextBuilder::new()
                .with_gl(GlRequest::Specific(OpenGl, (3, 2)))
                .with_multisampling(2)
                .with_vsync(true),
            &events_loop,
        );

        Self {
            events_loop,
            gl_window,
            render_ctx: RenderCtx::new(
                device,
                factory,
                tgt_color_view,
                tgt_depth_view,
            ),
        }
    }

    pub fn render_ctx(&self) -> &RenderCtx { &self.render_ctx }
    pub fn render_ctx_mut(&mut self) -> &mut RenderCtx { &mut self.render_ctx }

    pub fn poll_events<F: FnMut(Event)>(&mut self, mut f: F) {
        self.events_loop.poll_events(|event| match event {
            glutin::Event::WindowEvent { event, .. } => match event {
                glutin::WindowEvent::CloseRequested => f(Event::Close),
                _ => {},
            },
            _ => {},
        });
    }

    pub fn swap_buffers(&self) {
        self.gl_window
            .swap_buffers()
            .expect("Failed to swap window buffers");
    }
}

pub enum Event {
    Close,
}
