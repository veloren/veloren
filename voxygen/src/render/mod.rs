pub mod bound;
mod buffer;
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
    bound::Bound,
    consts::Consts,
    error::RenderError,
    instances::Instances,
    mesh::{Mesh, Quad, Tri},
    model::{DynamicModel, Model, SubModel},
    pipelines::{
        clouds::Locals as CloudsLocals,
        figure::{
            BoneData as FigureBoneData, BoneMeshes, FigureLayout, FigureModel,
            Locals as FigureLocals,
        },
        fluid::{BindGroup as FluidWaves, Vertex as FluidVertex},
        lod_terrain::{LodData, Vertex as LodTerrainVertex},
        particle::{Instance as ParticleInstance, Vertex as ParticleVertex},
        postprocess::Locals as PostProcessLocals,
        shadow::Locals as ShadowLocals,
        skybox::{create_mesh as create_skybox_mesh, Vertex as SkyboxVertex},
        sprite::{Instance as SpriteInstance, Locals as SpriteLocals, Vertex as SpriteVertex},
        terrain::{Locals as TerrainLocals, TerrainLayout, Vertex as TerrainVertex},
        ui::{
            create_quad as create_ui_quad,
            create_quad_vert_gradient as create_ui_quad_vert_gradient, create_tri as create_ui_tri,
            BoundLocals as UiBoundLocals, Locals as UiLocals, Mode as UiMode,
            TextureBindGroup as UiTextureBindGroup, Vertex as UiVertex,
        },
        GlobalModel, Globals, GlobalsBindGroup, GlobalsLayouts, Light, Shadow,
    },
    renderer::{
        drawer::{
            Drawer, FirstPassDrawer, ParticleDrawer, PreparedUiDrawer, SecondPassDrawer,
            ThirdPassDrawer, UiDrawer,
        },
        ColLightInfo, Renderer,
    },
    texture::Texture,
};
pub use wgpu::{AddressMode, FilterMode};

pub trait Vertex = Clone + bytemuck::Pod;

use serde::{Deserialize, Serialize};
/// Anti-aliasing modes
#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum AaMode {
    /// Fast approximate antialiasing.
    ///
    /// This is a screen-space technique, and therefore works fine with greedy
    /// meshing.
    Fxaa,
    /// Multisampling AA, up to 4 samples per pixel.
    ///
    /// NOTE: MSAA modes don't (currently) work with greedy meshing, and will
    /// also struggle in the future with deferred shading, so they may be
    /// removed in the future.
    MsaaX4,
    /// Multisampling AA, up to 8 samples per pixel.
    ///
    /// NOTE: MSAA modes don't (currently) work with greedy meshing, and will
    /// also struggle in the future with deferred shading, so they may be
    /// removed in the future.
    MsaaX8,
    /// Multisampling AA, up to 16 samples per pixel.
    ///
    /// NOTE: MSAA modes don't (currently) work with greedy meshing, and will
    /// also struggle in the future with deferred shading, so they may be
    /// removed in the future.
    MsaaX16,
    #[serde(other)]
    None,
}

impl Default for AaMode {
    fn default() -> Self { AaMode::Fxaa }
}

/// Cloud modes
#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum CloudMode {
    /// No clouds. As cheap as it gets.
    None,
    /// Clouds, but barely. Ideally, any machine should be able to handle this
    /// just fine.
    Minimal,
    /// Enough visual detail to be pleasing, but generally using poor-but-cheap
    /// approximations to derive parameters
    Low,
    /// More detail. Enough to look good in most cases. For those that value
    /// looks but also high framerates.
    Medium,
    /// High, but with extra compute power thrown at it to smooth out subtle
    /// imperfections
    Ultra,
    /// Lots of detail with good-but-costly derivation of parameters.
    #[serde(other)]
    High,
}

impl Default for CloudMode {
    fn default() -> Self { CloudMode::High }
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
    #[serde(other)]
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
    /// This model may not work as well with purely directional lighting, and is
    /// more expensive than the other models.
    Ashikhmin,
    /// Standard Lambertian lighting model, with only diffuse reflections.  The
    /// cheapest lighting model by a decent margin, but the performance
    /// difference between it and Blinn-Phong will probably only be
    /// significant on low-end machines that are bottlenecked on fragment
    /// shading.
    Lambertian,
    /// Standard Blinn-Phong shading, combing Lambertian diffuse reflections and
    /// specular highlights.
    #[serde(other)]
    BlinnPhong,
}

impl Default for LightingMode {
    fn default() -> Self { LightingMode::BlinnPhong }
}

/// Shadow map settings.
#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ShadowMapMode {
    /// Multiple of default resolution (default, which is 1.0, is currently
    /// the closest higher power of two above the length of the longest
    /// diagonal of the screen resolution, but this may change).
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
    /// Shadow map (render the scene from each light source, and also renders
    /// LOD shadows using horizon maps).
    Map(ShadowMapMode),
    /// Point shadows (draw circles under figures, up to a configured maximum;
    /// also render LOD shadows using horizon maps).  Can be expensive on
    /// some machines, probably mostly due to horizon mapping; the point
    /// shadows are not rendered too efficiently, but that can probably
    /// be addressed later.
    #[serde(other)] // Would normally be on `Map`, but only allowed on unit variants
    Cheap,
}

impl Default for ShadowMode {
    fn default() -> Self { ShadowMode::Map(Default::default()) }
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
    pub fn is_map(&self) -> bool { matches!(self, Self::Map(_)) }
}

/// Upscale mode settings.
#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct UpscaleMode {
    // Determines non-UI graphics upscaling. 0.25 to 2.0.
    pub factor: f32,
}

impl Default for UpscaleMode {
    fn default() -> Self { Self { factor: 1.0 } }
}

/// Render modes
#[derive(PartialEq, Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RenderMode {
    pub aa: AaMode,
    pub cloud: CloudMode,
    pub fluid: FluidMode,
    pub lighting: LightingMode,
    pub shadow: ShadowMode,
    pub upscale_mode: UpscaleMode,
}
