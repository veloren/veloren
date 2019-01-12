pub mod consts;
pub mod mesh;
pub mod model;
pub mod pipelines;
pub mod renderer;
mod util;

// Reexports
pub use self::{
    consts::Consts,
    mesh::{Mesh, Quad},
    model::Model,
    renderer::{Renderer, TgtColorFmt, TgtDepthFmt},
    pipelines::{
        Globals,
        character::{
            CharacterPipeline,
            Locals as CharacterLocals,
        },
        skybox::{
            create_mesh as create_skybox_mesh,
            SkyboxPipeline,
            Locals as SkyboxLocals,
        },
    },
};

#[cfg(feature = "gl")]
use gfx_device_gl as gfx_backend;

// Library
use gfx;

/// Used to represent one of many possible errors that may be omitted by the rendering code
#[derive(Debug)]
pub enum RenderError {
    PipelineError(gfx::PipelineStateError<String>),
    UpdateError(gfx::UpdateError<usize>),
}

/// Used to represent a specific rendering configuration
pub trait Pipeline {
    type Vertex:
        Clone +
        gfx::traits::Pod +
        gfx::pso::buffer::Structure<gfx::format::Format>;
}
