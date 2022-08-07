use crate::{Fonts, LanguageMetadata};
use serde::{Deserialize, Serialize};

use std::str::FromStr;

/// Localization metadata from manifest file
/// See `Language` for more info on each attributes
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub(crate) struct Manifest {
    pub(crate) convert_utf8_to_ascii: bool,
    pub(crate) fonts: Fonts,
    pub(crate) metadata: LanguageMetadata,
}

impl crate::assets::Asset for Manifest {
    type Loader = crate::assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

#[derive(Clone)]
pub(crate) struct Resource {
    pub(crate) src: String,
}

impl FromStr for Resource {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> { Ok(Self { src: s.to_owned() }) }
}

impl crate::assets::Asset for Resource {
    type Loader = crate::assets::loader::ParseLoader;

    const EXTENSION: &'static str = "ftl";
}
