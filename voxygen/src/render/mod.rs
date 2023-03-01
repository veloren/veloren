pub mod bound;
mod buffer;
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
    buffer::Buffer,
    consts::Consts,
    error::RenderError,
    instances::Instances,
    mesh::{Mesh, Quad, Tri},
    model::{DynamicModel, Model, SubModel},
    pipelines::{
        clouds::Locals as CloudsLocals,
        debug::{DebugLayout, DebugPipeline, Locals as DebugLocals, Vertex as DebugVertex},
        figure::{
            BoneData as FigureBoneData, BoneMeshes, FigureLayout, FigureModel,
            Locals as FigureLocals,
        },
        fluid::Vertex as FluidVertex,
        lod_object::{Instance as LodObjectInstance, Vertex as LodObjectVertex},
        lod_terrain::{LodData, Vertex as LodTerrainVertex},
        particle::{Instance as ParticleInstance, Vertex as ParticleVertex},
        postprocess::Locals as PostProcessLocals,
        rain_occlusion::Locals as RainOcclusionLocals,
        shadow::{Locals as ShadowLocals, PointLightMatrix},
        skybox::{create_mesh as create_skybox_mesh, Vertex as SkyboxVertex},
        sprite::{
            Instance as SpriteInstance, SpriteGlobalsBindGroup, SpriteVerts,
            Vertex as SpriteVertex, VERT_PAGE_SIZE as SPRITE_VERT_PAGE_SIZE,
        },
        terrain::{Locals as TerrainLocals, TerrainLayout, Vertex as TerrainVertex},
        trail::Vertex as TrailVertex,
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
            DebugDrawer, DebugShadowDrawer, Drawer, FigureDrawer, FigureShadowDrawer,
            FirstPassDrawer, ParticleDrawer, PreparedUiDrawer, ShadowPassDrawer, SpriteDrawer,
            TerrainDrawer, TerrainShadowDrawer, ThirdPassDrawer, TrailDrawer,
            TransparentPassDrawer, UiDrawer, VolumetricPassDrawer,
        },
        AltIndices, ColLightInfo, CullingMode, Renderer,
    },
    texture::Texture,
};
use hashbrown::HashSet;
pub use wgpu::{AddressMode, FilterMode};

pub trait Vertex: Clone + bytemuck::Pod {
    const STRIDE: wgpu::BufferAddress;
    // Whether these types of verts use the quad index buffer for drawing them
    const QUADS_INDEX: Option<wgpu::IndexFormat>;
}

use serde::{Deserialize, Serialize};
/// Anti-aliasing modes
#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
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
    /// Fast edge-detecting upscaling.
    ///
    /// Screen-space technique that attempts to reconstruct lines and edges
    /// in the original image. Useless at internal resolutions higher than 1.0x,
    /// but potentially very effective at much lower internal resolutions.
    Hqx,
    /// Fast upscaling informed by FXAA.
    ///
    /// Screen-space technique that uses a combination of FXAA and
    /// nearest-neighbour sample retargeting to produce crisp, clean upscaling.
    FxUpscale,
    #[serde(other)]
    None,
}

impl AaMode {
    pub fn samples(&self) -> u32 {
        match self {
            AaMode::None | AaMode::Fxaa | AaMode::Hqx | AaMode::FxUpscale => 1,
            AaMode::MsaaX4 => 4,
            AaMode::MsaaX8 => 8,
            AaMode::MsaaX16 => 16,
        }
    }
}

impl Default for AaMode {
    fn default() -> Self { AaMode::Fxaa }
}

/// Cloud modes
#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
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

impl CloudMode {
    pub fn is_enabled(&self) -> bool { *self != CloudMode::None }
}

impl Default for CloudMode {
    fn default() -> Self { CloudMode::High }
}

/// Fluid modes
#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum FluidMode {
    /// "Low" water.  This water implements no waves, no reflections, no
    /// diffraction, and no light attenuation through water.  As a result,
    /// it can be much cheaper than shiny reflection.
    Low,
    High,
    /// This water implements waves on the surfaces, some attempt at
    /// reflections, and tries to compute accurate light attenuation through
    /// water (this is what results in the colors changing as you descend
    /// into deep water).
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
    Medium,
}

impl Default for FluidMode {
    fn default() -> Self { FluidMode::Medium }
}

/// Reflection modes
#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ReflectionMode {
    /// No or minimal reflections.
    Low,
    /// High quality reflections with screen-space raycasting and
    /// all the bells & whistles.
    High,
    // Medium quality screen-space reflections.
    #[serde(other)]
    Medium,
}

impl Default for ReflectionMode {
    fn default() -> Self { ReflectionMode::High }
}

/// Lighting modes
#[derive(PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
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

impl TryFrom<ShadowMode> for ShadowMapMode {
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

/// Present modes
/// See https://docs.rs/wgpu/0.7.0/wgpu/enum.PresentMode.html
#[derive(Default, PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum PresentMode {
    Mailbox,
    Immediate,
    #[default]
    #[serde(other)]
    Fifo, // has to be last for `#[serde(other)]`
}

impl From<PresentMode> for wgpu::PresentMode {
    fn from(mode: PresentMode) -> Self {
        match mode {
            PresentMode::Fifo => wgpu::PresentMode::Fifo,
            PresentMode::Mailbox => wgpu::PresentMode::Mailbox,
            PresentMode::Immediate => wgpu::PresentMode::Immediate,
        }
    }
}

/// Bloom factor
/// Controls fraction of output image luminosity that is blurred bloom
#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum BloomFactor {
    Low,
    High,
    /// Max valid value is 1.0
    Custom(f32),
    // other variant has to be placed last
    #[serde(other)]
    Medium,
}

impl Default for BloomFactor {
    fn default() -> Self { Self::Medium }
}

impl BloomFactor {
    /// Fraction of output image luminosity that is blurred bloom
    pub fn fraction(self) -> f32 {
        match self {
            Self::Low => 0.1,
            Self::Medium => 0.2,
            Self::High => 0.3,
            Self::Custom(val) => val.clamp(0.0, 1.0),
        }
    }
}

/// Bloom settings
#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct BloomConfig {
    /// Controls fraction of output image luminosity that is blurred bloom
    ///
    /// Defaults to `Medium`
    pub factor: BloomFactor,
    /// Turning this on make the bloom blur less sharply concentrated around the
    /// high intensity phenomena (removes adding in less blurred layers to the
    /// final blur)
    ///
    /// Defaults to `false`
    pub uniform_blur: bool,
    // TODO: allow configuring the blur radius and/or the number of passes
}

#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum BloomMode {
    On(BloomConfig),
    #[serde(other)]
    Off,
}

impl Default for BloomMode {
    fn default() -> Self {
        Self::On(BloomConfig {
            factor: BloomFactor::default(),
            uniform_blur: false,
        })
    }
}

impl BloomMode {
    fn is_on(&self) -> bool { matches!(self, BloomMode::On(_)) }
}

/// Render modes
#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct RenderMode {
    pub aa: AaMode,
    pub cloud: CloudMode,
    pub reflection: ReflectionMode,
    pub fluid: FluidMode,
    pub lighting: LightingMode,
    pub shadow: ShadowMode,
    pub rain_occlusion: ShadowMapMode,
    pub bloom: BloomMode,
    /// 0.0..1.0
    pub point_glow: f32,

    pub flashing_lights_enabled: bool,

    pub experimental_shaders: HashSet<ExperimentalShader>,

    pub upscale_mode: UpscaleMode,
    pub present_mode: PresentMode,
    pub profiler_enabled: bool,
}

impl Default for RenderMode {
    fn default() -> Self {
        Self {
            aa: AaMode::default(),
            cloud: CloudMode::default(),
            fluid: FluidMode::default(),
            reflection: ReflectionMode::default(),
            lighting: LightingMode::default(),
            shadow: ShadowMode::default(),
            rain_occlusion: ShadowMapMode::default(),
            bloom: BloomMode::default(),
            point_glow: 0.35,
            flashing_lights_enabled: true,
            experimental_shaders: HashSet::default(),
            upscale_mode: UpscaleMode::default(),
            present_mode: PresentMode::default(),
            profiler_enabled: false,
        }
    }
}

impl RenderMode {
    fn split(self) -> (PipelineModes, OtherModes) {
        (
            PipelineModes {
                aa: self.aa,
                cloud: self.cloud,
                fluid: self.fluid,
                reflection: self.reflection,
                lighting: self.lighting,
                shadow: self.shadow,
                rain_occlusion: self.rain_occlusion,
                bloom: self.bloom,
                point_glow: self.point_glow,
                flashing_lights_enabled: self.flashing_lights_enabled,
                experimental_shaders: self.experimental_shaders,
            },
            OtherModes {
                upscale_mode: self.upscale_mode,
                present_mode: self.present_mode,
                profiler_enabled: self.profiler_enabled,
            },
        )
    }
}

/// Render modes that require pipeline recreation (e.g. shader recompilation)
/// when changed
#[derive(PartialEq, Clone, Debug)]
pub struct PipelineModes {
    aa: AaMode,
    pub cloud: CloudMode,
    fluid: FluidMode,
    reflection: ReflectionMode,
    lighting: LightingMode,
    pub shadow: ShadowMode,
    pub rain_occlusion: ShadowMapMode,
    bloom: BloomMode,
    point_glow: f32,
    flashing_lights_enabled: bool,
    experimental_shaders: HashSet<ExperimentalShader>,
}

/// Other render modes that don't effect pipelines
#[derive(PartialEq, Clone, Debug)]
struct OtherModes {
    upscale_mode: UpscaleMode,
    present_mode: PresentMode,
    profiler_enabled: bool,
}

/// Experimental shader modes.
///
/// You can enable these using Voxygen's `settings.ron`. See
/// [here](https://book.veloren.net/players/voxygen.html#experimental-shaders) for more information.
#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    strum::EnumIter,
    strum::Display,
    strum::EnumString,
)]
pub enum ExperimentalShader {
    /// Add brick-like normal mapping to the world.
    Brickloren,
    /// Remove the default procedural noise from terrain.
    NoNoise,
    /// Add a sobel filter that draws lines in post-process by detecting edges
    /// inbetween colors. This does perform 8 times more texture samples in
    /// post-processing so there is potentially a significant performance
    /// impact especially with anti aliasing enabled.
    Sobel,
    /// Simulate a curved world.
    CurvedWorld,
    /// Adds extra detail to distant LoD (Level of Detail) terrain procedurally.
    ProceduralLodDetail,
    /// Add a warping effect when underwater.
    Underwarper,
    /// Remove caustics from underwater terrain when shiny water is enabled.
    NoCaustics,
    /// Don't dither color in post-processing.
    NoDither,
    /// Don't use the nonlinear srgb space for dithering color.
    NonSrgbDither,
    /// Use triangle PDF noise for dithering instead of uniform noise.
    TriangleNoiseDither,
    /// Removes as many effects (including lighting) as possible in the name of
    /// performance.
    BareMinimum,
    /// Lowers strength of the glow effect for lights near the camera.
    LowGlowNearCamera,
    /// Disable the fake voxel effect on LoD features.
    NoLodVoxels,
    /// Disable the 'pop-in' effect when loading terrain.
    NoTerrainPop,
    /// Display grid lines to visualize the distribution of shadow map texels
    /// for the directional light from the sun.
    DirectionalShadowMapTexelGrid,
    /// Disable rainbows
    NoRainbows,
    /// Add extra detailing to puddles.
    PuddleDetails,
    /// Show gbuffer surface normals.
    ViewNormals,
    /// Show gbuffer materials.
    ViewMaterials,
    /// Rather than fading out screen-space reflections at view space borders,
    /// smear screen space to cover the reflection vector.
    SmearReflections,
    /// Apply the point shadows from cheap shadows on top of shadow mapping.
    PointShadowsWithShadowMapping,
}
