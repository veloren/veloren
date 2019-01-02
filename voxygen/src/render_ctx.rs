// Library
use gfx_device_gl::{Device, Resources, Factory, CommandBuffer};
use gfx::{
    handle::{RenderTargetView, DepthStencilView},
    Device as DeviceTrait,
    Encoder,
};
use vek::*;

type TgtColorView = RenderTargetView<Resources, gfx::format::Srgba8>;
type TgtDepthView = DepthStencilView<Resources, gfx::format::DepthStencil>;

pub struct RenderCtx {
    device: Device,
    encoder: Encoder<Resources, CommandBuffer>,
    factory: Factory,
    tgt_color_view: TgtColorView,
    tgt_depth_view: TgtDepthView,
}

impl RenderCtx {
    pub fn new(
        device: Device,
        mut factory: Factory,
        tgt_color_view: TgtColorView,
        tgt_depth_view: TgtDepthView,
    ) -> Self {
        Self {
            device,
            encoder: Encoder::from(factory.create_command_buffer()),
            factory,
            tgt_color_view,
            tgt_depth_view,
        }
    }

    pub fn clear(&mut self, col: Rgba<f32>) {
        self.encoder.clear(&self.tgt_color_view, col.into_array());
    }

    pub fn flush_and_cleanup(&mut self) {
        self.encoder.flush(&mut self.device);
        self.device.cleanup();
    }
}
