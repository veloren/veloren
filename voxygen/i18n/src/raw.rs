use crate::{
    assets::{loader, StringLoader},
    Fonts, LanguageMetadata,
};
use serde::{Deserialize, Serialize};

/// Localization metadata from manifest file
/// See `Language` for more info on each attributes
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub(crate) struct Manifest {
    /// Whether to convert the input text encoded in UTF-8
    /// into a ASCII version by using the `deunicode` crate.
    pub(crate) convert_utf8_to_ascii: bool,
    pub(crate) fonts: Fonts,
    pub(crate) metadata: LanguageMetadata,
}

impl crate::assets::Asset for Manifest {
    type Loader = crate::assets::RonLoader;

    const EXTENSION: &'static str = "ron";
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

impl From<String> for Resource {
    fn from(src: String) -> Self { Self { src } }
}

impl crate::assets::Asset for Resource {
    type Loader = loader::LoadFrom<String, StringLoader>;

    const EXTENSION: &'static str = "ftl";
}
