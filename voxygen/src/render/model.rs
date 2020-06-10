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
    pub vertex_range: Range<u32>,
}

impl<P: Pipeline> Model<P> {
    pub fn new(factory: &mut gfx_backend::Factory, mesh: &Mesh<P>) -> Self {
        Self {
            vbuf: factory.create_vertex_buffer(mesh.vertices()),
            vertex_range: 0..mesh.vertices().len() as u32,
        }
    }

    pub fn vertex_range(&self) -> Range<u32> { self.vertex_range.clone() }
}

/// Represents a mesh on the GPU which can be updated dynamically.
pub struct DynamicModel<P: Pipeline> {
    pub vbuf: gfx::handle::Buffer<gfx_backend::Resources, P::Vertex>,
}

impl<P: Pipeline> DynamicModel<P> {
    #[allow(clippy::redundant_closure)] // TODO: Pending review in #587
    pub fn new(factory: &mut gfx_backend::Factory, size: usize) -> Result<Self, RenderError> {
        Ok(Self {
            vbuf: factory
                .create_buffer(size, Role::Vertex, Usage::Dynamic, Bind::empty())
                .map_err(|err| RenderError::BufferCreationError(err))?,
        })
    }

    /// Create a model with a slice of a portion of this model to send to the
    /// renderer.
    pub fn submodel(&self, range: Range<usize>) -> Model<P> {
        Model {
            vbuf: self.vbuf.clone(),
            vertex_range: range.start as u32..range.end as u32,
        }
    }

    #[allow(clippy::redundant_closure)] // TODO: Pending review in #587
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
