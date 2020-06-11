use super::{gfx_backend, RenderError};
use gfx::{
    self,
    buffer::Role,
    memory::{Bind, Usage},
    Factory,
};

/// Represents a mesh that has been sent to the GPU.
pub struct Instances<T: Copy + gfx::traits::Pod> {
    pub ibuf: gfx::handle::Buffer<gfx_backend::Resources, T>,
}

impl<T: Copy + gfx::traits::Pod> Instances<T> {
    #[allow(clippy::redundant_closure)] // TODO: Pending review in #587
    pub fn new(factory: &mut gfx_backend::Factory, len: usize) -> Result<Self, RenderError> {
        Ok(Self {
            ibuf: factory
                .create_buffer(len, Role::Vertex, Usage::Dynamic, Bind::TRANSFER_DST)
                .map_err(|err| RenderError::BufferCreationError(err))?,
        })
    }

    pub fn count(&self) -> usize { self.ibuf.len() }

    #[allow(clippy::redundant_closure)] // TODO: Pending review in #587
    pub fn update(
        &mut self,
        encoder: &mut gfx::Encoder<gfx_backend::Resources, gfx_backend::CommandBuffer>,
        instances: &[T],
    ) -> Result<(), RenderError> {
        encoder
            .update_buffer(&self.ibuf, instances, 0)
            .map_err(|err| RenderError::UpdateError(err))
    }
}
