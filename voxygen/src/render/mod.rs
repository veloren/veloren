pub mod consts;
pub mod mesh;
pub mod model;
pub mod pipelines;
pub mod renderer;
pub mod texture;
mod util;

// Reexports
pub use self::{
    consts::Consts,
    mesh::{Mesh, Tri, Quad},
    model::Model,
    texture::Texture,
    renderer::{Renderer, TgtColorFmt, TgtDepthFmt},
    pipelines::{
        Globals,
        figure::{
            FigurePipeline,
            Locals as FigureLocals,
            BoneData as FigureBoneData,
        },
        skybox::{
            create_mesh as create_skybox_mesh,
            SkyboxPipeline,
            Locals as SkyboxLocals,
        },
        terrain::{
            TerrainPipeline,
            Locals as TerrainLocals,
        },
        ui::{
            create_quad_mesh as create_ui_quad_mesh,
            UiPipeline,
            Locals as UiLocals,
        },
    },
};

#[cfg(feature = "gl")]
use gfx_device_gl as gfx_backend;

// Library
use gfx;

/// Used to represent one of many possible errors that may be omitted by the rendering subsystem
#[derive(Debug)]
pub enum RenderError {
    PipelineError(gfx::PipelineStateError<String>),
    UpdateError(gfx::UpdateError<usize>),
    CombinedError(gfx::CombinedError),
}

/// Used to represent a specific rendering configuration.
///
/// Note that pipelines are tied to the
/// rendering backend, and as such it is necessary to modify the rendering subsystem when adding
/// new pipelines - custom pipelines are not currently an objective of the rendering subsystem.
///
/// # Examples
///
/// - `SkyboxPipeline`
/// - `FigurePipeline`
pub trait Pipeline {
    type Vertex:
        Clone +
        gfx::traits::Pod +
        gfx::pso::buffer::Structure<gfx::format::Format>;
}
