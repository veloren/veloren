mod mesh;
mod model;
mod renderer;
mod pipelines;
mod shader_set;

// Reexports
pub use self::{
    mesh::Mesh,
    model::Model,
    shader_set::ShaderSet,
    renderer::{Renderer, TgtColorFmt, TgtDepthFmt},
};

#[cfg(feature = "gl")]
use gfx_device_gl as gfx_backend;

/// Used to represent one of many possible errors that may be omitted by the rendering code
#[derive(Debug)]
pub enum RenderErr {}

/// Used to represent a specific rendering configuration
pub trait Pipeline {
    type Vertex;
}
