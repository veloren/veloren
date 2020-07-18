#[allow(clippy::single_component_path_imports)] // TODO: Pending review in #587
pub mod consts;
mod error;
pub mod instances;
pub mod mesh;
pub mod model;
pub mod pipelines;
pub mod renderer;
pub mod texture;

// Reexports
pub use self::{
    consts::Consts,
    error::RenderError,
    instances::Instances,
    mesh::{Mesh, Quad, Tri},
    model::{DynamicModel, Model},
    pipelines::{
        figure::{
            BoneData as FigureBoneData, BoneMeshes, FigureModel, FigurePipeline,
            Locals as FigureLocals,
        },
        fluid::FluidPipeline,
        lod_terrain::{Locals as LodTerrainLocals, LodTerrainPipeline},
        postprocess::{
            create_mesh as create_pp_mesh, Locals as PostProcessLocals, PostProcessPipeline,
        },
        shadow::{Locals as ShadowLocals, ShadowPipeline},
        skybox::{create_mesh as create_skybox_mesh, Locals as SkyboxLocals, SkyboxPipeline},
        sprite::{Instance as SpriteInstance, Locals as SpriteLocals, SpritePipeline},
        terrain::{Locals as TerrainLocals, TerrainPipeline},
        ui::{
            create_quad as create_ui_quad, create_tri as create_ui_tri, Locals as UiLocals,
            Mode as UiMode, UiPipeline,
        },
        Globals, Light, Shadow,
    },
    renderer::{
        ColLightFmt, ColLightInfo, LodAltFmt, LodColorFmt, LodTextureFmt, Renderer,
        ShadowDepthStencilFmt, TgtColorFmt, TgtDepthStencilFmt, WinColorFmt, WinDepthFmt,
    },
    texture::Texture,
};
pub use gfx::texture::{FilterMethod, WrapMode};

#[cfg(feature = "gl")]
use gfx_device_gl as gfx_backend;

/// Used to represent a specific rendering configuration.
///
/// Note that pipelines are tied to the
/// rendering backend, and as such it is necessary to modify the rendering
/// subsystem when adding new pipelines - custom pipelines are not currently an
/// objective of the rendering subsystem.
///
/// # Examples
///
/// - `SkyboxPipeline`
/// - `FigurePipeline`
pub trait Pipeline {
    type Vertex: Clone + gfx::traits::Pod + gfx::pso::buffer::Structure<gfx::format::Format>;
}

use serde_derive::{Deserialize, Serialize};
/// Anti-aliasing modes
#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum AaMode {
    None,
    /// Fast approximate antialiasing.
    ///
    /// This is a screen-space technique, and therefore
    Fxaa,
    /// Multisampling AA, up to 4 samples per pixel.
    ///
    /// NOTE: MSAA modes don't (currently) work with greedy meshing, and will
    /// also struggle in the futrue with deferred shading, so they may be
    /// removed in the future.
    MsaaX4,
    /// Multisampling AA, up to 8 samples per pixel.
    ///
    /// NOTE: MSAA modes don't (currently) work with greedy meshing, and will
    /// also struggle in the futrue with deferred shading, so they may be
    /// removed in the future.
    MsaaX8,
    /// Multisampling AA, up to 16 samples per pixel.
    ///
    /// NOTE: MSAA modes don't (currently) work with greedy meshing, and will
    /// also struggle in the futrue with deferred shading, so they may be
    /// removed in the future.
    MsaaX16,
    /// Super-sampling antialiasing, 4 samples per pixel.
    ///
    /// Unlike MSAA, SSAA *always* performs 4 samples per pixel, rather than
    /// trying to choose importance samples at boundary regions, so it works
    /// much better with techniques like deferred rendering and greedy
    /// meshing that (without significantly more work) invalidate the
    /// GPU's assumptions about importance sampling.
    SsaaX4,
}

impl Default for AaMode {
    fn default() -> Self { AaMode::Fxaa }
}

/// Cloud modes
#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum CloudMode {
    /// No clouds.  On computers that can't handle loops well, have performance
    /// issues in fragment shaders in general, or just have large
    /// resolutions, this can be a *very* impactful performance difference.
    /// Part of that is because of inefficiencies in how we implement
    /// regular clouds.  It is still not all that cheap on low-end machines, due
    /// to many calculations being performed that use relatively expensive
    /// functions, and at some point I'd like to both optimize the regular
    /// sky shader further and create an even cheaper option.
    None,
    /// Volumetric clouds.  This option can be *very* expensive on low-end
    /// machines, to the point of making the game unusable, for several
    /// reasons:
    ///
    /// - The volumetric clouds use raymarching, which will cause catastrophic
    ///   performance degradation on GPUs without good support for loops.  There
    ///   is an attempt to minimize the impact of this using a z-range check,
    ///   but on some low-end GPUs (such as some integraetd graphics cards) this
    ///   test doesn't appear to be able to be predicted well at shader
    ///   invocation time.
    /// - The cloud computations themselves are fairly involved, further
    ///   degrading performance.
    /// - Although the sky shader is always drawn at the outer edges of the
    ///   skybox, the clouds themselves are supposed to be positioned much
    ///   lower, which means the depth check for the skybox incorrectly cuts off
    ///   clouds in some places.  To compensate for these cases (e.g. where
    ///   terrain is occluded by clouds from above, and the camera is above the
    ///   clouds), we currently branch to see if we need to render the clouds in
    ///   *every* fragment shader.  For machines that can't optimize the check,
    ///   this is absurdly expensive, so we should look at alternatives in the
    ///   future that player better iwth the GPU.
    Regular,
}

impl Default for CloudMode {
    fn default() -> Self { CloudMode::Regular }
}

/// Fluid modes
#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum FluidMode {
    /// "Cheap" water.  This water implements no waves, no reflections, no
    /// diffraction, and no light attenuation through water.  As a result,
    /// it can be much cheaper than shiny reflection.
    Cheap,
    /// "Shiny" water.  This water implements waves on the surfaces, some
    /// attempt at reflections, and tries to compute accurate light
    /// attenuation through water (this is what results in the
    /// colors changing as you descend into deep water).
    ///
    /// Unfortunately, the way the engine is currently set up, calculating
    /// accurate attenuation is a bit difficult; we use estimates from
    /// horizon maps for the current water altitude, which can both be off
    /// by up to (max_altitude / 255) meters, only has per-chunk horizontal
    /// resolution, and cannot handle edge cases like horizontal water (e.g.
    /// waterfalls) well.  We are okay with the latter, and will try to fix
    /// the former soon.
    ///
    /// Another issue is that we don't always know whether light is *blocked*,
    /// which causes attenuation to be computed incorrectly; this can be
    /// addressed by using shadow maps (at least for terrain).
    Shiny,
}

impl Default for FluidMode {
    fn default() -> Self { FluidMode::Shiny }
}

/// Lighting modes
#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum LightingMode {
    /// Ashikhmin-Shirley BRDF lighting model.  Attempts to generate a
    /// physically plausible (to some extent) lighting distribution.
    ///
    /// This mdoel may not work as well with purely directional lighting, and is
    /// more expensive than the otehr models.
    Ashikhmin,
    /// Standard Blinn-Phong shading, combing Lambertian diffuse reflections and
    /// specular highlights.
    BlinnPhong,
    /// Standard Lambertian lighting model, with only diffuse reflections.  The
    /// cheapest lighting model by a decent margin, but the performance
    /// dfifference between it and Blinn-Phong will probably only be
    /// significant on low-end machines that are bottlenecked on fragment
    /// shading.
    Lambertian,
}

impl Default for LightingMode {
    fn default() -> Self { LightingMode::BlinnPhong }
}

/// Shadow map settings.
#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ShadowMapMode {
    /// Multiple of default resolution (default is currenttly the closest higher
    /// power of two above the length of the longest diagonal of the screen
    /// resolution, but this may change).
    pub resolution: f32,
}

impl Default for ShadowMapMode {
    fn default() -> Self { Self { resolution: 1.0 } }
}

/// Shadow modes
#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ShadowMode {
    /// No shadows at all.  By far the cheapest option.
    None,
    /// Point shadows (draw circles under figures, up to a configured maximum;
    /// also render LOD shadows using horizon maps).  Can be expensive on
    /// some machines, probably mostly due to horizon mapping; the point
    /// shadows are not rendered too efficiently, but that can probably
    /// be addressed later.
    Cheap,
    /// Shadow map (render the scene from each light source, and also renders
    /// LOD shadows using horizon maps).
    Map(ShadowMapMode),
}

impl Default for ShadowMode {
    fn default() -> Self { ShadowMode::Cheap }
}

impl core::convert::TryFrom<ShadowMode> for ShadowMapMode {
    type Error = ();

    /// Get the shadow map details if they exist.
    fn try_from(value: ShadowMode) -> Result<Self, Self::Error> {
        if let ShadowMode::Map(map) = value {
            Ok(map)
        } else {
            Err(())
        }
    }
}

impl ShadowMode {
    pub fn is_map(&self) -> bool { if let Self::Map(_) = self { true } else { false } }
}

/// Render modes
#[derive(PartialEq, Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct RenderMode {
    #[serde(default)]
    pub aa: AaMode,
    #[serde(default)]
    pub cloud: CloudMode,
    #[serde(default)]
    pub fluid: FluidMode,
    #[serde(default)]
    pub lighting: LightingMode,
    #[serde(default)]
    pub shadow: ShadowMode,
}
