use super::{gfx_backend, mesh::Mesh, Pipeline, RenderError};
use gfx::{
    buffer::Role,
    memory::{Bind, Usage},
    traits::FactoryExt,
    Factory,
};
use std::ops::Range;

/// Represents a mesh that has been sent to the GPU.
pub struct Model<P: Pipeline> {
    pub vbuf: gfx::handle::Buffer<gfx_backend::Resources, P::Vertex>,
    pub slice: gfx::Slice<gfx_backend::Resources>,
}

impl<P: Pipeline> Model<P> {
    pub fn new(factory: &mut gfx_backend::Factory, mesh: &Mesh<P>) -> Self {
        Self {
            vbuf: factory.create_vertex_buffer(mesh.vertices()),
            slice: gfx::Slice {
                start: 0,
                end: mesh.vertices().len() as u32,
                base_vertex: 0,
                instances: None,
                buffer: gfx::IndexBuffer::Auto,
            },
        }
    }
}

/// Represents a mesh on the GPU which can be updated dynamically.
pub struct DynamicModel<P: Pipeline> {
    pub vbuf: gfx::handle::Buffer<gfx_backend::Resources, P::Vertex>,
}

impl<P: Pipeline> DynamicModel<P> {
    pub fn new(factory: &mut gfx_backend::Factory, size: usize) -> Result<Self, RenderError> {
        Ok(Self {
            vbuf: factory
                .create_buffer(size, Role::Vertex, Usage::Dynamic, Bind::empty())
                .map_err(|err| RenderError::BufferCreationError(err))?,
        })
    }

    /// Create a model with a slice of a portion of this model to send to the renderer.
    pub fn submodel(&self, range: Range<usize>) -> Model<P> {
        Model {
            vbuf: self.vbuf.clone(),
            slice: gfx::Slice {
                start: range.start as u32,
                end: range.end as u32,
                base_vertex: 0,
                instances: None,
                buffer: gfx::IndexBuffer::Auto,
            },
        }
    }

    pub fn update(
        &self,
        encoder: &mut gfx::Encoder<gfx_backend::Resources, gfx_backend::CommandBuffer>,
        mesh: &Mesh<P>,
        offset: usize,
    ) -> Result<(), RenderError> {
        encoder
            .update_buffer(&self.vbuf, mesh.vertices(), offset)
            .map_err(|err| RenderError::UpdateError(err))
    }
}
