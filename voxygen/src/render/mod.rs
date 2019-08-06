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
    mesh::{Mesh, Quad, Tri},
    model::{DynamicModel, Model},
    pipelines::{
        figure::{BoneData as FigureBoneData, FigurePipeline, Locals as FigureLocals},
        postprocess::{
            create_mesh as create_pp_mesh, Locals as PostProcessLocals, PostProcessPipeline,
        },
        skybox::{create_mesh as create_skybox_mesh, Locals as SkyboxLocals, SkyboxPipeline},
        terrain::{Locals as TerrainLocals, TerrainPipeline},
        ui::{
            create_quad as create_ui_quad, create_tri as create_ui_tri, Locals as UiLocals,
            Mode as UiMode, UiPipeline,
        },
        fluid::{Locals as FluidLocals, FluidPipeline},
        Globals, Light,
    },
    renderer::{Renderer, TgtColorFmt, TgtDepthFmt, WinColorFmt, WinDepthFmt},
    texture::Texture,
};

#[cfg(feature = "gl")]
use gfx_device_gl as gfx_backend;

use gfx;

/// Used to represent one of many possible errors that may be omitted by the rendering subsystem.
#[derive(Debug)]
pub enum RenderError {
    PipelineError(gfx::PipelineStateError<String>),
    UpdateError(gfx::UpdateError<usize>),
    TexUpdateError(gfx::UpdateError<[u16; 3]>),
    CombinedError(gfx::CombinedError),
    BufferCreationError(gfx::buffer::CreationError),
    IncludeError(glsl_include::Error),
    MappingError(gfx::mapping::Error),
    CopyError(gfx::CopyError<[u16; 3], usize>),
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
    type Vertex: Clone + gfx::traits::Pod + gfx::pso::buffer::Structure<gfx::format::Format>;
}
