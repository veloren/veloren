use super::{buffer::Buffer, mesh::Mesh, RenderError, Vertex};
use std::ops::Range;

/// Represents a mesh that has been sent to the GPU.
pub struct SubModel<'a, V: Vertex> {
    pub vertex_range: Range<u32>,
    buf: &'a wgpu::Buffer,
    phantom_data: std::marker::PhantomData<V>,
}

impl<'a, V: Vertex> SubModel<'a, V> {
    pub fn buf(&self) -> &wgpu::Buffer { self.buf }
}

/// Represents a mesh that has been sent to the GPU.
pub struct Model<V: Vertex> {
    vbuf: Buffer<V>,
}

impl<V: Vertex> Model<V> {
    pub fn new(device: &wgpu::Device, mesh: &Mesh<V>) -> Self {
        Self {
            vbuf: Buffer::new_with_data(device, wgpu::BufferUsage::VERTEX, mesh.vertices()),
        }
    }

    /// Create a model with a slice of a portion of this model to send to the
    /// renderer.
    pub fn submodel(&self, vertex_range: Range<u32>) -> SubModel<V> {
        SubModel {
            vertex_range,
            buf: self.buf(),
            phantom_data: std::marker::PhantomData,
        }
    }

    pub fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        mesh: &Mesh<V>,
        offset: usize,
    ) -> Result<(), RenderError> {
        self.buf.update(device, queue, mesh.vertices(), offset)
    }

    pub fn buf(&self) -> &wgpu::Buffer { self.vbuf.buf }
}
