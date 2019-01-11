// Library
use gfx::{
    self,
    traits::FactoryExt,
};

// Local
use super::{
    RenderError,
    gfx_backend,
};

/// A handle to a series of constants sitting on the GPU. This is used to hold information used in
/// the rendering process that does not change throughout a single render pass.
#[derive(Clone)]
pub struct Consts<T: Copy + gfx::traits::Pod> {
    pub buf: gfx::handle::Buffer<gfx_backend::Resources, T>,
}

impl<T: Copy + gfx::traits::Pod> Consts<T> {
    /// Create a new `Const<T>`
    pub fn new(factory: &mut gfx_backend::Factory) -> Self {
        Self {
            buf: factory.create_constant_buffer(1),
        }
    }

    /// Update the GPU-side value represented by this constant handle.
    pub fn update(
        &mut self,
        encoder: &mut gfx::Encoder<gfx_backend::Resources, gfx_backend::CommandBuffer>,
        val: T,
    ) -> Result<(), RenderError> {
        encoder.update_buffer(&self.buf, &[val], 0)
            .map_err(|err| RenderError::UpdateError(err))
    }
}
