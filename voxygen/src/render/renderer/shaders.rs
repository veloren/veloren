use common::assets::{self, AssetExt, AssetHandle};
use hashbrown::HashMap;

/// Load from a GLSL file.
pub struct Glsl(pub String);

impl From<String> for Glsl {
    fn from(s: String) -> Glsl { Glsl(s) }
}

impl assets::Asset for Glsl {
    type Loader = assets::LoadFrom<String, assets::StringLoader>;

    const EXTENSION: &'static str = "glsl";
}

// Note: we use this clone to send the shaders to a background thread
// TODO: use Arc-ed asset and clone that instead
#[derive(Clone)]
pub struct Shaders {
    shaders: HashMap<String, AssetHandle<Glsl>>,
}

impl assets::Compound for Shaders {
    // TODO: Taking the specifier argument as a base for shaders specifiers
    // would allow to use several shaders groups easily
    fn load(_: assets::AnyCache, _: &assets::SharedString) -> Result<Shaders, assets::BoxedError> {
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
    pub fn get(&self, shader: &str) -> Option<impl core::ops::Deref<Target = Glsl>> {
        self.shaders.get(shader).map(|a| a.read())
    }
}
