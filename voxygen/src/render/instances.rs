use super::{buffer::Buffer, RenderError};
use bytemuck::Pod;

/// Represents a mesh that has been sent to the GPU.
#[derive(Clone)]
pub struct Instances<T: Copy + Pod> {
    buf: Buffer<T>,
}

impl<T: Copy + Pod> Instances<T> {
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
