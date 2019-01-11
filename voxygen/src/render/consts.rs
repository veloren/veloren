// Library
use gfx::{
    self,
    traits::FactoryExt,
};

// Local
use super::{
    RenderErr,
    gfx_backend,
};

#[derive(Clone)]
pub struct Consts<T: Copy + gfx::traits::Pod> {
    pub buf: gfx::handle::Buffer<gfx_backend::Resources, T>,
}

impl<T: Copy + gfx::traits::Pod> Consts<T> {
    pub fn new(factory: &mut gfx_backend::Factory) -> Self {
        Self {
            buf: factory.create_constant_buffer(1),
        }
    }

    pub fn update(
        &mut self,
        encoder: &mut gfx::Encoder<gfx_backend::Resources, gfx_backend::CommandBuffer>,
        data: T,
    ) -> Result<(), RenderErr> {
        encoder.update_buffer(&self.buf, &[data], 0)
            .map_err(|err| RenderErr::UpdateErr(err))
    }
}
