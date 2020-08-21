use super::{buffer::Buffer, RenderError};
use zerocopy::AsBytes;

/// Represents a mesh that has been sent to the GPU.
#[derive(Clone)]
pub struct Instances<T: Copy + AsBytes> {
    buf: Buffer<T>,
}

impl<T: Copy + AsBytes> Instances<T> {
    pub fn new(device: &mut wgpu::Device, len: usize) -> Self {
        Self {
            buf: Buffer::new(device, len, wgpu::BufferUsage::VERTEX),
        }
    }

    pub fn count(&self) -> usize { self.buf.count() }

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
