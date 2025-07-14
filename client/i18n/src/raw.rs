use crate::{Fonts, LanguageMetadata, assets};
use serde::{Deserialize, Serialize};

/// Localization metadata from manifest file
/// See `Language` for more info on each attributes
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub(crate) struct Manifest {
    pub(crate) fonts: Fonts,
    pub(crate) metadata: LanguageMetadata,
}

impl assets::FileAsset for Manifest {
    const EXTENSION: &'static str = "ron";

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Result<Self, assets::BoxedError> {
        assets::load_ron(&bytes)
    }
}

// Newtype wrapper representing fluent resource.
//
// NOTE:
// We store String, that later converted to FluentResource.
// We can't do it at load time, because we might want to do utf8 to ascii
// conversion and we know it only after we've loaded language manifest.
//
// Alternative solution is to make it hold Rc/Arc around FluentResource,
// implement methods that give us mutable control around resource entries,
// but doing it to eliminate Clone that happens N per programm life seems as
// overengineering.
//
// N is time of fluent files, so about 20 for English and the same for target
// localisation.
#[derive(Clone)]
pub(crate) struct Resource {
    pub(crate) src: String,
}

impl assets::FileAsset for Resource {
    const EXTENSION: &'static str = "ftl";

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Result<Self, assets::BoxedError> {
        Ok(Resource {
            src: String::from_bytes(bytes)?,
        })
    }
}
