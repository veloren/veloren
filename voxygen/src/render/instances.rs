use super::buffer::Buffer;
use bytemuck::Pod;

/// Represents a mesh that has been sent to the GPU.
pub struct Instances<T: Copy + Pod> {
    buf: Buffer<T>,
}

impl<T: Copy + Pod> Instances<T> {
    pub fn new(device: &wgpu::Device, len: u64) -> Self {
        Self {
            buf: Buffer::new(device, len, wgpu::BufferUsage::VERTEX),
        }
    }

    pub fn count(&self) -> usize { self.buf.count() }

    pub fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, vals: &[T], offset: u64) {
        self.buf.update(device, queue, vals, offset)
    }

    pub fn buf(&self) -> &wgpu::Buffer { &self.buf.buf }
}
