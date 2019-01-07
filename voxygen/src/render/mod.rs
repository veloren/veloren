mod mesh;
mod model;
mod renderer;
mod shader_set;

// Reexports
pub use self::{
    mesh::Mesh,
    model::Model,
    shader_set::ShaderSet,
    renderer::Renderer,
};

// Library
use rendy;

#[cfg(not(any(feature = "dx12", feature = "metal", feature = "vulkan")))]
type Backend = rendy::empty::Backend;

#[cfg(feature = "dx12")]
type Backend = rendy::dx12::Backend;

#[cfg(feature = "metal")]
type Backend = rendy::metal::Backend;

#[cfg(feature = "vulkan")]
type Backend = rendy::vulkan::Backend;

/// Used to represent one of many possible errors that may be omitted by the rendering code
#[derive(Debug)]
pub enum RenderErr {}

/// Used to represent a specific rendering configuration
pub trait Pipeline {
    type Vertex;
}
