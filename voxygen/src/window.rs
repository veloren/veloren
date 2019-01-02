// External
use gfx::handle::{RenderTargetView, DepthStencilView};
use gfx_device_gl::{Device, Resources, Factory};
use gfx_window_glutin;
use glutin::{
    self,
    Api::OpenGl,
    dpi::LogicalSize,
    ContextBuilder,
    EventsLoop,
    GlContext,
    GlRequest,
    GlWindow,
    WindowBuilder,
};

type TgtColorView = RenderTargetView<Resources, gfx::format::Srgba8>;
type TgtDepthView = DepthStencilView<Resources, gfx::format::DepthStencil>;

pub struct RenderCtx {
    device: Device,
    factory: Factory,
    tgt_color_view: TgtColorView,
    tgt_depth_view: TgtDepthView,
}

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
            render_ctx: RenderCtx {
                device,
                factory,
                tgt_color_view,
                tgt_depth_view,
            },
        }
    }

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
