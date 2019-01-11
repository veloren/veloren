mod consts;
mod mesh;
mod model;
mod pipelines;
mod renderer;

// Reexports
pub use self::{
    consts::Consts,
    mesh::{Mesh, Quad},
    model::Model,
    renderer::{Renderer, TgtColorFmt, TgtDepthFmt},
    pipelines::{
        character::CharacterPipeline,
        skybox::SkyboxPipeline,
    },
};

#[cfg(feature = "gl")]
use gfx_device_gl as gfx_backend;

// Library
use gfx;

/// Used to represent one of many possible errors that may be omitted by the rendering code
#[derive(Debug)]
pub enum RenderErr {
    PipelineErr(gfx::PipelineStateError<String>),
    UpdateErr(gfx::UpdateError<usize>),
}

/// Used to represent a specific rendering configuration
pub trait Pipeline {
    type Vertex:
        Clone +
        gfx::traits::Pod +
        gfx::pso::buffer::Structure<gfx::format::Format>;
}
