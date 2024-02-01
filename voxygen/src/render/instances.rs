use super::buffer::DynamicBuffer;
use bytemuck::Pod;
use std::ops::Range;

/// Represents a set of instances that has been sent to the GPU.
pub struct SubInstances<'a, T: Copy + Pod> {
    pub inst_range: Range<u32>,
    buf: &'a wgpu::Buffer,
    phantom_data: std::marker::PhantomData<T>,
}

impl<'a, T: Copy + Pod> SubInstances<'a, T> {
    pub(super) fn buf(&self) -> wgpu::BufferSlice<'a> {
        let start = self.inst_range.start as wgpu::BufferAddress
            * std::mem::size_of::<T>() as wgpu::BufferAddress;
        let end = self.inst_range.end as wgpu::BufferAddress
            * std::mem::size_of::<T>() as wgpu::BufferAddress;
        self.buf.slice(start..end)
    }

    pub fn count(&self) -> u32 { self.inst_range.end - self.inst_range.start }
}

/// Represents a mesh that has been sent to the GPU.
pub struct Instances<T: Copy + Pod> {
    buf: DynamicBuffer<T>,
}

impl<T: Copy + Pod> Instances<T> {
    pub fn new(device: &wgpu::Device, len: usize) -> Self {
        Self {
            // TODO: examine if we have Instances that are not updated (e.g. sprites) and if there
            // would be any gains from separating those out
            buf: DynamicBuffer::new(device, len, wgpu::BufferUsages::VERTEX),
        }
    }

    /// Create a set of instances with a slice of a portion of these instances
    /// to send to the renderer.
    pub fn subinstances(&self, inst_range: Range<u32>) -> SubInstances<T> {
        SubInstances {
            inst_range,
            buf: self.buf(),
            phantom_data: std::marker::PhantomData,
        }
    }

    // TODO: count vs len naming scheme??
    pub fn count(&self) -> usize { self.buf.len() }

    pub fn update(&mut self, queue: &wgpu::Queue, vals: &[T], offset: usize) {
        self.buf.update(queue, vals, offset)
    }

    pub fn buf(&self) -> &wgpu::Buffer { &self.buf.buf }
}
