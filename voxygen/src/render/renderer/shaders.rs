use common::assets::{
    self, Asset, AssetCache, AssetExt, AssetHandle, BoxedError, FileAsset, SharedString,
};
use hashbrown::HashMap;
use std::borrow::Cow;

/// Load from a GLSL file.
pub struct Glsl(pub String);

impl FileAsset for Glsl {
    const EXTENSION: &'static str = "glsl";

    fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, BoxedError> {
        Ok(String::from_utf8(bytes.into()).map(Self)?)
    }
}

// Note: we use this clone to send the shaders to a background thread
// TODO: use Arc-ed asset and clone that instead
#[derive(Clone)]
pub struct Shaders {
    shaders: HashMap<String, AssetHandle<Glsl>>,
}

impl Asset for Shaders {
    // TODO: Taking the specifier argument as a base for shaders specifiers
    // would allow to use several shaders groups easily
    fn load(_: &AssetCache, _: &SharedString) -> Result<Self, BoxedError> {
        let shaders = [
            "include.constants",
            "include.globals",
            "include.sky",
            "include.light",
            "include.srgb",
            "include.random",
            "include.lod",
            "include.shadows",
            "include.rain_occlusion",
            "include.point_glow",
            "include.fxaa",
            "antialias.none",
            "antialias.bilinear",
            "antialias.fxaa",
            "antialias.msaa-x4",
            "antialias.msaa-x8",
            "antialias.msaa-x16",
            "antialias.hqx",
            "antialias.fxupscale",
            "include.cloud.none",
            "include.cloud.regular",
            "figure-vert",
            "light-shadows-figure-vert",
            "light-shadows-directed-vert",
            "light-shadows-debug-vert",
            "rain-occlusion-figure-vert",
            "rain-occlusion-directed-vert",
            "point-light-shadows-vert",
            "skybox-vert",
            "skybox-frag",
            "debug-vert",
            "debug-frag",
            "figure-frag",
            "rope-vert",
            "rope-frag",
            "terrain-vert",
            "terrain-frag",
            "fluid-vert",
            "fluid-frag.cheap",
            "fluid-frag.shiny",
            "sprite-vert",
            "sprite-frag",
            "lod-object-vert",
            "lod-object-frag",
            "particle-vert",
            "particle-frag",
            "trail-vert",
            "trail-frag",
            "ui-vert",
            "ui-frag",
            "premultiply-alpha-vert",
            "premultiply-alpha-frag",
            "lod-terrain-vert",
            "lod-terrain-frag",
            "clouds-vert",
            "clouds-frag",
            "dual-downsample-filtered-frag",
            "dual-downsample-frag",
            "dual-upsample-frag",
            "clouds-frag",
            "postprocess-vert",
            "postprocess-frag",
            "blit-vert",
            "blit-frag",
            //"player-shadow-frag",
            //"light-shadows-geom",
        ];

        let shaders = shaders
            .iter()
            .map(|shader| {
                let full_specifier = ["voxygen.shaders.", shader].concat();
                let asset = AssetExt::load(&full_specifier)?;
                Ok((String::from(*shader), asset))
            })
            .collect::<Result<HashMap<_, _>, assets::Error>>()?;

        Ok(Self { shaders })
    }
}

impl Shaders {
    pub fn get(&self, shader: &str) -> Option<impl core::ops::Deref<Target = Glsl> + use<>> {
        self.shaders.get(shader).map(|a| a.read())
    }
}
