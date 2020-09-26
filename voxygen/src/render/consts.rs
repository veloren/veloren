use super::buffer::Buffer;
use bytemuck::Pod;

/// A handle to a series of constants sitting on the GPU. This is used to hold
/// information used in the rendering process that does not change throughout a
/// single render pass.
pub struct Consts<T: Copy + Pod> {
    buf: Buffer<T>,
}

impl<T: Copy + Pod> Consts<T> {
    /// Create a new `Const<T>`.
    pub fn new(device: &wgpu::Device, len: u64) -> Self {
        Self {
            buf: Buffer::new(device, len, wgpu::BufferUsage::UNIFORM),
        }
    }

    /// Update the GPU-side value represented by this constant handle.
    pub fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, vals: &[T], offset: u64) {
        self.buf.update(device, queue, vals, offset)
    }

    pub fn buf(&self) -> &wgpu::Buffer { &self.buf.buf }
}
