use super::{buffer::Buffer, RenderError};
use zerocopy::AsBytes;

/// A handle to a series of constants sitting on the GPU. This is used to hold
/// information used in the rendering process that does not change throughout a
/// single render pass.
#[derive(Clone)]
pub struct Consts<T: Copy + AsBytes> {
    buf: Buffer<T>,
}

impl<T: Copy + AsBytes> Consts<T> {
    /// Create a new `Const<T>`.
    pub fn new(device: &mut wgpu::Device, len: usize) -> Self {
        Self {
            buf: Buffer::new(device, len, wgpu::BufferUsage::UNIFORM),
        }
    }

    /// Update the GPU-side value represented by this constant handle.
    pub fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vals: &[T],
        offset: usize,
    ) -> Result<(), RenderError> {
        self.buf.update(device, queue, vals, offset)
    }

    pub fn buf(&self) -> &wgpu::Buffer { self.buf.buf }
}
