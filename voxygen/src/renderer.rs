use gfx::{Device, Encoder};
use gfx_device_gl;

use window::{RenderWindow, ColorView, DepthView};

pub struct Renderer {
    window: RenderWindow,
    color_view: ColorView,
    depth_view: DepthView,
    device: gfx_device_gl::Device,
    factory: gfx_device_gl::Factory,
    encoder: Encoder<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer>,
}

impl Renderer {
    pub fn new() -> Renderer {
        let (window, device, mut factory, color_view, depth_view) = RenderWindow::new();
        Renderer {
            color_view,
            depth_view,
            encoder: factory.create_command_buffer().into(),
            device,
            factory,
            window,
        }
    }

    pub fn window<'a>(&'a mut self) -> &'a mut RenderWindow {
        &mut self.window
    }

    pub fn begin_frame(&mut self) {
        self.encoder.clear(&self.color_view, [0.3, 0.3, 0.6, 1.0]);
        self.encoder.clear_depth(&self.depth_view, 1.0);
        self.encoder.flush(&mut self.device);
    }

    pub fn end_frame(&mut self) {
        self.window.swap_buffers();
        self.device.cleanup();
    }
}
