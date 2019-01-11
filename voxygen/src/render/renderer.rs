// Library
use vek::*;
use gfx::{
    self,
    traits::Device,
};

// Crate
use crate::VoxygenErr;

// Local
use super::{
    model::Model,
    mesh::Mesh,
    shader_set::ShaderSet,
    Pipeline,
    RenderErr,
    gfx_backend,
};

pub type TgtColorFmt = gfx::format::Srgba8;
pub type TgtDepthFmt = gfx::format::DepthStencil;

pub type TgtColorView = gfx::handle::RenderTargetView<gfx_backend::Resources, TgtColorFmt>;
pub type TgtDepthView = gfx::handle::DepthStencilView<gfx_backend::Resources, TgtDepthFmt>;

pub struct Renderer {
    device: gfx_backend::Device,
    encoder: gfx::Encoder<gfx_backend::Resources, gfx_backend::CommandBuffer>,
    factory: gfx_backend::Factory,

    tgt_color_view: TgtColorView,
    tgt_depth_view: TgtDepthView,
}

impl Renderer {
    pub fn new(
        device: gfx_backend::Device,
        mut factory: gfx_backend::Factory,
        tgt_color_view: TgtColorView,
        tgt_depth_view: TgtDepthView,
    ) -> Result<Self, VoxygenErr> {
        Ok(Self {
            device,
            encoder: factory.create_command_buffer().into(),
            factory,
            tgt_color_view,
            tgt_depth_view,
        })
    }

    pub fn clear(&mut self, col: Rgba<f32>) {
        self.encoder.clear(&self.tgt_color_view, col.into_array());
        self.encoder.clear_depth(&self.tgt_depth_view, 1.0);
    }

    pub fn flush(&mut self) {
        self.encoder.flush(&mut self.device);
        self.device.cleanup();
    }
}
