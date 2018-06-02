use gfx;
use gfx::{Device, Encoder, handle::RenderTargetView, handle::DepthStencilView};
use gfx_device_gl;

pub type ColorFormat = gfx::format::Srgba8;
pub type DepthFormat = gfx::format::DepthStencil;

pub type ColorView = RenderTargetView<gfx_device_gl::Resources, ColorFormat>;
pub type DepthView = DepthStencilView<gfx_device_gl::Resources, DepthFormat>;

pub struct Renderer {
    device: gfx_device_gl::Device,
    color_view: ColorView,
    depth_view: DepthView,
    factory: gfx_device_gl::Factory,
    encoder: Encoder<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer>,
}

impl Renderer {
    pub fn new(device: gfx_device_gl::Device, mut factory: gfx_device_gl::Factory, color_view: ColorView, depth_view: DepthView) -> Renderer {
        Renderer {
            device,
            color_view,
            depth_view,
            encoder: factory.create_command_buffer().into(),
            factory,
        }
    }

    pub fn begin_frame(&mut self) {
        self.encoder.clear(&self.color_view, [0.3, 0.3, 0.6, 1.0]);
        self.encoder.clear_depth(&self.depth_view, 1.0);
        self.encoder.flush(&mut self.device);
    }

    pub fn end_frame(&mut self) {
        self.device.cleanup();
    }
}
