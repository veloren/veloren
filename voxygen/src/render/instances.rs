use super::buffer::DynamicBuffer;
use bytemuck::Pod;

/// Represents a mesh that has been sent to the GPU.
pub struct Instances<T: Copy + Pod> {
    buf: DynamicBuffer<T>,
}

impl<T: Copy + Pod> Instances<T> {
    pub fn new(device: &wgpu::Device, len: usize) -> Self {
        Self {
            // TODO: examine if we have Intances that are not updated (e.g. sprites) and if there
            // would be any gains from separating those out
            buf: DynamicBuffer::new(device, len, wgpu::BufferUsage::VERTEX),
        }
    }

    // TODO: count vs len naming scheme??
    pub fn count(&self) -> usize { self.buf.len() }

    pub fn update(&mut self, queue: &wgpu::Queue, vals: &[T], offset: usize) {
        self.buf.update(queue, vals, offset)
    }

    pub fn buf(&self) -> &wgpu::Buffer { &self.buf.buf }
}
